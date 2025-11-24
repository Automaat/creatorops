use crate::modules::file_utils::{get_home_dir, get_timestamp};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportHistory {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub source_path: String,
    pub destination_path: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub photos_copied: usize,
    pub videos_copied: usize,
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

#[tauri::command(rename_all = "camelCase")]
#[allow(clippy::too_many_arguments)]
pub async fn save_import_history(
    project_id: String,
    project_name: String,
    source_path: String,
    destination_path: String,
    files_copied: usize,
    files_skipped: usize,
    total_bytes: u64,
    photos_copied: usize,
    videos_copied: usize,
    started_at: String,
    error_message: Option<String>,
) -> Result<ImportHistory, String> {
    let id = Uuid::new_v4().to_string();
    let completed_at = get_timestamp();

    let status = if files_copied == 0 {
        ImportStatus::Failed
    } else if files_skipped > 0 {
        ImportStatus::Partial
    } else {
        ImportStatus::Success
    };

    let history = ImportHistory {
        id: id.clone(),
        project_id: project_id.clone(),
        project_name,
        source_path,
        destination_path,
        files_copied,
        files_skipped,
        total_bytes,
        photos_copied,
        videos_copied,
        started_at,
        completed_at,
        status,
        error_message,
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
    let home_dir = get_home_dir()?;
    let base_path = home_dir.join("CreatorOps");
    fs::create_dir_all(&base_path).map_err(|e| e.to_string())?;
    Ok(base_path.join("import_history.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ImportStatus::Success).unwrap(),
            r#""success""#
        );
        assert_eq!(
            serde_json::to_string(&ImportStatus::Partial).unwrap(),
            r#""partial""#
        );
        assert_eq!(
            serde_json::to_string(&ImportStatus::Failed).unwrap(),
            r#""failed""#
        );
    }

    #[test]
    fn test_import_status_deserialization() {
        assert!(matches!(
            serde_json::from_str::<ImportStatus>(r#""success""#).unwrap(),
            ImportStatus::Success
        ));
        assert!(matches!(
            serde_json::from_str::<ImportStatus>(r#""partial""#).unwrap(),
            ImportStatus::Partial
        ));
        assert!(matches!(
            serde_json::from_str::<ImportStatus>(r#""failed""#).unwrap(),
            ImportStatus::Failed
        ));
    }

    #[test]
    fn test_import_history_serialization() {
        let history = ImportHistory {
            id: "test-id".to_string(),
            project_id: "proj-123".to_string(),
            project_name: "Test Project".to_string(),
            source_path: "/source".to_string(),
            destination_path: "/dest".to_string(),
            files_copied: 10,
            files_skipped: 2,
            total_bytes: 1024,
            photos_copied: 8,
            videos_copied: 2,
            started_at: "2024-01-01".to_string(),
            completed_at: "2024-01-01".to_string(),
            status: ImportStatus::Success,
            error_message: None,
        };

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("proj-123"));
        assert!(json.contains("Test Project"));
    }

    #[test]
    fn test_import_history_with_error() {
        let history = ImportHistory {
            id: "test-id".to_string(),
            project_id: "proj-123".to_string(),
            project_name: "Test Project".to_string(),
            source_path: "/source".to_string(),
            destination_path: "/dest".to_string(),
            files_copied: 5,
            files_skipped: 5,
            total_bytes: 512,
            photos_copied: 5,
            videos_copied: 0,
            started_at: "2024-01-01".to_string(),
            completed_at: "2024-01-01".to_string(),
            status: ImportStatus::Partial,
            error_message: Some("Some files failed".to_string()),
        };

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("Some files failed"));
        assert!(json.contains(r#""status":"partial""#));
    }
}
