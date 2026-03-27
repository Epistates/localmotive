use std::path::Path;

use ignore::WalkBuilder;

use crate::error::{Error, Result};

use super::file_info::FileInfo;
use super::language::detect_language;

/// Default directory/file patterns to always ignore.
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    // Build artifacts and dependencies
    "node_modules",
    "target",
    "build",
    "dist",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "__pycache__",
    ".pytest_cache",
    "vendor",
    ".venv",
    "venv",
    "env",
    // Version control
    ".git",
    ".hg",
    ".svn",
    // Minified / generated
    "*.min.js",
    "*.min.css",
    "*.map",
    // Lock files
    "*.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "poetry.lock",
    "Pipfile.lock",
    "bun.lockb",
    "bun.lock",
    // Generated code
    "*.pb.go",
    "*.generated.*",
    "*_generated.*",
    "*.d.ts",
    // OS artifacts
    ".DS_Store",
    "Thumbs.db",
    // Coverage
    "coverage",
    ".nyc_output",
    ".coverage",
    "htmlcov",
    // Binary / media
    "*.wasm",
    "*.so",
    "*.dylib",
    "*.dll",
    "*.a",
    "*.o",
    "*.obj",
    "*.exe",
    "*.jpg",
    "*.jpeg",
    "*.png",
    "*.gif",
    "*.svg",
    "*.ico",
    "*.webp",
    "*.mp3",
    "*.mp4",
    "*.wav",
    "*.avi",
    "*.webm",
    "*.zip",
    "*.tar",
    "*.gz",
    "*.bz2",
    "*.7z",
    "*.rar",
    "*.pdf",
    "*.doc",
    "*.docx",
    // IDE
    ".idea",
    ".vscode",
    "*.swp",
    "*.swo",
];

/// Configuration for file walking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkerConfig {
    /// Whether to respect .gitignore files.
    pub use_gitignore: bool,
    /// Additional ignore patterns (glob syntax).
    pub extra_ignore_patterns: Vec<String>,
    /// Maximum file size in bytes (files larger are skipped).
    pub max_file_size_bytes: u64,
    /// Whether to skip binary files.
    pub skip_binary: bool,
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            extra_ignore_patterns: Vec::new(),
            max_file_size_bytes: 1_000_000, // 1 MB
            skip_binary: true,
        }
    }
}

/// Walk a project directory and collect FileInfo for all source files.
///
/// Uses the `ignore` crate (from ripgrep) which respects .gitignore,
/// .ignore, and global gitignore patterns. Applies our default ignore
/// patterns on top plus any user-configured extras.
pub fn walk_project(root: &Path, config: &WalkerConfig) -> Result<Vec<FileInfo>> {
    if !root.is_dir() {
        return Err(Error::InvalidPath(format!(
            "Not a directory: {}",
            root.display()
        )));
    }

    // Pre-compile ignore globs from default + user patterns
    let ignore_globs: Vec<glob::Pattern> = DEFAULT_IGNORE_PATTERNS
        .iter()
        .copied()
        .chain(config.extra_ignore_patterns.iter().map(|s| s.as_str()))
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true) // skip hidden files by default
        .follow_links(false) // Security: never follow symlinks outside project root
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        .filter_entry(move |entry: &ignore::DirEntry| {
            let name = entry.file_name().to_string_lossy();
            // Prune entire directories and files matching ignore patterns
            !ignore_globs.iter().any(|g| g.matches(&name))
        });

    let mut files = Vec::new();

    for entry in builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip directories (they pass filter_entry but we only collect files)
        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            continue;
        }

        let path = entry.path().to_path_buf();

        // Skip files over size limit
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.len() > config.max_file_size_bytes {
            continue;
        }

        // Detect language
        let language = detect_language(&path);

        // Check binary
        let is_binary = check_binary(&path);
        if config.skip_binary && is_binary {
            continue;
        }

        // Compute content hash and line count
        let (content_hash, line_count) = match compute_file_stats(&path) {
            Ok(stats) => stats,
            Err(_) => continue,
        };

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        files.push(FileInfo {
            path,
            relative_path,
            language,
            size_bytes: metadata.len(),
            line_count,
            is_binary,
            content_hash,
        });
    }

    // Sort by relative path for deterministic output
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(files)
}

/// Check if a file appears to be binary by inspecting its first bytes.
fn check_binary(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let preview = if bytes.len() > 8192 {
        &bytes[..8192]
    } else {
        &bytes
    };
    content_inspector::inspect(preview).is_binary()
}

/// Compute xxh3 hash and line count for a file.
fn compute_file_stats(path: &Path) -> Result<(u64, usize)> {
    let content = std::fs::read(path)?;
    let hash = xxhash_rust::xxh3::xxh3_64(&content);
    let line_count = bytecount_lines(&content);
    Ok((hash, line_count))
}

/// Count lines in a file. A file with content always has at least 1 line,
/// even without a trailing newline.
fn bytecount_lines(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    let newlines = data.iter().filter(|&&b| b == b'\n').count();
    // If the last byte isn't a newline, the file has one more line than newlines
    if data.last() != Some(&b'\n') {
        newlines + 1
    } else {
        newlines
    }
}

/// Get the list of all default ignore patterns as owned strings.
pub fn default_ignore_patterns() -> Vec<String> {
    DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()
}
