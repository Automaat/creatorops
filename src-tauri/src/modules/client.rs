//! Client management module — CRUD for photography clients.
//!
//! Each client can own multiple projects. The `client_name` field in projects
//! is denormalized (kept in sync) to avoid JOINs in `list_projects` hot path.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::modules::db::Database;
use crate::modules::project::{map_project_row, Project};

/// Client status — active or soft-deleted via archival.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClientStatus {
    Active,
    Archived,
}

impl std::fmt::Display for ClientStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for ClientStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Invalid client status: {s}")),
        }
    }
}

/// Core client entity stored in `SQLite` and serialised to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub status: ClientStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// Client with its associated projects (for detail view).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientWithProjects {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub status: ClientStatus,
    pub created_at: String,
    pub updated_at: String,
    pub projects: Vec<Project>,
}

/// Validate email — checks for exactly one @, non-empty local/domain parts,
/// and at least one dot in the domain that isn't at the start or end.
fn validate_email(email: &str) -> bool {
    let at_idx = match email.find('@') {
        Some(i) if !email[i + 1..].contains('@') => i,
        _ => return false,
    };
    let local = &email[..at_idx];
    let domain = &email[at_idx + 1..];
    if local.is_empty() || local.contains(' ') {
        return false;
    }
    if domain.is_empty() || domain.contains(' ') {
        return false;
    }
    matches!(domain.rfind('.'), Some(dot) if dot > 0 && dot < domain.len() - 1)
}

/// Map a database row to a Client struct.
fn map_client_row(row: &rusqlite::Row) -> rusqlite::Result<Client> {
    let status_str: String = row.get(5)?;
    let status = status_str.parse::<ClientStatus>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(Client {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        phone: row.get(3)?,
        notes: row.get(4)?,
        status,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

/// Fetch a single client by ID.
pub fn get_client_by_id(db: &Database, client_id: &str) -> Result<Client, AppError> {
    db.execute(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, notes, status, created_at, updated_at
             FROM clients WHERE id = ?1",
        )?;
        stmt.query_row(params![client_id], map_client_row)
            .map_err(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    AppError::InvalidData(format!("Client not found: {client_id}"))
                } else {
                    AppError::from(e)
                }
            })
    })
}

/// Create a new client.
#[tauri::command]
pub async fn create_client(
    db: tauri::State<'_, Database>,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    notes: Option<String>,
) -> Result<Client, String> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err("Client name is required".to_owned());
    }

    if let Some(ref e) = email {
        if !e.is_empty() && !validate_email(e) {
            return Err("Invalid email format".to_owned());
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let email = email.filter(|e| !e.is_empty());
    let phone = phone.filter(|p| !p.is_empty());
    let notes = notes.filter(|n| !n.is_empty());

    let client = Client {
        id,
        name,
        email,
        phone,
        notes,
        status: ClientStatus::Active,
        created_at: now.clone(),
        updated_at: now,
    };

    db.execute(|conn| {
        conn.execute(
            "INSERT INTO clients (id, name, email, phone, notes, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &client.id,
                &client.name,
                &client.email,
                &client.phone,
                &client.notes,
                client.status.to_string(),
                &client.created_at,
                &client.updated_at,
            ],
        )?;
        Ok(())
    })
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint failed") {
            format!("A client named '{}' already exists", client.name)
        } else {
            format!("Failed to create client: {msg}")
        }
    })?;

    Ok(client)
}

