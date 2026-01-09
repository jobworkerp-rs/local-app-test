use std::sync::Arc;
use tonic::transport::Channel;

use crate::error::AppError;

// Generated proto modules
use super::data;
use super::service::{
    job_result_service_client::JobResultServiceClient, job_service_client::JobServiceClient,
    runner_service_client::RunnerServiceClient, worker_service_client::WorkerServiceClient,
    FindRunnerListRequest, FindWorkerListRequest, JobRequest, ListenRequest,
};

/// gRPC client for jobworkerp-rs
#[derive(Clone)]
pub struct JobworkerpClient {
    channel: Channel,
    auth_token: Option<String>,
}

impl JobworkerpClient {
    /// Create a new client with lazy connection
    pub fn new(url: &str) -> Result<Self, AppError> {
        let channel = Channel::from_shared(url.to_string())
            .map_err(|e| AppError::ConfigError(e.to_string()))?
            .connect_lazy();

        let auth_token = std::env::var("JOBWORKERP_AUTH_TOKEN").ok();

        Ok(Self { channel, auth_token })
    }

    /// Create a new client wrapped in Arc for shared ownership
    pub fn new_shared(url: &str) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(Self::new(url)?))
    }

    /// Get a JobService client
    fn job_client(&self) -> JobServiceClient<Channel> {
        JobServiceClient::new(self.channel.clone())
    }

    /// Get a JobResultService client
    fn result_client(&self) -> JobResultServiceClient<Channel> {
        JobResultServiceClient::new(self.channel.clone())
    }

    /// Get a WorkerService client
    fn worker_client(&self) -> WorkerServiceClient<Channel> {
        WorkerServiceClient::new(self.channel.clone())
    }

    /// Get a RunnerService client
    fn runner_client(&self) -> RunnerServiceClient<Channel> {
        RunnerServiceClient::new(self.channel.clone())
    }

    /// Add auth header to request if token is configured
    fn add_auth_header<T>(&self, mut request: tonic::Request<T>) -> tonic::Request<T> {
        if let Some(token) = &self.auth_token {
            if let Ok(value) = token.parse() {
                request.metadata_mut().insert("jobworkerp-auth", value);
            }
        }
        request
    }

    /// Check connection to jobworkerp-rs
    pub async fn check_connection(&self) -> Result<bool, AppError> {
        let mut client = self.worker_client();
        let request = self.add_auth_header(tonic::Request::new(FindWorkerListRequest {
            limit: Some(1),
            ..Default::default()
        }));

        match client.find_list(request).await {
            Ok(_) => Ok(true),
            Err(status) => {
                tracing::warn!("Connection check failed: {:?}", status);
                Ok(false)
            }
        }
    }

    /// Enqueue a job and return job ID
    pub async fn enqueue_job(
        &self,
        worker_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, AppError> {
        let mut client = self.job_client();

        let request = JobRequest {
            worker: Some(super::service::job_request::Worker::WorkerName(
                worker_name.to_string(),
            )),
            args: serde_json::to_vec(args)?,
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.enqueue(req).await?;
        let job_id = response
            .into_inner()
            .id
            .ok_or_else(|| AppError::GrpcError("No job ID returned".into()))?;

        Ok(job_id.value.to_string())
    }

    /// Enqueue a job and stream results
    pub async fn enqueue_for_stream(
        &self,
        worker_name: &str,
        args: &serde_json::Value,
    ) -> Result<(String, tonic::Streaming<data::ResultOutputItem>), AppError> {
        let mut client = self.job_client();

        let request = JobRequest {
            worker: Some(super::service::job_request::Worker::WorkerName(
                worker_name.to_string(),
            )),
            args: serde_json::to_vec(args)?,
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.enqueue_for_stream(req).await?;
        let stream = response.into_inner();

        // Job ID is returned in the stream metadata or first message
        // For now, we generate a placeholder - actual implementation would parse from stream
        let job_id = "pending".to_string();

        Ok((job_id, stream))
    }

    /// Listen to job result stream
    pub async fn listen_stream(
        &self,
        job_id: &str,
    ) -> Result<tonic::Streaming<data::ResultOutputItem>, AppError> {
        let mut client = self.result_client();

        let request = ListenRequest {
            job_id: Some(data::JobId {
                value: job_id
                    .parse()
                    .map_err(|_| AppError::InvalidInput("Invalid job ID".into()))?,
            }),
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.listen_stream(req).await?;
        Ok(response.into_inner())
    }

    /// Delete/cancel a job
    pub async fn delete_job(&self, job_id: &str) -> Result<(), AppError> {
        let mut client = self.job_client();

        let request = data::JobId {
            value: job_id
                .parse()
                .map_err(|_| AppError::InvalidInput("Invalid job ID".into()))?,
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        client.delete(req).await?;
        Ok(())
    }

    /// Find a worker by name
    pub async fn find_worker_by_name(&self, name: &str) -> Result<Option<data::Worker>, AppError> {
        let mut client = self.worker_client();

        let request = FindWorkerListRequest {
            name_filter: Some(name.to_string()),
            limit: Some(1),
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let mut stream = client.find_list(req).await?.into_inner();

        // Return first matching worker
        if let Some(result) = stream.message().await? {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// List MCP server runners
    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AppError> {
        let mut client = self.runner_client();

        let request = FindRunnerListRequest {
            runner_types: vec![data::RunnerType::McpServer as i32],
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let mut stream = client.find_list_by(req).await?.into_inner();

        let mut servers = Vec::new();
        while let Some(runner) = stream.message().await? {
            if let Some(runner_data) = runner.data {
                servers.push(McpServerInfo {
                    name: runner_data.name,
                    description: Some(runner_data.description),
                    runner_type: "MCP_SERVER".to_string(),
                });
            }
        }

        Ok(servers)
    }
}

/// MCP Server information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub description: Option<String>,
    pub runner_type: String,
}

/// Get default gRPC URL from environment or fallback
pub fn default_grpc_url() -> String {
    std::env::var("JOBWORKERP_GRPC_URL").unwrap_or_else(|_| "http://localhost:9000".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = JobworkerpClient::new("http://localhost:9000");
        assert!(client.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let client = JobworkerpClient::new("not a valid url");
        assert!(client.is_err());
    }

    #[test]
    fn test_default_grpc_url() {
        let url = default_grpc_url();
        assert!(!url.is_empty());
    }
}
