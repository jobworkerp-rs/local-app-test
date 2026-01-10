// Allow dead code for modules under development
#![allow(dead_code)]

mod commands;
mod crypto;
mod db;
mod error;
mod grpc;
mod state;

use dotenvy::dotenv;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Local Code Agent");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize application state inside setup hook where Tokio runtime is available
            let app_state = AppState::init().map_err(|e| {
                tracing::error!("Failed to initialize application state: {:?}", e);
                e.to_string()
            })?;

            // Register shared state
            app.manage(app_state.db);
            app.manage(app_state.grpc);
            app.manage(app_state.crypto);

            Ok(())
        })
        // Register commands
        .invoke_handler(tauri::generate_handler![
            commands::check_jobworkerp_connection,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::mcp_list_servers,
            commands::mcp_check_connection,
            commands::mcp_create_runner,
            commands::list_jobs,
            commands::get_job,
            commands::list_repositories,
            commands::get_repository,
            commands::create_repository,
            commands::delete_repository,
            commands::list_issues,
            commands::get_issue,
            commands::list_pulls,
            commands::find_related_prs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