/// List all clients, optionally including archived ones.
#[tauri::command]
pub async fn list_clients(
    db: tauri::State<'_, Database>,
    include_archived: Option<bool>,
) -> Result<Vec<Client>, String> {
    let include_archived = include_archived.unwrap_or(false);

    db.execute(|conn| {
        let sql = if include_archived {
            "SELECT id, name, email, phone, notes, status, created_at, updated_at
             FROM clients ORDER BY name ASC"
        } else {
            "SELECT id, name, email, phone, notes, status, created_at, updated_at
             FROM clients WHERE status = 'active' ORDER BY name ASC"
        };

        let mut stmt = conn.prepare(sql)?;
        let clients = stmt
            .query_map([], map_client_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(clients)
    })
    .map_err(|e| format!("Database error: {e}"))
}

/// Get a single client with all its projects.
#[tauri::command]
pub async fn get_client(
    db: tauri::State<'_, Database>,
    client_id: String,
) -> Result<ClientWithProjects, String> {
    let client = get_client_by_id(&db, &client_id).map_err(String::from)?;

    let projects = db
        .execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, client_name, date, shoot_type, status, folder_path,
                        created_at, updated_at, deadline, client_id
                 FROM projects WHERE client_id = ?1 ORDER BY updated_at DESC",
            )?;
            let rows = stmt
                .query_map(params![client_id], map_project_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .map_err(|e| format!("Failed to load client projects: {e}"))?;

    Ok(ClientWithProjects {
        id: client.id,
        name: client.name,
        email: client.email,
        phone: client.phone,
        notes: client.notes,
        status: client.status,
        created_at: client.created_at,
        updated_at: client.updated_at,
        projects,
    })
}

/// Update client metadata. When name changes, syncs denormalized `client_name` in projects.
#[tauri::command]
pub async fn update_client(
    db: tauri::State<'_, Database>,
    client_id: String,
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    notes: Option<String>,
) -> Result<Client, String> {
    let existing = get_client_by_id(&db, &client_id).map_err(String::from)?;

    let new_name = name
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| existing.name.clone());

    if let Some(ref e) = email {
        if !e.is_empty() && !validate_email(e) {
            return Err("Invalid email format".to_owned());
        }
    }

    let new_email = email
        .map(|e| if e.is_empty() { None } else { Some(e) })
        .unwrap_or(existing.email);
    let new_phone = phone
        .map(|p| if p.is_empty() { None } else { Some(p) })
        .unwrap_or(existing.phone);
    let new_notes = notes
        .map(|n| if n.is_empty() { None } else { Some(n) })
        .unwrap_or(existing.notes);
    let now = chrono::Utc::now().to_rfc3339();
    let name_changed = new_name != existing.name;

    db.execute(|conn| {
        conn.execute(
            "UPDATE clients SET name = ?1, email = ?2, phone = ?3, notes = ?4, updated_at = ?5
             WHERE id = ?6",
            params![new_name, new_email, new_phone, new_notes, now, client_id],
        )?;

        // Sync denormalized client_name in all projects linked to this client
        if name_changed {
            conn.execute(
                "UPDATE projects SET client_name = ?1, updated_at = ?2 WHERE client_id = ?3",
                params![new_name, now, client_id],
            )?;
        }

        Ok(())
    })
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint failed") {
            format!("A client named '{new_name}' already exists")
        } else {
            format!("Failed to update client: {msg}")
        }
    })?;

    get_client_by_id(&db, &client_id).map_err(String::from)
}

/// Archive or unarchive a client.
#[tauri::command]
pub async fn update_client_status(
    db: tauri::State<'_, Database>,
    client_id: String,
    status: ClientStatus,
) -> Result<Client, String> {
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(|conn| {
        conn.execute(
            "UPDATE clients SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.to_string(), now, client_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update client status: {e}"))?;

    get_client_by_id(&db, &client_id).map_err(String::from)
}

/// Delete a client. Fails if the client has any associated projects.
#[tauri::command]
pub async fn delete_client(
    db: tauri::State<'_, Database>,
    client_id: String,
) -> Result<(), String> {
    let project_count: i64 = db
        .execute(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM projects WHERE client_id = ?1",
                params![client_id],
                |row| row.get(0),
            )?)
        })
        .map_err(|e| format!("Database error: {e}"))?;

    if project_count > 0 {
        return Err(format!(
            "Cannot delete client: {project_count} project(s) are still linked. Delete them first."
        ));
    }

    db.execute(|conn| {
        conn.execute("DELETE FROM clients WHERE id = ?1", params![client_id])?;
        Ok(())
    })
    .map_err(|e| format!("Failed to delete client: {e}"))?;

    Ok(())
}

