mod commands;
mod crypto;
mod db;
mod error;
mod grpc;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Local Code Agent");

    // Initialize application state
    let app_state = match AppState::init() {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("Failed to initialize application state: {:?}", e);
            panic!("Failed to initialize application: {:?}", e);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register shared state
        .manage(app_state.db)
        .manage(app_state.grpc)
        .manage(app_state.crypto)
        // Register commands
        .invoke_handler(tauri::generate_handler![
            commands::check_jobworkerp_connection,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::mcp_list_servers,
            commands::mcp_check_connection,
            commands::list_jobs,
            commands::get_job,
            commands::list_repositories,
            commands::create_repository,
            commands::delete_repository,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
