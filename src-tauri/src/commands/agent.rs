use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{AgentJobStatus, DbPool, Platform, Repository};
use crate::error::AppError;
use crate::grpc::data;
use crate::grpc::JobworkerpClient;

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
#[derive(Debug, Clone, Serialize)]
struct WorkflowInput {
    owner: String,
    repo: String,
    issue_number: i32,
    issue_title: String,
    base_branch: String,
    worktree_base_path: String,
    local_repo_path: String,
    mcp_server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_prompt: Option<String>,
}

/// Workflow run arguments for jobworkerp-rs
#[derive(Debug, Clone, Serialize)]
struct WorkflowRunArgs {
    workflow_url: Option<String>,
    workflow_data: Option<String>,
    input: String,
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
    grpc: State<'_, Arc<JobworkerpClient>>,
    request: StartAgentRequest,
) -> Result<StartAgentResponse, AppError> {
    tracing::info!(
        "Starting agent for repository_id={}, issue_number={}",
        request.repository_id,
        request.issue_number
    );

    // 1. Get repository info
    let repo = get_repository_internal(&db, request.repository_id)?;

    // Validate local_path is set
    let local_repo_path = repo
        .local_path
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput("Repository local_path is not configured".into()))?;

    // 2. Get app settings
    let settings = get_settings_internal(&db)?;

    // 3. Get workflow file path
    let workflow_path = get_workflow_path(&app)?;

    // 4. Build workflow input
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
        worktree_base_path: settings.worktree_base_path.clone(),
        local_repo_path: local_repo_path.clone(),
        mcp_server: mcp_server.to_string(),
        custom_prompt: request.custom_prompt.clone(),
    };

    let workflow_args = WorkflowRunArgs {
        workflow_url: Some(format!("file://{}", workflow_path.display())),
        workflow_data: None,
        input: serde_json::to_string(&workflow_input)?,
    };

    tracing::debug!("Workflow args: {:?}", workflow_args);

    // 5. Enqueue workflow job
    let args_json = serde_json::to_value(&workflow_args)?;
    let jobworkerp_job_id = grpc.enqueue_job("WORKFLOW", &args_json).await?;

    tracing::info!("Enqueued job with id: {}", jobworkerp_job_id);

    // 6. Create agent job record in DB
    let branch_name = format!("issue-{}", request.issue_number);
    let worktree_path = format!(
        "{}/issue-{}",
        settings.worktree_base_path, request.issue_number
    );

    let job_id = create_agent_job_internal(
        &db,
        request.repository_id,
        request.issue_number,
        &jobworkerp_job_id,
        Some(&branch_name),
        Some(&worktree_path),
    )?;

    tracing::info!("Created agent job record with id: {}", job_id);

    // 7. Spawn background task for stream listening
    let db_pool = db.inner().clone();
    let grpc_client = grpc.inner().clone();
    let jobworkerp_job_id_clone = jobworkerp_job_id.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            stream_job_results(app, db_pool, grpc_client, job_id, jobworkerp_job_id_clone).await
        {
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
    grpc: State<'_, Arc<JobworkerpClient>>,
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

/// Stream job results and emit events
async fn stream_job_results(
    app: AppHandle,
    db: DbPool,
    grpc: Arc<JobworkerpClient>,
    job_id: i64,
    jobworkerp_job_id: String,
) -> Result<(), AppError> {
    let event_name = format!("job-stream-{}", job_id);
    tracing::debug!("Starting stream listener for job {}", job_id);

    // Update status to indicate we're preparing
    update_job_status(&db, job_id, AgentJobStatus::PreparingWorkspace)?;

    // Listen to the job stream
    let mut stream = grpc.listen_stream(&jobworkerp_job_id).await?;

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
fn get_workflow_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let resource_path = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::Internal(format!("Failed to get resource dir: {}", e)))?;

    let workflow_path = resource_path
        .join("workflows")
        .join("code-agent-workflow.yaml");

    if !workflow_path.exists() {
        return Err(AppError::NotFound(format!(
            "Workflow file not found: {}",
            workflow_path.display()
        )));
    }

    Ok(workflow_path)
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
