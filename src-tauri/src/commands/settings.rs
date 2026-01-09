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

    let settings = conn.query_row(
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
    )?;

    Ok(settings)
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

    // Build dynamic UPDATE using named parameters for Send safety
    let mut updates = Vec::new();
    if request.worktree_base_path.is_some() {
        updates.push("worktree_base_path = :worktree_base_path");
    }
    if request.default_base_branch.is_some() {
        updates.push("default_base_branch = :default_base_branch");
    }
    if request.agent_timeout_minutes.is_some() {
        updates.push("agent_timeout_minutes = :agent_timeout_minutes");
    }
    if request.sync_interval_minutes.is_some() {
        updates.push("sync_interval_minutes = :sync_interval_minutes");
    }
    updates.push("updated_at = datetime('now')");

    let sql = format!(
        "UPDATE app_settings SET {} WHERE id = 1",
        updates.join(", ")
    );

    let mut stmt = conn.prepare(&sql)?;

    stmt.execute(rusqlite::named_params! {
        ":worktree_base_path": request.worktree_base_path,
        ":default_base_branch": request.default_base_branch,
        ":agent_timeout_minutes": request.agent_timeout_minutes,
        ":sync_interval_minutes": request.sync_interval_minutes,
    })?;

    fetch_settings(&conn)
}
