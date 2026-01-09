use std::sync::Arc;
use tauri::State;

use crate::error::AppError;
use crate::grpc::{JobworkerpClient, McpServerInfo};

/// List configured MCP servers from jobworkerp-rs
#[tauri::command]
pub async fn mcp_list_servers(
    grpc: State<'_, Arc<JobworkerpClient>>,
) -> Result<Vec<McpServerInfo>, AppError> {
    grpc.list_mcp_servers().await
}

/// Check MCP server connection
#[tauri::command]
pub async fn mcp_check_connection(
    server_name: String,
    grpc: State<'_, Arc<JobworkerpClient>>,
) -> Result<bool, AppError> {
    // Check if server exists by finding the worker
    let worker = grpc.find_worker_by_name(&server_name).await?;
    Ok(worker.is_some())
}
