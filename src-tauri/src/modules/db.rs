use crate::error::AppError;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Database wrapper for dependency injection
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create new database instance with default path
    pub fn new() -> Result<Self, AppError> {
        let db_path = Self::get_default_path()?;
        Self::new_with_path(&db_path)
    }

    /// Create new database instance with custom path (for testing)
    pub fn new_with_path(db_path: &Path) -> Result<Self, AppError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // Initialize schema
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialize database schema
    fn init_schema(conn: &Connection) -> Result<(), AppError> {
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

        // Create google_drive_accounts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS google_drive_accounts (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT NOT NULL,
                parent_folder_id TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_authenticated TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_drive_accounts_email ON google_drive_accounts(email)",
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
    fn get_default_path() -> Result<PathBuf, AppError> {
        let home_dir = crate::modules::file_utils::get_home_dir()
            .map_err(|e| AppError::Config(format!("Failed to get home directory: {e}")))?;

        Ok(home_dir.join("CreatorOps").join("creatorops.db"))
    }

    /// Execute a query with the database connection
    pub fn execute<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&Connection) -> Result<R, AppError>,
    {
        let conn = self.conn.lock().map_err(|_| AppError::LockFailed)?;
        f(&conn)
    }

    /// Execute a transaction with the database connection
    ///
    /// This method ensures atomic operations by wrapping multiple database operations
    /// in a transaction. If any operation fails, all changes are rolled back.
    #[allow(dead_code)]
    pub fn transaction<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<R, AppError>,
    {
        let mut conn = self.conn.lock().map_err(|_| AppError::LockFailed)?;
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        drop(conn);
        Ok(result)
    }
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_with_path_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("subdir").join("test.db");

        // Parent doesn't exist yet
        assert!(!db_path.parent().unwrap().exists());

        let db = Database::new_with_path(&db_path).unwrap();

        // Parent was created
        assert!(db_path.parent().unwrap().exists());
        assert!(db_path.exists());

        // Verify schema was initialized
        let result = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='projects'",
                )?;
                let exists = stmt.exists([])?;
                Ok(exists)
            })
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_schema_initialization_creates_all_indexes() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        let indexes = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='projects'",
                )?;
                let index_names: Vec<String> = stmt
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(index_names)
            })
            .unwrap();

        assert!(indexes.contains(&"idx_projects_status".to_owned()));
        assert!(indexes.contains(&"idx_projects_updated_at".to_owned()));
        assert!(indexes.contains(&"idx_projects_date".to_owned()));
        assert!(indexes.contains(&"idx_projects_client_name".to_owned()));
    }

    #[test]
    fn test_execute_with_callback() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        // Test successful query
        let result = db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    "test-id",
                    "Test",
                    "Client",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    "/path",
                    "2024-01-01T00:00:00Z",
                    "2024-01-01T00:00:00Z",
                    None::<String>,
                ],
            )?;
            Ok(())
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_query_returns_value() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        let count: usize = db
            .execute(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?)
            })
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_default_path_structure() {
        let path = Database::get_default_path().unwrap();
        let path_str = path.to_string_lossy();

        assert!(path_str.contains("CreatorOps"));
        assert!(path_str.ends_with("creatorops.db"));
    }

    #[test]
    fn test_multiple_connections() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        // Multiple operations should work
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["id1", "Name1", "Client1", "2024-01-01", "Type1", "New", "/path1", "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", None::<String>],
            )?;
            Ok(())
        })
        .unwrap();

        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["id2", "Name2", "Client2", "2024-01-02", "Type2", "Editing", "/path2", "2024-01-02T00:00:00Z", "2024-01-02T00:00:00Z", None::<String>],
            )?;
            Ok(())
        })
        .unwrap();

        let count: usize = db
            .execute(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?)
            })
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_transaction_commits_on_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        // Transaction should commit successfully
        let result = db.transaction(|tx| {
            tx.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["tx1", "Name1", "Client1", "2024-01-01", "Type1", "New", "/path1", "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", None::<String>],
            )?;

            tx.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["tx2", "Name2", "Client2", "2024-01-02", "Type2", "Editing", "/path2", "2024-01-02T00:00:00Z", "2024-01-02T00:00:00Z", None::<String>],
            )?;

            Ok(())
        });

        assert!(result.is_ok());

        // Verify both records were committed
        let count: usize = db
            .execute(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?)
            })
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_transaction_rolls_back_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();

        // Transaction should rollback on error
        let result = db.transaction(|tx| {
            tx.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["tx1", "Name1", "Client1", "2024-01-01", "Type1", "New", "/path1", "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", None::<String>],
            )?;

            // This should fail due to duplicate ID
            tx.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["tx1", "Name2", "Client2", "2024-01-02", "Type2", "Editing", "/path2", "2024-01-02T00:00:00Z", "2024-01-02T00:00:00Z", None::<String>],
            )?;

            Ok(())
        });

        // Transaction should have failed
        assert!(result.is_err());

        // Verify no records were committed
        let count: usize = db
            .execute(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?)
            })
            .unwrap();

        assert_eq!(count, 0);
    }
}