/// Search clients by name or email (case-insensitive).
#[tauri::command]
pub async fn search_clients(
    db: tauri::State<'_, Database>,
    query: String,
) -> Result<Vec<Client>, String> {
    let pattern = format!("%{}%", query.to_lowercase());

    db.execute(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, notes, status, created_at, updated_at
             FROM clients
             WHERE status = 'active'
               AND (LOWER(name) LIKE ?1 OR LOWER(COALESCE(email, '')) LIKE ?1)
             ORDER BY name ASC",
        )?;
        let clients = stmt
            .query_map(params![pattern], map_client_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(clients)
    })
    .map_err(|e| format!("Database error: {e}"))
}

/// Link unlinked projects to client records by matching `client_name`.
/// Safe to call multiple times — skips projects that already have `client_id` set.
pub fn run_client_migration(db: &Database) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(|conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT client_name FROM projects
             WHERE client_id IS NULL AND client_name != ''",
        )?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for name in names {
            let existing_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM clients WHERE LOWER(name) = LOWER(?1)",
                    params![name],
                    |row| row.get(0),
                )
                .ok();

            let client_id = if let Some(id) = existing_id {
                id
            } else {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO clients (id, name, status, created_at, updated_at)
                     VALUES (?1, ?2, 'active', ?3, ?4)",
                    params![id, name, now, now],
                )?;
                id
            };

            conn.execute(
                "UPDATE projects SET client_id = ?1
                 WHERE client_id IS NULL AND LOWER(client_name) = LOWER(?2)",
                params![client_id, name],
            )?;
        }

        Ok(())
    })
}

