use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{AgentJobStatus, DbPool, Platform, Repository};
use crate::error::AppError;
use crate::grpc::data;
use crate::grpc::LocalCodeAgentClient;

/// Request to start an agent job
#[derive(Debug, Clone, Deserialize)]
pub struct StartAgentRequest {
    pub repository_id: i64,
    pub issue_number: i32,
    pub issue_title: String,
    pub custom_prompt: Option<String>,
}

/// Response from starting an agent job
#[derive(Debug, Clone, Serialize)]
pub struct StartAgentResponse {
    pub job_id: i64,
    pub jobworkerp_job_id: String,
}

/// Workflow input parameters
/// Note: Debug is manually implemented to mask clone_url
#[derive(Clone, Serialize)]
struct WorkflowInput {
    owner: String,
    repo: String,
    issue_number: i32,
    issue_title: String,
    base_branch: String,
    clone_url: String,
    base_clone_path: String,
    worktree_path: String,
    branch_name: String,
    mcp_server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_prompt: Option<String>,
}

impl std::fmt::Debug for WorkflowInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowInput")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("issue_number", &self.issue_number)
            .field("issue_title", &self.issue_title)
            .field("base_branch", &self.base_branch)
            .field("clone_url", &"[REDACTED]")
            .field("base_clone_path", &self.base_clone_path)
            .field("worktree_path", &self.worktree_path)
            .field("branch_name", &self.branch_name)
            .field("mcp_server", &self.mcp_server)
            .field("custom_prompt", &self.custom_prompt)
            .finish()
    }
}

/// Event types for job streaming
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum JobStreamEvent {
    /// Streaming data chunk
    Data { data: Vec<u8> },
    /// Stream ended
    End,
    /// Final collected result
    FinalCollected {
        status: String,
        pr_number: Option<i32>,
        pr_url: Option<String>,
    },
    /// Error occurred
    Error { message: String },
}

/// Start an agent to process an issue
#[tauri::command]
pub async fn agent_start(
    app: AppHandle,
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<LocalCodeAgentClient>>,
    request: StartAgentRequest,
) -> Result<StartAgentResponse, AppError> {
    tracing::info!(
        "Starting agent for repository_id={}, issue_number={}",
        request.repository_id,
        request.issue_number
    );

    // 1. Get repository info
    let repo = get_repository_internal(&db, request.repository_id)?;

    // 2. Get app settings
    let settings = get_settings_internal(&db)?;

    // 3. Get workflow file path
    let workflow_path = get_workflow_path(&app)?;

    // 4. Calculate repository identifier and paths
    let repo_identifier = repo
        .local_path
        .clone()
        .unwrap_or_else(|| format!("{}/{}", repo.owner, repo.repo_name));

    let base_clone_path = format!("{}/{}", settings.worktree_base_path, repo_identifier);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let branch_name = format!("issue-{}", request.issue_number);
    let worktree_dir = format!("issue-{}-{}", request.issue_number, timestamp);
    let worktree_path = format!("{}/{}", base_clone_path, worktree_dir);

    // 5. Get Runner to extract token for authenticated clone URL
    let runner = grpc
        .find_runner_by_exact_name(&repo.mcp_server_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Runner '{}' not found", repo.mcp_server_name))
        })?;

    let token = extract_token_from_runner(&runner, repo.platform)?;
    let clone_url = build_authenticated_clone_url(&repo.url, &token, repo.platform);

    // 6. Build workflow input
    let mcp_server = match repo.platform {
        Platform::GitHub => "github",
        Platform::Gitea => "gitea",
    };

    let workflow_input = WorkflowInput {
        owner: repo.owner.clone(),
        repo: repo.repo_name.clone(),
        issue_number: request.issue_number,
        issue_title: request.issue_title.clone(),
        base_branch: settings.default_base_branch.clone(),
        clone_url,
        base_clone_path: base_clone_path.clone(),
        worktree_path: worktree_path.clone(),
        branch_name: branch_name.clone(),
        mcp_server: mcp_server.to_string(),
        custom_prompt: request.custom_prompt.clone(),
    };

    tracing::debug!("Workflow input: {:?}", workflow_input);

    // 7. Enqueue workflow job using LocalCodeAgentClient (auto-creates worker if needed)
    let workflow_url = format!("file://{}", workflow_path.display());
    let input_json = serde_json::to_string(&workflow_input)?;

    let (jobworkerp_job_id, stream) = grpc
        .enqueue_workflow_for_stream(&workflow_url, &input_json, None)
        .await?;

    tracing::info!("Enqueued job with id: {}", jobworkerp_job_id);

    // 8. Create agent job record in DB
    let job_id = create_agent_job_internal(
        &db,
        request.repository_id,
        request.issue_number,
        &jobworkerp_job_id,
        Some(&branch_name),
        Some(&worktree_path),
    )?;

    tracing::info!("Created agent job record with id: {}", job_id);

    // 9. Spawn background task for stream listening
    let db_pool = db.inner().clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = stream_job_results_from_stream(app, db_pool, job_id, stream).await {
            tracing::error!("Stream listener error: {:?}", e);
        }
    });

    Ok(StartAgentResponse {
        job_id,
        jobworkerp_job_id,
    })
}

