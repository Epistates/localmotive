use std::collections::HashMap;
use std::time::Instant;

use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

use crate::analyzer::{self, CodeAnalysis};
use crate::error::{Error, Result};
use crate::formatter;
use crate::generator::{self, GenerationContext, TrainingSample};
use crate::pipeline::cache::AnalysisCache;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::progress::{PipelinePhase, ProgressEvent};
use crate::quality;
use crate::scanner::ProjectManifest;

/// Result of a complete pipeline run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub project_id: String,
    pub total_samples: usize,
    pub samples_by_type: HashMap<String, usize>,
    pub quality_stats: quality::QualityStats,
    pub output_files: Vec<String>,
    pub total_duration_ms: u64,
}

/// Run the full pipeline: scan → analyze → generate → quality → format → export.
pub async fn run_pipeline(
    manifest: &ProjectManifest,
    config: &PipelineConfig,
    output_dir: &str,
    channel: &Channel<ProgressEvent>,
    cancel: &CancellationToken,
) -> Result<PipelineResult> {
    let start = Instant::now();

    // Notify pipeline started
    let _ = channel.send(ProgressEvent::PipelineStarted {
        project_id: manifest.id.clone(),
        project_name: manifest.name.clone(),
        total_files: manifest.files.len(),
    });

    // ── Phase 1: Analyze ─────────────────────────────────────────────
    check_cancelled(cancel)?;
    let (analyses, file_contents) =
        run_analysis_phase(manifest, channel, cancel).await?;

    // ── Phase 2: Generate ────────────────────────────────────────────
    check_cancelled(cancel)?;
    let samples = run_generation_phase(
        manifest,
        &analyses,
        &file_contents,
        config,
        channel,
        cancel,
    )
    .await?;

    // ── Phase 3: Quality Filter ──────────────────────────────────────
    check_cancelled(cancel)?;
    let (filtered_samples, quality_stats) =
        run_quality_phase(samples, config, channel).await?;

    // ── Phase 4: Format & Export ─────────────────────────────────────
    check_cancelled(cancel)?;
    let output_files = run_export_phase(
        &filtered_samples,
        manifest,
        config,
        output_dir,
        channel,
    )
    .await?;

    let total_duration_ms = start.elapsed().as_millis() as u64;

    // Compute samples by type
    let mut samples_by_type: HashMap<String, usize> = HashMap::new();
    for sample in &filtered_samples {
        *samples_by_type
            .entry(format!("{:?}", sample.sample_type))
            .or_insert(0) += 1;
    }

    let result = PipelineResult {
        project_id: manifest.id.clone(),
        total_samples: filtered_samples.len(),
        samples_by_type,
        quality_stats,
        output_files: output_files.clone(),
        total_duration_ms,
    };

    let _ = channel.send(ProgressEvent::PipelineCompleted {
        total_duration_ms,
        total_samples: filtered_samples.len(),
        output_files,
    });

    Ok(result)
}

/// Phase 1: Analyze all files with tree-sitter.
async fn run_analysis_phase(
    manifest: &ProjectManifest,
    channel: &Channel<ProgressEvent>,
    cancel: &CancellationToken,
) -> Result<(Vec<CodeAnalysis>, HashMap<String, String>)> {
    let phase_start = Instant::now();
    let total = manifest.files.len();

    let _ = channel.send(ProgressEvent::PhaseStarted {
        phase: PipelinePhase::Analyzing,
        total_items: total,
    });

    let mut analyses = Vec::new();
    let mut file_contents: HashMap<String, String> = HashMap::new();
    let mut cache = AnalysisCache::new();

    for (i, file) in manifest.files.iter().enumerate() {
        check_cancelled(cancel)?;

        let _ = channel.send(ProgressEvent::PhaseProgress {
            phase: PipelinePhase::Analyzing,
            completed: i,
            total,
            current_item: Some(file.relative_path.clone()),
        });

        // Read file content
        if let Ok(content) = std::fs::read_to_string(&file.path) {
            file_contents.insert(file.relative_path.clone(), content);
        }

        // Check cache
        if let Some(cached) = cache.get(file.content_hash) {
            analyses.push(cached.clone());
            continue;
        }

        // Analyze
        match analyzer::analyze_file(file) {
            Ok(Some(analysis)) => {
                cache.insert(file.content_hash, analysis.clone());
                analyses.push(analysis);
            }
            Ok(None) => {} // Unsupported language, skip
            Err(e) => {
                log::warn!("Failed to analyze {}: {}", file.relative_path, e);
            }
        }
    }

    let _ = channel.send(ProgressEvent::PhaseCompleted {
        phase: PipelinePhase::Analyzing,
        duration_ms: phase_start.elapsed().as_millis() as u64,
        items_processed: analyses.len(),
    });

    Ok((analyses, file_contents))
}

