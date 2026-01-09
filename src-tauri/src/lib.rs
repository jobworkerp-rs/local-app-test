mod commands;
mod crypto;
mod db;
mod error;
mod grpc;
mod state;

use state::AppState;
use std::sync::Arc;

pub use crypto::TokenCrypto;
pub use db::Database;
pub use error::{AppError, AppResult};
pub use grpc::JobworkerpClient;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn get_app_data_dir() -> String {
    directories::ProjectDirs::from("com", "local-code-agent", "LocalCodeAgent")
        .map(|dirs| dirs.data_dir().to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = get_app_data_dir();
    let db_path = format!("{}/local-code-agent.db", data_dir);
    let grpc_url = "http://localhost:9000";

    let app_state = AppState::new(&db_path, grpc_url)
        .expect("Failed to initialize application state");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(app_state))
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::get_settings,
            commands::update_settings,
            commands::list_repositories,
            commands::create_repository,
            commands::delete_repository,
            commands::list_jobs,
            commands::get_job,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