/// Cancel a running agent job
#[tauri::command]
pub async fn agent_cancel(
    db: State<'_, DbPool>,
    grpc: State<'_, Arc<LocalCodeAgentClient>>,
    jobworkerp_job_id: String,
) -> Result<(), AppError> {
    tracing::info!("Cancelling job: {}", jobworkerp_job_id);

    // 1. Delete/cancel job in jobworkerp-rs
    grpc.delete_job(&jobworkerp_job_id).await?;

    // 2. Update status in local DB
    update_job_status_by_jobworkerp_id(&db, &jobworkerp_job_id, AgentJobStatus::Cancelled)?;

    tracing::info!("Job cancelled: {}", jobworkerp_job_id);

    Ok(())
}

/// Stream job results from an existing stream and emit events
async fn stream_job_results_from_stream(
    app: AppHandle,
    db: DbPool,
    job_id: i64,
    mut stream: tonic::Streaming<data::ResultOutputItem>,
) -> Result<(), AppError> {
    let event_name = format!("job-stream-{}", job_id);
    tracing::debug!("Starting stream listener for job {}", job_id);

    // Update status to indicate we're preparing
    update_job_status(&db, job_id, AgentJobStatus::PreparingWorkspace)?;

    while let Some(item) = stream
        .message()
        .await
        .map_err(|e| AppError::Grpc(e.to_string()))?
    {
        match item.item {
            Some(data::result_output_item::Item::Data(data)) => {
                tracing::trace!("Received data chunk: {} bytes", data.len());
                let event = JobStreamEvent::Data {
                    data: data.to_vec(),
                };
                let _ = app.emit(&event_name, &event);
            }
            Some(data::result_output_item::Item::End(_trailer)) => {
                tracing::debug!("Stream ended for job {}", job_id);
                let event = JobStreamEvent::End;
                let _ = app.emit(&event_name, &event);
                break;
            }
            Some(data::result_output_item::Item::FinalCollected(data)) => {
                tracing::debug!("Final collected result for job {}", job_id);

                // Parse workflow result
                match parse_workflow_result(&data) {
                    Ok(result) => {
                        // Update DB based on result
                        if result.status == "success" {
                            if let (Some(pr_number), Some(pr_url)) =
                                (result.pr_number, &result.pr_url)
                            {
                                update_job_with_pr(&db, job_id, pr_number, pr_url)?;
                            } else {
                                update_job_status(&db, job_id, AgentJobStatus::Completed)?;
                            }
                        } else if result.status == "no_changes" {
                            update_job_status(&db, job_id, AgentJobStatus::Completed)?;
                        } else {
                            update_job_error(
                                &db,
                                job_id,
                                result.error.as_deref().unwrap_or("Unknown error"),
                            )?;
                        }

                        let event = JobStreamEvent::FinalCollected {
                            status: result.status,
                            pr_number: result.pr_number,
                            pr_url: result.pr_url,
                        };
                        let _ = app.emit(&event_name, &event);
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse workflow result: {:?}", e);
                        update_job_error(&db, job_id, &format!("Failed to parse result: {}", e))?;

                        let event = JobStreamEvent::Error {
                            message: e.to_string(),
                        };
                        let _ = app.emit(&event_name, &event);
                    }
                }
                break;
            }
            None => {}
        }
    }

    Ok(())
}

