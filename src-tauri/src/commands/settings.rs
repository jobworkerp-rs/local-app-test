use std::sync::Arc;
use tauri::State;

use crate::db::AppSettings;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct UpdateSettingsRequest {
    pub worktree_base_path: Option<String>,
    pub default_base_branch: Option<String>,
    pub agent_timeout_minutes: Option<i32>,
    pub sync_interval_minutes: Option<i32>,
    pub grpc_server_url: Option<String>,
    pub locale: Option<String>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    state
        .db
        .with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, worktree_base_path, default_base_branch, agent_timeout_minutes,
                        sync_interval_minutes, grpc_server_url, locale, created_at, updated_at
                 FROM app_settings WHERE id = 1",
            )?;

            let settings = stmt.query_row([], |row| {
                Ok(AppSettings {
                    id: row.get(0)?,
                    worktree_base_path: row.get(1)?,
                    default_base_branch: row.get(2)?,
                    agent_timeout_minutes: row.get(3)?,
                    sync_interval_minutes: row.get(4)?,
                    grpc_server_url: row.get(5)?,
                    locale: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?;

            Ok(settings)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, Arc<AppState>>,
    request: UpdateSettingsRequest,
) -> Result<AppSettings, String> {
    state
        .db
        .with_connection(|conn| {
            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref path) = request.worktree_base_path {
                updates.push("worktree_base_path = ?");
                params.push(Box::new(path.clone()));
            }
            if let Some(ref branch) = request.default_base_branch {
                updates.push("default_base_branch = ?");
                params.push(Box::new(branch.clone()));
            }
            if let Some(timeout) = request.agent_timeout_minutes {
                updates.push("agent_timeout_minutes = ?");
                params.push(Box::new(timeout));
            }
            if let Some(interval) = request.sync_interval_minutes {
                updates.push("sync_interval_minutes = ?");
                params.push(Box::new(interval));
            }
            if let Some(ref url) = request.grpc_server_url {
                updates.push("grpc_server_url = ?");
                params.push(Box::new(url.clone()));
            }
            if let Some(ref locale) = request.locale {
                updates.push("locale = ?");
                params.push(Box::new(locale.clone()));
            }

            if !updates.is_empty() {
                updates.push("updated_at = datetime('now')");
                let sql = format!(
                    "UPDATE app_settings SET {} WHERE id = 1",
                    updates.join(", ")
                );

                let params_ref: Vec<&dyn rusqlite::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                conn.execute(&sql, params_ref.as_slice())?;
            }

            // Return updated settings
            let mut stmt = conn.prepare(
                "SELECT id, worktree_base_path, default_base_branch, agent_timeout_minutes,
                        sync_interval_minutes, grpc_server_url, locale, created_at, updated_at
                 FROM app_settings WHERE id = 1",
            )?;

            let settings = stmt.query_row([], |row| {
                Ok(AppSettings {
                    id: row.get(0)?,
                    worktree_base_path: row.get(1)?,
                    default_base_branch: row.get(2)?,
                    agent_timeout_minutes: row.get(3)?,
                    sync_interval_minutes: row.get(4)?,
                    grpc_server_url: row.get(5)?,
                    locale: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?;

            Ok(settings)
        })
        .map_err(|e| e.to_string())
}
