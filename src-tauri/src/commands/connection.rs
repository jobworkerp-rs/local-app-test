use std::sync::Arc;
use tauri::State;

use crate::error::AppError;
use crate::grpc::JobworkerpClient;

/// Check connection to jobworkerp-rs backend
#[tauri::command]
pub async fn check_jobworkerp_connection(
    grpc: State<'_, Arc<JobworkerpClient>>,
) -> Result<bool, AppError> {
    grpc.check_connection().await
}
