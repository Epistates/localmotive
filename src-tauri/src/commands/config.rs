use crate::error::Error;
use crate::pipeline::config::{available_formats, FormatInfo, PipelineConfig};

/// Get the current pipeline configuration.
/// Returns defaults if no config has been persisted.
#[tauri::command]
pub async fn cmd_get_config() -> Result<PipelineConfig, Error> {
    // TODO: Load from tauri-plugin-store. For now, return defaults.
    Ok(PipelineConfig::default())
}

/// Update the pipeline configuration.
#[tauri::command]
pub async fn cmd_update_config(config: PipelineConfig) -> Result<(), Error> {
    // TODO: Persist to tauri-plugin-store.
    // Validate the config
    if config.quality.min_tokens_per_sample > config.quality.max_tokens_per_sample {
        return Err(Error::Config(
            "min_tokens_per_sample must be <= max_tokens_per_sample".to_string(),
        ));
    }
    if config.generation.token_budget == 0 {
        return Err(Error::Config("token_budget must be > 0".to_string()));
    }
    let _ = config; // Will be persisted when store is wired
    Ok(())
}

/// Get available output format metadata for UI display.
#[tauri::command]
pub async fn cmd_get_formats() -> Result<Vec<FormatInfo>, Error> {
    Ok(available_formats())
}
