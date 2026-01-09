use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::DbPool;
use crate::error::AppError;

/// Application settings
#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub id: i64,
    pub worktree_base_path: String,
    pub default_base_branch: String,
    pub agent_timeout_minutes: i64,
    pub sync_interval_minutes: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub worktree_base_path: Option<String>,
    pub default_base_branch: Option<String>,
    pub agent_timeout_minutes: Option<i64>,
    pub sync_interval_minutes: Option<i64>,
}

/// Get application settings
#[tauri::command]
pub async fn get_app_settings(db: State<'_, DbPool>) -> Result<AppSettings, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;
    fetch_settings(&conn)
}

/// Fetch settings from connection (internal helper)
fn fetch_settings(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
) -> Result<AppSettings, AppError> {
    conn.query_row(
        "SELECT id, worktree_base_path, default_base_branch, agent_timeout_minutes,
                sync_interval_minutes, created_at, updated_at
         FROM app_settings WHERE id = 1",
        [],
        |row| {
            Ok(AppSettings {
                id: row.get(0)?,
                worktree_base_path: row.get(1)?,
                default_base_branch: row.get(2)?,
                agent_timeout_minutes: row.get(3)?,
                sync_interval_minutes: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// Validate and sanitize update request, returning validated values or None
fn validate_update_request(
    request: &UpdateSettingsRequest,
) -> Result<UpdateSettingsRequest, AppError> {
    let worktree_base_path = match &request.worktree_base_path {
        Some(path) => {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                return Err(AppError::InvalidInput(
                    "worktree_base_path cannot be empty".into(),
                ));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };

    let default_base_branch = match &request.default_base_branch {
        Some(branch) => {
            let trimmed = branch.trim();
            if trimmed.is_empty() {
                return Err(AppError::InvalidInput(
                    "default_base_branch cannot be empty".into(),
                ));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };

    let agent_timeout_minutes = match request.agent_timeout_minutes {
        Some(minutes) if minutes <= 0 => {
            return Err(AppError::InvalidInput(
                "agent_timeout_minutes must be a positive number".into(),
            ));
        }
        other => other,
    };

    let sync_interval_minutes = match request.sync_interval_minutes {
        Some(minutes) if minutes <= 0 => {
            return Err(AppError::InvalidInput(
                "sync_interval_minutes must be a positive number".into(),
            ));
        }
        other => other,
    };

    Ok(UpdateSettingsRequest {
        worktree_base_path,
        default_base_branch,
        agent_timeout_minutes,
        sync_interval_minutes,
    })
}

/// Update application settings
#[tauri::command]
pub async fn update_app_settings(
    request: UpdateSettingsRequest,
    db: State<'_, DbPool>,
) -> Result<AppSettings, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    // Check if any updates requested
    if request.worktree_base_path.is_none()
        && request.default_base_branch.is_none()
        && request.agent_timeout_minutes.is_none()
        && request.sync_interval_minutes.is_none()
    {
        return fetch_settings(&conn);
    }

    // Validate input before DB operations
    let validated = validate_update_request(&request)?;

    // Use COALESCE to handle optional updates - if param is NULL, keep existing value
    let sql = "UPDATE app_settings SET
        worktree_base_path = COALESCE(:worktree_base_path, worktree_base_path),
        default_base_branch = COALESCE(:default_base_branch, default_base_branch),
        agent_timeout_minutes = COALESCE(:agent_timeout_minutes, agent_timeout_minutes),
        sync_interval_minutes = COALESCE(:sync_interval_minutes, sync_interval_minutes),
        updated_at = datetime('now')
        WHERE id = 1";

    let mut stmt = conn.prepare(sql)?;

    stmt.execute(rusqlite::named_params! {
        ":worktree_base_path": validated.worktree_base_path,
        ":default_base_branch": validated.default_base_branch,
        ":agent_timeout_minutes": validated.agent_timeout_minutes,
        ":sync_interval_minutes": validated.sync_interval_minutes,
    })?;

    fetch_settings(&conn)
}
