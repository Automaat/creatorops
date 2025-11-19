use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportHistory {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub source_path: String,
    pub destination_path: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub started_at: String,
    pub completed_at: String,
    pub status: ImportStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct SaveImportHistoryParams {
    pub project_id: String,
    pub project_name: String,
    pub source_path: String,
    pub destination_path: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub started_at: String,
    pub error_message: Option<String>,
}

#[tauri::command]
pub async fn save_import_history(params: SaveImportHistoryParams) -> Result<ImportHistory, String> {
    let id = Uuid::new_v4().to_string();
    let completed_at = chrono::Utc::now().to_rfc3339();

    let status = if params.files_copied == 0 {
        ImportStatus::Failed
    } else if params.files_skipped > 0 {
        ImportStatus::Partial
    } else {
        ImportStatus::Success
    };

    let history = ImportHistory {
        id: id.clone(),
        project_id: params.project_id,
        project_name: params.project_name,
        source_path: params.source_path,
        destination_path: params.destination_path,
        files_copied: params.files_copied,
        files_skipped: params.files_skipped,
        total_bytes: params.total_bytes,
        started_at: params.started_at,
        completed_at,
        status,
        error_message: params.error_message,
    };

    // Save to history file
    let history_path = get_history_file_path()?;
    let mut histories = load_all_histories().await.unwrap_or_default();
    histories.insert(0, history.clone());

    // Keep only last 100 imports
    if histories.len() > 100 {
        histories.truncate(100);
    }

    let json_data = serde_json::to_string_pretty(&histories).map_err(|e| e.to_string())?;
    fs::write(&history_path, json_data).map_err(|e| e.to_string())?;

    Ok(history)
}

#[tauri::command]
pub async fn get_import_history(limit: Option<usize>) -> Result<Vec<ImportHistory>, String> {
    let histories = load_all_histories().await?;
    let limit = limit.unwrap_or(50);
    Ok(histories.into_iter().take(limit).collect())
}

#[tauri::command]
pub async fn get_project_import_history(project_id: String) -> Result<Vec<ImportHistory>, String> {
    let histories = load_all_histories().await?;
    Ok(histories
        .into_iter()
        .filter(|h| h.project_id == project_id)
        .collect())
}

async fn load_all_histories() -> Result<Vec<ImportHistory>, String> {
    let history_path = get_history_file_path()?;

    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let json_data = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json_data).map_err(|e| e.to_string())
}

fn get_history_file_path() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_path = home_dir.join("CreatorOps");
    fs::create_dir_all(&base_path).map_err(|e| e.to_string())?;
    Ok(base_path.join("import_history.json"))
}

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
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            format!("{}", duration.as_secs())
        }
    }
}
