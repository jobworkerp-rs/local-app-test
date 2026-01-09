use regex::Regex;
use std::sync::Arc;
use tauri::State;

use crate::db::{DbPool, Platform, PullRequest, Repository};
use crate::error::AppError;
use crate::grpc::JobworkerpClient;

/// Get repository by ID from database
fn get_repo_by_id(db: &DbPool, id: i64) -> Result<Repository, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, mcp_server_name, platform, base_url, name, url, owner, repo_name,
                local_path, last_synced_at, created_at, updated_at
         FROM repositories WHERE id = ?1",
    )?;

    let repo = stmt.query_row([id], |row| {
        let platform_str: String = row.get(2)?;
        Ok(Repository {
            id: row.get(0)?,
            mcp_server_name: row.get(1)?,
            platform: platform_str.parse().unwrap_or(Platform::GitHub),
            base_url: row.get(3)?,
            name: row.get(4)?,
            url: row.get(5)?,
            owner: row.get(6)?,
            repo_name: row.get(7)?,
            local_path: row.get(8)?,
            last_synced_at: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;

    Ok(repo)
}

/// Get the MCP tool name for listing pull requests based on platform
fn get_list_pulls_tool(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "list_pull_requests",
        Platform::Gitea => "list_repo_pull_requests",
    }
}

/// Parse pull request from MCP result JSON (handles both GitHub and Gitea formats)
fn parse_pull_request(value: &serde_json::Value) -> Option<PullRequest> {
    let number = value.get("number")?.as_i64()? as i32;
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
        .or_else(|| value.get("head_branch").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    // Base branch - GitHub: base.ref, Gitea: base_branch or base.ref
    let base_branch = value
        .get("base")
        .and_then(|b| b.get("ref").and_then(|r| r.as_str()))
        .or_else(|| value.get("base_branch").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

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
        head_branch,
        base_branch,
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

    // Check branch name patterns
    let issue_str = issue_number.to_string();
    let branch = &pr.head_branch;

    branch.contains(&format!("issue-{}", issue_str))
        || branch.contains(&format!("issue/{}", issue_str))
        || branch.contains(&format!("fix-{}", issue_str))
        || branch.contains(&format!("fix/{}", issue_str))
        || branch.contains(&format!("feature/issue-{}", issue_str))
        || branch.ends_with(&format!("/{}", issue_str))
}

/// List pull requests for a repository via MCP server
#[tauri::command]
pub async fn list_pulls(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<JobworkerpClient>>,
    repository_id: i64,
    state: Option<String>,
) -> Result<Vec<PullRequest>, AppError> {
    let repo = get_repo_by_id(&db, repository_id)?;
    let tool_name = get_list_pulls_tool(repo.platform);

    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "state": state.unwrap_or_else(|| "open".to_string()),
    });

    let result = grpc.call_mcp_tool(&repo.mcp_server_name, tool_name, &args).await?;
    extract_pulls_from_result(&result)
}

/// Find pull requests related to a specific issue
#[tauri::command]
pub async fn find_related_prs(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<JobworkerpClient>>,
    repository_id: i64,
    issue_number: i32,
) -> Result<Vec<PullRequest>, AppError> {
    let repo = get_repo_by_id(&db, repository_id)?;
    let tool_name = get_list_pulls_tool(repo.platform);

    // Fetch all PRs (open and closed) to find related ones
    let args = serde_json::json!({
        "owner": repo.owner,
        "repo": repo.repo_name,
        "state": "all",
    });

    let result = grpc.call_mcp_tool(&repo.mcp_server_name, tool_name, &args).await?;
    let all_prs = extract_pulls_from_result(&result)?;

    // Filter to related PRs
    let related: Vec<PullRequest> = all_prs
        .into_iter()
        .filter(|pr| is_related_pr(pr, issue_number))
        .collect();

    Ok(related)
}
