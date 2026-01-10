#[allow(clippy::doc_overindented_list_items, clippy::doc_lazy_continuation)]
pub mod data {
    include!("generated/jobworkerp.data.rs");
}

#[allow(clippy::doc_overindented_list_items, clippy::doc_lazy_continuation)]
pub mod service {
    include!("generated/jobworkerp.service.rs");
}

pub mod client;

pub use client::{default_grpc_url, JobworkerpClient, McpServerInfo};
