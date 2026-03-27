use crate::error::Error;
use crate::export::statistics::{compute_statistics, DatasetStatistics};
use crate::formatter;
use crate::generator::types::TrainingSample;
use crate::pipeline::config::OutputFormat;

/// Preview formatted samples for a given format.
/// Takes raw samples and returns them formatted as JSON strings.
#[tauri::command]
pub async fn cmd_preview_samples(
    samples: Vec<TrainingSample>,
    format: OutputFormat,
    offset: usize,
    limit: usize,
) -> Result<Vec<serde_json::Value>, Error> {
    let end = (offset + limit).min(samples.len());
    if offset >= samples.len() {
        return Ok(Vec::new());
    }

    let previews: Vec<serde_json::Value> = samples[offset..end]
        .iter()
        .map(|s| formatter::format_sample(s, &format))
        .collect();

    Ok(previews)
}

/// Compute statistics for a set of samples.
#[tauri::command]
pub async fn cmd_get_statistics(
    samples: Vec<TrainingSample>,
) -> Result<DatasetStatistics, Error> {
    Ok(compute_statistics(&samples))
}
