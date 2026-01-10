// Re-export proto types from jobworkerp-client crate
pub use jobworkerp_client::jobworkerp::data;
pub use jobworkerp_client::jobworkerp::service;

pub mod client;

pub use client::{default_grpc_url, JobworkerpClient, McpServerInfo};
