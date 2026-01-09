mod migrations;
mod models;

pub use models::*;

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use crate::error::{AppError, AppResult};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> AppResult<Self> {
        let path = Path::new(db_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Run migrations
        db.run_migrations()?;

        Ok(db)
    }

    fn run_migrations(&self) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Internal(format!("Database mutex poisoned: {}", e)))?;

        conn.execute_batch(migrations::INITIAL_MIGRATION)?;

        Ok(())
    }

    pub fn with_connection<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Internal(format!("Database mutex poisoned: {}", e)))?;
        f(&conn)
    }
}
