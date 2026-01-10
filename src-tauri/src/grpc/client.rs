use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};

use crate::error::AppError;

// Generated proto modules
use super::data;
use super::service::{
    job_result_service_client::JobResultServiceClient, job_service_client::JobServiceClient,
    runner_service_client::RunnerServiceClient, worker_service_client::WorkerServiceClient,
    CreateRunnerRequest, FindRunnerListRequest, FindWorkerListRequest, JobRequest, ListenRequest,
    RunnerNameRequest, WorkerNameRequest,
};

// jobworkerp-client for dynamic protobuf decoding
use command_utils::protobuf::ProtobufDescriptor;
use jobworkerp_client::proto::JobworkerpProto;

/// gRPC client for jobworkerp-rs
///
/// Uses lazy channel initialization to avoid requiring Tokio runtime at construction time.
pub struct JobworkerpClient {
    endpoint: Endpoint,
    channel: OnceCell<Channel>,
    auth_metadata: Option<MetadataValue<tonic::metadata::Ascii>>,
}

impl JobworkerpClient {
    /// Create a new client with deferred connection
    ///
    /// The actual gRPC channel is created lazily on first use to avoid
    /// requiring a Tokio runtime at construction time.
    pub fn new(url: &str) -> Result<Self, AppError> {
        let endpoint =
            Endpoint::from_shared(url.to_string()).map_err(|e| AppError::Config(e.to_string()))?;

        // Parse auth token at construction time to fail early on invalid tokens
        let auth_metadata = match std::env::var("JOBWORKERP_AUTH_TOKEN") {
            Ok(token) => {
                let value: MetadataValue<tonic::metadata::Ascii> = token
                    .parse()
                    .map_err(|e| AppError::Config(format!("Invalid auth token format: {}", e)))?;
                Some(value)
            }
            Err(_) => None,
        };

        Ok(Self {
            endpoint,
            channel: OnceCell::new(),
            auth_metadata,
        })
    }

