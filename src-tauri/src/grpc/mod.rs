mod client;

pub mod data {
    include!("generated/jobworkerp.data.rs");
}

pub mod service {
    include!("generated/jobworkerp.service.rs");
}

pub use client::JobworkerpClient;
