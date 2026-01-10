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

/// Convert issue state to platform-specific format
/// Returns a vector of states to query (for "all" we need both open and closed)
/// GitHub MCP expects uppercase: "OPEN", "CLOSED"
/// Gitea MCP expects lowercase: "open", "closed"
fn normalize_issue_states(state: &str, platform: Platform) -> Vec<String> {
    let normalized = state.to_lowercase();
    match platform {
        Platform::GitHub => match normalized.as_str() {
            "all" => vec!["OPEN".to_string(), "CLOSED".to_string()],
            _ => vec![normalized.to_uppercase()],
        },
        Platform::Gitea => match normalized.as_str() {
            "all" => vec!["open".to_string(), "closed".to_string()],
            _ => vec![normalized],
        },
    }
}

/// Build issue URL from repository URL and issue number
fn build_issue_url(repo_url: &str, issue_number: i32, platform: Platform) -> String {
    let base = repo_url.trim_end_matches('/');
    match platform {
        Platform::GitHub => format!("{}/issues/{}", base, issue_number),
        Platform::Gitea => format!("{}/issues/{}", base, issue_number),
    }
}

/// Parse issue from MCP result JSON (handles both GitHub and Gitea formats)
fn parse_issue(value: &serde_json::Value, repo_url: &str, platform: Platform) -> Option<Issue> {
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

    // Use html_url from response if available, otherwise build from repo URL
    let html_url = value
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| build_issue_url(repo_url, number, platform));

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
/// Handles multiple formats:
/// 1. GitHub MCP: {"issues": [...], "pageInfo": {...}, "totalCount": N}
/// 2. MCP content structure: {"content": [{"text": "..."}]}
/// 3. Direct array: [...]
/// 4. Single issue object: {"number": ...}
fn extract_issues_from_result(
    result: &serde_json::Value,
    repo_url: &str,
    platform: Platform,
) -> Result<Vec<Issue>, AppError> {
    tracing::debug!("extract_issues_from_result: {:?}", result);

    // GitHub MCP format: {"issues": [...], "pageInfo": {...}}
    if let Some(issues_arr) = result.get("issues").and_then(|i| i.as_array()) {
        tracing::debug!("Found 'issues' field with {} items", issues_arr.len());
        return Ok(issues_arr
            .iter()
            .filter_map(|v| parse_issue(v, repo_url, platform))
            .collect());
    }

    // MCP content structure: {"content": [{"text": {"text": "..."}}]} or {"content": [{"text": "..."}]}
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        tracing::debug!("Found content array with {} items", content.len());
        for item in content {
            // Try nested text.text structure first (Protobuf decoded format)
            let text_str = item.get("text").and_then(|t| {
                // Try {"text": {"text": "..."}} format
                t.get("text")
                    .and_then(|inner| inner.as_str())
                    // Fallback to {"text": "..."} format
                    .or_else(|| t.as_str())
            });

            if let Some(text) = text_str {
                tracing::debug!("Found text content: {}", &text[..text.len().min(500)]);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    // Try GitHub format within text
                    if let Some(issues_arr) = parsed.get("issues").and_then(|i| i.as_array()) {
                        tracing::debug!(
                            "Parsed text contains 'issues' field with {} items",
                            issues_arr.len()
                        );
                        return Ok(issues_arr
                            .iter()
                            .filter_map(|v| parse_issue(v, repo_url, platform))
                            .collect());
                    }
                    // Try direct array within text
                    if let Some(arr) = parsed.as_array() {
                        tracing::debug!("Parsed as array with {} items", arr.len());
                        return Ok(arr
                            .iter()
                            .filter_map(|v| parse_issue(v, repo_url, platform))
                            .collect());
                    }
                    tracing::debug!("Parsed JSON has neither 'issues' nor array: {:?}", parsed);
                } else {
                    tracing::debug!("Failed to parse text as JSON");
                }
            }
        }
    }

    // Direct array format
    if let Some(arr) = result.as_array() {
        tracing::debug!("Result is direct array with {} items", arr.len());
        return Ok(arr
            .iter()
            .filter_map(|v| parse_issue(v, repo_url, platform))
            .collect());
    }

    // Single issue
    if result.get("number").is_some() {
        tracing::debug!("Result is single issue");
        if let Some(issue) = parse_issue(result, repo_url, platform) {
            return Ok(vec![issue]);
        }
    }

    tracing::debug!("No issues found in result");
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
    let state_values =
        normalize_issue_states(&state.unwrap_or_else(|| "open".to_string()), repo.platform);

    // GitHub MCP uses "states" (array), Gitea uses "state" (string)
    // For Gitea with "all", we need to make two separate calls
    let args = match repo.platform {
        Platform::GitHub => serde_json::json!({
            "owner": repo.owner,
            "repo": repo.repo_name,
            "states": state_values,
        }),
        Platform::Gitea => {
            // Gitea only supports single state, so for "all" we'll use first state
            // and handle separately if needed
            serde_json::json!({
                "owner": repo.owner,
                "repo": repo.repo_name,
                "state": state_values.first().unwrap_or(&"open".to_string()),
            })
        }
    };

    tracing::debug!("list_issues args: {:?}", args);

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;
    extract_issues_from_result(&result, &repo.url, repo.platform)
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
            // Handle nested text.text structure (Protobuf decoded format)
            let text_str = item.get("text").and_then(|t| {
                t.get("text")
                    .and_then(|inner| inner.as_str())
                    .or_else(|| t.as_str())
            });
            if let Some(text) = text_str {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(issue) = parse_issue(&parsed, &repo.url, repo.platform) {
                        return Ok(issue);
                    }
                }
            }
        }
    }

    // Direct format
    parse_issue(&result, &repo.url, repo.platform)
        .ok_or_else(|| AppError::NotFound(format!("Issue #{} not found", issue_number)))
}
