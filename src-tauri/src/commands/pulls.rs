use regex::Regex;
use std::sync::Arc;
use tauri::State;

use crate::db::{get_repository_by_id, DbPool, Platform, PullRequest};
use crate::error::AppError;
use crate::grpc::LocalCodeAgentClient;

/// Get the MCP tool name for listing pull requests based on platform
fn get_list_pulls_tool(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "list_pull_requests",
        Platform::Gitea => "list_repo_pull_requests",
    }
}

/// Parse pull request from MCP result JSON (handles both GitHub and Gitea formats)
fn parse_pull_request(value: &serde_json::Value) -> Option<PullRequest> {
    let number_i64 = value.get("number")?.as_i64()?;
    let number: i32 = number_i64.try_into().ok()?;

    let title = value.get("title")?.as_str()?.to_string();
    let body = value.get("body").and_then(|v| v.as_str()).map(String::from);
    let state = value
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("open")
        .to_string();

    // Head branch - GitHub: head.ref, Gitea: head_branch or head.ref
    let head_branch = value
        .get("head")
        .and_then(|h| h.get("ref").and_then(|r| r.as_str()))
        .or_else(|| value.get("head_branch").and_then(|v| v.as_str()));

    // Base branch - GitHub: base.ref, Gitea: base_branch or base.ref
    let base_branch = value
        .get("base")
        .and_then(|b| b.get("ref").and_then(|r| r.as_str()))
        .or_else(|| value.get("base_branch").and_then(|v| v.as_str()));

    let html_url = value
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Merged status
    let merged = value
        .get("merged")
        .and_then(|v| v.as_bool())
        .or_else(|| value.get("merged_at").map(|v| !v.is_null()))
        .unwrap_or(false);

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

    Some(PullRequest {
        number,
        title,
        body,
        state,
        head_branch: head_branch.map(String::from),
        base_branch: base_branch.map(String::from),
        html_url,
        merged,
        created_at,
        updated_at,
    })
}

/// Extract pull requests from MCP result
fn extract_pulls_from_result(result: &serde_json::Value) -> Result<Vec<PullRequest>, AppError> {
    // First, try to extract from MCP content structure
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(arr) = parsed.as_array() {
                        return Ok(arr.iter().filter_map(parse_pull_request).collect());
                    }
                }
            }
        }
    }

    // Direct array format
    if let Some(arr) = result.as_array() {
        return Ok(arr.iter().filter_map(parse_pull_request).collect());
    }

    // Single PR
    if result.get("number").is_some() {
        if let Some(pr) = parse_pull_request(result) {
            return Ok(vec![pr]);
        }
    }

    Ok(vec![])
}

/// Check if a PR is related to a specific issue number
fn is_related_pr(pr: &PullRequest, issue_number: i32) -> bool {
    let pattern = format!(
        r"(?i)(#{}|fixes\s+#{}|closes\s+#{}|resolves\s+#{})",
        issue_number, issue_number, issue_number, issue_number
    );

    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Check title
    if re.is_match(&pr.title) {
        return true;
    }

    // Check body
    if let Some(ref body) = pr.body {
        if re.is_match(body) {
            return true;
        }
    }

    // Check branch name patterns only if head_branch is available
    if let Some(ref branch) = pr.head_branch {
        // Use regex to match exact issue number in branch name patterns
        // Patterns: issue-N, issue/N, fix-N, fix/N, feature/issue-N, or /N at end
        // The pattern ensures N is the exact issue number (not a suffix like /21 matching issue 1)
        let branch_pattern = format!(
            r"(?:issue[-/]{}|fix[-/]{}|feature/issue-{}|/{}$)(?:[^0-9]|$)",
            issue_number, issue_number, issue_number, issue_number
        );

        if let Ok(branch_re) = Regex::new(&branch_pattern) {
            if branch_re.is_match(branch) {
                return true;
            }
        }
    }

    false
}

/// List pull requests for a repository via MCP server
#[tauri::command]
pub async fn list_pulls(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<LocalCodeAgentClient>>,
    repository_id: i64,
    state: Option<String>,
) -> Result<Vec<PullRequest>, AppError> {
    let repo = get_repository_by_id(&db, repository_id)?;
    let tool_name = get_list_pulls_tool(repo.platform);

    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "state": state.unwrap_or_else(|| "open".to_string()),
    });

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;
    extract_pulls_from_result(&result)
}

/// Find pull requests related to a specific issue
#[tauri::command]
pub async fn find_related_prs(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<LocalCodeAgentClient>>,
    repository_id: i64,
    issue_number: i32,
) -> Result<Vec<PullRequest>, AppError> {
    let repo = get_repository_by_id(&db, repository_id)?;
    let tool_name = get_list_pulls_tool(repo.platform);

    // Fetch all PRs (open and closed) to find related ones
    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "state": "all",
    });

    let result = grpc
        .call_mcp_tool(&repo.mcp_server_name, tool_name, &args)
        .await?;
    let all_prs = extract_pulls_from_result(&result)?;

    // Filter to related PRs
    let related: Vec<PullRequest> = all_prs
        .into_iter()
        .filter(|pr| is_related_pr(pr, issue_number))
        .collect();

    Ok(related)
}
