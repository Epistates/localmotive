use std::collections::HashMap;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::scanner::ProjectManifest;

/// Pipeline execution status
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationStatus {
    Idle,
    Running { project_id: String, phase: String },
    Complete { project_id: String },
    Error { message: String },
}

impl Default for GenerationStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Core application state behind a Mutex.
/// Held briefly for reads/writes, never across await points.
#[derive(Default)]
pub struct AppStateInner {
    pub projects: HashMap<String, ProjectManifest>,
    pub generation_status: GenerationStatus,
    /// Guard against concurrent pipeline runs.
    pub pipeline_running: bool,
}

pub type AppState = Mutex<AppStateInner>;

/// Separate lock-free state for pipeline cancellation.
/// The pipeline checks this without acquiring the main state mutex.
pub struct CancellationState {
    pub token: CancellationToken,
}

impl CancellationState {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Reset the cancellation token for a new pipeline run.
    pub fn reset(&mut self) {
        self.token = CancellationToken::new();
    }
}

impl Default for CancellationState {
    fn default() -> Self {
        Self::new()
    }
}