/// Workflow result from jobworkerp-rs
#[derive(Debug, Clone, Deserialize)]
struct WorkflowResult {
    status: String,
    pr_number: Option<i32>,
    pr_url: Option<String>,
    #[serde(default)]
    no_changes: bool,
    error: Option<String>,
}

/// Parse workflow result from bytes
fn parse_workflow_result(data: &[u8]) -> Result<WorkflowResult, AppError> {
    serde_json::from_slice(data).map_err(|e| AppError::Internal(format!("JSON parse error: {}", e)))
}

/// Get workflow file path from app resources
/// In development mode, falls back to project root workflows directory
fn get_workflow_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let workflow_filename = "code-agent-workflow.yaml";

    // Try production path first (bundled resources)
    if let Ok(resource_path) = app.path().resource_dir() {
        let workflow_path = resource_path.join("workflows").join(workflow_filename);
        if workflow_path.exists() {
            return Ok(workflow_path);
        }
    }

    // Development fallback: check relative to manifest dir (src-tauri)
    // This works when running `tauri dev`
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("workflows").join(workflow_filename));

    if let Some(path) = dev_path {
        if path.exists() {
            tracing::debug!("Using development workflow path: {}", path.display());
            return Ok(path);
        }
    }

    // Final fallback: current working directory
    let cwd_path = std::env::current_dir()
        .map(|p| p.join("workflows").join(workflow_filename))
        .ok();

    if let Some(path) = cwd_path {
        if path.exists() {
            tracing::debug!("Using CWD workflow path: {}", path.display());
            return Ok(path);
        }
    }

    Err(AppError::NotFound(format!(
        "Workflow file not found. Checked: resource_dir/workflows/{}, CARGO_MANIFEST_DIR/../workflows/{}, CWD/workflows/{}",
        workflow_filename, workflow_filename, workflow_filename
    )))
}

// ============================================================================
// Internal DB helper functions
// ============================================================================

/// App settings from DB
#[derive(Debug)]
struct AppSettingsInternal {
    worktree_base_path: String,
    default_base_branch: String,
}

/// Get repository by ID (internal)
fn get_repository_internal(db: &DbPool, repository_id: i64) -> Result<Repository, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, mcp_server_name, platform, base_url, name, url, owner, repo_name,
                local_path, last_synced_at, created_at, updated_at
         FROM repositories WHERE id = ?1",
    )?;

    let repo = stmt.query_row([repository_id], |row| {
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

/// Get app settings (internal)
fn get_settings_internal(db: &DbPool) -> Result<AppSettingsInternal, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.query_row(
        "SELECT worktree_base_path, default_base_branch FROM app_settings WHERE id = 1",
        [],
        |row| {
            Ok(AppSettingsInternal {
                worktree_base_path: row.get(0)?,
                default_base_branch: row.get(1)?,
            })
        },
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// Create agent job record
fn create_agent_job_internal(
    db: &DbPool,
    repository_id: i64,
    issue_number: i32,
    jobworkerp_job_id: &str,
    branch_name: Option<&str>,
    worktree_path: Option<&str>,
) -> Result<i64, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.execute(
        "INSERT INTO agent_jobs (repository_id, issue_number, jobworkerp_job_id, status, branch_name, worktree_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            repository_id,
            issue_number,
            jobworkerp_job_id,
            AgentJobStatus::Pending.to_string(),
            branch_name,
            worktree_path,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Update job status
fn update_job_status(db: &DbPool, job_id: i64, status: AgentJobStatus) -> Result<(), AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.execute(
        "UPDATE agent_jobs SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![status.to_string(), job_id],
    )?;

    Ok(())
}

/// Update job status by jobworkerp_job_id
fn update_job_status_by_jobworkerp_id(
    db: &DbPool,
    jobworkerp_job_id: &str,
    status: AgentJobStatus,
) -> Result<(), AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.execute(
        "UPDATE agent_jobs SET status = ?1, updated_at = datetime('now') WHERE jobworkerp_job_id = ?2",
        rusqlite::params![status.to_string(), jobworkerp_job_id],
    )?;

    Ok(())
}

/// Update job with PR info
fn update_job_with_pr(
    db: &DbPool,
    job_id: i64,
    pr_number: i32,
    pr_url: &str,
) -> Result<(), AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.execute(
        "UPDATE agent_jobs SET status = ?1, pr_number = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![AgentJobStatus::PrCreated.to_string(), pr_number, job_id],
    )?;

    // Log PR URL for debugging
    tracing::info!("PR created: {} - {}", pr_number, pr_url);

    Ok(())
}

/// Update job with error
fn update_job_error(db: &DbPool, job_id: i64, error_message: &str) -> Result<(), AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    conn.execute(
        "UPDATE agent_jobs SET status = ?1, error_message = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![AgentJobStatus::Failed.to_string(), error_message, job_id],
    )?;

    Ok(())
}

