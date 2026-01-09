use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

use crate::error::AppError;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

/// Embedded migrations using refinery
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("src/db/migrations");
}

/// Create a new database connection pool
pub fn create_pool(db_path: &Path) -> Result<DbPool, AppError> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        // Enable foreign key constraints
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;",
        )
    });

    let pool = Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(pool)
}

/// Run database migrations
pub fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    // Run embedded migrations
    embedded::migrations::runner()
        .run(&mut *conn)
        .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Get default database path
pub fn default_db_path() -> Result<std::path::PathBuf, AppError> {
    let project_dirs =
        directories::ProjectDirs::from("com", "local-code-agent", "LocalCodeAgent")
            .ok_or_else(|| AppError::Config("Cannot determine data directory".into()))?;

    let data_dir = project_dirs.data_local_dir();
    Ok(data_dir.join("local-code-agent.db"))
}

/// Initialize database: create pool and run migrations
pub fn init_database(db_path: Option<&Path>) -> Result<DbPool, AppError> {
    let path = match db_path {
        Some(p) => p.to_path_buf(),
        None => default_db_path()?,
    };

    tracing::info!("Initializing database at {:?}", path);

    let pool = create_pool(&path)?;
    run_migrations(&pool)?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_pool_and_migrations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = init_database(Some(&db_path)).unwrap();

        // Verify tables exist
        let conn = pool.get().unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"app_settings".to_string()));
        assert!(tables.contains(&"repositories".to_string()));
        assert!(tables.contains(&"agent_jobs".to_string()));
        assert!(tables.contains(&"platform_configs".to_string()));
        assert!(tables.contains(&"token_stores".to_string()));
    }

    #[test]
    fn test_default_settings_inserted() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = init_database(Some(&db_path)).unwrap();
        let conn = pool.get().unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = init_database(Some(&db_path)).unwrap();
        let conn = pool.get().unwrap();

        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();

        assert_eq!(fk_enabled, 1);
    }
}
