use std::collections::HashMap;
use std::sync::Arc;

use tokio_stream::StreamExt;

use crate::error::AppError;

// Re-export jobworkerp-client types
use jobworkerp_client::client::helper::UseJobworkerpClientHelper;
pub use jobworkerp_client::client::wrapper::JobworkerpClientWrapper;
pub use jobworkerp_client::client::UseJobworkerpClient;
use jobworkerp_client::jobworkerp::data::{
    QueueType, ResponseType, RetryPolicy, RetryType, Runner, RunnerType, Worker, WorkerData,
};
use jobworkerp_client::proto::JobworkerpProto;

use command_utils::protobuf::ProtobufDescriptor;

// Re-export data types for convenience
use super::data;

/// Extended client wrapper with additional helper methods for local-code-agent
pub struct LocalCodeAgentClient {
    inner: JobworkerpClientWrapper,
    metadata: Arc<HashMap<String, String>>,
}

impl LocalCodeAgentClient {
    /// Create a new client
    pub async fn new(url: &str) -> Result<Self, AppError> {
        let inner = JobworkerpClientWrapper::new(url, Some(1800))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        // Build metadata from environment (auth token if configured)
        let mut metadata = HashMap::new();
        if let Ok(token) = std::env::var("JOBWORKERP_AUTH_TOKEN") {
            metadata.insert("jobworkerp-auth".to_string(), token);
        }

        Ok(Self {
            inner,
            metadata: Arc::new(metadata),
        })
    }

