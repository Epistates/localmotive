use serde::Serialize;

/// Progress events streamed to the frontend via Tauri Channel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ProgressEvent {
    PipelineStarted {
        project_id: String,
        project_name: String,
        total_files: usize,
    },
    PhaseStarted {
        phase: PipelinePhase,
        total_items: usize,
    },
    PhaseProgress {
        phase: PipelinePhase,
        completed: usize,
        total: usize,
        current_item: Option<String>,
    },
    PhaseCompleted {
        phase: PipelinePhase,
        duration_ms: u64,
        items_processed: usize,
    },
    SamplesGenerated {
        count: usize,
        by_type: Vec<(String, usize)>,
    },
    QualityReport {
        total_samples: usize,
        passed: usize,
        filtered: usize,
        duplicates_removed: usize,
    },
    ExportProgress {
        format: String,
        samples_written: usize,
        total_samples: usize,
    },
    PipelineCompleted {
        total_duration_ms: u64,
        total_samples: usize,
        output_files: Vec<String>,
    },
    PipelineError {
        message: String,
        phase: Option<PipelinePhase>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelinePhase {
    Scanning,
    Analyzing,
    Generating,
    QualityFiltering,
    Formatting,
    Exporting,
}

impl std::fmt::Display for PipelinePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scanning => write!(f, "Scanning"),
            Self::Analyzing => write!(f, "Analyzing"),
            Self::Generating => write!(f, "Generating"),
            Self::QualityFiltering => write!(f, "Quality Filtering"),
            Self::Formatting => write!(f, "Formatting"),
            Self::Exporting => write!(f, "Exporting"),
        }
    }
}
