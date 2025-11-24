use crate::modules::file_utils::{
    collect_files_recursive, count_files_and_size, get_home_dir, get_timestamp, verify_checksum,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use uuid::Uuid;

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks
const MAX_RETRY_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupJob {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub source_path: String,
    pub destination_id: String,
    pub destination_name: String,
    pub destination_path: String,
    pub status: BackupStatus,
    pub total_files: usize,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub bytes_transferred: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupProgress {
    pub job_id: String,
    pub file_name: String,
    pub current_file: usize,
    pub total_files: usize,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub eta: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupHistory {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub destination_name: String,
    pub destination_path: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub started_at: String,
    pub completed_at: String,
    pub status: BackupStatus,
    pub error_message: Option<String>,
}

// Global backup queue state
type BackupQueue = Arc<Mutex<HashMap<String, BackupJob>>>;

lazy_static::lazy_static! {
    static ref BACKUP_QUEUE: BackupQueue = Arc::new(Mutex::new(HashMap::new()));
}

/// Add a backup job to the queue
#[tauri::command]
pub async fn queue_backup(
    project_id: String,
    project_name: String,
    source_path: String,
    destination_id: String,
    destination_name: String,
    destination_path: String,
) -> Result<BackupJob, String> {
    let id = Uuid::new_v4().to_string();
    let now = get_timestamp();

    // Count files and calculate total size
    let (total_files, total_bytes) = count_files_and_size(&source_path)?;

    let job = BackupJob {
        id: id.clone(),
        project_id,
        project_name,
        source_path,
        destination_id,
        destination_name,
        destination_path,
        status: BackupStatus::Pending,
        total_files,
        files_copied: 0,
        files_skipped: 0,
        total_bytes,
        bytes_transferred: 0,
        created_at: now,
        started_at: None,
        completed_at: None,
        error_message: None,
    };

    let mut queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;
    queue.insert(id, job.clone());

    Ok(job)
}

/// Get all backup jobs in the queue
#[tauri::command]
pub async fn get_backup_queue() -> Result<Vec<BackupJob>, String> {
    let queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;

    let mut jobs: Vec<BackupJob> = queue.values().cloned().collect();
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(jobs)
}

/// Start a backup job
#[tauri::command]
pub async fn start_backup(window: tauri::Window, job_id: String) -> Result<BackupJob, String> {
    // Get job from queue
    let job = {
        let mut queue = BACKUP_QUEUE
            .lock()
            .map_err(|e| format!("Failed to lock queue: {}", e))?;
        let job = queue
            .get_mut(&job_id)
            .ok_or("Backup job not found")?
            .clone();

        if job.status != BackupStatus::Pending {
            return Err("Backup job is not pending".to_string());
        }

        job
    };

    // Update status to in-progress
    {
        let mut queue = BACKUP_QUEUE
            .lock()
            .map_err(|e| format!("Failed to lock queue: {}", e))?;
        if let Some(j) = queue.get_mut(&job_id) {
            j.status = BackupStatus::InProgress;
            j.started_at = Some(get_timestamp());
        }
    }

    // Perform backup in background
    let job_id_clone = job_id.clone();
    let window_clone = window.clone();
    tokio::spawn(async move {
        let result = perform_backup(&window_clone, &job_id_clone, &job).await;

        // Update job status
        let queue_result = BACKUP_QUEUE.lock();
        if let Ok(mut queue) = queue_result {
            if let Some(j) = queue.get_mut(&job_id_clone) {
                match result {
                    Ok((files_copied, files_skipped, bytes_transferred)) => {
                        j.status = BackupStatus::Completed;
                        j.files_copied = files_copied;
                        j.files_skipped = files_skipped;
                        j.bytes_transferred = bytes_transferred;
                        j.completed_at = Some(get_timestamp());

                        // Save to history
                        let _ = save_backup_to_history(j);
                    }
                    Err(e) => {
                        j.status = BackupStatus::Failed;
                        j.error_message = Some(e);
                        j.completed_at = Some(get_timestamp());
                    }
                }

                // Emit job update
                let _ = window_clone.emit("backup-job-updated", j.clone());
            }
        } else {
            log::error!("Failed to lock backup queue in background task");
        }
    });

    // Return immediately with in-progress status
    let queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;
    queue
        .get(&job_id)
        .cloned()
        .ok_or_else(|| "Backup job not found".to_string())
}

/// Cancel a backup job
#[tauri::command]
pub async fn cancel_backup(job_id: String) -> Result<(), String> {
    let mut queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;

    if let Some(job) = queue.get_mut(&job_id) {
        if job.status == BackupStatus::Pending {
            job.status = BackupStatus::Cancelled;
            job.completed_at = Some(get_timestamp());
            Ok(())
        } else {
            Err("Can only cancel pending backups".to_string())
        }
    } else {
        Err("Backup job not found".to_string())
    }
}

/// Remove a completed/failed/cancelled backup job from queue
#[tauri::command]
pub async fn remove_backup_job(job_id: String) -> Result<(), String> {
    let mut queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;

    if let Some(job) = queue.get(&job_id) {
        if job.status == BackupStatus::InProgress {
            return Err("Cannot remove in-progress backup".to_string());
        }
    }

    queue.remove(&job_id);
    Ok(())
}

/// Get backup history
#[tauri::command]
pub async fn get_backup_history() -> Result<Vec<BackupHistory>, String> {
    let home_dir = get_home_dir()?;
    let history_path = home_dir.join("CreatorOps").join("backup_history.json");

    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    let mut history: Vec<BackupHistory> = serde_json::from_str(&data).unwrap_or_default();

    // Sort by completed_at descending
    history.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

    Ok(history)
}

/// Get backup history for a specific project
#[tauri::command]
pub async fn get_project_backup_history(project_id: String) -> Result<Vec<BackupHistory>, String> {
    let all_history = get_backup_history().await?;
    Ok(all_history
        .into_iter()
        .filter(|h| h.project_id == project_id)
        .collect())
}

// Helper functions

async fn perform_backup(
    window: &tauri::Window,
    job_id: &str,
    job: &BackupJob,
) -> Result<(usize, usize, u64), String> {
    let src_path = Path::new(&job.source_path);
    let dest_base = Path::new(&job.destination_path);

    // Create destination with project folder name
    let project_folder_name = src_path
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy();
    let dest_path = dest_base.join(project_folder_name.as_ref());

    // Collect all files to copy
    let files_to_copy = collect_files_recursive(src_path)?;

    let total_files = files_to_copy.len();
    let start_time = std::time::Instant::now();
    let mut bytes_transferred = 0u64;
    let mut files_copied = 0;
    let mut files_skipped = 0;

    for (index, src_file) in files_to_copy.iter().enumerate() {
        let relative_path = src_file.strip_prefix(src_path).map_err(|e| e.to_string())?;
        let dest_file = dest_path.join(relative_path);

        // Create parent directory
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let file_name = src_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Attempt copy with retries
        match copy_file_with_retry(src_file, &dest_file).await {
            Ok(size) => {
                bytes_transferred += size;
                files_copied += 1;
            }
            Err(e) => {
                log::error!("Failed to copy {} after retries: {}", file_name, e);
                files_skipped += 1;
            }
        }

        // Emit progress
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            bytes_transferred as f64 / elapsed
        } else {
            0.0
        };

        let remaining_bytes = job.total_bytes - bytes_transferred;
        let eta = if speed > 0.0 {
            (remaining_bytes as f64 / speed) as u64
        } else {
            0
        };

        let progress = BackupProgress {
            job_id: job_id.to_string(),
            file_name,
            current_file: index + 1,
            total_files,
            bytes_transferred,
            total_bytes: job.total_bytes,
            speed,
            eta,
        };

        let _ = window.emit("backup-progress", progress);
    }

    Ok((files_copied, files_skipped, bytes_transferred))
}

async fn copy_file_with_retry(src: &Path, dest: &Path) -> Result<u64, String> {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter)
        .take(MAX_RETRY_ATTEMPTS);

    Retry::spawn(retry_strategy, || async {
        // Copy file
        let size = copy_file(src, dest).await?;

        // Verify checksum
        match verify_checksum(src, dest).await {
            Ok(true) => Ok(size),
            Ok(false) => {
                log::warn!("Checksum mismatch for {:?}, retrying...", src);
                let _ = tokio::fs::remove_file(dest).await;
                Err("Checksum verification failed".to_string())
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(dest).await;
                Err(format!("Checksum calculation failed: {}", e))
            }
        }
    })
    .await
}

async fn copy_file(src: &Path, dest: &Path) -> Result<u64, String> {
    let mut src_file = tokio::fs::File::open(src)
        .await
        .map_err(|e| e.to_string())?;

    let mut dest_file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| e.to_string())?;

    let file_size = src_file.metadata().await.map_err(|e| e.to_string())?.len();

    let mut buffer = vec![0u8; CHUNK_SIZE];

    loop {
        let bytes_read = src_file
            .read(&mut buffer)
            .await
            .map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            break;
        }

        dest_file
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(file_size)
}

