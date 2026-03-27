use std::collections::HashMap;

use xxhash_rust::xxh3::xxh3_64;

use crate::generator::types::TrainingSample;

const NUM_PERM: usize = 128;
const BAND_SIZE: usize = 8;
const NUM_BANDS: usize = NUM_PERM / BAND_SIZE; // 16 bands
const SHINGLE_SIZE: usize = 5; // 5-gram character shingles

/// MinHash LSH deduplicator.
/// Uses 128 hash permutations with 16 bands of 8 rows each.
/// At Jaccard threshold 0.85, the probability of a true duplicate being
/// caught is ~0.98 (very high recall).
pub struct Deduplicator {
    /// Hash function seeds: (a, b) pairs for h(x) = (a*x + b) mod p
    seeds: Vec<(u64, u64)>,
}

impl Deduplicator {
    pub fn new() -> Self {
        // Generate deterministic seeds
        let mut seeds = Vec::with_capacity(NUM_PERM);
        for i in 0..NUM_PERM {
            let a = xxh3_64(&(i as u64).to_le_bytes());
            let b = xxh3_64(&((i as u64 + 1000).to_le_bytes()));
            seeds.push((a, b));
        }
        Self { seeds }
    }

    /// Deduplicate a list of samples. Returns the deduplicated list,
    /// keeping the first sample from each cluster.
    pub fn deduplicate(
        &self,
        samples: Vec<TrainingSample>,
        threshold: f64,
    ) -> (Vec<TrainingSample>, usize) {
        if samples.len() <= 1 {
            return (samples, 0);
        }

        // Compute signatures for all samples
        let signatures: Vec<Vec<u64>> = samples
            .iter()
            .map(|s| self.compute_signature(&sample_text(s)))
            .collect();

        // LSH: group candidates by band hashes
        let mut candidate_pairs: Vec<(usize, usize)> = Vec::new();
        for band in 0..NUM_BANDS {
            let start = band * BAND_SIZE;
            let end = start + BAND_SIZE;

            let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
            for (idx, sig) in signatures.iter().enumerate() {
                let band_hash = hash_band(&sig[start..end]);
                buckets.entry(band_hash).or_default().push(idx);
            }

            for indices in buckets.values() {
                if indices.len() > 1 {
                    for i in 0..indices.len() {
                        for j in (i + 1)..indices.len() {
                            candidate_pairs.push((indices[i], indices[j]));
                        }
                    }
                }
            }
        }

        // Verify candidates with actual Jaccard similarity
        let mut to_remove: Vec<bool> = vec![false; samples.len()];
        candidate_pairs.sort_unstable();
        candidate_pairs.dedup();

        for (i, j) in &candidate_pairs {
            if to_remove[*i] || to_remove[*j] {
                continue;
            }
            let jaccard = estimated_jaccard(&signatures[*i], &signatures[*j]);
            if jaccard >= threshold {
                // Remove the later sample (keep the first)
                to_remove[*j] = true;
            }
        }

        let removed_count = to_remove.iter().filter(|&&r| r).count();
        let deduped: Vec<TrainingSample> = samples
            .into_iter()
            .zip(to_remove.iter())
            .filter(|(_, &remove)| !remove)
            .map(|(s, _)| s)
            .collect();

        (deduped, removed_count)
    }

    /// Compute MinHash signature for a text string.
    fn compute_signature(&self, text: &str) -> Vec<u64> {
        let shingles = text_to_shingles(text, SHINGLE_SIZE);
        if shingles.is_empty() {
            return vec![u64::MAX; NUM_PERM];
        }

        let mut signature = vec![u64::MAX; NUM_PERM];

        for shingle_hash in &shingles {
            for (i, (a, b)) in self.seeds.iter().enumerate() {
                let h = a.wrapping_mul(*shingle_hash).wrapping_add(*b);
                if h < signature[i] {
                    signature[i] = h;
                }
            }
        }

        signature
    }
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert text to a set of character n-gram hashes.
fn text_to_shingles(text: &str, n: usize) -> Vec<u64> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < n {
        return if chars.is_empty() {
            Vec::new()
        } else {
            vec![xxh3_64(text.as_bytes())]
        };
    }

    let mut hashes = Vec::with_capacity(chars.len() - n + 1);
    for window in chars.windows(n) {
        let s: String = window.iter().collect();
        hashes.push(xxh3_64(s.as_bytes()));
    }

    // Deduplicate hashes
    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

/// Hash a band (slice of signature) to a single bucket key.
fn hash_band(band: &[u64]) -> u64 {
    let mut combined = Vec::with_capacity(band.len() * 8);
    for &val in band {
        combined.extend_from_slice(&val.to_le_bytes());
    }
    xxh3_64(&combined)
}

/// Estimate Jaccard similarity from two MinHash signatures.
fn estimated_jaccard(sig_a: &[u64], sig_b: &[u64]) -> f64 {
    let matching = sig_a
        .iter()
        .zip(sig_b.iter())
        .filter(|(a, b)| a == b)
        .count();
    matching as f64 / sig_a.len() as f64
}

/// Extract the text content from a sample for hashing.
fn sample_text(sample: &TrainingSample) -> String {
    sample
        .conversation
        .iter()
        .map(|turn| turn.content.as_str())
        .collect::<Vec<&str>>()
        .join("\n")
}
