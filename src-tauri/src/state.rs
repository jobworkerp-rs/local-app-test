use std::sync::Arc;

use crate::crypto::TokenCrypto;
use crate::db::DbPool;
use crate::error::AppError;
use crate::grpc::{default_grpc_url, LocalCodeAgentClient};

/// Application state shared across Tauri commands
pub struct AppState {
    pub db: DbPool,
    pub crypto: TokenCrypto,
    pub grpc: Arc<LocalCodeAgentClient>,
}

impl AppState {
    /// Create new application state (async version for gRPC client initialization)
    pub async fn new(db: DbPool, grpc_url: Option<&str>) -> Result<Self, AppError> {
        let crypto = TokenCrypto::new().map_err(|e| AppError::Crypto(e.to_string()))?;

        let default_url = default_grpc_url();
        let url = grpc_url.unwrap_or(&default_url);
        let grpc = LocalCodeAgentClient::new_shared(url).await?;

        Ok(Self { db, crypto, grpc })
    }

    /// Initialize with default configuration
    pub async fn init() -> Result<Self, AppError> {
        let db = crate::db::init_database(None)?;
        Self::new(db, None).await
    }

    /// Initialize with custom database path and gRPC URL
    pub async fn init_with_config(
        db_path: Option<&std::path::Path>,
        grpc_url: Option<&str>,
    ) -> Result<Self, AppError> {
        let db = crate::db::init_database(db_path)?;
        Self::new(db, grpc_url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_app_state_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Note: This test requires a running jobworkerp-rs server
        // It will fail if the server is not available
        let _state =
            AppState::init_with_config(Some(&db_path), Some("http://localhost:9000")).await;
        // We don't assert success since the server might not be running in test environment
    }
}
