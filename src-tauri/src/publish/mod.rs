pub mod card;
pub mod client;
pub mod types;

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::export::statistics::DatasetStatistics;
use crate::pipeline::config::OutputFormat;
use crate::pipeline::orchestrator::PipelineResult;

use self::card::{format_config_name, generate_dataset_card};
use self::client::HfClient;
use self::types::{PublishConfig, PublishProgress, PublishResult};

/// Allowed licenses — validated server-side to prevent YAML injection.
const ALLOWED_LICENSES: &[&str] = &[
    "mit",
    "apache-2.0",
    "cc-by-4.0",
    "cc-by-sa-4.0",
    "cc-by-nc-4.0",
    "gpl-3.0",
    "other",
];

/// Full publish flow: validate → create repo → generate card → upload files → commit.
pub async fn publish_dataset(
    config: &PublishConfig,
    pipeline_result: &PipelineResult,
    stats: &DatasetStatistics,
    project_name: &str,
    formats: &[OutputFormat],
    on_progress: impl Fn(PublishProgress) + Send + Sync,
) -> Result<PublishResult> {
    // ── Validation ──────────────────────────────────────────────────
    client::validate_hf_name(&config.repo_name, "Repository name")?;
    if let Some(ns) = &config.namespace {
        client::validate_hf_name(ns, "Namespace")?;
    }
    if !ALLOWED_LICENSES.contains(&config.license.as_str()) {
        return Err(Error::Publish(format!(
            "Unknown license: {}. Use one of: {}",
            config.license,
            ALLOWED_LICENSES.join(", ")
        )));
    }

    // Validate all output files exist and are within allowed extensions
    let validated_files = validate_output_files(&config.output_files)?;

    let client = HfClient::new(config.token.clone())?;

    // 1. Resolve namespace
    on_progress(PublishProgress::CreatingRepo);
    let whoami = client.whoami().await?;
    let namespace = config.namespace.as_deref().unwrap_or(&whoami.name);

    // 2. Create repo
    let repo_url = client
        .create_dataset_repo(
            &config.repo_name,
            namespace,
            config.private,
            Some(&config.license),
        )
        .await?;

    // 3. Generate dataset card
    on_progress(PublishProgress::GeneratingCard);
    let card = generate_dataset_card(
        pipeline_result,
        stats,
        project_name,
        formats,
        &config.license,
    );

    // 4. Prepare files for upload
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    // README.md (dataset card)
    files.push(("README.md".to_string(), card.into_bytes()));

    // JSONL output files — read one at a time to limit peak memory
    let total_files = validated_files.len();
    for (i, local_path) in validated_files.iter().enumerate() {
        let filename = local_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        on_progress(PublishProgress::UploadingFile {
            name: filename,
            index: i,
            total: total_files,
        });

        let content = std::fs::read(local_path).map_err(|e| {
            Error::Publish(format!("Failed to read output file: {e}"))
        })?;

        let remote_path = resolve_remote_path(local_path, formats);
        files.push((remote_path, content));
    }

    // 5. Commit all files
    let files_uploaded = files.len();
    let commit_resp = client
        .commit(
            namespace,
            &config.repo_name,
            "main",
            "Upload training data from Localmotive",
            files,
            &on_progress,
        )
        .await?;

    on_progress(PublishProgress::Done);

    Ok(PublishResult {
        repo_url,
        commit_url: commit_resp.commit_url,
        files_uploaded,
    })
}

/// Validate that all output file paths exist and have allowed extensions.
/// Returns canonical paths to prevent path traversal.
fn validate_output_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let allowed_extensions = ["jsonl", "json"];
    let mut validated = Vec::with_capacity(paths.len());

    for path in paths {
        let canonical = PathBuf::from(path)
            .canonicalize()
            .map_err(|_| Error::Publish("Output file not found".into()))?;

        let ext = canonical
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !allowed_extensions.contains(&ext) {
            return Err(Error::Publish(format!(
                "Only .jsonl and .json files can be published (got .{ext})"
            )));
        }

        validated.push(canonical);
    }

    Ok(validated)
}

/// Map a local output filename to a Hub-friendly path.
/// Uses `format_config_name` for reliable matching rather than Debug format strings.
fn resolve_remote_path(local_path: &Path, formats: &[OutputFormat]) -> String {
    let filename = local_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    // manifest.json passes through
    if filename == "manifest.json" {
        return "manifest.json".to_string();
    }

    // Match against the canonical config names used by the formatter
    for fmt in formats {
        let config_name = format_config_name(fmt);
        if filename.contains(&config_name) {
            return format!("data/{config_name}/train.jsonl");
        }
    }

    // Fallback
    "data/train.jsonl".to_string()
}
