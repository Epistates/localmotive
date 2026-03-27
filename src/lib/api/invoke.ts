import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  ProjectSummary,
  ProjectManifest,
  PipelineConfig,
  FormatInfo,
  PipelineResult,
  ProgressEvent,
  GenerationStatus,
  DatasetStatistics,
  WhoamiResponse,
  PublishConfig,
  PublishResult,
  PublishProgress,
  OutputFormat,
} from "./types.js";

/** Discover projects within a directory. */
export async function discoverProjects(path: string): Promise<ProjectSummary[]> {
  return invoke("cmd_discover_projects", { path });
}

/** Full scan of a single project. */
export async function scanProject(path: string): Promise<ProjectManifest> {
  return invoke("cmd_scan_project", { path });
}

/** Get a previously scanned project by ID. */
export async function getProject(projectId: string): Promise<ProjectManifest> {
  return invoke("cmd_get_project", { projectId });
}

/** Update the description for a scanned project. */
export async function updateProjectDescription(
  projectId: string,
  description: string,
): Promise<void> {
  return invoke("cmd_update_project_description", { projectId, description });
}

/** Extract the README description from a project path. */
export async function extractReadmeDescription(
  path: string,
): Promise<string | null> {
  return invoke("cmd_extract_readme_description", { path });
}

/** Get the default ignore patterns. */
export async function getDefaultIgnorePatterns(): Promise<string[]> {
  return invoke("cmd_get_default_ignore_patterns");
}

/** Open a native directory picker dialog. */
export async function pickDirectory(): Promise<string | null> {
  return invoke("cmd_pick_directory");
}

/** Get the current pipeline configuration. */
export async function getConfig(): Promise<PipelineConfig> {
  return invoke("cmd_get_config");
}

/** Update the pipeline configuration. */
export async function updateConfig(config: PipelineConfig): Promise<void> {
  return invoke("cmd_update_config", { config });
}

/** Get available output formats with metadata. */
export async function getFormats(): Promise<FormatInfo[]> {
  return invoke("cmd_get_formats");
}

/** Start the generation pipeline with progress streaming. */
export async function startGeneration(
  projectId: string,
  config: PipelineConfig,
  outputDir: string,
  onProgress: (event: ProgressEvent) => void,
): Promise<PipelineResult> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke("cmd_start_generation", {
    projectId,
    config,
    outputDir,
    onProgress: channel,
  });
}

/** Cancel a running generation pipeline. */
export async function cancelGeneration(): Promise<void> {
  return invoke("cmd_cancel_generation");
}

/** Get the current generation status. */
export async function getGenerationStatus(): Promise<GenerationStatus> {
  return invoke("cmd_get_generation_status");
}

/** Get statistics for a set of samples. */
export async function getStatistics(
  samples: unknown[],
): Promise<DatasetStatistics> {
  return invoke("cmd_get_statistics", { samples });
}

// ── Hugging Face Publish ──────────────────────────────────────────

/** Validate a Hugging Face token. */
export async function validateHfToken(token: string): Promise<WhoamiResponse> {
  return invoke("cmd_validate_hf_token", { token });
}

/** Save HF token to the secure store. */
export async function saveHfToken(token: string): Promise<void> {
  return invoke("cmd_save_hf_token", { token });
}

/** Get stored HF token. */
export async function getHfToken(): Promise<string | null> {
  return invoke("cmd_get_hf_token");
}

/** Delete stored HF token. */
export async function deleteHfToken(): Promise<void> {
  return invoke("cmd_delete_hf_token");
}

/** Publish a dataset to Hugging Face Hub with progress streaming. */
export async function publishDataset(
  config: PublishConfig,
  pipelineResult: PipelineResult,
  projectName: string,
  formats: OutputFormat[],
  onProgress: (progress: PublishProgress) => void,
): Promise<PublishResult> {
  const channel = new Channel<PublishProgress>();
  channel.onmessage = onProgress;
  return invoke("cmd_publish_dataset", {
    config,
    pipelineResult,
    projectName,
    formats,
    onProgress: channel,
  });
}