// Global mutex for backup history file access
lazy_static::lazy_static! {
    static ref HISTORY_MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

fn save_backup_to_history(job: &BackupJob) -> Result<(), String> {
    // Acquire lock to prevent race conditions when multiple backups complete
    let _lock = HISTORY_MUTEX
        .lock()
        .map_err(|e| format!("Failed to lock history mutex: {}", e))?;
    let home_dir = get_home_dir()?;
    let history_dir = home_dir.join("CreatorOps");
    fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;

    let history_path = history_dir.join("backup_history.json");

    let mut history: Vec<BackupHistory> = if history_path.exists() {
        let data = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };

    let entry = BackupHistory {
        id: job.id.clone(),
        project_id: job.project_id.clone(),
        project_name: job.project_name.clone(),
        destination_name: job.destination_name.clone(),
        destination_path: job.destination_path.clone(),
        files_copied: job.files_copied,
        files_skipped: job.files_skipped,
        total_bytes: job.bytes_transferred,
        started_at: job.started_at.clone().unwrap_or_default(),
        completed_at: job.completed_at.clone().unwrap_or_default(),
        status: job.status.clone(),
        error_message: job.error_message.clone(),
    };

    history.push(entry);

    let json_data = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    fs::write(&history_path, json_data).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_status_serialization() {
        assert_eq!(
            serde_json::to_string(&BackupStatus::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&BackupStatus::InProgress).unwrap(),
            r#""inprogress""#
        );
        assert_eq!(
            serde_json::to_string(&BackupStatus::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&BackupStatus::Failed).unwrap(),
            r#""failed""#
        );
        assert_eq!(
            serde_json::to_string(&BackupStatus::Cancelled).unwrap(),
            r#""cancelled""#
        );
    }

    #[test]
    fn test_backup_job_serialization() {
        let job = BackupJob {
            id: "job-123".to_string(),
            project_id: "proj-456".to_string(),
            project_name: "Test Project".to_string(),
            source_path: "/source".to_string(),
            destination_id: "dest-789".to_string(),
            destination_name: "Backup Drive".to_string(),
            destination_path: "/backup".to_string(),
            status: BackupStatus::Pending,
            total_files: 100,
            files_copied: 0,
            files_skipped: 0,
            total_bytes: 1024000,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_string(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("job-123"));
        assert!(json.contains("Test Project"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_backup_progress_serialization() {
        let progress = BackupProgress {
            job_id: "job-123".to_string(),
            file_name: "test.jpg".to_string(),
            current_file: 5,
            total_files: 10,
            bytes_transferred: 512,
            total_bytes: 1024,
            speed: 100.5,
            eta: 5,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("job-123"));
        assert!(json.contains("test.jpg"));
        assert!(json.contains("100.5"));
    }

    #[test]
    fn test_backup_history_serialization() {
        let history = BackupHistory {
            id: "hist-123".to_string(),
            project_id: "proj-456".to_string(),
            project_name: "Test Project".to_string(),
            destination_name: "Backup Drive".to_string(),
            destination_path: "/backup".to_string(),
            files_copied: 100,
            files_skipped: 5,
            total_bytes: 1024000,
            started_at: "2024-01-01T10:00:00Z".to_string(),
            completed_at: "2024-01-01T11:00:00Z".to_string(),
            status: BackupStatus::Completed,
            error_message: None,
        };

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("hist-123"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_backup_job_with_error() {
        let job = BackupJob {
            id: "job-fail".to_string(),
            project_id: "proj-456".to_string(),
            project_name: "Failed Project".to_string(),
            source_path: "/source".to_string(),
            destination_id: "dest-789".to_string(),
            destination_name: "Backup Drive".to_string(),
            destination_path: "/backup".to_string(),
            status: BackupStatus::Failed,
            total_files: 100,
            files_copied: 50,
            files_skipped: 50,
            total_bytes: 1024000,
            bytes_transferred: 512000,
            created_at: "2024-01-01".to_string(),
            started_at: Some("2024-01-01T10:00:00Z".to_string()),
            completed_at: Some("2024-01-01T10:30:00Z".to_string()),
            error_message: Some("Disk full".to_string()),
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("failed"));
        assert!(json.contains("Disk full"));
    }
}
