use crate::db::{DbPool, Platform, Repository};
use crate::error::AppError;

/// Get repository by ID from database
#[allow(clippy::type_complexity)]
pub fn get_repository_by_id(db: &DbPool, id: i64) -> Result<Repository, AppError> {
    let conn = db.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, mcp_server_name, platform, base_url, name, url, owner, repo_name,
                local_path, last_synced_at, created_at, updated_at
         FROM repositories WHERE id = ?1",
    )?;

    let row_data: (
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
    ) = stmt.query_row([id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
            row.get(11)?,
        ))
    })?;

    let platform: Platform = row_data
        .2
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("Invalid platform value: {}", row_data.2)))?;

    Ok(Repository {
        id: row_data.0,
        mcp_server_name: row_data.1,
        platform,
        base_url: row_data.3,
        name: row_data.4,
        url: row_data.5,
        owner: row_data.6,
        repo_name: row_data.7,
        local_path: row_data.8,
        last_synced_at: row_data.9,
        created_at: row_data.10,
        updated_at: row_data.11,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_database;
    use tempfile::tempdir;

    #[test]
    fn test_get_repository_by_id_not_found() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = init_database(Some(&db_path)).unwrap();

        let result = get_repository_by_id(&pool, 999);
        assert!(result.is_err());
    }
}
