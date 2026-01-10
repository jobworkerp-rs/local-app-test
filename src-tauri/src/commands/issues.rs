use std::sync::Arc;
use tauri::State;

use crate::db::{get_repository_by_id, DbPool, Issue, IssueComment, Platform};
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

/// Get the MCP tool name for listing issue comments based on platform
/// Note: GitHub MCP uses "issue_read" with method="get_comments"
fn get_list_issue_comments_tool(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "issue_read",
        Platform::Gitea => "get_issue_comments",
    }
}

/// Convert issue state to platform-specific format
/// GitHub MCP expects uppercase: "OPEN", "CLOSED", or omit for all
/// Gitea MCP expects lowercase: "open", "closed", "all"
/// Returns None for "all" on GitHub (omitting the parameter returns both)
fn normalize_issue_state(state: &str, platform: Platform) -> Option<String> {
    let normalized = state.to_lowercase();
    match platform {
        Platform::GitHub => match normalized.as_str() {
            "all" => None, // Omit parameter to get both open and closed
            _ => Some(normalized.to_uppercase()),
        },
        Platform::Gitea => Some(normalized),
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
    let state_str = state.unwrap_or_else(|| "open".to_string());
    tracing::debug!("list_issues called with state: '{}'", state_str);
    let state_value = normalize_issue_state(&state_str, repo.platform);
    tracing::debug!("normalized state_value: {:?}", state_value);

    // Build args - GitHub MCP uses "state" (singular), omit for "all"
    let mut args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
    });

    // Add state parameter only if not "all" (for GitHub, omitting returns both)
    if let Some(state_val) = state_value {
        args["state"] = serde_json::Value::String(state_val);
    }

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

    // Build args - GitHub MCP requires "method" parameter
    let args = match repo.platform {
        Platform::GitHub => serde_json::json!({
            "owner": repo.owner,
            "repo": repo.repo_name,
            "issue_number": issue_number,
            "method": "get",
        }),
        Platform::Gitea => serde_json::json!({
            "owner": repo.owner,
            "repo": repo.repo_name,
            "issue_number": issue_number,
        }),
    };

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

/// Parse a single comment from MCP result JSON
fn parse_comment(value: &serde_json::Value) -> Option<IssueComment> {
    let id = value.get("id")?.as_i64()?;
    let body = value.get("body").and_then(|v| v.as_str())?.to_string();

    let user = value
        .get("user")
        .and_then(|u| {
            u.as_str()
                .map(String::from)
                .or_else(|| u.get("login").and_then(|l| l.as_str()).map(String::from))
        })
        .unwrap_or_default();

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

    Some(IssueComment {
        id,
        user,
        body,
        created_at,
        updated_at,
    })
}

/// Extract comments from MCP result
fn extract_comments_from_result(result: &serde_json::Value) -> Result<Vec<IssueComment>, AppError> {
    tracing::debug!("extract_comments_from_result: {:?}", result);

    // GitHub MCP format: {"comments": [...]}
    if let Some(comments_arr) = result.get("comments").and_then(|c| c.as_array()) {
        tracing::debug!("Found 'comments' field with {} items", comments_arr.len());
        return Ok(comments_arr.iter().filter_map(parse_comment).collect());
    }

    // MCP content structure: {"content": [{"text": "..."}]}
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            let text_str = item.get("text").and_then(|t| {
                t.get("text")
                    .and_then(|inner| inner.as_str())
                    .or_else(|| t.as_str())
            });

            if let Some(text) = text_str {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(comments_arr) = parsed.get("comments").and_then(|c| c.as_array()) {
                        return Ok(comments_arr.iter().filter_map(parse_comment).collect());
                    }
                    if let Some(arr) = parsed.as_array() {
                        return Ok(arr.iter().filter_map(parse_comment).collect());
                    }
                }
            }
        }
    }

    // Direct array format
    if let Some(arr) = result.as_array() {
        return Ok(arr.iter().filter_map(parse_comment).collect());
    }

    Ok(vec![])
}

/// Get comments for a specific issue
#[tauri::command]
pub async fn get_issue_comments(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<JobworkerpClient>>,
    repository_id: i64,
    issue_number: i32,
) -> Result<Vec<IssueComment>, AppError> {
    let repo = get_repository_by_id(&db, repository_id)?;
    let tool_name = get_list_issue_comments_tool(repo.platform);

    // Build args - GitHub MCP uses issue_read with method="get_comments"
    let args = match repo.platform {
        Platform::GitHub => serde_json::json!({
            "owner": repo.owner,
            "repo": repo.repo_name,
            "issue_number": issue_number,
            "method": "get_comments",
        }),
        Platform::Gitea => serde_json::json!({
            "owner": repo.owner,
            "repo": repo.repo_name,
            "issue_number": issue_number,
        }),
    };

    tracing::debug!("get_issue_comments args: {:?}", args);

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;

    extract_comments_from_result(&result)
}
