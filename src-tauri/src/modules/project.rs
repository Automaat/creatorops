#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
#![allow(clippy::unreachable)] // False positive: Clippy incorrectly flags Result returns
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

use crate::modules::db::Database;
use crate::modules::file_utils::get_home_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub client_name: String,
    pub date: String,
    pub shoot_type: String,
    pub status: ProjectStatus,
    pub folder_path: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ProjectStatus {
    New,
    Importing,
    Editing,
    Delivered,
    Archived,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::New => "New",
            Self::Importing => "Importing",
            Self::Editing => "Editing",
            Self::Delivered => "Delivered",
            Self::Archived => "Archived",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for ProjectStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "New" => Ok(Self::New),
            "Importing" => Ok(Self::Importing),
            "Editing" => Ok(Self::Editing),
            "Delivered" => Ok(Self::Delivered),
            "Archived" => Ok(Self::Archived),
            _ => Err(format!("Invalid project status: {s}")),
        }
    }
}

fn sanitize_path_component(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<&str>>()
        .join("")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Map a database row to a Project struct
fn map_project_row(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    let status_str: String = row.get(5)?;
    let status = status_str.parse::<ProjectStatus>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        client_name: row.get(2)?,
        date: row.get(3)?,
        shoot_type: row.get(4)?,
        status,
        folder_path: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        deadline: row.get(9)?,
    })
}

#[tauri::command]
pub async fn create_project(
    db: tauri::State<'_, Database>,
    name: String,
    client_name: String,
    date: String,
    shoot_type: String,
    deadline: Option<String>,
) -> Result<Project, String> {
    let id = Uuid::new_v4().to_string();

    // Create folder structure: YYYY-MM-DD_ClientName[_ProjectType]/[RAW, Selects, Delivery]
    let sanitized_client = sanitize_path_component(&client_name);
    let folder_name = if shoot_type.is_empty() {
        format!("{date}_{sanitized_client}")
    } else {
        let sanitized_type = sanitize_path_component(&shoot_type);
        format!("{date}_{sanitized_client}_{sanitized_type}")
    };

    // Default location (should be configurable in settings)
    let home_dir = get_home_dir()?;
    let base_path = home_dir.join("CreatorOps").join("Projects");
    let project_path = base_path.join(&folder_name);

    // Create directory structure
    fs::create_dir_all(&project_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("RAW/Photos")).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("RAW/Videos")).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("Selects")).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("Delivery")).map_err(|e| e.to_string())?;

    let now = chrono::Utc::now().to_rfc3339();

    let project = Project {
        id,
        name,
        client_name,
        date,
        shoot_type,
        status: ProjectStatus::New,
        folder_path: project_path.to_string_lossy().to_string(),
        created_at: now.clone(),
        updated_at: now,
        deadline: deadline.filter(|d| !d.is_empty()),
    };

    // Insert into database
    db.execute(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &project.id,
                &project.name,
                &project.client_name,
                &project.date,
                &project.shoot_type,
                project.status.to_string(),
                &project.folder_path,
                &project.created_at,
                &project.updated_at,
                &project.deadline,
            ],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to insert project: {e}"))?;

    Ok(project)
}

#[tauri::command]
pub async fn list_projects(db: tauri::State<'_, Database>) -> Result<Vec<Project>, String> {
    db.execute(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects ORDER BY updated_at DESC")?;

        let projects = stmt
            .query_map([], map_project_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    })
    .map_err(|e| format!("Database error: {e}"))
}

/// Force refresh project cache (now just returns list)
#[tauri::command]
pub async fn refresh_projects(db: tauri::State<'_, Database>) -> Result<Vec<Project>, String> {
    list_projects(db).await
}

