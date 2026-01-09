use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crypto::TokenCrypto;
use crate::db::Database;
use crate::error::AppResult;
use crate::grpc::JobworkerpClient;

pub struct AppState {
    pub db: Database,
    pub crypto: TokenCrypto,
    pub grpc: Arc<RwLock<Option<JobworkerpClient>>>,
    grpc_url: RwLock<String>,
}

impl AppState {
    pub fn new(db_path: &str, grpc_url: &str) -> AppResult<Self> {
        let db = Database::new(db_path)?;
        let crypto = TokenCrypto::new()?;

        Ok(Self {
            db,
            crypto,
            grpc: Arc::new(RwLock::new(None)),
            grpc_url: RwLock::new(grpc_url.to_string()),
        })
    }

    pub async fn connect_grpc(&self) -> AppResult<()> {
        let url = self.grpc_url.read().await.clone();
        let client = JobworkerpClient::connect(&url).await?;
        *self.grpc.write().await = Some(client);
        Ok(())
    }

    pub async fn get_grpc_client(&self) -> AppResult<JobworkerpClient> {
        let guard = self.grpc.read().await;
        guard
            .clone()
            .ok_or_else(|| crate::error::AppError::Grpc("gRPC client not connected".to_string()))
    }

    pub async fn update_grpc_url(&self, url: &str) -> AppResult<()> {
        *self.grpc_url.write().await = url.to_string();
        self.connect_grpc().await
    }
}
