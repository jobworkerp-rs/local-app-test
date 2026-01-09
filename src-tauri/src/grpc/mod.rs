pub mod data {
    include!("generated/jobworkerp.data.rs");
}

pub mod service {
    include!("generated/jobworkerp.service.rs");
}

pub mod client;

pub use client::{default_grpc_url, JobworkerpClient, McpServerInfo};
