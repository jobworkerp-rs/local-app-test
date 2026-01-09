use std::sync::Arc;
use tauri::State;

use crate::db::{get_repository_by_id, DbPool, Issue, Platform};
use crate::error::AppError;
use crate::grpc::JobworkerpClient;

/// Get the MCP tool name for listing issues based on platform
fn get_list_issues_tool(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "list_issues",
        Platform::Gitea => "list_repo_issues",
    }
}

/// Get the MCP tool name for reading a single issue based on platform
fn get_read_issue_tool(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "issue_read",
        Platform::Gitea => "get_issue_by_index",
    }
}

/// Parse issue from MCP result JSON (handles both GitHub and Gitea formats)
fn parse_issue(value: &serde_json::Value) -> Option<Issue> {
    let number_i64 = value.get("number")?.as_i64()?;
    let number: i32 = number_i64.try_into().ok()?;
    let title = value.get("title")?.as_str()?.to_string();
    let body = value.get("body").and_then(|v| v.as_str()).map(String::from);
    let state = value
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("open")
        .to_string();

    // Labels can be array of strings or array of objects with "name" field
    let labels = value
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| {
                    l.as_str()
                        .map(String::from)
                        .or_else(|| l.get("name").and_then(|n| n.as_str()).map(String::from))
                })
                .collect()
        })
        .unwrap_or_default();

    // User can be a string or object with "login" field
    let user = value
        .get("user")
        .and_then(|u| {
            u.as_str()
                .map(String::from)
                .or_else(|| u.get("login").and_then(|l| l.as_str()).map(String::from))
        })
        .unwrap_or_default();

    let html_url = value
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let created_at = value
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let updated_at = value
        .get("updated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(Issue {
        number,
        title,
        body,
        state,
        labels,
        user,
        html_url,
        created_at,
        updated_at,
    })
}

/// Extract issues from MCP result
/// MCP results typically have a "content" array with text content
fn extract_issues_from_result(result: &serde_json::Value) -> Result<Vec<Issue>, AppError> {
    // First, try to extract from MCP content structure
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                // Parse the text as JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(arr) = parsed.as_array() {
                        return Ok(arr.iter().filter_map(parse_issue).collect());
                    }
                }
            }
        }
    }

    // Direct array format
    if let Some(arr) = result.as_array() {
        return Ok(arr.iter().filter_map(parse_issue).collect());
    }

    // Single issue
    if result.get("number").is_some() {
        if let Some(issue) = parse_issue(result) {
            return Ok(vec![issue]);
        }
    }

    Ok(vec![])
}

/// List issues for a repository via MCP server
#[tauri::command]
pub async fn list_issues(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<JobworkerpClient>>,
    repository_id: i64,
    state: Option<String>,
) -> Result<Vec<Issue>, AppError> {
    let repo = get_repository_by_id(&db, repository_id)?;
    let tool_name = get_list_issues_tool(repo.platform);

    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "state": state.unwrap_or_else(|| "open".to_string()),
    });

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;
    extract_issues_from_result(&result)
}

/// Get a single issue by number
#[tauri::command]
pub async fn get_issue(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<JobworkerpClient>>,
    repository_id: i64,
    issue_number: i32,
) -> Result<Issue, AppError> {
    let repo = get_repository_by_id(&db, repository_id)?;
    let tool_name = get_read_issue_tool(repo.platform);

    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "issue_number": issue_number,
    });

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;

    // Try to extract from MCP content structure first
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(issue) = parse_issue(&parsed) {
                        return Ok(issue);
                    }
                }
            }
        }
    }

    // Direct format
    parse_issue(&result)
        .ok_or_else(|| AppError::NotFound(format!("Issue #{} not found", issue_number)))
}
