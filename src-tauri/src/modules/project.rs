use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;
use walkdir::WalkDir;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProjectStatus {
    Importing,
    Editing,
    Delivered,
    Archived,
}

#[tauri::command]
pub async fn create_project(
    name: String,
    client_name: String,
    date: String,
    shoot_type: String,
) -> Result<Project, String> {
    let id = Uuid::new_v4().to_string();

    // Create folder structure: YYYY-MM-DD_ClientName_ProjectType/[RAW, Selects, Delivery]
    let folder_name = format!(
        "{}_{}_{}",
        date,
        client_name.replace(" ", ""),
        shoot_type.replace(" ", "")
    );

    // Default location (should be configurable in settings)
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps").join("Projects");
    let project_path = base_path.join(&folder_name);

    // Create directory structure
    fs::create_dir_all(&project_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("RAW")).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("Selects")).map_err(|e| e.to_string())?;
    fs::create_dir_all(project_path.join("Delivery")).map_err(|e| e.to_string())?;

    let now = chrono::Utc::now().to_rfc3339();

    let project = Project {
        id,
        name,
        client_name,
        date,
        shoot_type,
        status: ProjectStatus::Editing,
        folder_path: project_path.to_string_lossy().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    // Save project metadata to a JSON file
    let metadata_path = project_path.join("project.json");
    let json_data = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    fs::write(&metadata_path, json_data).map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps").join("Projects");

    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    // Scan for project directories
    for entry in WalkDir::new(&base_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == base_path {
            continue;
        }

        let metadata_path = path.join("project.json");
        if metadata_path.exists() {
            if let Ok(json_data) = fs::read_to_string(&metadata_path) {
                if let Ok(project) = serde_json::from_str::<Project>(&json_data) {
                    projects.push(project);
                }
            }
        }
    }

    // Sort by updated_at descending
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(projects)
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
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps").join("Projects");

    if !base_path.exists() {
        return Err("Projects directory does not exist".to_string());
    }

    // Find project by ID
    for entry in WalkDir::new(&base_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == base_path {
            continue;
        }

        let metadata_path = path.join("project.json");
        if metadata_path.exists() {
            if let Ok(json_data) = fs::read_to_string(&metadata_path) {
                if let Ok(project) = serde_json::from_str::<Project>(&json_data) {
                    if project.id == project_id {
                        // Delete entire project folder
                        fs::remove_dir_all(path)
                            .map_err(|e| format!("Failed to delete project folder: {}", e))?;
                        return Ok(());
                    }
                }
            }
        }
    }

    Err("Project not found".to_string())
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
