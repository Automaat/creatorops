use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref DB_CONNECTION: Mutex<Option<Connection>> = Mutex::new(None);
}

/// Initialize database connection and create schema
pub fn init_db() -> Result<()> {
    let db_path = get_db_path()?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    }

    let conn = Connection::open(&db_path)?;

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

    // Store connection in global state
    let mut db = DB_CONNECTION
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    *db = Some(conn);

    Ok(())
}

/// Get database file path
fn get_db_path() -> Result<PathBuf> {
    let home_dir = crate::modules::file_utils::get_home_dir().map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            e,
        )))
    })?;

    Ok(home_dir.join("CreatorOps").join("creatorops.db"))
}

/// Execute a query with the database connection
pub fn with_db<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&Connection) -> Result<R>,
{
    let db = DB_CONNECTION
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let conn = db.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
    f(conn)
}
