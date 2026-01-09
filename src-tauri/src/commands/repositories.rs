use std::sync::Arc;
use tauri::State;

use crate::db::{CreateRepository, Platform, Repository};
use crate::state::AppState;

#[tauri::command]
pub fn list_repositories(state: State<'_, Arc<AppState>>) -> Result<Vec<Repository>, String> {
    state
        .db
        .with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, mcp_server_name, platform, base_url, name, url, owner, repo_name,
                        local_path, last_synced_at, created_at, updated_at
                 FROM repositories ORDER BY created_at DESC",
            )?;

            let repos = stmt
                .query_map([], |row| {
                    let platform_str: String = row.get(2)?;
                    Ok(Repository {
                        id: row.get(0)?,
                        mcp_server_name: row.get(1)?,
                        platform: platform_str
                            .parse()
                            .unwrap_or(Platform::GitHub),
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
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(repos)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_repository(
    state: State<'_, Arc<AppState>>,
    request: CreateRepository,
) -> Result<Repository, String> {
    state
        .db
        .with_connection(|conn| {
            conn.execute(
                "INSERT INTO repositories (mcp_server_name, platform, base_url, name, url, owner, repo_name, local_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    request.mcp_server_name,
                    request.platform.to_string(),
                    request.base_url,
                    request.name,
                    request.url,
                    request.owner,
                    request.repo_name,
                    request.local_path,
                ],
            )?;

            let id = conn.last_insert_rowid();

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
                    platform: platform_str
                        .parse()
                        .unwrap_or(Platform::GitHub),
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
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_repository(state: State<'_, Arc<AppState>>, id: i64) -> Result<(), String> {
    state
        .db
        .with_connection(|conn| {
            let affected = conn.execute("DELETE FROM repositories WHERE id = ?1", [id])?;

            if affected == 0 {
                return Err(crate::error::AppError::NotFound(format!(
                    "Repository with id {} not found",
                    id
                )));
            }

            Ok(())
        })
        .map_err(|e| e.to_string())
}
