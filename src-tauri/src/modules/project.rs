use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

use crate::modules::db::with_db;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            ProjectStatus::New => "New",
            ProjectStatus::Importing => "Importing",
            ProjectStatus::Editing => "Editing",
            ProjectStatus::Delivered => "Delivered",
            ProjectStatus::Archived => "Archived",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for ProjectStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "New" => Ok(ProjectStatus::New),
            "Importing" => Ok(ProjectStatus::Importing),
            "Editing" => Ok(ProjectStatus::Editing),
            "Delivered" => Ok(ProjectStatus::Delivered),
            "Archived" => Ok(ProjectStatus::Archived),
            _ => Err(format!("Invalid project status: {}", s)),
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
        format!("{}_{}", date, sanitized_client)
    } else {
        let sanitized_type = sanitize_path_component(&shoot_type);
        format!("{}_{}_{}", date, sanitized_client, sanitized_type)
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
    with_db(|conn| {
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
    .map_err(|e| format!("Failed to insert project: {}", e))?;

    Ok(project)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects ORDER BY updated_at DESC")?;

        let projects = stmt
            .query_map([], map_project_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    })
    .map_err(|e| format!("Database error: {}", e))
}

/// Force refresh project cache (now just returns list)
#[tauri::command]
pub async fn refresh_projects() -> Result<Vec<Project>, String> {
    list_projects().await
}

#[tauri::command]
pub async fn delete_project(project_id: String) -> Result<(), String> {
    // Get project folder path before deletion
    let folder_path = with_db(|conn| {
        let mut stmt = conn.prepare("SELECT folder_path FROM projects WHERE id = ?1")?;

        let path: String = stmt.query_row(params![project_id], |row| row.get(0))?;

        Ok(path)
    })
    .map_err(|e| format!("Database error: {}", e))?;

    // Delete project folder first (if this fails, DB remains consistent)
    fs::remove_dir_all(&folder_path)
        .map_err(|e| format!("Failed to delete project folder: {}", e))?;

    // Delete from database (only after filesystem deletion succeeds)
    with_db(|conn| {
        conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        Ok(())
    })
    .map_err(|e| format!("Failed to delete project from database: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_project_status(
    project_id: String,
    new_status: ProjectStatus,
) -> Result<Project, String> {
    let now = chrono::Utc::now().to_rfc3339();

    // Update in database
    with_db(|conn| {
        conn.execute(
            "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.to_string(), now, project_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update project status: {}", e))?;

    // Fetch and return updated project
    get_project_by_id(&project_id)
}

#[tauri::command]
pub async fn update_project_deadline(
    project_id: String,
    deadline: Option<String>,
) -> Result<Project, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let deadline_value = deadline.filter(|d| !d.is_empty());

    // Update in database
    with_db(|conn| {
        conn.execute(
            "UPDATE projects SET deadline = ?1, updated_at = ?2 WHERE id = ?3",
            params![deadline_value, now, project_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update project deadline: {}", e))?;

    // Fetch and return updated project
    get_project_by_id(&project_id)
}

/// Helper function to get project by ID
fn get_project_by_id(project_id: &str) -> Result<Project, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline FROM projects WHERE id = ?1")?;

        let project = stmt.query_row(params![project_id], map_project_row)?;

        Ok(project)
    })
    .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn get_project(project_id: String) -> Result<Project, String> {
    get_project_by_id(&project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db::init_db;
    use tempfile::TempDir;

    fn setup_test_db() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        init_db().unwrap();
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
            id: "test-123".to_string(),
            name: "Test Project".to_string(),
            client_name: "Test Client".to_string(),
            date: "2024-01-15".to_string(),
            shoot_type: "Wedding".to_string(),
            status: ProjectStatus::New,
            folder_path: "/path/to/project".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
            deadline: Some("2024-02-01".to_string()),
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

    #[tokio::test]
    #[ignore] // Skip DB tests due to parallel execution conflicts
    async fn test_create_project() {
        setup_test_db();

        let result = create_project(
            "Wedding Shoot".to_string(),
            "John Doe".to_string(),
            "2024-06-15".to_string(),
            "Wedding".to_string(),
            Some("2024-07-01".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.name, "Wedding Shoot");
        assert_eq!(project.client_name, "John Doe");
        assert_eq!(project.status, ProjectStatus::New);
        assert_eq!(project.deadline, Some("2024-07-01".to_string()));

        std::env::remove_var("HOME");
    }

    #[test]
    fn test_project_struct_fields() {
        let project = Project {
            id: "test-123".to_string(),
            name: "Wedding Shoot".to_string(),
            client_name: "John Doe".to_string(),
            date: "2024-06-15".to_string(),
            shoot_type: "Wedding".to_string(),
            status: ProjectStatus::New,
            folder_path: "/path/to/project".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
            deadline: Some("2024-07-01".to_string()),
        };

        assert_eq!(project.id, "test-123");
        assert_eq!(project.name, "Wedding Shoot");
        assert_eq!(project.client_name, "John Doe");
        assert_eq!(project.status, ProjectStatus::New);
        assert_eq!(project.deadline, Some("2024-07-01".to_string()));
    }

    #[test]
    fn test_project_without_deadline() {
        let project = Project {
            id: "test-456".to_string(),
            name: "Portrait".to_string(),
            client_name: "Jane Smith".to_string(),
            date: "2024-05-20".to_string(),
            shoot_type: "Portrait".to_string(),
            status: ProjectStatus::Editing,
            folder_path: "/path".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
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
                id: "test".to_string(),
                name: "Test".to_string(),
                client_name: "Client".to_string(),
                date: "2024-01-01".to_string(),
                shoot_type: "Event".to_string(),
                status: status.clone(),
                folder_path: "/path".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                deadline: None,
            };

            assert_eq!(project.status, status);
        }
    }
}
