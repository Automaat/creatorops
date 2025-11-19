use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
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
#[allow(dead_code)]
pub struct BackupDestination {
    pub id: String,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub created_at: String,
}

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
    queue.insert(id.clone(), job.clone());

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
        let mut queue = BACKUP_QUEUE.lock().unwrap();
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
    });

    // Return immediately with in-progress status
    let queue = BACKUP_QUEUE
        .lock()
        .map_err(|e| format!("Failed to lock queue: {}", e))?;
    Ok(queue.get(&job_id).unwrap().clone())
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
                eprintln!("Checksum mismatch, retrying...");
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

async fn verify_checksum(src: &Path, dest: &Path) -> Result<bool, String> {
    let src_hash = calculate_file_hash(src).await?;
    let dest_hash = calculate_file_hash(dest).await?;
    Ok(src_hash == dest_hash)
}

async fn calculate_file_hash(path: &Path) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| e.to_string())?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer).await.map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files_recursive(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                files.push(entry_path);
            } else if entry_path.is_dir() {
                let mut sub_files = collect_files_recursive(&entry_path)?;
                files.append(&mut sub_files);
            }
        }
    }

    Ok(files)
}

fn count_files_and_size(path: &str) -> Result<(usize, u64), String> {
    let files = collect_files_recursive(Path::new(path))?;
    let mut total_size = 0u64;

    for file in &files {
        if let Ok(metadata) = fs::metadata(file) {
            total_size += metadata.len();
        }
    }

    Ok((files.len(), total_size))
}

fn save_backup_to_history(job: &BackupJob) -> Result<(), String> {
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

fn get_home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .and_then(|h| if h.is_empty() { None } else { Some(h) })
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to get home directory".to_string())
}

fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}
