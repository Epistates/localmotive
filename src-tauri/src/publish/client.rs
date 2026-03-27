use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

use super::types::*;

const HF_BASE_URL: &str = "https://huggingface.co";
const LFS_CONTENT_TYPE: &str = "application/vnd.git-lfs+json";

/// Allowed host suffixes for LFS signed upload URLs.
const LFS_ALLOWED_HOSTS: &[&str] = &[
    "huggingface.co",
    "s3.amazonaws.com",
    "s3.us-east-1.amazonaws.com",
    "storage.googleapis.com",
];

/// Header keys that must not be injected from LFS server responses.
const BLOCKED_LFS_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "host",
    "x-forwarded",
    "x-real-ip",
    "x-auth",
];

/// Validate that a HF Hub name (repo or namespace) contains only safe characters.
pub fn validate_hf_name(name: &str, label: &str) -> Result<()> {
    if name.is_empty() || name.len() > 96 {
        return Err(Error::Publish(format!(
            "{label} must be between 1 and 96 characters"
        )));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !valid || name.starts_with('.') || name.starts_with('-') {
        return Err(Error::Publish(format!(
            "{label} must contain only alphanumeric characters, hyphens, underscores, or dots"
        )));
    }
    Ok(())
}

pub struct HfClient {
    http: reqwest::Client,
    base_url: String,
}

impl HfClient {
    pub fn new(token: String) -> Result<Self> {
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(Error::Publish("Token cannot be empty".into()));
        }

        let mut headers = HeaderMap::new();
        let header_val = HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
            Error::Publish("Token contains invalid characters".into())
        })?;
        headers.insert(AUTHORIZATION, header_val);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("localmotive/0.1.0")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| Error::Publish(format!("Failed to initialize HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: HF_BASE_URL.to_string(),
        })
    }

    /// Validate the token and return the user's username + orgs.
    pub async fn whoami(&self) -> Result<WhoamiResponse> {
        let resp = self
            .http
            .get(format!("{}/api/whoami-v2", self.base_url))
            .send()
            .await
            .map_err(|e| Error::Publish(format!("Network error: {e}")))?;

        let status = resp.status();
        if status == 401 {
            return Err(Error::Publish(
                "Invalid or expired token. Please check your Hugging Face token.".into(),
            ));
        }
        if !status.is_success() {
            return Err(Error::Publish(format!(
                "Whoami failed ({status})"
            )));
        }

        resp.json::<WhoamiResponse>()
            .await
            .map_err(|e| Error::Publish(format!("Failed to parse whoami response: {e}")))
    }

    /// Create a dataset repository. Treats 409 (already exists) as success.
    pub async fn create_dataset_repo(
        &self,
        name: &str,
        namespace: &str,
        private: bool,
        license: Option<&str>,
    ) -> Result<String> {
        validate_hf_name(name, "Repository name")?;
        validate_hf_name(namespace, "Namespace")?;

        let req = CreateRepoRequest {
            name: name.to_string(),
            repo_type: "dataset".to_string(),
            private,
            organization: Some(namespace.to_string()),
            license: license.map(|s| s.to_string()),
        };

        let resp = self
            .http
            .post(format!("{}/api/repos/create", self.base_url))
            .json(&req)
            .send()
            .await
            .map_err(|e| Error::Publish(format!("Network error: {e}")))?;

        let status = resp.status();

        // 409 = already exists — treat as success
        if status.as_u16() == 409 {
            return Ok(format!(
                "{}/datasets/{}/{}",
                self.base_url, namespace, name
            ));
        }

        if status == 401 {
            return Err(Error::Publish("Invalid or expired token.".into()));
        }
        if status == 403 {
            return Err(Error::Publish(
                "Insufficient permissions. Token needs write scope.".into(),
            ));
        }
        if status.as_u16() == 422 {
            return Err(Error::Publish(
                "Invalid repository name. Use lowercase letters, digits, and hyphens.".into(),
            ));
        }
        if !status.is_success() {
            return Err(Error::Publish(format!(
                "Failed to create repo ({status})"
            )));
        }

        let create_resp: CreateRepoResponse = resp
            .json()
            .await
            .map_err(|e| Error::Publish(format!("Failed to parse create response: {e}")))?;

        Ok(create_resp.url)
    }

    /// Commit files to a dataset repo. Handles inline vs LFS upload based on
    /// the server's pre-upload response rather than a static size threshold.
    pub async fn commit(
        &self,
        namespace: &str,
        repo: &str,
        branch: &str,
        message: &str,
        files: Vec<(String, Vec<u8>)>,
        on_progress: &(dyn Fn(PublishProgress) + Send + Sync),
    ) -> Result<CommitResponse> {
        // Ask the server which files need LFS
        let preupload_resp = self
            .preupload_check(namespace, repo, branch, &files)
            .await?;

        // Build a set of paths the server says need LFS
        let lfs_paths: std::collections::HashSet<String> = preupload_resp
            .files
            .iter()
            .filter(|f| f.upload_mode == "lfs")
            .map(|f| f.path.clone())
            .collect();

        let mut inline_files = Vec::new();
        let mut lfs_candidates: Vec<(String, Vec<u8>)> = Vec::new();

        for (path, content) in files {
            if lfs_paths.contains(&path) {
                lfs_candidates.push((path, content));
            } else {
                // Try UTF-8 first; fall back to base64 for non-UTF-8 content
                let (encoded_content, encoding) = match String::from_utf8(content.clone()) {
                    Ok(s) => (s, "utf-8".to_string()),
                    Err(_) => (
                        base64::engine::general_purpose::STANDARD.encode(&content),
                        "base64".to_string(),
                    ),
                };
                inline_files.push(CommitFile {
                    path,
                    content: encoded_content,
                    encoding,
                });
            }
        }

        let mut lfs_refs = Vec::new();

        // Handle LFS uploads
        for (path, content) in &lfs_candidates {
            on_progress(PublishProgress::UploadingLfsFile {
                name: path.clone(),
                bytes_sent: 0,
                bytes_total: content.len() as u64,
            });

            let lfs_ref = self
                .upload_lfs_file(namespace, repo, path, content)
                .await?;
            lfs_refs.push(lfs_ref);

            on_progress(PublishProgress::UploadingLfsFile {
                name: path.clone(),
                bytes_sent: content.len() as u64,
                bytes_total: content.len() as u64,
            });
        }

        on_progress(PublishProgress::Committing);

        let commit_req = CommitRequest {
            summary: message.to_string(),
            description: None,
            parent_commit: None,
            files: inline_files,
            deleted_entries: Vec::new(),
            lfs_files: lfs_refs,
        };

        let resp = self
            .send_with_retry(|| {
                self.http
                    .post(format!(
                        "{}/api/datasets/{}/{}/commit/{}",
                        self.base_url, namespace, repo, branch
                    ))
                    .json(&commit_req)
            })
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Publish(format!(
                "Commit failed ({status})"
            )));
        }

        resp.json::<CommitResponse>()
            .await
            .map_err(|e| Error::Publish(format!("Failed to parse commit response: {e}")))
    }

    /// Pre-upload check — ask the server which files need LFS.
    async fn preupload_check(
        &self,
        namespace: &str,
        repo: &str,
        branch: &str,
        files: &[(String, Vec<u8>)],
    ) -> Result<PreUploadResponse> {
        let preupload_files: Vec<PreUploadFile> = files
            .iter()
            .map(|(path, content)| {
                let sample_bytes = &content[..content.len().min(512)];
                PreUploadFile {
                    path: path.clone(),
                    size: content.len() as u64,
                    sample: base64::engine::general_purpose::STANDARD.encode(sample_bytes),
                }
            })
            .collect();

        let resp = self
            .http
            .post(format!(
                "{}/api/datasets/{}/{}/preupload/{}",
                self.base_url, namespace, repo, branch
            ))
            .json(&PreUploadRequest {
                files: preupload_files,
            })
            .send()
            .await
            .map_err(|e| Error::Publish(format!("Pre-upload check failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Error::Publish("Pre-upload check failed".into()));
        }

        resp.json::<PreUploadResponse>()
            .await
            .map_err(|e| Error::Publish(format!("Failed to parse pre-upload response: {e}")))
    }

    /// Upload a single file via LFS (batch → PUT).
    async fn upload_lfs_file(
        &self,
        namespace: &str,
        repo: &str,
        path: &str,
        content: &[u8],
    ) -> Result<LfsFileRef> {
        let size = content.len() as u64;

        // SHA-256 for LFS OID
        let mut hasher = Sha256::new();
        hasher.update(content);
        let oid = format!("{:x}", hasher.finalize());

        // LFS batch request
        let batch_req = LfsBatchRequest {
            operation: "upload".to_string(),
            transfers: vec!["basic".to_string(), "multipart".to_string()],
            objects: vec![LfsObject {
                oid: oid.clone(),
                size,
            }],
        };

        let resp = self
            .http
            .post(format!(
                "{}/datasets/{}/{}.git/info/lfs/objects/batch",
                self.base_url, namespace, repo
            ))
            .header(CONTENT_TYPE, LFS_CONTENT_TYPE)
            .header("Accept", LFS_CONTENT_TYPE)
            .json(&batch_req)
            .send()
            .await
            .map_err(|e| Error::Publish(format!("LFS batch request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Error::Publish("LFS batch request failed".into()));
        }

        let batch_resp: LfsBatchResponse = resp
            .json()
            .await
            .map_err(|e| Error::Publish(format!("Failed to parse LFS batch response: {e}")))?;

        // Upload to signed URL if actions are present
        if let Some(obj) = batch_resp.objects.first() {
            if let Some(actions) = &obj.actions {
                if let Some(upload_action) = &actions.upload {
                    self.execute_lfs_upload(upload_action, content).await?;
                }
            }
        }

        Ok(LfsFileRef {
            path: path.to_string(),
            oid,
            algo: "sha256".to_string(),
            size,
        })
    }

    /// Execute the actual PUT to the LFS signed URL with safety checks.
    async fn execute_lfs_upload(
        &self,
        action: &LfsAction,
        content: &[u8],
    ) -> Result<()> {
        // Validate the upload URL
        let url = reqwest::Url::parse(&action.href).map_err(|_| {
            Error::Publish("Invalid LFS upload URL".into())
        })?;
        if url.scheme() != "https" {
            return Err(Error::Publish("LFS upload URL must use HTTPS".into()));
        }
        let host = url.host_str().unwrap_or("");
        let host_allowed = LFS_ALLOWED_HOSTS.iter().any(|allowed| {
            host == *allowed || host.ends_with(&format!(".{allowed}"))
        });
        if !host_allowed {
            return Err(Error::Publish(format!(
                "LFS upload URL host not in allow-list: {host}"
            )));
        }

        let mut req = self
            .http
            .put(url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(content.to_vec());

        // Add allowed headers from the LFS response, blocking sensitive ones
        for (key, value) in &action.header {
            let lower = key.to_lowercase();
            if BLOCKED_LFS_HEADERS
                .iter()
                .any(|blocked| lower.starts_with(blocked))
            {
                continue;
            }
            if let Ok(val) = HeaderValue::from_str(value) {
                req = req.header(key.as_str(), val);
            }
        }

        let upload_resp = req
            .send()
            .await
            .map_err(|e| Error::Publish(format!("LFS upload failed: {e}")))?;

        if !upload_resp.status().is_success() {
            return Err(Error::Publish("LFS upload failed".into()));
        }

        Ok(())
    }

    /// Send a request with retry on 429 (rate limit). Reads `Retry-After` header.
    async fn send_with_retry(
        &self,
        build_request: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        let max_retries = 3;
        for attempt in 0..=max_retries {
            let resp = build_request()
                .send()
                .await
                .map_err(|e| Error::Publish(format!("Network error: {e}")))?;

            if resp.status().as_u16() != 429 || attempt == max_retries {
                return Ok(resp);
            }

            // Parse Retry-After header, default to exponential backoff
            let wait_secs = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(2u64.pow(attempt as u32));

            let wait = std::time::Duration::from_secs(wait_secs.min(30));
            tokio::time::sleep(wait).await;
        }

        unreachable!()
    }
}
