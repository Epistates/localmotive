use std::path::PathBuf;

use tauri::State;

use crate::error::Error;
use crate::scanner::{
    discover_projects, scan_project, ProjectManifest, ProjectSummary, WalkerConfig,
    default_ignore_patterns, default_sensitive_patterns,
};
use crate::state::AppState;

/// Discover projects within a directory.
/// Returns summaries (not full scans) for each detected project.
#[tauri::command]
pub async fn cmd_discover_projects(path: String) -> Result<Vec<ProjectSummary>, Error> {
    let path = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || discover_projects(&path))
        .await
        .map_err(|e| Error::Scanner(e.to_string()))?
}

/// Full scan of a single project directory.
/// Returns the complete manifest with all file info, language stats, and description.
#[tauri::command]
pub async fn cmd_scan_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectManifest, Error> {
    let path = PathBuf::from(&path);
    let config = WalkerConfig::default();

    let manifest = tokio::task::spawn_blocking(move || scan_project(&path, &config))
        .await
        .map_err(|e| Error::Scanner(e.to_string()))??;

    // Store in app state
    let mut state = state
        .lock()
        .map_err(|e| Error::Scanner(format!("State lock poisoned: {e}")))?;
    state
        .projects
        .insert(manifest.id.clone(), manifest.clone());

    Ok(manifest)
}

/// Get a previously scanned project by ID.
#[tauri::command]
pub async fn cmd_get_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<ProjectManifest, Error> {
    let state = state
        .lock()
        .map_err(|e| Error::Scanner(format!("State lock poisoned: {e}")))?;
    state
        .projects
        .get(&project_id)
        .cloned()
        .ok_or_else(|| Error::ProjectNotFound(project_id))
}

/// Update the description for a scanned project.
#[tauri::command]
pub async fn cmd_update_project_description(
    project_id: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<(), Error> {
    let mut state = state
        .lock()
        .map_err(|e| Error::Scanner(format!("State lock poisoned: {e}")))?;
    let project = state
        .projects
        .get_mut(&project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id))?;
    project.description = description;
    Ok(())
}

/// Extract the README description from a project path.
#[tauri::command]
pub async fn cmd_extract_readme_description(path: String) -> Result<Option<String>, Error> {
    let path = PathBuf::from(&path);
    Ok(crate::scanner::readme::extract_readme_description(&path))
}

/// Get the default ignore patterns.
#[tauri::command]
pub async fn cmd_get_default_ignore_patterns() -> Result<Vec<String>, Error> {
    Ok(default_ignore_patterns())
}

/// Get the default sensitive file patterns.
#[tauri::command]
pub async fn cmd_get_default_sensitive_patterns() -> Result<Vec<String>, Error> {
    Ok(default_sensitive_patterns())
}

/// Open a native directory picker dialog and return the selected path.
#[tauri::command]
pub async fn cmd_pick_directory(app: tauri::AppHandle) -> Result<Option<String>, Error> {
    use tauri_plugin_dialog::DialogExt;

    let result = app.dialog().file().blocking_pick_folder();
    Ok(result.map(|p| p.to_string()))
}