/// Phase 2: Generate training samples.
async fn run_generation_phase(
    manifest: &ProjectManifest,
    analyses: &[CodeAnalysis],
    file_contents: &HashMap<String, String>,
    config: &PipelineConfig,
    channel: &Channel<ProgressEvent>,
    cancel: &CancellationToken,
) -> Result<Vec<TrainingSample>> {
    let phase_start = Instant::now();

    let _ = channel.send(ProgressEvent::PhaseStarted {
        phase: PipelinePhase::Generating,
        total_items: analyses.len(),
    });

    check_cancelled(cancel)?;

    let ctx = GenerationContext {
        project_name: &manifest.name,
        project_description: &manifest.description,
        analyses,
        file_contents,
    };

    let samples = generator::generate_all(&ctx, &config.generation.enabled_types);

    // Report generation stats
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for sample in &samples {
        *by_type
            .entry(format!("{:?}", sample.sample_type))
            .or_insert(0) += 1;
    }
    let by_type_vec: Vec<(String, usize)> = by_type.into_iter().collect();

    let _ = channel.send(ProgressEvent::SamplesGenerated {
        count: samples.len(),
        by_type: by_type_vec,
    });

    let _ = channel.send(ProgressEvent::PhaseCompleted {
        phase: PipelinePhase::Generating,
        duration_ms: phase_start.elapsed().as_millis() as u64,
        items_processed: samples.len(),
    });

    Ok(samples)
}

/// Phase 3: Quality filtering and deduplication.
async fn run_quality_phase(
    samples: Vec<TrainingSample>,
    config: &PipelineConfig,
    channel: &Channel<ProgressEvent>,
) -> Result<(Vec<TrainingSample>, quality::QualityStats)> {
    let phase_start = Instant::now();

    let _ = channel.send(ProgressEvent::PhaseStarted {
        phase: PipelinePhase::QualityFiltering,
        total_items: samples.len(),
    });

    let (filtered, stats) = quality::run_quality_pipeline(samples, &config.quality);

    let _ = channel.send(ProgressEvent::QualityReport {
        total_samples: stats.total_input,
        passed: stats.passed,
        filtered: stats.filtered,
        duplicates_removed: stats.duplicates_removed,
    });

    let _ = channel.send(ProgressEvent::PhaseCompleted {
        phase: PipelinePhase::QualityFiltering,
        duration_ms: phase_start.elapsed().as_millis() as u64,
        items_processed: stats.passed,
    });

    Ok((filtered, stats))
}

/// Phase 4: Format and export to JSONL files.
async fn run_export_phase(
    samples: &[TrainingSample],
    manifest: &ProjectManifest,
    config: &PipelineConfig,
    output_dir: &str,
    channel: &Channel<ProgressEvent>,
) -> Result<Vec<String>> {
    let phase_start = Instant::now();

    let _ = channel.send(ProgressEvent::PhaseStarted {
        phase: PipelinePhase::Exporting,
        total_items: config.output_formats.len(),
    });

    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)
        .map_err(|e| Error::Export(format!("Failed to create output dir: {e}")))?;

    let mut output_files = Vec::new();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    // Sanitize project name for safe filenames (alphanumeric + hyphen + underscore only)
    let safe_name: String = manifest
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();

    for format in &config.output_formats {
        let filename = format!(
            "{}_{:?}_{}.jsonl",
            safe_name,
            format,
            timestamp
        );
        let filepath = std::path::Path::new(output_dir).join(&filename);

        let _ = channel.send(ProgressEvent::ExportProgress {
            format: format!("{format}"),
            samples_written: 0,
            total_samples: samples.len(),
        });

        // Write JSONL
        let mut file = std::io::BufWriter::new(
            std::fs::File::create(&filepath)
                .map_err(|e| Error::Export(format!("Failed to create {}: {e}", filepath.display())))?,
        );

        use std::io::Write;
        for (i, sample) in samples.iter().enumerate() {
            let line = formatter::format_as_jsonl_line(sample, format);
            writeln!(file, "{}", line)
                .map_err(|e| Error::Export(format!("Write error: {e}")))?;

            if (i + 1) % 100 == 0 {
                let _ = channel.send(ProgressEvent::ExportProgress {
                    format: format!("{format}"),
                    samples_written: i + 1,
                    total_samples: samples.len(),
                });
            }
        }

        file.flush()
            .map_err(|e| Error::Export(format!("Flush error: {e}")))?;

        output_files.push(filepath.to_string_lossy().to_string());
    }

    // Write manifest.json if configured
    if config.export.include_manifest {
        let manifest_path = std::path::Path::new(output_dir).join("manifest.json");
        let manifest_data = serde_json::json!({
            "project": manifest.name,
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "total_samples": samples.len(),
            "formats": config.output_formats,
            "output_files": output_files,
        });
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest_data).unwrap_or_default(),
        )
        .map_err(|e| Error::Export(format!("Failed to write manifest: {e}")))?;
        output_files.push(manifest_path.to_string_lossy().to_string());
    }

    let _ = channel.send(ProgressEvent::PhaseCompleted {
        phase: PipelinePhase::Exporting,
        duration_ms: phase_start.elapsed().as_millis() as u64,
        items_processed: output_files.len(),
    });

    Ok(output_files)
}

fn check_cancelled(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
