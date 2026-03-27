use serde::{Deserialize, Serialize};

// ── Repo creation ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateRepoRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub repo_type: String,
    pub private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepoResponse {
    pub url: String,
}

// ── Commit ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CommitRequest {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "parentCommit", skip_serializing_if = "Option::is_none")]
    pub parent_commit: Option<String>,
    pub files: Vec<CommitFile>,
    #[serde(rename = "deletedEntries", skip_serializing_if = "Vec::is_empty")]
    pub deleted_entries: Vec<DeletedEntry>,
    #[serde(rename = "lfsFiles", skip_serializing_if = "Vec::is_empty")]
    pub lfs_files: Vec<LfsFileRef>,
}

#[derive(Debug, Serialize)]
pub struct CommitFile {
    pub path: String,
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Serialize)]
pub struct DeletedEntry {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct LfsFileRef {
    pub path: String,
    pub oid: String,
    pub algo: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct CommitResponse {
    #[serde(rename = "commitOid")]
    pub commit_oid: String,
    #[serde(rename = "commitUrl")]
    pub commit_url: String,
}

// ── Pre-upload ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PreUploadRequest {
    pub files: Vec<PreUploadFile>,
}

#[derive(Debug, Serialize)]
pub struct PreUploadFile {
    pub path: String,
    pub size: u64,
    pub sample: String,
}

#[derive(Debug, Deserialize)]
pub struct PreUploadResponse {
    pub files: Vec<PreUploadFileResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PreUploadFileResponse {
    pub path: String,
    #[serde(rename = "uploadMode")]
    pub upload_mode: String,
}

// ── LFS batch ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LfsBatchRequest {
    pub operation: String,
    pub transfers: Vec<String>,
    pub objects: Vec<LfsObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LfsObject {
    pub oid: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct LfsBatchResponse {
    pub objects: Vec<LfsBatchObject>,
}

#[derive(Debug, Deserialize)]
pub struct LfsBatchObject {
    pub oid: String,
    pub size: u64,
    pub actions: Option<LfsBatchActions>,
}

#[derive(Debug, Deserialize)]
pub struct LfsBatchActions {
    pub upload: Option<LfsAction>,
    pub verify: Option<LfsAction>,
}

#[derive(Debug, Deserialize)]
pub struct LfsAction {
    pub href: String,
    #[serde(default)]
    pub header: std::collections::HashMap<String, String>,
}

// ── Whoami ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoamiResponse {
    pub name: String,
    pub fullname: Option<String>,
    #[serde(default)]
    pub orgs: Vec<OrgInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInfo {
    pub name: String,
}

// ── Publish config (from frontend — no token field) ─────────────────

/// Config as received from the frontend IPC. Token is resolved in the
/// Rust backend from the secure store and never crosses the IPC boundary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfigFromFrontend {
    pub repo_name: String,
    pub namespace: Option<String>,
    pub private: bool,
    pub license: String,
    pub output_files: Vec<String>,
}

/// Internal config with token attached (used after store resolution).
#[derive(Debug, Clone)]
pub struct PublishConfig {
    pub token: String,
    pub repo_name: String,
    pub namespace: Option<String>,
    pub private: bool,
    pub license: String,
    pub output_files: Vec<String>,
}

// ── Publish result ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishResult {
    pub repo_url: String,
    pub commit_url: String,
    pub files_uploaded: usize,
}

// ── Publish progress (streamed to frontend) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum PublishProgress {
    CreatingRepo,
    GeneratingCard,
    UploadingFile {
        name: String,
        index: usize,
        total: usize,
    },
    UploadingLfsFile {
        name: String,
        #[serde(rename = "bytesSent")]
        bytes_sent: u64,
        #[serde(rename = "bytesTotal")]
        bytes_total: u64,
    },
    Committing,
    Done,
}
