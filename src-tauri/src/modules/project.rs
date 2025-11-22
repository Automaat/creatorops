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

/// Public function to invalidate cache from other modules (now no-op with SQLite)
pub fn invalidate_project_cache() {
    // No-op: SQLite handles consistency automatically
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

/// Migrate existing projects from JSON files to SQLite
#[tauri::command]
pub async fn migrate_projects_to_db() -> Result<usize, String> {
    let home_dir = get_home_dir()?;
    let base_path = home_dir.join("CreatorOps").join("Projects");

    if !base_path.exists() {
        return Ok(0);
    }

    let mut migrated = 0;

    for entry in fs::read_dir(&base_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        // Skip non-directories and hidden files
        if !path.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        let metadata_path = path.join("project.json");
        if let Ok(json_data) = fs::read_to_string(&metadata_path) {
            if let Ok(project) = serde_json::from_str::<Project>(&json_data) {
                // Check if already exists
                let exists = with_db(|conn| {
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM projects WHERE id = ?1",
                        params![project.id],
                        |row| row.get(0),
                    )?;
                    Ok(count > 0)
                })
                .map_err(|e| format!("Failed to check project existence: {}", e))?;

                if !exists {
                    // Insert into database
                    with_db(|conn| {
                        conn.execute(
                            "INSERT INTO projects (id, name, client_name, date, shoot_type, status, folder_path, created_at, updated_at, deadline)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                            params![
                                project.id,
                                project.name,
                                project.client_name,
                                project.date,
                                project.shoot_type,
                                project.status.to_string(),
                                project.folder_path,
                                project.created_at,
                                project.updated_at,
                                project.deadline,
                            ],
                        )?;
                        Ok(())
                    })
                    .map_err(|e| format!("Failed to insert project: {}", e))?;

                    migrated += 1;
                }
            }
        }
    }

    Ok(migrated)
}
