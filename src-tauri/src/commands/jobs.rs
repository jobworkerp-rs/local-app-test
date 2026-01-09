use tauri::State;

use crate::db::{AgentJob, AgentJobStatus, DbPool};
use crate::error::AppError;

#[tauri::command]
pub async fn list_jobs(
    db: State<'_, DbPool>,
    repository_id: Option<i64>,
    status: Option<String>,
) -> Result<Vec<AgentJob>, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let mut sql = String::from(
        "SELECT id, repository_id, issue_number, jobworkerp_job_id, status,
                worktree_path, branch_name, pr_number, error_message, created_at, updated_at
         FROM agent_jobs WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(repo_id) = repository_id {
        sql.push_str(" AND repository_id = ?");
        params.push(Box::new(repo_id));
    }

    if let Some(ref status_str) = status {
        sql.push_str(" AND status = ?");
        params.push(Box::new(status_str.clone()));
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let jobs = stmt
        .query_map(params_ref.as_slice(), |row| {
            let status_str: String = row.get(4)?;
            Ok(AgentJob {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                issue_number: row.get(2)?,
                jobworkerp_job_id: row.get(3)?,
                status: status_str.parse().unwrap_or(AgentJobStatus::Pending),
                worktree_path: row.get(5)?,
                branch_name: row.get(6)?,
                pr_number: row.get(7)?,
                error_message: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(jobs)
}

#[tauri::command]
pub async fn get_job(db: State<'_, DbPool>, id: i64) -> Result<AgentJob, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, repository_id, issue_number, jobworkerp_job_id, status,
                worktree_path, branch_name, pr_number, error_message, created_at, updated_at
         FROM agent_jobs WHERE id = ?1",
    )?;

    let job = stmt.query_row([id], |row| {
        let status_str: String = row.get(4)?;
        Ok(AgentJob {
            id: row.get(0)?,
            repository_id: row.get(1)?,
            issue_number: row.get(2)?,
            jobworkerp_job_id: row.get(3)?,
            status: status_str.parse().unwrap_or(AgentJobStatus::Pending),
            worktree_path: row.get(5)?,
            branch_name: row.get(6)?,
            pr_number: row.get(7)?,
            error_message: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;

    Ok(job)
}
