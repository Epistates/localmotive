pub mod autogen_detect;
pub mod dedup;
pub mod filters;
pub mod token_counter;

use crate::generator::types::TrainingSample;
use crate::pipeline::config::QualityConfig;

pub use dedup::Deduplicator;
pub use filters::FilterResult;

/// Run the full quality pipeline on a set of samples.
/// Returns (passed_samples, filter_stats).
pub fn run_quality_pipeline(
    samples: Vec<TrainingSample>,
    config: &QualityConfig,
) -> (Vec<TrainingSample>, QualityStats) {
    let total = samples.len();

    // Phase 1: Filter by quality heuristics
    let mut passed = Vec::new();
    let mut filtered_count = 0;

    for sample in samples {
        match filters::filter_sample(&sample, config) {
            FilterResult::Pass => passed.push(sample),
            FilterResult::Reject(_reason) => {
                filtered_count += 1;
            }
        }
    }

    // Phase 2: Deduplicate
    let pre_dedup_count = passed.len();
    let deduplicator = Deduplicator::new();
    let (deduped, duplicates_removed) =
        deduplicator.deduplicate(passed, config.dedup_jaccard_threshold);

    let stats = QualityStats {
        total_input: total,
        passed: deduped.len(),
        filtered: filtered_count,
        duplicates_removed,
        pre_dedup_count,
    };

    (deduped, stats)
}

/// Statistics from the quality pipeline run.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityStats {
    pub total_input: usize,
    pub passed: usize,
    pub filtered: usize,
    pub duplicates_removed: usize,
    pub pre_dedup_count: usize,
}