// ============================================================================
// Token extraction and clone URL building
// ============================================================================

/// Extract token from Runner's definition
/// Supports both direct envs and Docker -e argument patterns
fn extract_token_from_runner(
    runner: &data::Runner,
    platform: Platform,
) -> Result<String, AppError> {
    let runner_data = runner
        .data
        .as_ref()
        .ok_or_else(|| AppError::Internal("Runner has no data".into()))?;

    let definition: serde_json::Value = serde_json::from_str(&runner_data.definition)
        .map_err(|e| AppError::Internal(format!("Failed to parse runner definition: {}", e)))?;

    let token_key = match platform {
        Platform::GitHub => "GITHUB_PERSONAL_ACCESS_TOKEN",
        Platform::Gitea => "GITEA_ACCESS_TOKEN",
    };

    // Priority 1: Check envs field directly
    if let Some(envs) = definition.get("envs").and_then(|e| e.as_object()) {
        if let Some(token) = envs.get(token_key).and_then(|t| t.as_str()) {
            return Ok(token.to_string());
        }
    }

    // Priority 2: Check Docker -e arguments for KEY=VALUE pattern
    if let Some(args) = definition.get("args").and_then(|a| a.as_array()) {
        let args_str: Vec<&str> = args.iter().filter_map(|a| a.as_str()).collect();

        for (i, arg) in args_str.iter().enumerate() {
            if *arg == "-e" {
                if let Some(next_arg) = args_str.get(i + 1) {
                    let prefix = format!("{}=", token_key);
                    if next_arg.starts_with(&prefix) {
                        let value = next_arg.strip_prefix(&prefix).unwrap();
                        return Ok(value.to_string());
                    }
                    // KEY only pattern - value should be in envs
                    if *next_arg == token_key {
                        if let Some(envs) = definition.get("envs").and_then(|e| e.as_object()) {
                            if let Some(token) = envs.get(token_key).and_then(|t| t.as_str()) {
                                return Ok(token.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Err(AppError::Internal(format!(
        "Token '{}' not found in runner definition",
        token_key
    )))
}

/// Build authenticated clone URL
/// SECURITY: This URL contains credentials - never log it
fn build_authenticated_clone_url(repo_url: &str, token: &str, platform: Platform) -> String {
    let url_without_scheme = repo_url.strip_prefix("https://").unwrap_or(repo_url);
    let base_url = if url_without_scheme.ends_with(".git") {
        url_without_scheme.to_string()
    } else {
        format!("{}.git", url_without_scheme)
    };

    match platform {
        Platform::GitHub => format!("https://x-access-token:{}@{}", token, base_url),
        Platform::Gitea => format!("https://git:{}@{}", token, base_url),
    }
}
