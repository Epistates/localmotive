pub mod discovery;
pub mod file_info;
pub mod language;
pub mod readme;
pub mod walker;

// Re-export key types at module level
pub use discovery::{ProjectManifest, ProjectSummary, LanguageStats, scan_project, discover_projects};
pub use file_info::{FileInfo, Language};
pub use walker::{WalkerConfig, default_ignore_patterns, default_sensitive_patterns};
