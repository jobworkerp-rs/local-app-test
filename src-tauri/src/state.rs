use std::sync::Arc;

use crate::crypto::TokenCrypto;
use crate::db::DbPool;
use crate::error::AppError;
use crate::grpc::{default_grpc_url, JobworkerpClient};

/// Application state shared across Tauri commands
pub struct AppState {
    pub db: DbPool,
    pub crypto: TokenCrypto,
    pub grpc: Arc<JobworkerpClient>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: DbPool, grpc_url: Option<&str>) -> Result<Self, AppError> {
        let crypto = TokenCrypto::new().map_err(|e| AppError::Crypto(e.to_string()))?;

        let default_url = default_grpc_url();
        let url = grpc_url.unwrap_or(&default_url);
        let grpc = JobworkerpClient::new_shared(url)?;

        Ok(Self { db, crypto, grpc })
    }

    /// Initialize with default configuration
    pub fn init() -> Result<Self, AppError> {
        let db = crate::db::init_database(None)?;
        Self::new(db, None)
    }

    /// Initialize with custom database path and gRPC URL
    pub fn init_with_config(
        db_path: Option<&std::path::Path>,
        grpc_url: Option<&str>,
    ) -> Result<Self, AppError> {
        let db = crate::db::init_database(db_path)?;
        Self::new(db, grpc_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_app_state_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let state = AppState::init_with_config(Some(&db_path), Some("http://localhost:9000"));
        assert!(state.is_ok());
    }
}