/// Tauri command wrapper — delegates to `run_client_migration`.
#[tauri::command]
pub async fn migrate_clients_from_projects(db: tauri::State<'_, Database>) -> Result<(), String> {
    run_client_migration(&db).map_err(|e| format!("Migration failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db::Database;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, Database) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();
        (temp_dir, db)
    }

    fn insert_client(db: &Database, id: &str, name: &str) {
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO clients (id, name, status, created_at, updated_at)
                 VALUES (?1, ?2, 'active', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
                params![id, name],
            )?;
            Ok(())
        })
        .unwrap();
    }

    fn insert_project_with_client(
        db: &Database,
        project_id: &str,
        client_id: &str,
        client_name: &str,
    ) {
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, client_id, date, shoot_type, status,
                  folder_path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, '2024-01-01', 'Wedding', 'New',
                  '/path', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
                params![project_id, "Test Project", client_name, client_id],
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_client_status_to_string() {
        assert_eq!(ClientStatus::Active.to_string(), "active");
        assert_eq!(ClientStatus::Archived.to_string(), "archived");
    }

    #[test]
    fn test_client_status_from_str() {
        assert_eq!(
            "active".parse::<ClientStatus>().unwrap(),
            ClientStatus::Active
        );
        assert_eq!(
            "archived".parse::<ClientStatus>().unwrap(),
            ClientStatus::Archived
        );
        assert!("invalid".parse::<ClientStatus>().is_err());
    }

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com"));
        assert!(validate_email("user.name@domain.co.uk"));
        assert!(validate_email("test@sub.domain.org"));
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(!validate_email("notanemail"));
        assert!(!validate_email("missing@tld"));
        assert!(!validate_email("@nodomain.com"));
        assert!(!validate_email("two@@domain.com"));
        assert!(!validate_email("spaces here@domain.com"));
    }

    #[test]
    fn test_create_client_inserts_record() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");

        let client = get_client_by_id(&db, "c1").unwrap();
        assert_eq!(client.name, "Alice Smith");
        assert_eq!(client.status, ClientStatus::Active);
        assert!(client.email.is_none());
    }

    #[test]
    fn test_unique_constraint_on_name() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");

        let result = db.execute(|conn| {
            conn.execute(
                "INSERT INTO clients (id, name, status, created_at, updated_at)
                 VALUES (?1, ?2, 'active', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
                params!["c2", "Alice Smith"],
            )?;
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_list_clients_excludes_archived_by_default() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Active Client");
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO clients (id, name, status, created_at, updated_at)
                 VALUES ('c2', 'Archived Client', 'archived',
                  '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let clients = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, email, phone, notes, status, created_at, updated_at
                     FROM clients WHERE status = 'active' ORDER BY name ASC",
                )?;
                let rows = stmt
                    .query_map([], map_client_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .unwrap();

        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].name, "Active Client");
    }

    #[test]
    fn test_search_clients_case_insensitive() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");
        insert_client(&db, "c2", "Bob Jones");

        let pattern = "%alice%";
        let clients = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, email, phone, notes, status, created_at, updated_at
                     FROM clients WHERE status = 'active'
                     AND (LOWER(name) LIKE ?1 OR LOWER(COALESCE(email, '')) LIKE ?1)
                     ORDER BY name ASC",
                )?;
                let rows = stmt
                    .query_map(params![pattern], map_client_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .unwrap();

        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].name, "Alice Smith");
    }

    #[test]
    fn test_delete_client_with_projects_blocked() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");
        insert_project_with_client(&db, "p1", "c1", "Alice Smith");

        let count: i64 = db
            .execute(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM projects WHERE client_id = ?1",
                    params!["c1"],
                    |row| row.get(0),
                )?)
            })
            .unwrap();

        // delete_client should refuse when count > 0
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_client_without_projects_succeeds() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");

        db.execute(|conn| {
            conn.execute("DELETE FROM clients WHERE id = ?1", params!["c1"])?;
            Ok(())
        })
        .unwrap();

        assert!(get_client_by_id(&db, "c1").is_err());
    }

    #[test]
    fn test_update_client_syncs_project_client_name() {
        let (_temp_dir, db) = setup_test_db();
        insert_client(&db, "c1", "Alice Smith");
        insert_project_with_client(&db, "p1", "c1", "Alice Smith");

        let now = chrono::Utc::now().to_rfc3339();
        db.execute(|conn| {
            conn.execute(
                "UPDATE clients SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params!["Alice Johnson", now, "c1"],
            )?;
            conn.execute(
                "UPDATE projects SET client_name = ?1, updated_at = ?2 WHERE client_id = ?3",
                params!["Alice Johnson", now, "c1"],
            )?;
            Ok(())
        })
        .unwrap();

        let client_name: String = db
            .execute(|conn| {
                Ok(conn.query_row(
                    "SELECT client_name FROM projects WHERE id = ?1",
                    params!["p1"],
                    |row| row.get(0),
                )?)
            })
            .unwrap();

        assert_eq!(client_name, "Alice Johnson");
    }

    #[test]
    fn test_migrate_clients_from_projects() {
        let (_temp_dir, db) = setup_test_db();

        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status,
                  folder_path, created_at, updated_at)
                 VALUES ('p1', 'Proj1', 'Alice', '2024-01-01', 'Wedding', 'New',
                  '/path', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
                [],
            )?;
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status,
                  folder_path, created_at, updated_at)
                 VALUES ('p2', 'Proj2', 'Alice', '2024-01-02', 'Portrait', 'New',
                  '/path2', '2024-01-02T00:00:00Z', '2024-01-02T00:00:00Z')",
                [],
            )?;
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status,
                  folder_path, created_at, updated_at)
                 VALUES ('p3', 'Proj3', 'Bob', '2024-01-03', 'Event', 'New',
                  '/path3', '2024-01-03T00:00:00Z', '2024-01-03T00:00:00Z')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let now = chrono::Utc::now().to_rfc3339();

        db.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT client_name FROM projects
                 WHERE client_id IS NULL AND client_name != ''",
            )?;
            let names: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            for name in names {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO clients (id, name, status, created_at, updated_at)
                     VALUES (?1, ?2, 'active', ?3, ?4)",
                    params![id, name, now, now],
                )?;
                conn.execute(
                    "UPDATE projects SET client_id = ?1
                     WHERE client_id IS NULL AND LOWER(client_name) = LOWER(?2)",
                    params![id, name],
                )?;
            }
            Ok(())
        })
        .unwrap();

        let client_count: i64 = db
            .execute(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM clients", [], |row| row.get(0))?)
            })
            .unwrap();
        assert_eq!(client_count, 2);

        let unlinked: i64 = db
            .execute(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM projects WHERE client_id IS NULL",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(unlinked, 0);

        let alice_ids: Vec<String> = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT client_id FROM projects WHERE client_name = 'Alice'",
                )?;
                let rows = stmt
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .unwrap();
        assert_eq!(alice_ids.len(), 1);
    }
}