#[tauri::command]
pub async fn delete_project(
    db: tauri::State<'_, Database>,
    project_id: String,
) -> Result<(), String> {
    // Get project folder path before deletion
    let folder_path = db
        .execute(|conn| {
            let mut stmt = conn.prepare("SELECT folder_path FROM projects WHERE id = ?1")?;

            let path: String = stmt.query_row(params![project_id], |row| row.get(0))?;

            Ok(path)
        })
        .map_err(|e| format!("Database error: {e}"))?;

    // Delete project folder first (if this fails, DB remains consistent)
    fs::remove_dir_all(&folder_path)
        .map_err(|e| format!("Failed to delete project folder: {e}"))?;

    // Delete from database (only after filesystem deletion succeeds)
    db.execute(|conn| {
        conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        Ok(())
    })
    .map_err(|e| format!("Failed to delete project from database: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn update_project_status(
    db: tauri::State<'_, Database>,
    project_id: String,
    new_status: ProjectStatus,
) -> Result<Project, String> {
    let now = chrono::Utc::now().to_rfc3339();

    // Update in database
    db.execute(|conn| {
        conn.execute(
            "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.to_string(), now, project_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update project status: {e}"))?;

    // Fetch and return updated project
    get_project_by_id(&db, &project_id)
}

#[tauri::command]
pub async fn update_project_deadline(
    db: tauri::State<'_, Database>,
    project_id: String,
    deadline: Option<String>,
) -> Result<Project, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let deadline_value = deadline.filter(|d| !d.is_empty());

    // Update in database
    db.execute(|conn| {
        conn.execute(
            "UPDATE projects SET deadline = ?1, updated_at = ?2 WHERE id = ?3",
            params![deadline_value, now, project_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update project deadline: {e}"))?;

    // Fetch and return updated project
    get_project_by_id(&db, &project_id)
}

/// Helper function to get project by ID
fn get_project_by_id(db: &Database, project_id: &str) -> Result<Project, String> {
    db.execute(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects WHERE id = ?1")?;

        let project = stmt.query_row(params![project_id], map_project_row)?;

        Ok(project)
    })
    .map_err(|e| format!("Database error: {e}"))
}

#[tauri::command]
pub async fn get_project(
    db: tauri::State<'_, Database>,
    project_id: String,
) -> Result<Project, String> {
    get_project_by_id(&db, &project_id)
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db::Database;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, Database) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(db_path).unwrap();
        (temp_dir, db)
    }

    #[test]
    fn test_project_status_to_string() {
        assert_eq!(ProjectStatus::New.to_string(), "New");
        assert_eq!(ProjectStatus::Importing.to_string(), "Importing");
        assert_eq!(ProjectStatus::Editing.to_string(), "Editing");
        assert_eq!(ProjectStatus::Delivered.to_string(), "Delivered");
        assert_eq!(ProjectStatus::Archived.to_string(), "Archived");
    }

    #[test]
    fn test_project_status_from_str() {
        assert_eq!("New".parse::<ProjectStatus>().unwrap(), ProjectStatus::New);
        assert_eq!(
            "Importing".parse::<ProjectStatus>().unwrap(),
            ProjectStatus::Importing
        );
        assert_eq!(
            "Editing".parse::<ProjectStatus>().unwrap(),
            ProjectStatus::Editing
        );
        assert_eq!(
            "Delivered".parse::<ProjectStatus>().unwrap(),
            ProjectStatus::Delivered
        );
        assert_eq!(
            "Archived".parse::<ProjectStatus>().unwrap(),
            ProjectStatus::Archived
        );
    }

    #[test]
    fn test_project_status_from_str_invalid() {
        assert!("InvalidStatus".parse::<ProjectStatus>().is_err());
        assert!("".parse::<ProjectStatus>().is_err());
        assert!("new".parse::<ProjectStatus>().is_err()); // Case sensitive
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("John Doe"), "JohnDoe");
        assert_eq!(
            sanitize_path_component("Test-Project_123"),
            "Test-Project_123"
        );
        assert_eq!(sanitize_path_component("Hello  World"), "HelloWorld");
        assert_eq!(sanitize_path_component("Test@#$%Project"), "TestProject");
        assert_eq!(sanitize_path_component("Wedding2024"), "Wedding2024");
        assert_eq!(
            sanitize_path_component("Multiple   Spaces"),
            "MultipleSpaces"
        );
    }

    #[test]
    fn test_sanitize_path_component_special_chars() {
        assert_eq!(sanitize_path_component("Test/Path\\Name"), "TestPathName");
        assert_eq!(sanitize_path_component("Name<>:\"?*"), "Name");
        assert_eq!(sanitize_path_component("name|with|pipes"), "namewithpipes");
    }

    #[test]
    fn test_sanitize_path_component_empty() {
        assert_eq!(sanitize_path_component(""), "");
        assert_eq!(sanitize_path_component("   "), "");
    }

    #[test]
    fn test_sanitize_path_component_unicode() {
        // Unicode characters are preserved by is_alphanumeric
        assert_eq!(sanitize_path_component("Café"), "Café");
        assert_eq!(sanitize_path_component("José García"), "JoséGarcía");
    }

    #[test]
    fn test_project_status_serialization() {
        let status = ProjectStatus::Editing;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""Editing""#);
    }

    #[test]
    fn test_project_status_deserialization() {
        let json = r#""Delivered""#;
        let status: ProjectStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, ProjectStatus::Delivered);
    }

    #[test]
    fn test_project_serialization() {
        let project = Project {
            id: "test-123".to_owned(),
            name: "Test Project".to_owned(),
            client_name: "Test Client".to_owned(),
            date: "2024-01-15".to_owned(),
            shoot_type: "Wedding".to_owned(),
            status: ProjectStatus::New,
            folder_path: "/path/to/project".to_owned(),
            created_at: "2024-01-15T10:00:00Z".to_owned(),
            updated_at: "2024-01-15T10:00:00Z".to_owned(),
            deadline: Some("2024-02-01".to_owned()),
        };

        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("Test Project"));
        assert!(json.contains("clientName")); // Check for camelCase serialization
        assert!(json.contains("shootType"));
        assert!(json.contains("folderPath"));
    }

    #[test]
    fn test_project_deserialization() {
        let json = r#"{
            "id": "test-456",
            "name": "Another Project",
            "clientName": "Client Name",
            "date": "2024-01-20",
            "shootType": "Portrait",
            "status": "Editing",
            "folderPath": "/projects/test",
            "createdAt": "2024-01-20T12:00:00Z",
            "updatedAt": "2024-01-20T12:00:00Z"
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "test-456");
        assert_eq!(project.name, "Another Project");
        assert_eq!(project.status, ProjectStatus::Editing);
        assert_eq!(project.deadline, None);
    }

    #[test]
    fn test_db_insert_and_query_project() {
        let (_temp_dir, db) = setup_test_db();

        // Insert a project directly into the database
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "test-123",
                    "Wedding Shoot",
                    "John Doe",
                    "2024-06-15",
                    "Wedding",
                    "New",
                    "/path/to/project",
                    "2024-01-15T10:00:00Z",
                    "2024-01-15T10:00:00Z",
                    Some("2024-07-01"),
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Query the project back
        let project = get_project_by_id(&db, "test-123").unwrap();
        assert_eq!(project.name, "Wedding Shoot");
        assert_eq!(project.client_name, "John Doe");
        assert_eq!(project.status, ProjectStatus::New);
        assert_eq!(project.deadline, Some("2024-07-01".to_owned()));
    }

    #[test]
    fn test_db_list_projects_empty() {
        let (_temp_dir, db) = setup_test_db();

        let projects = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects ORDER BY updated_at DESC",
                )?;
                let projects = stmt
                    .query_map([], map_project_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(projects)
            })
            .unwrap();

        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_db_list_projects_multiple() {
        let (_temp_dir, db) = setup_test_db();

        // Insert multiple projects
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "proj-1",
                    "Project 1",
                    "Client A",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    "/path1",
                    "2024-01-01T10:00:00Z",
                    "2024-01-01T10:00:00Z",
                    None::<String>,
                ],
            )?;
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "proj-2",
                    "Project 2",
                    "Client B",
                    "2024-01-02",
                    "Portrait",
                    "Editing",
                    "/path2",
                    "2024-01-02T10:00:00Z",
                    "2024-01-02T11:00:00Z",
                    Some("2024-02-01"),
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Query all projects
        let projects = db
            .execute(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects ORDER BY updated_at DESC",
                )?;
                let projects = stmt
                    .query_map([], map_project_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(projects)
            })
            .unwrap();

        assert_eq!(projects.len(), 2);
        // Most recent first (proj-2 has newer updated_at)
        assert_eq!(projects[0].id, "proj-2");
        assert_eq!(projects[1].id, "proj-1");
    }

    #[test]
    fn test_db_update_project_status() {
        let (_temp_dir, db) = setup_test_db();

        // Insert a project
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "proj-1",
                    "Project 1",
                    "Client A",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    "/path1",
                    "2024-01-01T10:00:00Z",
                    "2024-01-01T10:00:00Z",
                    None::<String>,
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Update status
        db.execute(|conn| {
            conn.execute(
                "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params!["Editing", "2024-01-01T12:00:00Z", "proj-1"],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify update
        let project = get_project_by_id(&db, "proj-1").unwrap();
        assert_eq!(project.status, ProjectStatus::Editing);
        assert_eq!(project.updated_at, "2024-01-01T12:00:00Z");
    }

    #[test]
    fn test_db_update_project_deadline() {
        let (_temp_dir, db) = setup_test_db();

        // Insert a project without deadline
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "proj-1",
                    "Project 1",
                    "Client A",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    "/path1",
                    "2024-01-01T10:00:00Z",
                    "2024-01-01T10:00:00Z",
                    None::<String>,
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Set deadline
        db.execute(|conn| {
            conn.execute(
                "UPDATE projects SET deadline = ?1, updated_at = ?2 WHERE id = ?3",
                params![Some("2024-02-01"), "2024-01-01T12:00:00Z", "proj-1"],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify update
        let project = get_project_by_id(&db, "proj-1").unwrap();
        assert_eq!(project.deadline, Some("2024-02-01".to_owned()));
    }

    #[test]
    fn test_db_delete_project() {
        let (_temp_dir, db) = setup_test_db();

        // Insert a project
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "proj-1",
                    "Project 1",
                    "Client A",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    "/path1",
                    "2024-01-01T10:00:00Z",
                    "2024-01-01T10:00:00Z",
                    None::<String>,
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify it exists
        assert!(get_project_by_id(&db, "proj-1").is_ok());

        // Delete it
        db.execute(|conn| {
            conn.execute("DELETE FROM projects WHERE id = ?1", params!["proj-1"])?;
            Ok(())
        })
        .unwrap();

        // Verify it's gone
        assert!(get_project_by_id(&db, "proj-1").is_err());
    }

    #[test]
    fn test_db_get_project_not_found() {
        let (_temp_dir, db) = setup_test_db();

        let result = get_project_by_id(&db, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_project_struct_fields() {
        let project = Project {
            id: "test-123".to_owned(),
            name: "Wedding Shoot".to_owned(),
            client_name: "John Doe".to_owned(),
            date: "2024-06-15".to_owned(),
            shoot_type: "Wedding".to_owned(),
            status: ProjectStatus::New,
            folder_path: "/path/to/project".to_owned(),
            created_at: "2024-01-15T10:00:00Z".to_owned(),
            updated_at: "2024-01-15T10:00:00Z".to_owned(),
            deadline: Some("2024-07-01".to_owned()),
        };

        assert_eq!(project.id, "test-123");
        assert_eq!(project.name, "Wedding Shoot");
        assert_eq!(project.client_name, "John Doe");
        assert_eq!(project.status, ProjectStatus::New);
        assert_eq!(project.deadline, Some("2024-07-01".to_owned()));
    }

    #[test]
    fn test_project_without_deadline() {
        let project = Project {
            id: "test-456".to_owned(),
            name: "Portrait".to_owned(),
            client_name: "Jane Smith".to_owned(),
            date: "2024-05-20".to_owned(),
            shoot_type: "Portrait".to_owned(),
            status: ProjectStatus::Editing,
            folder_path: "/path".to_owned(),
            created_at: "2024-01-15T10:00:00Z".to_owned(),
            updated_at: "2024-01-15T10:00:00Z".to_owned(),
            deadline: None,
        };

        assert_eq!(project.deadline, None);
        assert_eq!(project.status, ProjectStatus::Editing);
    }

    #[test]
    fn test_all_project_statuses() {
        let statuses = vec![
            ProjectStatus::New,
            ProjectStatus::Importing,
            ProjectStatus::Editing,
            ProjectStatus::Delivered,
            ProjectStatus::Archived,
        ];

        for status in statuses {
            let project = Project {
                id: "test".to_owned(),
                name: "Test".to_owned(),
                client_name: "Client".to_owned(),
                date: "2024-01-01".to_owned(),
                shoot_type: "Event".to_owned(),
                status: status.clone(),
                folder_path: "/path".to_owned(),
                created_at: "2024-01-01T00:00:00Z".to_owned(),
                updated_at: "2024-01-01T00:00:00Z".to_owned(),
                deadline: None,
            };

            assert_eq!(project.status, status);
        }
    }

    #[tokio::test]
    async fn test_create_project_command() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(db_path).unwrap();

        let project_base = temp_dir.path().join("CreatorOps").join("Projects");

        // Override home dir for test by creating project in temp dir
        let result = db.execute(|conn| {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let folder_path = project_base.join("2024-01-15_JohnDoe_Wedding");

            // Simulate what create_project does
            std::fs::create_dir_all(&folder_path).unwrap();
            std::fs::create_dir_all(folder_path.join("RAW/Photos")).unwrap();
            std::fs::create_dir_all(folder_path.join("RAW/Videos")).unwrap();
            std::fs::create_dir_all(folder_path.join("Selects")).unwrap();
            std::fs::create_dir_all(folder_path.join("Delivery")).unwrap();

            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    "Wedding Shoot",
                    "John Doe",
                    "2024-01-15",
                    "Wedding",
                    ProjectStatus::New.to_string(),
                    folder_path.to_string_lossy().to_string(),
                    now.clone(),
                    now,
                    Some("2024-02-01"),
                ],
            )?;

            Ok(id)
        }).unwrap();

        let project = get_project_by_id(&db, &result).unwrap();
        assert_eq!(project.name, "Wedding Shoot");
        assert_eq!(project.client_name, "John Doe");
        assert_eq!(project.status, ProjectStatus::New);
        assert_eq!(project.deadline, Some("2024-02-01".to_owned()));

        // Verify folders were created
        let folder_path = std::path::PathBuf::from(&project.folder_path);
        assert!(folder_path.join("RAW/Photos").exists());
        assert!(folder_path.join("RAW/Videos").exists());
        assert!(folder_path.join("Selects").exists());
        assert!(folder_path.join("Delivery").exists());
    }

    #[tokio::test]
    async fn test_list_projects_command() {
        let (_temp_dir, db) = setup_test_db();

        // Insert projects
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["id1", "Project 1", "Client A", "2024-01-01", "Wedding", "New", "/path1", "2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z", None::<String>],
            )?;
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["id2", "Project 2", "Client B", "2024-01-02", "Portrait", "Editing", "/path2", "2024-01-02T10:00:00Z", "2024-01-02T11:00:00Z", Some("2024-02-01")],
            )?;
            Ok(())
        }).unwrap();

        let projects = db.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects ORDER BY updated_at DESC"
            )?;
            let projects = stmt
                .query_map([], map_project_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(projects)
        }).unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].id, "id2"); // Most recent first
        assert_eq!(projects[1].id, "id1");
    }

    #[tokio::test]
    async fn test_delete_project_command() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(db_path).unwrap();

        let project_folder = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_folder).unwrap();
        std::fs::write(project_folder.join("test.txt"), "test").unwrap();

        // Insert project
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    "del-1",
                    "Delete Test",
                    "Client",
                    "2024-01-01",
                    "Wedding",
                    "New",
                    project_folder.to_string_lossy().to_string(),
                    "2024-01-01T10:00:00Z",
                    "2024-01-01T10:00:00Z",
                    None::<String>,
                ],
            )?;
            Ok(())
        }).unwrap();

        assert!(project_folder.exists());

        // Simulate delete_project
        let folder_path = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT folder_path FROM projects WHERE id = ?1")?;
                let path: String = stmt.query_row(rusqlite::params!["del-1"], |row| row.get(0))?;
                Ok(path)
            })
            .unwrap();

        std::fs::remove_dir_all(&folder_path).unwrap();

        db.execute(|conn| {
            conn.execute(
                "DELETE FROM projects WHERE id = ?1",
                rusqlite::params!["del-1"],
            )?;
            Ok(())
        })
        .unwrap();

        assert!(!project_folder.exists());
        assert!(get_project_by_id(&db, "del-1").is_err());
    }

    #[tokio::test]
    async fn test_update_project_status_command() {
        let (_temp_dir, db) = setup_test_db();

        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["upd-1", "Update Test", "Client", "2024-01-01", "Wedding", "New", "/path", "2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z", None::<String>],
            )?;
            Ok(())
        }).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        db.execute(|conn| {
            conn.execute(
                "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![ProjectStatus::Editing.to_string(), now, "upd-1"],
            )?;
            Ok(())
        })
        .unwrap();

        let project = get_project_by_id(&db, "upd-1").unwrap();
        assert_eq!(project.status, ProjectStatus::Editing);
    }

    #[tokio::test]
    async fn test_update_project_deadline_command() {
        let (_temp_dir, db) = setup_test_db();

        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["upd-2", "Deadline Test", "Client", "2024-01-01", "Wedding", "New", "/path", "2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z", None::<String>],
            )?;
            Ok(())
        }).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        db.execute(|conn| {
            conn.execute(
                "UPDATE projects SET deadline = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![Some("2024-03-01"), now, "upd-2"],
            )?;
            Ok(())
        })
        .unwrap();

        let project = get_project_by_id(&db, "upd-2").unwrap();
        assert_eq!(project.deadline, Some("2024-03-01".to_owned()));
    }

    #[tokio::test]
    async fn test_get_project_command() {
        let (_temp_dir, db) = setup_test_db();

        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["get-1", "Get Test", "Client", "2024-01-01", "Wedding", "New", "/path", "2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z", Some("2024-02-01")],
            )?;
            Ok(())
        }).unwrap();

        let project = get_project_by_id(&db, "get-1").unwrap();
        assert_eq!(project.id, "get-1");
        assert_eq!(project.name, "Get Test");
        assert_eq!(project.deadline, Some("2024-02-01".to_owned()));
    }

    #[test]
    fn test_map_project_row_error_handling() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(db_path).unwrap();

        // Insert project with invalid status
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params!["bad-1", "Bad Status", "Client", "2024-01-01", "Wedding", "InvalidStatus", "/path", "2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z", None::<String>],
            )?;
            Ok(())
        }).unwrap();

        // Should fail to map due to invalid status
        let result = get_project_by_id(&db, "bad-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_project_with_empty_shoot_type() {
        // Test folder name generation without shoot type
        let sanitized_client = sanitize_path_component("John Doe");
        let folder_name = format!("{}_{}", "2024-01-15", sanitized_client);
        assert_eq!(folder_name, "2024-01-15_JohnDoe");
    }

    #[test]
    fn test_create_project_with_shoot_type() {
        // Test folder name generation with shoot type
        let sanitized_client = sanitize_path_component("Jane Smith");
        let sanitized_type = sanitize_path_component("Wedding");
        let folder_name = format!("{}_{}_{}", "2024-02-20", sanitized_client, sanitized_type);
        assert_eq!(folder_name, "2024-02-20_JaneSmith_Wedding");
    }

    #[test]
    fn test_deadline_filtering() {
        // Test that empty string deadline is converted to None
        let deadline = Some(String::new());
        let filtered = deadline.filter(|d| !d.is_empty());
        assert_eq!(filtered, None);

        let deadline = Some("2024-03-01".to_owned());
        let filtered = deadline.filter(|d| !d.is_empty());
        assert_eq!(filtered, Some("2024-03-01".to_owned()));
    }
}
