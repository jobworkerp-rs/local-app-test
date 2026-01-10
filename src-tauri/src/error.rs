use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<tonic::Status> for AppError {
    fn from(status: tonic::Status) -> Self {
        tracing::error!("gRPC error: {:?}", status);
        AppError::Grpc(status.message().to_string())
    }
}

impl From<tonic::transport::Error> for AppError {
    fn from(err: tonic::transport::Error) -> Self {
        tracing::error!("gRPC transport error: {:?}", err);
        AppError::Grpc(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON error: {:?}", err);
        AppError::InvalidInput(format!("Invalid JSON: {}", err))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Error: {:?}", err);
        AppError::Internal(err.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Generalize error messages to prevent information leakage
        let user_message = match self {
            AppError::Database(_) => "Database error occurred",
            AppError::Grpc(_) => "Backend communication failed",
            AppError::Crypto(_) => "Encryption error occurred",
            AppError::Io(_) => "File operation failed",
            AppError::InvalidInput(msg) => msg.as_str(),
            AppError::NotFound(msg) => msg.as_str(),
            AppError::Config(_) => "Configuration error",
            AppError::Internal(_) => "Internal error occurred",
        };
        serializer.serialize_str(user_message)
    }
}

pub type AppResult<T> = Result<T, AppError>;
