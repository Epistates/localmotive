use std::sync::Mutex;

use tauri::ipc::Channel;
use tauri::State;

use crate::error::Error;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::orchestrator::PipelineResult;
use crate::pipeline::progress::ProgressEvent;
use crate::state::{AppState, CancellationState, GenerationStatus};

/// Start the full generation pipeline for a scanned project.
/// Progress is streamed via the Channel parameter.
#[tauri::command]
pub async fn cmd_start_generation(
    project_id: String,
    config: PipelineConfig,
    output_dir: String,
    on_progress: Channel<ProgressEvent>,
    state: State<'_, AppState>,
    cancel_state: State<'_, Mutex<CancellationState>>,
) -> Result<PipelineResult, Error> {
    // Get the project manifest from state and check for concurrent runs
    let manifest = {
        let mut state = state
            .lock()
            .map_err(|_| Error::Scanner("Internal state error".into()))?;
        if state.pipeline_running {
            return Err(Error::Config("Pipeline already running. Cancel or wait for it to finish.".into()));
        }
        state.pipeline_running = true;
        state
            .projects
            .get(&project_id)
            .cloned()
            .ok_or_else(|| Error::ProjectNotFound(project_id.clone()))?
    };

    // Reset cancellation token
    let cancel_token = {
        let mut cs = cancel_state
            .lock()
            .map_err(|e| Error::Scanner(format!("Cancel state lock poisoned: {e}")))?;
        cs.reset();
        cs.token.clone()
    };

    // Update status to running
    {
        let mut state = state
            .lock()
            .map_err(|e| Error::Scanner(format!("State lock poisoned: {e}")))?;
        state.generation_status = GenerationStatus::Running {
            project_id: project_id.clone(),
            phase: "Starting".to_string(),
        };
    }

    // Run the pipeline
    let result = crate::pipeline::run_pipeline(
        &manifest,
        &config,
        &output_dir,
        &on_progress,
        &cancel_token,
    )
    .await;

    // Update status and clear running flag in both success and error paths
    match &result {
        Ok(_) => {
            if let Ok(mut state) = state.lock() {
                state.generation_status = GenerationStatus::Complete {
                    project_id: project_id.clone(),
                };
                state.pipeline_running = false;
            }
        }
        Err(e) => {
            if let Ok(mut state) = state.lock() {
                state.generation_status = GenerationStatus::Error {
                    message: e.to_string(),
                };
                state.pipeline_running = false;
            }
        }
    }

    result
}

/// Cancel a running generation pipeline.
#[tauri::command]
pub async fn cmd_cancel_generation(
    cancel_state: State<'_, Mutex<CancellationState>>,
) -> Result<(), Error> {
    let cs = cancel_state
        .lock()
        .map_err(|e| Error::Scanner(format!("Cancel state lock poisoned: {e}")))?;
    cs.token.cancel();
    Ok(())
}

/// Get the current generation status.
#[tauri::command]
pub async fn cmd_get_generation_status(
    state: State<'_, AppState>,
) -> Result<GenerationStatus, Error> {
    let state = state
        .lock()
        .map_err(|e| Error::Scanner(format!("State lock poisoned: {e}")))?;
    Ok(state.generation_status.clone())
}
