use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    Ok(project)
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