    /// Create a new client wrapped in Arc for shared ownership
    pub fn new_shared(url: &str) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(Self::new(url)?))
    }

    /// Get or create the gRPC channel lazily
    async fn get_channel(&self) -> Channel {
        self.channel
            .get_or_init(|| async { self.endpoint.connect_lazy() })
            .await
            .clone()
    }

    /// Get a JobService client
    async fn job_client(&self) -> JobServiceClient<Channel> {
        JobServiceClient::new(self.get_channel().await)
    }

    /// Get a JobResultService client
    async fn result_client(&self) -> JobResultServiceClient<Channel> {
        JobResultServiceClient::new(self.get_channel().await)
    }

    /// Get a WorkerService client
    async fn worker_client(&self) -> WorkerServiceClient<Channel> {
        WorkerServiceClient::new(self.get_channel().await)
    }

    /// Get a RunnerService client
    async fn runner_client(&self) -> RunnerServiceClient<Channel> {
        RunnerServiceClient::new(self.get_channel().await)
    }

    /// Add auth header to request if token is configured
    fn add_auth_header<T>(&self, mut request: tonic::Request<T>) -> tonic::Request<T> {
        if let Some(value) = &self.auth_metadata {
            request
                .metadata_mut()
                .insert("jobworkerp-auth", value.clone());
        }
        request
    }

    /// Check connection to jobworkerp-rs
    pub async fn check_connection(&self) -> Result<bool, AppError> {
        let mut client = self.worker_client().await;
        let request = self.add_auth_header(tonic::Request::new(FindWorkerListRequest {
            limit: Some(1),
            ..Default::default()
        }));

        client.find_list(request).await?;
        Ok(true)
    }

    /// Enqueue a job and return job ID
    pub async fn enqueue_job(
        &self,
        worker_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, AppError> {
        let mut client = self.job_client().await;

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
            .ok_or_else(|| AppError::Grpc("No job ID returned".into()))?;

        Ok(job_id.value.to_string())
    }

    /// Enqueue a job and stream results
    ///
    /// Returns only the stream. If you need the job_id, use `enqueue_job()` followed
    /// by `listen_stream()` instead. The job_id can be extracted from the first
    /// `ResultOutputItem` in the stream if needed by the caller.
    pub async fn enqueue_for_stream(
        &self,
        worker_name: &str,
        args: &serde_json::Value,
    ) -> Result<tonic::Streaming<data::ResultOutputItem>, AppError> {
        let mut client = self.job_client().await;

        let request = JobRequest {
            worker: Some(super::service::job_request::Worker::WorkerName(
                worker_name.to_string(),
            )),
            args: serde_json::to_vec(args)?,
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.enqueue_for_stream(req).await?;
        Ok(response.into_inner())
    }

    /// Listen to job result stream
    pub async fn listen_stream(
        &self,
        job_id: &str,
    ) -> Result<tonic::Streaming<data::ResultOutputItem>, AppError> {
        let mut client = self.result_client().await;

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
        let mut client = self.job_client().await;

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
        let mut client = self.worker_client().await;

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

    /// Call an MCP server tool and return the result as JSON
    ///
    /// This method enqueues a job to call an MCP tool and waits for the result.
    /// The `using` field specifies which tool to call on the MCP server.
    ///
    /// Worker auto-provisioning: If a Worker doesn't exist for the given MCP server,
    /// one will be automatically created (provided a Runner exists).
    ///
    /// Result decoding: The result is decoded using the result_proto schema from
    /// the Runner's method_proto_map, then converted to JSON.
    pub async fn call_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        tracing::debug!(
            "call_mcp_tool: server='{}', tool='{}'",
            server_name,
            tool_name
        );

        // Get Runner info for result_proto schema
        let runner = self
            .find_runner_by_exact_name(server_name)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Runner '{}' not found", server_name))
            })?;

        let runner_data = runner
            .data
            .as_ref()
            .ok_or_else(|| AppError::Internal("Runner has no data".into()))?;

        // Get result_proto descriptor for this tool
        let result_descriptor = JobworkerpProto::parse_result_schema_descriptor(
            runner_data,
            Some(tool_name),
        )
        .map_err(|e| AppError::Internal(format!("Failed to parse result schema: {}", e)))?;

        // Ensure worker exists (auto-create if needed)
        let worker = match self.ensure_mcp_worker(server_name).await {
            Ok(w) => {
                tracing::debug!("ensure_mcp_worker succeeded for '{}'", server_name);
                w
            }
            Err(e) => {
                tracing::error!("ensure_mcp_worker failed for '{}': {:?}", server_name, e);
                return Err(e);
            }
        };
        // Use worker_id for more reliable job submission (avoids name lookup issues)
        let worker_id = worker
            .id
            .ok_or_else(|| AppError::Internal("Worker has no ID".into()))?;

        tracing::debug!(
            "Using worker_id={} (name='{}') for enqueue",
            worker_id.value,
            worker
                .data
                .as_ref()
                .map(|d| d.name.as_str())
                .unwrap_or(server_name)
        );

        let mut client = self.job_client().await;

        let request = JobRequest {
            worker: Some(super::service::job_request::Worker::WorkerId(worker_id)),
            args: serde_json::to_vec(args)?,
            using: Some(tool_name.to_string()),
            ..Default::default()
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.enqueue_for_stream(req).await?;
        let mut stream = response.into_inner();

        // Collect stream data
        let mut result_bytes = Vec::new();
        while let Some(item) = stream.message().await? {
            match item.item {
                Some(data::result_output_item::Item::Data(data)) => {
                    result_bytes.extend(data);
                }
                Some(data::result_output_item::Item::FinalCollected(data)) => {
                    // Prefer final collected result if available
                    result_bytes = data;
                }
                Some(data::result_output_item::Item::End(_)) => {
                    // Stream ended
                    break;
                }
                None => {}
            }
        }

        // Decode result using result_proto schema
        if result_bytes.is_empty() {
            return Ok(serde_json::json!(null));
        }

        match result_descriptor {
            Some(desc) => {
                // Decode protobuf using dynamic schema
                let dynamic_message =
                    ProtobufDescriptor::get_message_from_bytes(desc, &result_bytes).map_err(
                        |e| {
                            tracing::error!("Failed to decode protobuf: {}", e);
                            AppError::Internal(format!("Failed to decode protobuf: {}", e))
                        },
                    )?;

                // Convert to JSON
                ProtobufDescriptor::message_to_json_value(&dynamic_message).map_err(|e| {
                    tracing::error!("Failed to convert protobuf to JSON: {}", e);
                    AppError::Internal(format!("Failed to convert to JSON: {}", e))
                })
            }
            None => {
                // No result_proto schema, try JSON fallback
                tracing::debug!(
                    "No result_proto for tool '{}', attempting JSON parse",
                    tool_name
                );
                serde_json::from_slice(&result_bytes).map_err(|e| {
                    let raw_content = String::from_utf8_lossy(&result_bytes);
                    tracing::error!(
                        "Failed to parse result as JSON: {}. Raw content: {}",
                        e,
                        raw_content
                    );
                    AppError::Internal(format!("Failed to parse as JSON: {}", e))
                })
            }
        }
    }

    /// List MCP server runners
    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AppError> {
        let mut client = self.runner_client().await;

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

    // ===== Runner Management =====

    /// Find a runner by exact name match
    pub async fn find_runner_by_exact_name(
        &self,
        name: &str,
    ) -> Result<Option<data::Runner>, AppError> {
        let mut client = self.runner_client().await;

        let request = RunnerNameRequest {
            name: name.to_string(),
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.find_by_name(req).await?.into_inner();

        Ok(response.data)
    }

    /// Create a new MCP server runner
    ///
    /// The definition should be a TOML string defining the MCP server configuration.
    pub async fn create_runner(
        &self,
        name: &str,
        description: &str,
        definition: &str,
    ) -> Result<i64, AppError> {
        let mut client = self.runner_client().await;

        let request = CreateRunnerRequest {
            name: name.to_string(),
            description: description.to_string(),
            runner_type: data::RunnerType::McpServer as i32,
            definition: definition.to_string(),
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.create(req).await?.into_inner();

        let id = response
            .id
            .ok_or_else(|| AppError::Grpc("No runner ID returned".into()))?;

        Ok(id.value)
    }

    // ===== Worker Management =====

    /// Find a worker by exact name match
    pub async fn find_worker_by_exact_name(
        &self,
        name: &str,
    ) -> Result<Option<data::Worker>, AppError> {
        let mut client = self.worker_client().await;

        let request = WorkerNameRequest {
            name: name.to_string(),
        };

        let req = self.add_auth_header(tonic::Request::new(request));
        let response = client.find_by_name(req).await?.into_inner();

        Ok(response.data)
    }

    /// Create a new worker
    pub async fn create_worker(&self, worker_data: data::WorkerData) -> Result<i64, AppError> {
        let mut client = self.worker_client().await;

        let req = self.add_auth_header(tonic::Request::new(worker_data));
        let response = client.create(req).await?.into_inner();

        let id = response
            .id
            .ok_or_else(|| AppError::Grpc("No worker ID returned".into()))?;

        Ok(id.value)
    }

    /// Ensure an MCP worker exists for the given MCP server name
    ///
    /// This method implements the automatic worker provisioning logic:
    /// 1. Worker lookup by name → return if exists
    /// 2. Runner lookup by name → error if not exists (Runner must be pre-registered)
    /// 3. Create Worker with same name as Runner
    /// 4. Return created Worker
    pub async fn ensure_mcp_worker(&self, mcp_server_name: &str) -> Result<data::Worker, AppError> {
        tracing::debug!("ensure_mcp_worker: checking for '{}'", mcp_server_name);

        // 1. Check if worker already exists
        if let Some(worker) = self.find_worker_by_exact_name(mcp_server_name).await? {
            tracing::debug!("Worker '{}' already exists", mcp_server_name);
            return Ok(worker);
        }

        tracing::debug!("Worker '{}' not found, looking for runner", mcp_server_name);

        // 2. Find the runner (must exist)
        let runner = self.find_runner_by_exact_name(mcp_server_name).await?;

        tracing::debug!(
            "Runner lookup result for '{}': {:?}",
            mcp_server_name,
            runner.is_some()
        );

        let runner = runner.ok_or_else(|| {
            tracing::error!(
                "MCP server runner '{}' not found. Please register the MCP server first.",
                mcp_server_name
            );
            AppError::NotFound(format!(
                "MCP server runner '{}' not found. Please register the MCP server first.",
                mcp_server_name
            ))
        })?;

        let runner_id = runner
            .id
            .ok_or_else(|| AppError::Internal("Runner has no ID".into()))?;

        tracing::info!(
            "Creating worker '{}' for runner ID {}",
            mcp_server_name,
            runner_id.value
        );

        // 3. Create the worker
        let worker_data = data::WorkerData {
            name: mcp_server_name.to_string(),
            description: format!("Auto-created worker for MCP server '{}'", mcp_server_name),
            runner_id: Some(runner_id),
            runner_settings: Vec::new(),
            retry_policy: Some(data::RetryPolicy {
                r#type: data::RetryType::Constant as i32,
                interval: 1000,
                max_retry: 3,
                max_interval: 0,
                basis: 2.0, // Required to be > 1.0 by server validation
            }),
            periodic_interval: 0,
            channel: None,
            queue_type: data::QueueType::Normal as i32,
            response_type: data::ResponseType::Direct as i32,
            store_success: false,
            store_failure: true,
            use_static: false,
            broadcast_results: true,
        };

        let worker_id = match self.create_worker(worker_data.clone()).await {
            Ok(id) => {
                tracing::info!(
                    "Successfully created worker '{}' with ID {}",
                    mcp_server_name,
                    id
                );
                id
            }
            Err(e) => {
                tracing::error!("Failed to create worker '{}': {:?}", mcp_server_name, e);
                return Err(e);
            }
        };

        // 4. Return the created worker
        Ok(data::Worker {
            id: Some(data::WorkerId { value: worker_id }),
            data: Some(worker_data),
        })
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
