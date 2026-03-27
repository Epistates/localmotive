pub mod analyzer;
pub mod commands;
pub mod error;
pub mod export;
pub mod formatter;
pub mod generator;
pub mod pipeline;
pub mod quality;
pub mod scanner;
pub mod state;

use state::{AppState, AppStateInner, CancellationState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::new(AppStateInner::default()))
        .manage(std::sync::Mutex::new(CancellationState::new()))
        .invoke_handler(tauri::generate_handler![
            // Project commands
            commands::project::cmd_discover_projects,
            commands::project::cmd_scan_project,
            commands::project::cmd_get_project,
            commands::project::cmd_update_project_description,
            commands::project::cmd_extract_readme_description,
            commands::project::cmd_get_default_ignore_patterns,
            commands::project::cmd_pick_directory,
            // Config commands
            commands::config::cmd_get_config,
            commands::config::cmd_update_config,
            commands::config::cmd_get_formats,
            // Pipeline commands
            commands::pipeline::cmd_start_generation,
            commands::pipeline::cmd_cancel_generation,
            commands::pipeline::cmd_get_generation_status,
            // Preview commands
            commands::preview::cmd_preview_samples,
            commands::preview::cmd_get_statistics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
