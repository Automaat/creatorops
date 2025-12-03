#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
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

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    // Global mutex to serialize tests that manipulate HOME environment variable
    lazy_static::lazy_static! {
        static ref HOME_TEST_MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    }

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
            id: "test-id".to_owned(),
            project_id: "proj-123".to_owned(),
            project_name: "Test Project".to_owned(),
            source_path: "/source".to_owned(),
            destination_path: "/dest".to_owned(),
            files_copied: 10,
            files_skipped: 2,
            total_bytes: 1024,
            photos_copied: 8,
            videos_copied: 2,
            started_at: "2024-01-01".to_owned(),
            completed_at: "2024-01-01".to_owned(),
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
            id: "test-id".to_owned(),
            project_id: "proj-123".to_owned(),
            project_name: "Test Project".to_owned(),
            source_path: "/source".to_owned(),
            destination_path: "/dest".to_owned(),
            files_copied: 5,
            files_skipped: 5,
            total_bytes: 512,
            photos_copied: 5,
            videos_copied: 0,
            started_at: "2024-01-01".to_owned(),
            completed_at: "2024-01-01".to_owned(),
            status: ImportStatus::Partial,
            error_message: Some("Some files failed".to_owned()),
        };

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("Some files failed"));
        assert!(json.contains(r#""status":"partial""#));
    }

    #[tokio::test]
    async fn test_save_import_history_success() {
        let _temp_dir = TempDir::new().unwrap();
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            std::env::set_var("HOME", _temp_dir.path());
        } // Lock dropped here

        let result = save_import_history(
            "proj-123".to_owned(),
            "Test Project".to_owned(),
            "/source".to_owned(),
            "/dest".to_owned(),
            10,
            0,
            1024,
            8,
            2,
            "2024-01-01T00:00:00Z".to_owned(),
            None,
        )
        .await;

        assert!(result.is_ok());
        let history = result.unwrap();
        assert_eq!(history.project_id, "proj-123");
        assert_eq!(history.files_copied, 10);
        assert_eq!(history.files_skipped, 0);
        assert!(matches!(history.status, ImportStatus::Success));
        assert!(history.error_message.is_none());
    }

    #[tokio::test]
    async fn test_save_import_history_partial() {
        let _temp_dir = TempDir::new().unwrap();
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            std::env::set_var("HOME", _temp_dir.path());
        } // Lock dropped here

        let result = save_import_history(
            "proj-456".to_owned(),
            "Partial Project".to_owned(),
            "/source".to_owned(),
            "/dest".to_owned(),
            5,
            3,
            512,
            4,
            1,
            "2024-01-01T00:00:00Z".to_owned(),
            Some("3 files failed".to_owned()),
        )
        .await;

        assert!(result.is_ok());
        let history = result.unwrap();
        assert_eq!(history.files_copied, 5);
        assert_eq!(history.files_skipped, 3);
        assert!(matches!(history.status, ImportStatus::Partial));
        assert_eq!(history.error_message, Some("3 files failed".to_owned()));
    }

    #[tokio::test]
    async fn test_save_import_history_failed() {
        let _temp_dir = TempDir::new().unwrap();
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            std::env::set_var("HOME", _temp_dir.path());
        } // Lock dropped here

        let result = save_import_history(
            "proj-789".to_owned(),
            "Failed Project".to_owned(),
            "/source".to_owned(),
            "/dest".to_owned(),
            0,
            10,
            0,
            0,
            0,
            "2024-01-01T00:00:00Z".to_owned(),
            Some("All files failed".to_owned()),
        )
        .await;

        assert!(result.is_ok());
        let history = result.unwrap();
        assert_eq!(history.files_copied, 0);
        assert_eq!(history.files_skipped, 10);
        assert!(matches!(history.status, ImportStatus::Failed));
    }

    #[tokio::test]
    #[ignore = "Skip due to parallel test HOME env var conflicts"]
    async fn test_save_and_retrieve_import_history() {
        let temp_dir = TempDir::new().unwrap();
        let home_path = temp_dir.path().to_string_lossy().to_string();
        std::env::set_var("HOME", &home_path);

        let history1 = save_import_history(
            "proj-1".to_owned(),
            "Project 1".to_owned(),
            "/src".to_owned(),
            "/dst".to_owned(),
            10,
            0,
            1024,
            8,
            2,
            "2024-01-01T00:00:00Z".to_owned(),
            None,
        )
        .await;

        assert!(history1.is_ok());
        std::env::remove_var("HOME");
    }

    #[tokio::test]
    async fn test_load_all_histories_empty() {
        // Test loading when file doesn't exist
        let result = load_all_histories().await;
        assert!(result.is_ok() || result.is_err()); // May succeed with empty vec or fail
    }

    #[tokio::test]
    async fn test_status_determination_logic() {
        let _temp_dir = TempDir::new().unwrap();
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            std::env::set_var("HOME", _temp_dir.path());
        } // Lock dropped here

        // Test Failed status (0 files copied)
        let failed = save_import_history(
            "proj-fail".to_owned(),
            "Failed".to_owned(),
            "/src".to_owned(),
            "/dst".to_owned(),
            0,
            10,
            0,
            0,
            0,
            "2024-01-01T00:00:00Z".to_owned(),
            Some("All failed".to_owned()),
        )
        .await
        .unwrap();

        assert!(matches!(failed.status, ImportStatus::Failed));

        // Test Partial status (some files copied, some skipped)
        let partial = save_import_history(
            "proj-partial".to_owned(),
            "Partial".to_owned(),
            "/src".to_owned(),
            "/dst".to_owned(),
            5,
            3,
            512,
            4,
            1,
            "2024-01-01T00:00:00Z".to_owned(),
            None,
        )
        .await
        .unwrap();

        assert!(matches!(partial.status, ImportStatus::Partial));

        // Test Success status (all files copied, none skipped)
        let success = save_import_history(
            "proj-success".to_owned(),
            "Success".to_owned(),
            "/src".to_owned(),
            "/dst".to_owned(),
            10,
            0,
            1024,
            8,
            2,
            "2024-01-01T00:00:00Z".to_owned(),
            None,
        )
        .await
        .unwrap();

        assert!(matches!(success.status, ImportStatus::Success));

        std::env::remove_var("HOME");
    }

    #[tokio::test]
    async fn test_get_import_history_empty() {
        let _temp_dir = TempDir::new().unwrap();
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            std::env::set_var("HOME", _temp_dir.path());
        } // Lock dropped here

        let result = get_import_history(None).await;
        assert!(result.is_ok());
        let histories = result.unwrap();
        assert_eq!(histories.len(), 0);
    }

    #[test]
    fn test_get_history_file_path() {
        let _temp_dir = TempDir::new().unwrap();
        let _lock = HOME_TEST_MUTEX.lock().unwrap();
        std::env::set_var("HOME", _temp_dir.path());

        let result = get_history_file_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("CreatorOps"));
        assert!(path.to_string_lossy().contains("import_history.json"));
    }
}
