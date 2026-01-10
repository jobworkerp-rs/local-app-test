use std::sync::Arc;
use tauri::State;

use crate::error::AppError;
use crate::grpc::LocalCodeAgentClient;

/// Check connection to jobworkerp-rs backend
#[tauri::command]
pub async fn check_jobworkerp_connection(
    grpc: State<'_, Arc<LocalCodeAgentClient>>,
) -> Result<bool, AppError> {
    grpc.check_connection().await
}
