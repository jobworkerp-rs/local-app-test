use tauri::State;

use crate::db::{CreateRepository, DbPool, Platform, Repository};
use crate::error::AppError;

#[tauri::command]
pub async fn list_repositories(db: State<'_, DbPool>) -> Result<Vec<Repository>, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

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
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(repos)
}

#[tauri::command]
pub async fn create_repository(
    db: State<'_, DbPool>,
    request: CreateRepository,
) -> Result<Repository, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

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

#[tauri::command]
pub async fn get_repository(db: State<'_, DbPool>, id: i64) -> Result<Repository, AppError> {
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

#[tauri::command]
pub async fn delete_repository(db: State<'_, DbPool>, id: i64) -> Result<(), AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let affected = conn.execute("DELETE FROM repositories WHERE id = ?1", [id])?;

    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Repository with id {} not found",
            id
        )));
    }

    Ok(())
}
