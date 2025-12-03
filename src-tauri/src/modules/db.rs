use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

/// Database wrapper for dependency injection
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create new database instance with default path
    pub fn new() -> Result<Self> {
        let db_path = Self::get_default_path()?;
        Self::new_with_path(db_path)
    }

    /// Create new database instance with custom path (for testing)
    pub fn new_with_path(db_path: PathBuf) -> Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }

        let conn = Connection::open(&db_path)?;

        // Initialize schema
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialize database schema
    fn init_schema(conn: &Connection) -> Result<()> {
        // Create projects table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                client_name TEXT NOT NULL,
                date TEXT NOT NULL,
                shoot_type TEXT NOT NULL,
                status TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deadline TEXT
            )",
            [],
        )?;

        // Create indexes for common queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_updated_at ON projects(updated_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_date ON projects(date)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_client_name ON projects(client_name)",
            [],
        )?;

        Ok(())
    }

    /// Get default database file path
    fn get_default_path() -> Result<PathBuf> {
        let home_dir = crate::modules::file_utils::get_home_dir().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e,
            )))
        })?;

        Ok(home_dir.join("CreatorOps").join("creatorops.db"))
    }

    /// Execute a query with the database connection
    pub fn execute<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        f(&conn)
    }
}
