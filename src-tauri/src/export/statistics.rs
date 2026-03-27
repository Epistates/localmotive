use std::collections::HashMap;

use serde::Serialize;

use crate::generator::types::TrainingSample;
use crate::quality::token_counter;

/// Dataset statistics computed from generated samples.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStatistics {
    pub total_samples: usize,
    pub samples_by_type: HashMap<String, usize>,
    pub samples_by_language: HashMap<String, usize>,
    pub token_stats: TokenStats,
    pub complexity_distribution: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub min: usize,
    pub max: usize,
    pub mean: usize,
    pub median: usize,
    pub p90: usize,
    pub p99: usize,
    pub total: usize,
}

/// Compute statistics for a set of samples.
pub fn compute_statistics(samples: &[TrainingSample]) -> DatasetStatistics {
    let total_samples = samples.len();

    // Samples by type
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for sample in samples {
        *by_type
            .entry(format!("{:?}", sample.sample_type))
            .or_insert(0) += 1;
    }

    // Samples by language
    let mut by_language: HashMap<String, usize> = HashMap::new();
    for sample in samples {
        let lang = sample
            .metadata
            .language
            .as_ref()
            .map(|l| l.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        *by_language.entry(lang).or_insert(0) += 1;
    }

    // Token statistics
    let mut token_counts: Vec<usize> = samples
        .iter()
        .map(|s| token_counter::count_sample_tokens(&s.conversation))
        .collect();
    token_counts.sort_unstable();

    let token_stats = if token_counts.is_empty() {
        TokenStats {
            min: 0,
            max: 0,
            mean: 0,
            median: 0,
            p90: 0,
            p99: 0,
            total: 0,
        }
    } else {
        let total: usize = token_counts.iter().sum();
        let len = token_counts.len();
        TokenStats {
            min: token_counts[0],
            max: token_counts[len - 1],
            mean: total / len,
            median: token_counts[len / 2],
            p90: token_counts[((len - 1) as f64 * 0.9) as usize],
            p99: token_counts[(((len - 1) as f64 * 0.99) as usize).min(len - 1)],
            total,
        }
    };

    // Complexity distribution
    let mut complexity: HashMap<String, usize> = HashMap::new();
    for sample in samples {
        *complexity
            .entry(format!("{:?}", sample.metadata.complexity_tier))
            .or_insert(0) += 1;
    }

    DatasetStatistics {
        total_samples,
        samples_by_type: by_type,
        samples_by_language: by_language,
        token_stats,
        complexity_distribution: complexity,
    }
}
