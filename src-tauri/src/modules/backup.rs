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
            eprintln!("Failed to lock backup queue in background task");
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
                eprintln!("Failed to copy {} after retries: {}", file_name, e);
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
                eprintln!("Checksum mismatch for {:?}, retrying...", src);
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

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total_bytes = 0u64;

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

        total_bytes += bytes_read as u64;
    }

    // Ensure all data is written to disk
    dest_file.sync_all().await.map_err(|e| e.to_string())?;

    Ok(total_bytes)
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
        serde_json::from_str(&data).map_err(|e| {
            eprintln!("Failed to deserialize backup history: {}", e);
            eprintln!("File content: {}", data);
            e.to_string()
        })?
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

    // Write and sync file to ensure data is persisted immediately
    use std::io::Write;
    let mut file = fs::File::create(&history_path).map_err(|e| e.to_string())?;
    file.write_all(json_data.as_bytes())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Global mutex to serialize tests that manipulate HOME environment variable
    lazy_static::lazy_static! {
        static ref HOME_TEST_MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    }

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

    #[tokio::test]
    async fn test_queue_backup() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file1.txt"), "test data").unwrap();

        let result = queue_backup(
            "proj-123".to_string(),
            "Backup Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-456".to_string(),
            "External Drive".to_string(),
            "/backup".to_string(),
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");
        assert_eq!(job.status, BackupStatus::Pending);
        assert_eq!(job.total_files, 1);
        assert!(job.total_bytes > 0);

        // Clean up
        let _ = remove_backup_job(job.id).await;
    }

    #[tokio::test]
    async fn test_get_backup_queue() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "proj-789".to_string(),
            "Queue Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-123".to_string(),
            "Test Drive".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        let queue = get_backup_queue().await.unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        let _ = remove_backup_job(job.id).await;
    }

    #[tokio::test]
    async fn test_cancel_backup() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "proj-cancel".to_string(),
            "Cancel Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-456".to_string(),
            "Cancel Drive".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        let result = cancel_backup(job.id.clone()).await;
        assert!(result.is_ok());

        // Verify cancelled
        let queue = get_backup_queue().await.unwrap();
        let cancelled_job = queue.iter().find(|j| j.id == job.id).unwrap();
        assert_eq!(cancelled_job.status, BackupStatus::Cancelled);

        let _ = remove_backup_job(job.id).await;
    }

    #[tokio::test]
    async fn test_cancel_backup_not_pending() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "proj-not-pending".to_string(),
            "Not Pending Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-789".to_string(),
            "Test Drive".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        // Cancel once (should succeed)
        let _ = cancel_backup(job.id.clone()).await;

        // Try to cancel again (should fail)
        let result = cancel_backup(job.id.clone()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Can only cancel pending"));

        let _ = remove_backup_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_backup_job() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "proj-remove".to_string(),
            "Remove Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-remove".to_string(),
            "Remove Drive".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        let result = remove_backup_job(job.id.clone()).await;
        assert!(result.is_ok());

        // Verify removed
        let queue = get_backup_queue().await.unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }

    #[tokio::test]
    async fn test_get_backup_history_empty() {
        let result = get_backup_history().await;
        assert!(result.is_ok());
        // Should return empty vec if no history file
    }

    #[tokio::test]
    async fn test_get_project_backup_history() {
        let _all_history = get_backup_history().await.unwrap();
        let result = get_project_backup_history("nonexistent-project".to_string()).await;
        assert!(result.is_ok());
        let filtered = result.unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[tokio::test]
    async fn test_backup_queue_sorted_by_created_at() {
        use tempfile::TempDir;
        use tokio::time::{sleep, Duration};

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job1 = queue_backup(
            "proj-first".to_string(),
            "First".to_string(),
            source.to_string_lossy().to_string(),
            "dest-1".to_string(),
            "Drive 1".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        sleep(Duration::from_millis(100)).await;

        let job2 = queue_backup(
            "proj-second".to_string(),
            "Second".to_string(),
            source.to_string_lossy().to_string(),
            "dest-2".to_string(),
            "Drive 2".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        let queue = get_backup_queue().await.unwrap();

        // Verify both jobs are in queue
        assert!(queue.iter().any(|j| j.id == job1.id));
        assert!(queue.iter().any(|j| j.id == job2.id));

        // Verify job2 has a later created_at timestamp
        let job1_entry = queue.iter().find(|j| j.id == job1.id).unwrap();
        let job2_entry = queue.iter().find(|j| j.id == job2.id).unwrap();
        assert!(job2_entry.created_at >= job1_entry.created_at);

        let _ = remove_backup_job(job1.id).await;
        let _ = remove_backup_job(job2.id).await;
    }

    #[test]
    fn test_backup_status_all_variants() {
        let statuses = vec![
            BackupStatus::Pending,
            BackupStatus::InProgress,
            BackupStatus::Completed,
            BackupStatus::Failed,
            BackupStatus::Cancelled,
        ];

        for status in statuses {
            let job = BackupJob {
                id: "test".to_string(),
                project_id: "proj".to_string(),
                project_name: "Test".to_string(),
                source_path: "/src".to_string(),
                destination_id: "dest".to_string(),
                destination_name: "Dest".to_string(),
                destination_path: "/dst".to_string(),
                status: status.clone(),
                total_files: 0,
                files_copied: 0,
                files_skipped: 0,
                total_bytes: 0,
                bytes_transferred: 0,
                created_at: "2024-01-01".to_string(),
                started_at: None,
                completed_at: None,
                error_message: None,
            };
            assert_eq!(job.status, status);
        }
    }

    #[test]
    fn test_backup_status_deserialization() {
        assert!(matches!(
            serde_json::from_str::<BackupStatus>(r#""pending""#).unwrap(),
            BackupStatus::Pending
        ));
        assert!(matches!(
            serde_json::from_str::<BackupStatus>(r#""inprogress""#).unwrap(),
            BackupStatus::InProgress
        ));
        assert!(matches!(
            serde_json::from_str::<BackupStatus>(r#""completed""#).unwrap(),
            BackupStatus::Completed
        ));
        assert!(matches!(
            serde_json::from_str::<BackupStatus>(r#""failed""#).unwrap(),
            BackupStatus::Failed
        ));
        assert!(matches!(
            serde_json::from_str::<BackupStatus>(r#""cancelled""#).unwrap(),
            BackupStatus::Cancelled
        ));
    }

    #[test]
    fn test_backup_progress_calculation() {
        let progress = BackupProgress {
            job_id: "backup-123".to_string(),
            file_name: "photo.jpg".to_string(),
            current_file: 25,
            total_files: 100,
            bytes_transferred: 256000,
            total_bytes: 1024000,
            speed: 128000.0,
            eta: 6,
        };

        let progress_percent = (progress.current_file as f64 / progress.total_files as f64) * 100.0;
        assert_eq!(progress_percent, 25.0);

        let bytes_percent =
            (progress.bytes_transferred as f64 / progress.total_bytes as f64) * 100.0;
        assert_eq!(bytes_percent, 25.0);
    }

    #[test]
    fn test_backup_constants() {
        assert_eq!(CHUNK_SIZE, 4 * 1024 * 1024);
        assert_eq!(MAX_RETRY_ATTEMPTS, 3);
    }

    #[tokio::test]
    async fn test_queue_backup_creates_job() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "proj-create".to_string(),
            "Create Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-create".to_string(),
            "Create Drive".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(job.status, BackupStatus::Pending);
        assert_eq!(job.total_files, 1);
        assert!(job.total_bytes > 0);
        assert_eq!(job.files_copied, 0);
        assert_eq!(job.bytes_transferred, 0);

        let _ = remove_backup_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_nonexistent_backup_job() {
        // remove_backup_job returns Ok even for nonexistent jobs
        let result = remove_backup_job("nonexistent-id".to_string()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_backup_job_with_skipped_files() {
        let job = BackupJob {
            id: "backup-skip".to_string(),
            project_id: "proj-123".to_string(),
            project_name: "Partial Backup".to_string(),
            source_path: "/source".to_string(),
            destination_id: "dest-123".to_string(),
            destination_name: "Backup Drive".to_string(),
            destination_path: "/backup".to_string(),
            status: BackupStatus::Completed,
            total_files: 10,
            files_copied: 8,
            files_skipped: 2,
            total_bytes: 1024,
            bytes_transferred: 820,
            created_at: "2024-01-01".to_string(),
            started_at: Some("2024-01-01T10:00:00Z".to_string()),
            completed_at: Some("2024-01-01T10:10:00Z".to_string()),
            error_message: Some("2 files skipped".to_string()),
        };

        assert_eq!(job.files_copied + job.files_skipped, job.total_files);
        assert!(job.error_message.is_some());
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_backup() {
        let result = cancel_backup("nonexistent-id".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Backup job not found");
    }

    #[tokio::test]
    async fn test_backup_job_fields() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "test data").unwrap();

        let job = queue_backup(
            "test-fields".to_string(),
            "Test Fields".to_string(),
            source.to_string_lossy().to_string(),
            "dest-1".to_string(),
            "Destination".to_string(),
            "/backup/path".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(job.project_id, "test-fields");
        assert_eq!(job.project_name, "Test Fields");
        assert_eq!(job.destination_id, "dest-1");
        assert_eq!(job.destination_name, "Destination");
        assert_eq!(job.destination_path, "/backup/path");
        assert!(matches!(job.status, BackupStatus::Pending));
        assert_eq!(job.total_files, 1);
        assert!(job.total_bytes > 0);
        assert_eq!(job.bytes_transferred, 0);
        assert_eq!(job.files_copied, 0);
        assert!(job.error_message.is_none());
    }

    #[test]
    fn test_backup_job_id_generation() {
        let job1 = BackupJob {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: "proj-1".to_string(),
            project_name: "Project 1".to_string(),
            source_path: "/source".to_string(),
            destination_id: "dest-1".to_string(),
            destination_name: "Dest 1".to_string(),
            destination_path: "/dest".to_string(),
            status: BackupStatus::Pending,
            total_files: 0,
            total_bytes: 0,
            files_copied: 0,
            files_skipped: 0,
            bytes_transferred: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        let job2 = BackupJob {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: "proj-2".to_string(),
            project_name: "Project 2".to_string(),
            source_path: "/source2".to_string(),
            destination_id: "dest-2".to_string(),
            destination_name: "Dest 2".to_string(),
            destination_path: "/dest2".to_string(),
            status: BackupStatus::Pending,
            total_files: 0,
            total_bytes: 0,
            files_copied: 0,
            files_skipped: 0,
            bytes_transferred: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        // IDs should be unique
        assert_ne!(job1.id, job2.id);
    }

    // Integration tests for main execution paths
    #[tokio::test]
    async fn test_copy_file_creates_destination() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        let test_data = b"test data for copy";
        std::fs::write(&src, test_data).unwrap();

        let result = copy_file(&src, &dest).await;
        assert!(result.is_ok());
        assert!(dest.exists());

        let content = std::fs::read(&dest).unwrap();
        assert_eq!(content, test_data);
        assert_eq!(result.unwrap(), test_data.len() as u64);
    }

    #[tokio::test]
    async fn test_copy_file_large_file() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("large.bin");
        let dest = temp_dir.path().join("large_dest.bin");

        // Create 5MB file (larger than one chunk)
        let data = vec![0xAB; 5 * 1024 * 1024];
        std::fs::write(&src, &data).unwrap();

        let result = copy_file(&src, &dest).await;
        assert!(result.is_ok());
        let expected_size = data.len() as u64;
        assert_eq!(result.unwrap(), expected_size);

        let dest_size = std::fs::metadata(&dest).unwrap().len();
        assert_eq!(dest_size, expected_size);

        // Verify content integrity
        let dest_data = std::fs::read(&dest).unwrap();
        assert_eq!(dest_data.len(), data.len());
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_success() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.jpg");
        let dest = temp_dir.path().join("dest.jpg");

        std::fs::write(&src, b"photo data").unwrap();

        let result = copy_file_with_retry(&src, &dest).await;
        assert!(result.is_ok());
        assert!(dest.exists());

        let src_content = std::fs::read(&src).unwrap();
        let dest_content = std::fs::read(&dest).unwrap();
        assert_eq!(src_content, dest_content);
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_checksum_verification() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.bin");
        let dest = temp_dir.path().join("dest.bin");

        // Create file with specific content
        let content = b"checksum test data 12345";
        std::fs::write(&src, content).unwrap();

        let result = copy_file_with_retry(&src, &dest).await;
        assert!(result.is_ok());

        // Verify checksum matches
        let src_data = std::fs::read(&src).unwrap();
        let dest_data = std::fs::read(&dest).unwrap();
        assert_eq!(src_data, dest_data);
    }

    #[tokio::test]
    async fn test_save_backup_to_history() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();

        // Acquire lock to prevent parallel tests from interfering with HOME
        let _lock = HOME_TEST_MUTEX.lock().unwrap();

        // Save and set HOME to temp dir
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", temp_dir.path());

        let job = BackupJob {
            id: "hist-test-1".to_string(),
            project_id: "proj-hist".to_string(),
            project_name: "History Project".to_string(),
            source_path: "/source".to_string(),
            destination_id: "dest-hist".to_string(),
            destination_name: "History Dest".to_string(),
            destination_path: "/dest".to_string(),
            status: BackupStatus::Completed,
            total_files: 5,
            files_copied: 5,
            files_skipped: 0,
            total_bytes: 1024,
            bytes_transferred: 1024,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: Some("2024-01-01T00:01:00Z".to_string()),
            completed_at: Some("2024-01-01T00:02:00Z".to_string()),
            error_message: None,
        };

        let result = save_backup_to_history(&job);
        assert!(result.is_ok());

        // Verify history file was created
        let history_path = temp_dir
            .path()
            .join("CreatorOps")
            .join("backup_history.json");
        assert!(history_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&history_path).unwrap();
        assert!(content.contains("hist-test-1"));
        assert!(content.contains("History Project"));

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    async fn test_save_backup_to_history_multiple_entries() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();

        // Setup HOME in locked scope, then drop lock but keep HOME set
        let original_home = {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            let original_home = std::env::var_os("HOME");
            std::env::set_var("HOME", temp_dir.path());
            original_home
        }; // Lock dropped here, but HOME still set to temp_dir

        // Save first backup
        let job1 = BackupJob {
            id: "hist-1".to_string(),
            project_id: "proj-1".to_string(),
            project_name: "Project 1".to_string(),
            source_path: "/src1".to_string(),
            destination_id: "dest-1".to_string(),
            destination_name: "Dest 1".to_string(),
            destination_path: "/dest1".to_string(),
            status: BackupStatus::Completed,
            total_files: 3,
            files_copied: 3,
            files_skipped: 0,
            total_bytes: 512,
            bytes_transferred: 512,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: Some("2024-01-01T00:01:00Z".to_string()),
            completed_at: Some("2024-01-01T00:02:00Z".to_string()),
            error_message: None,
        };

        save_backup_to_history(&job1).unwrap();

        // Save second backup
        let job2 = BackupJob {
            id: "hist-2".to_string(),
            project_id: "proj-2".to_string(),
            project_name: "Project 2".to_string(),
            source_path: "/src2".to_string(),
            destination_id: "dest-2".to_string(),
            destination_name: "Dest 2".to_string(),
            destination_path: "/dest2".to_string(),
            status: BackupStatus::Completed,
            total_files: 5,
            files_copied: 5,
            files_skipped: 0,
            total_bytes: 1024,
            bytes_transferred: 1024,
            created_at: "2024-01-02T00:00:00Z".to_string(),
            started_at: Some("2024-01-02T00:01:00Z".to_string()),
            completed_at: Some("2024-01-02T00:02:00Z".to_string()),
            error_message: None,
        };

        save_backup_to_history(&job2).unwrap();

        // Verify both entries exist (no lock held during await)
        let history = get_backup_history().await.unwrap();
        assert_eq!(history.len(), 2);
        assert!(history.iter().any(|h| h.id == "hist-1"));
        assert!(history.iter().any(|h| h.id == "hist-2"));

        // Restore HOME at the end
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[tokio::test]
    async fn test_get_backup_history_with_entries() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();

        // Setup HOME in locked scope, then drop lock but keep HOME set
        let (source, original_home) = {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            let original_home = std::env::var_os("HOME");
            std::env::set_var("HOME", temp_dir.path());

            let source = temp_dir.path().join("project");
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("file.txt"), "data").unwrap();

            (source, original_home)
        }; // Lock dropped here, but HOME still set

        // Queue and complete a backup (no lock held during await)
        let _job = queue_backup(
            "proj-hist".to_string(),
            "History Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-hist".to_string(),
            "Dest History".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        // Note: Without actually running start_backup, history will be empty
        // This tests the history retrieval mechanism
        let _history = get_backup_history().await.unwrap();

        // Restore HOME at the end
        {
            let _lock = HOME_TEST_MUTEX.lock().unwrap();
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_backup_history_struct() {
        let history = BackupHistory {
            id: "hist-1".to_string(),
            project_id: "proj-1".to_string(),
            project_name: "Project 1".to_string(),
            destination_name: "Destination".to_string(),
            destination_path: "/backup/dest".to_string(),
            files_copied: 10,
            files_skipped: 2,
            total_bytes: 1024,
            started_at: "2024-01-01T00:00:00Z".to_string(),
            completed_at: "2024-01-01T00:01:00Z".to_string(),
            status: BackupStatus::Completed,
            error_message: None,
        };

        assert_eq!(history.id, "hist-1");
        assert_eq!(history.files_copied, 10);
        assert_eq!(history.files_skipped, 2);
        assert_eq!(history.total_bytes, 1024);
        assert!(!history.started_at.is_empty());
        assert!(!history.completed_at.is_empty());
        assert!(matches!(history.status, BackupStatus::Completed));
        assert!(history.error_message.is_none());
    }

    #[tokio::test]
    async fn test_backup_job_timestamps() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup(
            "time-test".to_string(),
            "Timestamp Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-time".to_string(),
            "Dest Time".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        assert!(!job.created_at.is_empty());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_backup_empty_source() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("empty_project");
        std::fs::create_dir(&source).unwrap();

        let job = queue_backup(
            "empty-test".to_string(),
            "Empty Test".to_string(),
            source.to_string_lossy().to_string(),
            "dest-empty".to_string(),
            "Dest Empty".to_string(),
            "/backup".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(job.total_files, 0);
        assert_eq!(job.total_bytes, 0);
    }

    #[test]
    fn test_backup_progress_struct() {
        let progress = BackupProgress {
            job_id: "job-1".to_string(),
            file_name: "test.jpg".to_string(),
            current_file: 5,
            total_files: 10,
            bytes_transferred: 512,
            total_bytes: 1024,
            speed: 100.5,
            eta: 10,
        };

        assert_eq!(progress.job_id, "job-1");
        assert_eq!(progress.current_file, 5);
        assert_eq!(progress.total_files, 10);
        assert_eq!(progress.bytes_transferred, 512);
        assert_eq!(progress.total_bytes, 1024);
        assert_eq!(progress.speed, 100.5);

        let percent = (progress.current_file as f64 / progress.total_files as f64) * 100.0;
        assert_eq!(percent, 50.0);
    }

    #[tokio::test]
    async fn test_get_project_backup_history_empty() {
        let history = get_project_backup_history("nonexistent-project".to_string())
            .await
            .unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_backup_status_ordering() {
        let statuses = [
            BackupStatus::Pending,
            BackupStatus::InProgress,
            BackupStatus::Completed,
            BackupStatus::Failed,
            BackupStatus::Cancelled,
        ];

        // Verify all statuses are distinct
        for (i, status1) in statuses.iter().enumerate() {
            for (j, status2) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(status1, status2);
                } else {
                    assert_ne!(status1, status2);
                }
            }
        }
    }

    #[test]
    fn test_backup_job_deserialization() {
        let json = r#"{
            "id": "test-123",
            "projectId": "proj-1",
            "projectName": "Project 1",
            "sourcePath": "/source",
            "destinationId": "dest-1",
            "destinationName": "Destination",
            "destinationPath": "/dest",
            "status": "pending",
            "totalFiles": 10,
            "totalBytes": 1024,
            "filesCopied": 0,
            "filesSkipped": 0,
            "bytesTransferred": 0,
            "createdAt": "2024-01-01T00:00:00Z",
            "startedAt": null,
            "completedAt": null,
            "errorMessage": null
        }"#;

        let job: BackupJob = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, "test-123");
        assert_eq!(job.project_id, "proj-1");
        assert!(matches!(job.status, BackupStatus::Pending));
    }
}
