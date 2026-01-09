mod commands;
mod crypto;
mod db;
mod error;
mod grpc;
mod state;

use state::AppState;
use std::sync::Arc;
use tracing::{error, info};

pub use crypto::TokenCrypto;
pub use db::Database;
pub use error::{AppError, AppResult};
pub use grpc::JobworkerpClient;

const DEFAULT_GRPC_URL: &str = "http://localhost:9000";

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn get_app_data_dir() -> AppResult<String> {
    directories::ProjectDirs::from("com", "local-code-agent", "LocalCodeAgent")
        .map(|dirs| dirs.data_dir().to_string_lossy().to_string())
        .ok_or_else(|| {
            AppError::Internal("Failed to determine application data directory".to_string())
        })
}

fn get_grpc_url() -> String {
    std::env::var("GRPC_URL").unwrap_or_else(|_| DEFAULT_GRPC_URL.to_string())
}

fn initialize_app() -> Result<Arc<AppState>, String> {
    let data_dir = get_app_data_dir().map_err(|e| e.to_string())?;
    let db_path = format!("{}/local-code-agent.db", data_dir);
    let grpc_url = get_grpc_url();

    info!("Initializing application with db_path: {}", db_path);
    info!("Using gRPC server URL: {}", grpc_url);

    let app_state = AppState::new(&db_path, &grpc_url).map_err(|e| {
        error!("Failed to initialize application state: {}", e);
        format!("Failed to initialize application: {}", e)
    })?;

    Ok(Arc::new(app_state))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing (ignore if already initialized to avoid panic on double initialization)
    if let Err(e) = tracing_subscriber::fmt::try_init() {
        eprintln!("Tracing already initialized or failed: {}", e);
    }

    let app_state = match initialize_app() {
        Ok(state) => state,
        Err(e) => {
            error!("Application initialization failed: {}", e);
            // Desktop platforms: exit gracefully with error message
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            {
                use std::process::exit;
                eprintln!("Error: {}", e);
                exit(1);
            }
            // Mobile platforms: panic to trigger crash reporting
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                panic!("Application initialization failed: {}", e);
            }
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
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
