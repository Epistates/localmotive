use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::error::Error;
use crate::export::statistics;
use crate::pipeline::config::OutputFormat;
use crate::pipeline::orchestrator::PipelineResult;
use crate::publish::client::HfClient;
use crate::publish::types::{
    PublishConfigFromFrontend, PublishConfig, PublishProgress, PublishResult, WhoamiResponse,
};

const HF_TOKEN_STORE_KEY: &str = "hf_token";
const STORE_FILENAME: &str = "settings.json";

/// Resolve HF token: env var first (CI/automation), then store.
fn load_token(app: &AppHandle) -> Result<String, Error> {
    // Check HF_TOKEN env var first (for CI/CD and power users)
    if let Ok(token) = std::env::var("HF_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // Fall back to saved store
    let store = app
        .store(STORE_FILENAME)
        .map_err(|e| Error::Publish(format!("Failed to open store: {e}")))?;
    store
        .get(HF_TOKEN_STORE_KEY)
        .and_then(|v: serde_json::Value| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| {
            Error::Publish("No Hugging Face token configured. Set HF_TOKEN env var or add one in Settings.".into())
        })
}

/// Validate a Hugging Face token and return user info.
#[tauri::command]
pub async fn cmd_validate_hf_token(token: String) -> Result<WhoamiResponse, Error> {
    let client = HfClient::new(token)?;
    client.whoami().await
}

/// Save the HF token to the secure store (trimmed and basic-validated).
#[tauri::command]
pub async fn cmd_save_hf_token(app: AppHandle, token: String) -> Result<(), Error> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(Error::Publish("Token cannot be empty".into()));
    }
    if !token.starts_with("hf_") {
        return Err(Error::Publish(
            "Token should start with 'hf_'. Check that you copied the full token.".into(),
        ));
    }

    let store = app
        .store(STORE_FILENAME)
        .map_err(|e| Error::Publish(format!("Failed to open store: {e}")))?;
    store.set(HF_TOKEN_STORE_KEY, serde_json::Value::String(token));
    store
        .save()
        .map_err(|e| Error::Publish(format!("Failed to save store: {e}")))?;
    Ok(())
}

/// Retrieve the HF token (env var or store). Returns None if not set.
#[tauri::command]
pub async fn cmd_get_hf_token(app: AppHandle) -> Result<Option<String>, Error> {
    // Check env var first
    if let Ok(token) = std::env::var("HF_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }

    // Fall back to store
    let store = app
        .store(STORE_FILENAME)
        .map_err(|e| Error::Publish(format!("Failed to open store: {e}")))?;

    let token = store
        .get(HF_TOKEN_STORE_KEY)
        .and_then(|v: serde_json::Value| v.as_str().map(|s| s.to_string()));
    Ok(token)
}

/// Delete the HF token from the store.
#[tauri::command]
pub async fn cmd_delete_hf_token(app: AppHandle) -> Result<(), Error> {
    let store = app
        .store(STORE_FILENAME)
        .map_err(|e| Error::Publish(format!("Failed to open store: {e}")))?;
    let _ = store.delete(HF_TOKEN_STORE_KEY);
    store
        .save()
        .map_err(|e| Error::Publish(format!("Failed to save store: {e}")))?;
    Ok(())
}

/// Publish a dataset to Hugging Face Hub.
/// Token is resolved from the store — never sent from the frontend.
#[tauri::command]
pub async fn cmd_publish_dataset(
    app: AppHandle,
    config: PublishConfigFromFrontend,
    pipeline_result: PipelineResult,
    project_name: String,
    formats: Vec<OutputFormat>,
    on_progress: Channel<PublishProgress>,
) -> Result<PublishResult, Error> {
    // Resolve token from store — it never crosses IPC
    let token = load_token(&app)?;

    let publish_config = PublishConfig {
        token,
        repo_name: config.repo_name,
        namespace: config.namespace,
        private: config.private,
        license: config.license,
        output_files: config.output_files,
    };

    let stats = statistics::DatasetStatistics {
        total_samples: pipeline_result.total_samples,
        samples_by_type: pipeline_result.samples_by_type.clone(),
        samples_by_language: std::collections::HashMap::new(),
        token_stats: statistics::TokenStats {
            min: 0,
            max: 0,
            mean: 0,
            median: 0,
            p90: 0,
            p99: 0,
            total: 0,
        },
        complexity_distribution: std::collections::HashMap::new(),
    };

    crate::publish::publish_dataset(
        &publish_config,
        &pipeline_result,
        &stats,
        &project_name,
        &formats,
        |progress| {
            let _ = on_progress.send(progress);
        },
    )
    .await
}
