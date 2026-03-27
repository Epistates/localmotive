use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file_info::{FileInfo, Language};
use super::readme::extract_readme_description;
use super::walker::{walk_project, WalkerConfig};
use crate::error::Result;

/// Project type markers and their corresponding files.
const PROJECT_MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("package.json", "node"),
    ("pyproject.toml", "python"),
    ("setup.py", "python"),
    ("go.mod", "go"),
    ("pom.xml", "java"),
    ("build.gradle", "java"),
    ("build.gradle.kts", "kotlin"),
    ("Gemfile", "ruby"),
    ("Package.swift", "swift"),
    ("mix.exs", "elixir"),
    ("stack.yaml", "haskell"),
    ("dune-project", "ocaml"),
    ("pubspec.yaml", "dart"),
    ("composer.json", "php"),
    ("build.zig", "zig"),
    (".sln", "dotnet"),
    (".csproj", "dotnet"),
    ("Makefile", "make"),
    ("CMakeLists.txt", "cmake"),
];

/// Summary of a detected project (before full scanning).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub path: PathBuf,
    pub name: String,
    pub project_type: String,
    pub has_readme: bool,
}

/// Full manifest of a scanned project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub id: String,
    pub root_path: PathBuf,
    pub name: String,
    pub description: String,
    pub languages: Vec<LanguageStats>,
    pub files: Vec<FileInfo>,
    pub total_size_bytes: u64,
    pub total_lines: usize,
    pub file_count: usize,
    pub project_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStats {
    pub language: Language,
    pub file_count: usize,
    pub total_lines: usize,
    pub total_bytes: u64,
}

/// Discover projects within a directory.
///
/// Looks at the top-level directory and immediate subdirectories for
/// project marker files (Cargo.toml, package.json, etc.). Returns a
/// summary for each detected project.
pub fn discover_projects(root: &Path) -> Result<Vec<ProjectSummary>> {
    let mut projects = Vec::new();

    // Check if the root itself is a project
    if let Some(project_type) = detect_project_type(root) {
        projects.push(ProjectSummary {
            name: dir_name(root),
            path: root.to_path_buf(),
            project_type,
            has_readme: has_readme(root),
        });
        return Ok(projects);
    }

    // Check immediate subdirectories
    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories and common non-project dirs
        let name = dir_name(&path);
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        if let Some(project_type) = detect_project_type(&path) {
            projects.push(ProjectSummary {
                name,
                path,
                project_type,
                has_readme: has_readme(&entry.path()),
            });
        }
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(projects)
}

/// Perform a full scan of a single project directory.
pub fn scan_project(root: &Path, config: &WalkerConfig) -> Result<ProjectManifest> {
    let files = walk_project(root, config)?;
    let description = extract_readme_description(root).unwrap_or_default();
    let project_type = detect_project_type(root).unwrap_or_else(|| "unknown".to_string());

    // Compute language statistics
    let mut lang_map: HashMap<Language, (usize, usize, u64)> = HashMap::new();
    let mut total_size: u64 = 0;
    let mut total_lines: usize = 0;

    for file in &files {
        total_size += file.size_bytes;
        total_lines += file.line_count;

        if let Some(lang) = &file.language {
            let entry = lang_map.entry(lang.clone()).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += file.line_count;
            entry.2 += file.size_bytes;
        }
    }

    let mut languages: Vec<LanguageStats> = lang_map
        .into_iter()
        .map(|(language, (file_count, total_lines, total_bytes))| LanguageStats {
            language,
            file_count,
            total_lines,
            total_bytes,
        })
        .collect();
    languages.sort_by(|a, b| b.file_count.cmp(&a.file_count));

    let file_count = files.len();

    Ok(ProjectManifest {
        id: uuid::Uuid::new_v4().to_string(),
        root_path: root.to_path_buf(),
        name: dir_name(root),
        description,
        languages,
        files,
        total_size_bytes: total_size,
        total_lines,
        file_count,
        project_type,
    })
}

/// Detect project type by checking for marker files.
fn detect_project_type(dir: &Path) -> Option<String> {
    for (marker, ptype) in PROJECT_MARKERS {
        if dir.join(marker).exists() {
            return Some(ptype.to_string());
        }
    }

    // Also check for .git as a fallback marker
    if dir.join(".git").exists() {
        return Some("git".to_string());
    }

    None
}

fn has_readme(dir: &Path) -> bool {
    dir.join("README.md").exists()
        || dir.join("readme.md").exists()
        || dir.join("README").exists()
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
