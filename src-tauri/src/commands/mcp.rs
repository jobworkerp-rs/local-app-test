use std::sync::Arc;
use tauri::State;
use url::Url;

use crate::error::AppError;
use crate::grpc::{JobworkerpClient, McpServerInfo};

/// List configured MCP servers from jobworkerp-rs
#[tauri::command]
pub async fn mcp_list_servers(
    grpc: State<'_, Arc<JobworkerpClient>>,
) -> Result<Vec<McpServerInfo>, AppError> {
    grpc.list_mcp_servers().await
}

/// Check MCP server connection
#[tauri::command]
pub async fn mcp_check_connection(
    server_name: String,
    grpc: State<'_, Arc<JobworkerpClient>>,
) -> Result<bool, AppError> {
    // Check if server exists by finding the worker
    let worker = grpc.find_worker_by_name(&server_name).await?;
    Ok(worker.is_some())
}

/// Create a new GitHub/Gitea MCP server (Runner) dynamically
///
/// The TOML definition is auto-generated based on the platform.
/// Docker execution format is used for MCP servers.
#[tauri::command]
pub async fn mcp_create_runner(
    grpc: State<'_, Arc<JobworkerpClient>>,
    platform: String,
    name: String,
    url: String,
    token: String,
) -> Result<McpServerInfo, AppError> {
    // Generate TOML definition based on platform
    let definition = match platform.as_str() {
        "GitHub" => github_mcp_toml(&url, &token)?,
        "Gitea" => gitea_mcp_toml(&url, &token)?,
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported platform: {}. Only 'GitHub' and 'Gitea' are supported.",
                platform
            )))
        }
    };

    let description = format!("{} MCP Server", platform);

    // Create runner via gRPC
    grpc.create_runner(&name, &description, &definition).await?;

    Ok(McpServerInfo {
        name,
        description: Some(description),
        runner_type: "MCP_SERVER".to_string(),
    })
}

/// Generate GitHub MCP Server TOML definition (Docker execution format)
///
/// Reference: https://github.com/github/github-mcp-server
/// Docker: `docker run -i --rm -e GITHUB_PERSONAL_ACCESS_TOKEN ghcr.io/github/github-mcp-server`
fn github_mcp_toml(url: &str, token: &str) -> Result<String, AppError> {
    let parsed =
        Url::parse(url).map_err(|e| AppError::InvalidInput(format!("Invalid URL: {}", e)))?;
    let host = parsed.host_str().unwrap_or("github.com");
    let is_ghes = host != "github.com";

    let mut args = vec![
        "run".to_string(),
        "-i".to_string(),
        "--rm".to_string(),
        "-e".to_string(),
        "GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),
    ];

    if is_ghes {
        args.push("-e".to_string());
        args.push("GITHUB_HOST".to_string());
    }

    args.push("ghcr.io/github/github-mcp-server".to_string());

    let args_toml = args
        .iter()
        .map(|a| format!("\"{}\"", a))
        .collect::<Vec<_>>()
        .join(", ");

    let mut toml = format!(
        r#"[server]
type = "stdio"
command = "docker"
args = [{args}]

[env]
GITHUB_PERSONAL_ACCESS_TOKEN = "{token}"
"#,
        args = args_toml,
        token = token
    );

    if is_ghes {
        toml.push_str(&format!("GITHUB_HOST = \"{}\"\n", url));
    }

    Ok(toml)
}

/// Generate Gitea MCP Server TOML definition (Docker execution format)
///
/// Reference: https://gitea.com/gitea/gitea-mcp
/// Docker: `docker run -i --rm -e GITEA_ACCESS_TOKEN -e GITEA_HOST docker.gitea.com/gitea-mcp-server`
fn gitea_mcp_toml(url: &str, token: &str) -> Result<String, AppError> {
    let parsed =
        Url::parse(url).map_err(|e| AppError::InvalidInput(format!("Invalid URL: {}", e)))?;
    let is_insecure = parsed.scheme() == "http";

    let mut args = vec![
        "run".to_string(),
        "-i".to_string(),
        "--rm".to_string(),
        "-e".to_string(),
        "GITEA_ACCESS_TOKEN".to_string(),
        "-e".to_string(),
        "GITEA_HOST".to_string(),
    ];

    if is_insecure {
        args.push("-e".to_string());
        args.push("GITEA_INSECURE".to_string());
    }

    args.push("docker.gitea.com/gitea-mcp-server".to_string());

    let args_toml = args
        .iter()
        .map(|a| format!("\"{}\"", a))
        .collect::<Vec<_>>()
        .join(", ");

    let mut toml = format!(
        r#"[server]
type = "stdio"
command = "docker"
args = [{args}]

[env]
GITEA_ACCESS_TOKEN = "{token}"
GITEA_HOST = "{url}"
"#,
        args = args_toml,
        token = token,
        url = url
    );

    if is_insecure {
        toml.push_str("GITEA_INSECURE = \"true\"\n");
    }

    Ok(toml)
}
