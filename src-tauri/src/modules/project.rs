use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use uuid::Uuid;

// Cached project list to avoid repeated directory scans
static PROJECT_CACHE: Mutex<Option<Vec<Project>>> = Mutex::new(None);

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

fn sanitize_path_component(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<&str>>()
        .join("")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
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
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
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

    // Save project metadata to a JSON file
    let metadata_path = project_path.join("project.json");
    let json_data = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    fs::write(&metadata_path, json_data).map_err(|e| e.to_string())?;

    // Invalidate cache since we created a new project
    invalidate_cache();

    Ok(project)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    // Check cache first
    {
        let cache = PROJECT_CACHE.lock().unwrap();
        if let Some(projects) = &*cache {
            return Ok(projects.clone());
        }
    }

    // Cache miss - scan directory
    let projects = scan_projects_directory()?;

    // Update cache
    {
        let mut cache = PROJECT_CACHE.lock().unwrap();
        *cache = Some(projects.clone());
    }

    Ok(projects)
}

/// Invalidate cache (call after project mutations)
fn invalidate_cache() {
    let mut cache = PROJECT_CACHE.lock().unwrap();
    *cache = None;
}

/// Public function to invalidate cache from other modules
pub fn invalidate_project_cache() {
    invalidate_cache();
}

/// Force refresh project cache
#[tauri::command]
pub async fn refresh_projects() -> Result<Vec<Project>, String> {
    invalidate_cache();
    list_projects().await
}

/// Scan projects directory (expensive operation)
fn scan_projects_directory() -> Result<Vec<Project>, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps").join("Projects");

    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    // Use fs::read_dir instead of WalkDir - much faster, only reads top-level
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
                projects.push(project);
            }
        }
    }

    // Sort by updated_at descending
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(projects)
}

/// Find project by ID and return (project, metadata_path)
fn find_project_by_id(project_id: &str) -> Result<(Project, std::path::PathBuf), String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps").join("Projects");

    if !base_path.exists() {
        return Err("Projects directory does not exist".to_string());
    }

    for entry in fs::read_dir(&base_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let metadata_path = path.join("project.json");
        if let Ok(json_data) = fs::read_to_string(&metadata_path) {
            if let Ok(project) = serde_json::from_str::<Project>(&json_data) {
                if project.id == project_id {
                    return Ok((project, metadata_path));
                }
            }
        }
    }

    Err("Project not found".to_string())
}

/// Add dirs crate dependency
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .and_then(|h| if h.is_empty() { None } else { Some(h) })
            .map(PathBuf::from)
    }
}

#[tauri::command]
pub async fn delete_project(project_id: String) -> Result<(), String> {
    let (_, metadata_path) = find_project_by_id(&project_id)?;

    // Get project folder path from metadata path
    let project_folder = metadata_path
        .parent()
        .ok_or("Failed to get project folder")?;

    // Delete entire project folder
    fs::remove_dir_all(project_folder)
        .map_err(|e| format!("Failed to delete project folder: {}", e))?;

    // Invalidate cache since we deleted a project
    invalidate_cache();

    Ok(())
}

#[tauri::command]
pub async fn update_project_status(
    project_id: String,
    new_status: ProjectStatus,
) -> Result<Project, String> {
    let (mut project, metadata_path) = find_project_by_id(&project_id)?;

    // Update status and timestamp
    project.status = new_status;
    project.updated_at = chrono::Utc::now().to_rfc3339();

    // Save updated project
    let json_data = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    fs::write(&metadata_path, json_data).map_err(|e| e.to_string())?;

    // Invalidate cache
    invalidate_cache();

    Ok(project)
}

#[tauri::command]
pub async fn update_project_deadline(
    project_id: String,
    deadline: Option<String>,
) -> Result<Project, String> {
    let (mut project, metadata_path) = find_project_by_id(&project_id)?;

    // Update deadline and timestamp
    project.deadline = deadline.filter(|d| !d.is_empty());
    project.updated_at = chrono::Utc::now().to_rfc3339();

    // Save updated project
    let json_data = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    fs::write(&metadata_path, json_data).map_err(|e| e.to_string())?;

    // Invalidate cache
    invalidate_cache();

    Ok(project)
}

mod chrono {
    pub struct Utc;

    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }

    pub struct DateTime;

    impl DateTime {
        pub fn to_rfc3339(&self) -> String {
            // Simple timestamp for now
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            format!("{}", duration.as_secs())
        }
    }
}
