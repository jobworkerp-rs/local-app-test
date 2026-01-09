use tonic::transport::Channel;

use super::service::{
    job_result_service_client::JobResultServiceClient, job_service_client::JobServiceClient,
    runner_service_client::RunnerServiceClient, worker_service_client::WorkerServiceClient,
};
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct JobworkerpClient {
    channel: Channel,
}

impl JobworkerpClient {
    pub async fn connect(url: &str) -> AppResult<Self> {
        let channel = Channel::from_shared(url.to_string())
            .map_err(|e| AppError::Grpc(e.to_string()))?
            .connect()
            .await?;

        Ok(Self { channel })
    }

    pub fn job_client(&self) -> JobServiceClient<Channel> {
        JobServiceClient::new(self.channel.clone())
    }

    pub fn result_client(&self) -> JobResultServiceClient<Channel> {
        JobResultServiceClient::new(self.channel.clone())
    }

    pub fn worker_client(&self) -> WorkerServiceClient<Channel> {
        WorkerServiceClient::new(self.channel.clone())
    }

    pub fn runner_client(&self) -> RunnerServiceClient<Channel> {
        RunnerServiceClient::new(self.channel.clone())
    }
}