    /// Create a new client wrapped in Arc for shared ownership
    pub async fn new_shared(url: &str) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(Self::new(url).await?))
    }

    /// Get the server address
    pub fn address(&self) -> &str {
        self.inner.address()
    }

    /// Check connection to jobworkerp-rs
    pub async fn check_connection(&self) -> Result<bool, AppError> {
        // Try to find any runner as a health check
        self.inner
            .find_runner_by_name(None, self.metadata.clone(), "COMMAND")
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;
        Ok(true)
    }

    /// Execute a workflow with automatic worker creation
    ///
    /// This method uses jobworkerp-client's execute_workflow which:
    /// 1. Finds the WORKFLOW runner
    /// 2. Creates a worker if needed
    /// 3. Enqueues the job and waits for result
    pub async fn execute_workflow(
        &self,
        workflow_url: &str,
        input: &str,
        channel: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        self.inner
            .execute_workflow(None, self.metadata.clone(), workflow_url, input, channel)
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))
    }

    /// Execute a workflow and return job ID for streaming
    ///
    /// Unlike execute_workflow which waits for completion, this method
    /// returns immediately with a job ID that can be used for streaming results.
    pub async fn enqueue_workflow_for_stream(
        &self,
        workflow_url: &str,
        input: &str,
        channel: Option<&str>,
    ) -> Result<(String, tonic::Streaming<data::ResultOutputItem>), AppError> {
        use jobworkerp_client::jobworkerp::service::JobRequest;
        use serde_json::json;

        let using = Some("run");
        let job_args = json!({
            "workflow_url": workflow_url,
            "input": input,
        });

        // Find the WORKFLOW runner
        let runner = self
            .inner
            .find_runner_by_name(
                None,
                self.metadata.clone(),
                RunnerType::Workflow.as_str_name(),
            )
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("WORKFLOW runner not found".into()))?;

        let runner_data = runner
            .data
            .as_ref()
            .ok_or_else(|| AppError::Internal("Runner has no data".into()))?;

        let runner_id = runner
            .id
            .ok_or_else(|| AppError::Internal("Runner has no ID".into()))?;

        // Parse job args using schema
        let args_descriptor =
            JobworkerpProto::parse_job_args_schema_descriptor(runner_data, using)
                .map_err(|e| AppError::Internal(format!("Failed to parse args schema: {}", e)))?;

        let job_args_bytes = if let Some(desc) = args_descriptor {
            JobworkerpProto::json_value_to_message(desc, &job_args, true)
                .map_err(|e| AppError::Internal(format!("Failed to encode job args: {}", e)))?
        } else {
            serde_json::to_vec(&job_args)?
        };

        // Create worker data
        let worker_name = format!("workflow-{}", uuid_v7_string());
        let worker_data = WorkerData {
            name: worker_name.clone(),
            description: "Workflow execution worker".to_string(),
            runner_id: Some(runner_id),
            runner_settings: Vec::new(),
            retry_policy: Some(RetryPolicy {
                r#type: RetryType::Constant as i32,
                interval: 1000,
                max_retry: 3,
                max_interval: 0,
                basis: 2.0,
            }),
            periodic_interval: 0,
            channel: channel.map(|s| s.to_string()),
            queue_type: QueueType::Normal as i32,
            response_type: ResponseType::Direct as i32,
            store_success: false,
            store_failure: true,
            use_static: false,
            broadcast_results: true,
        };

        // Find or create worker
        let worker = self
            .inner
            .find_or_create_worker(None, self.metadata.clone(), &worker_data)
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        let worker_id = worker
            .id
            .ok_or_else(|| AppError::Internal("Worker has no ID".into()))?;

        // Enqueue job for streaming
        let job_request = JobRequest {
            worker: Some(
                jobworkerp_client::jobworkerp::service::job_request::Worker::WorkerId(worker_id),
            ),
            args: job_args_bytes,
            using: using.map(|s| s.to_string()),
            timeout: Some(1800 * 1000), // 30 minutes in milliseconds
            ..Default::default()
        };

        let response = self
            .inner
            .jobworkerp_client()
            .job_client()
            .await
            .enqueue_for_stream(tonic::Request::new(job_request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        // Extract job ID from response metadata
        let metadata = response.metadata();
        let job_id = metadata
            .get("x-job-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| worker_name.clone());

        Ok((job_id, response.into_inner()))
    }

    /// Listen to job result stream by job ID
    pub async fn listen_stream(
        &self,
        job_id: &str,
    ) -> Result<tonic::Streaming<data::ResultOutputItem>, AppError> {
        use jobworkerp_client::jobworkerp::service::ListenRequest;

        let job_id_value: i64 = job_id
            .parse()
            .map_err(|_| AppError::InvalidInput("Invalid job ID format".into()))?;

        let request = ListenRequest {
            job_id: Some(data::JobId {
                value: job_id_value,
            }),
            ..Default::default()
        };

        let response = self
            .inner
            .jobworkerp_client()
            .job_result_client()
            .await
            .listen_stream(tonic::Request::new(request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        Ok(response.into_inner())
    }

    /// Delete/cancel a job
    pub async fn delete_job(&self, job_id: &str) -> Result<(), AppError> {
        let job_id_value: i64 = job_id
            .parse()
            .map_err(|_| AppError::InvalidInput("Invalid job ID format".into()))?;

        let request = data::JobId {
            value: job_id_value,
        };

        self.inner
            .jobworkerp_client()
            .job_client()
            .await
            .delete(tonic::Request::new(request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        Ok(())
    }

    /// Find a runner by exact name match
    pub async fn find_runner_by_exact_name(&self, name: &str) -> Result<Option<Runner>, AppError> {
        self.inner
            .find_runner_by_name(None, self.metadata.clone(), name)
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))
    }

    /// Find a worker by exact name match
    pub async fn find_worker_by_exact_name(&self, name: &str) -> Result<Option<Worker>, AppError> {
        let result = self
            .inner
            .find_worker_by_name(None, self.metadata.clone(), name)
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        Ok(result.map(|(id, data)| Worker {
            id: Some(id),
            data: Some(data),
        }))
    }

    /// List MCP server runners
    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AppError> {
        use jobworkerp_client::jobworkerp::service::FindRunnerListRequest;

        let request = FindRunnerListRequest {
            runner_types: vec![RunnerType::McpServer as i32],
            ..Default::default()
        };

        let mut stream = self
            .inner
            .jobworkerp_client()
            .runner_client()
            .await
            .find_list_by(tonic::Request::new(request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?
            .into_inner();

        let mut servers = Vec::new();
        while let Some(runner) = stream.next().await {
            let runner = runner.map_err(|e| AppError::Grpc(e.to_string()))?;
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

    /// Call an MCP server tool
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

        // Get Runner info
        let runner = self
            .find_runner_by_exact_name(server_name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Runner '{}' not found", server_name)))?;

        let runner_data = runner
            .data
            .as_ref()
            .ok_or_else(|| AppError::Internal("Runner has no data".into()))?;

        let runner_id = runner
            .id
            .ok_or_else(|| AppError::Internal("Runner has no ID".into()))?;

        // Ensure worker exists
        let worker_data = WorkerData {
            name: server_name.to_string(),
            description: format!("Auto-created worker for MCP server '{}'", server_name),
            runner_id: Some(runner_id),
            runner_settings: Vec::new(),
            retry_policy: Some(RetryPolicy {
                r#type: RetryType::Constant as i32,
                interval: 1000,
                max_retry: 3,
                max_interval: 0,
                basis: 2.0,
            }),
            periodic_interval: 0,
            channel: None,
            queue_type: QueueType::Normal as i32,
            response_type: ResponseType::Direct as i32,
            store_success: false,
            store_failure: true,
            use_static: true, // Keep the worker for reuse
            broadcast_results: true,
        };

        let worker = self
            .inner
            .find_or_create_worker(None, self.metadata.clone(), &worker_data)
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        let worker_id = worker
            .id
            .ok_or_else(|| AppError::Internal("Worker has no ID".into()))?;

        // Encode args
        let args_bytes = serde_json::to_vec(args)?;

        // Enqueue job
        use jobworkerp_client::jobworkerp::service::JobRequest;

        let request = JobRequest {
            worker: Some(
                jobworkerp_client::jobworkerp::service::job_request::Worker::WorkerId(worker_id),
            ),
            args: args_bytes,
            using: Some(tool_name.to_string()),
            ..Default::default()
        };

        let response = self
            .inner
            .jobworkerp_client()
            .job_client()
            .await
            .enqueue_for_stream(tonic::Request::new(request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        let mut stream = response.into_inner();

        // Collect stream data
        let mut result_bytes = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| AppError::Grpc(e.to_string()))?;
            match item.item {
                Some(data::result_output_item::Item::Data(data)) => {
                    result_bytes.extend(data);
                }
                Some(data::result_output_item::Item::FinalCollected(data)) => {
                    result_bytes = data;
                }
                Some(data::result_output_item::Item::End(_)) => {
                    break;
                }
                None => {}
            }
        }

        // Decode result
        if result_bytes.is_empty() {
            return Ok(serde_json::json!(null));
        }

        let result_descriptor =
            JobworkerpProto::parse_result_schema_descriptor(runner_data, Some(tool_name))
                .map_err(|e| AppError::Internal(format!("Failed to parse result schema: {}", e)))?;

        match result_descriptor {
            Some(desc) => {
                let dynamic_message =
                    ProtobufDescriptor::get_message_from_bytes(desc, &result_bytes).map_err(
                        |e| AppError::Internal(format!("Failed to decode protobuf: {}", e)),
                    )?;

                ProtobufDescriptor::message_to_json_value(&dynamic_message)
                    .map_err(|e| AppError::Internal(format!("Failed to convert to JSON: {}", e)))
            }
            None => serde_json::from_slice(&result_bytes)
                .map_err(|e| AppError::Internal(format!("Failed to parse as JSON: {}", e))),
        }
    }

    /// Create a new MCP server runner
    pub async fn create_runner(
        &self,
        name: &str,
        description: &str,
        definition: &str,
    ) -> Result<i64, AppError> {
        use jobworkerp_client::jobworkerp::service::CreateRunnerRequest;

        let request = CreateRunnerRequest {
            name: name.to_string(),
            description: description.to_string(),
            runner_type: RunnerType::McpServer as i32,
            definition: definition.to_string(),
        };

        let response = self
            .inner
            .jobworkerp_client()
            .runner_client()
            .await
            .create(tonic::Request::new(request))
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;

        let id = response
            .into_inner()
            .id
            .ok_or_else(|| AppError::Grpc("No runner ID returned".into()))?;

        Ok(id.value)
    }
}

/// MCP Server information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub description: Option<String>,
    pub runner_type: String,
}

/// Generate a UUID v7 string (time-ordered UUID)
fn uuid_v7_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let random: u64 = rand::random();
    format!("{:016x}{:016x}", timestamp, random)
}

/// Get default gRPC URL from environment or fallback
pub fn default_grpc_url() -> String {
    std::env::var("JOBWORKERP_GRPC_URL").unwrap_or_else(|_| "http://localhost:9000".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_grpc_url() {
        let url = default_grpc_url();
        assert!(!url.is_empty());
    }

    #[test]
    fn test_uuid_v7_string() {
        let uuid = uuid_v7_string();
        assert_eq!(uuid.len(), 32);
    }
}
