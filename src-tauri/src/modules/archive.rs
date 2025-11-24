use crate::modules::file_utils::{count_files_and_size, get_timestamp};
use crate::modules::project::ProjectStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveJob {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub source_path: String,
    pub archive_path: String,
    pub compress: bool,
    pub compression_format: Option<String>,
    pub status: ArchiveStatus,
    pub total_files: usize,
    pub files_archived: usize,
    pub total_bytes: u64,
    pub bytes_transferred: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveProgress {
    pub job_id: String,
    pub file_name: String,
    pub current_file: usize,
    pub total_files: usize,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
}

// Global archive queue state
type ArchiveQueue = Arc<Mutex<HashMap<String, ArchiveJob>>>;

lazy_static::lazy_static! {
    static ref ARCHIVE_QUEUE: ArchiveQueue = Arc::new(Mutex::new(HashMap::new()));
}

/// Create an archive job
#[tauri::command]
pub async fn create_archive(
    project_id: String,
    project_name: String,
    source_path: String,
    archive_location: String,
    compress: bool,
    compression_format: Option<String>,
) -> Result<ArchiveJob, String> {
    let id = Uuid::new_v4().to_string();
    let now = get_timestamp();

    // Calculate total size and count files
    let (total_files, total_bytes) = count_files_and_size(&source_path)?;

    // Create archive path
    let archive_path = Path::new(&archive_location).join(&project_name);

    let job = ArchiveJob {
        id: id.clone(),
        project_id,
        project_name,
        source_path,
        archive_path: archive_path.to_string_lossy().to_string(),
        compress,
        compression_format,
        status: ArchiveStatus::Pending,
        total_files,
        files_archived: 0,
        total_bytes,
        bytes_transferred: 0,
        created_at: now,
        started_at: None,
        completed_at: None,
        error_message: None,
    };

    // Add to queue
    {
        let mut queue = ARCHIVE_QUEUE.lock().unwrap();
        queue.insert(id.clone(), job.clone());
    }

    Ok(job)
}

/// Start an archive job
#[tauri::command]
pub async fn start_archive(job_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // Get job from queue
    let job = {
        let mut queue = ARCHIVE_QUEUE.lock().unwrap();
        let job = queue.get_mut(&job_id).ok_or("Job not found")?;

        if job.status != ArchiveStatus::Pending {
            return Err("Job is not in pending status".to_string());
        }

        job.status = ArchiveStatus::InProgress;
        job.started_at = Some(get_timestamp());
        job.clone()
    };

    // Spawn background task
    tokio::spawn(async move {
        let result = process_archive(job.clone(), app_handle.clone()).await;

        // Update job status
        let mut queue = ARCHIVE_QUEUE.lock().unwrap();
        if let Some(job) = queue.get_mut(&job_id) {
            match result {
                Ok(_) => {
                    job.status = ArchiveStatus::Completed;
                    job.completed_at = Some(get_timestamp());
                }
                Err(e) => {
                    job.status = ArchiveStatus::Failed;
                    job.error_message = Some(e);
                    job.completed_at = Some(get_timestamp());
                }
            }
        }
    });

    Ok(())
}

async fn process_archive(mut job: ArchiveJob, app_handle: tauri::AppHandle) -> Result<(), String> {
    let source_path_str = job.source_path.clone();
    let archive_path_str = job.archive_path.clone();
    let source_path = Path::new(&source_path_str);
    let archive_path = Path::new(&archive_path_str);

    if job.compress {
        // Compression not implemented in MVP - just move files
        // TODO: Implement zip/tar compression in future phase
        return Err("Compression not yet implemented".to_string());
    } else {
        // Move entire directory to archive location
        move_directory_recursive(source_path, archive_path, &mut job, &app_handle).await?;
    }

    // Update project status to Archived
    update_project_status(&job.project_id, ProjectStatus::Archived)?;

    Ok(())
}

async fn move_directory_recursive(
    source: &Path,
    dest: &Path,
    job: &mut ArchiveJob,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    // Create destination directory
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    // Copy all files and subdirectories using walkdir to avoid recursion
    use walkdir::WalkDir;

    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        // Get relative path from source
        let relative = path.strip_prefix(source).map_err(|e| e.to_string())?;
        let dest_path = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            // Copy file
            fs::copy(path, &dest_path).map_err(|e| e.to_string())?;

            job.files_archived += 1;
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            job.bytes_transferred += metadata.len();

            // Update queue
            {
                let mut queue = ARCHIVE_QUEUE.lock().unwrap();
                if let Some(q_job) = queue.get_mut(&job.id) {
                    q_job.files_archived = job.files_archived;
                    q_job.bytes_transferred = job.bytes_transferred;
                }
            }

            // Emit progress event
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let progress = ArchiveProgress {
                job_id: job.id.clone(),
                file_name,
                current_file: job.files_archived,
                total_files: job.total_files,
                bytes_transferred: job.bytes_transferred,
                total_bytes: job.total_bytes,
            };

            let _ = app_handle.emit("archive-progress", &progress);
        }
    }

    // After successful copy, remove source directory
    fs::remove_dir_all(source).map_err(|e| e.to_string())?;

    Ok(())
}

fn update_project_status(project_id: &str, new_status: ProjectStatus) -> Result<(), String> {
    use crate::modules::db::with_db;
    use rusqlite::params;

    let now = get_timestamp();
    with_db(|conn| {
        conn.execute(
            "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.to_string(), now, project_id],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Failed to update project status: {}", e))
}

/// Get archive queue
#[tauri::command]
pub async fn get_archive_queue() -> Result<Vec<ArchiveJob>, String> {
    let queue = ARCHIVE_QUEUE.lock().unwrap();
    Ok(queue.values().cloned().collect())
}

/// Remove an archive job from queue
#[tauri::command]
pub async fn remove_archive_job(job_id: String) -> Result<(), String> {
    let mut queue = ARCHIVE_QUEUE.lock().unwrap();
    queue.remove(&job_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ArchiveStatus::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&ArchiveStatus::InProgress).unwrap(),
            r#""inprogress""#
        );
        assert_eq!(
            serde_json::to_string(&ArchiveStatus::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&ArchiveStatus::Failed).unwrap(),
            r#""failed""#
        );
    }

    #[test]
    fn test_archive_job_serialization() {
        let job = ArchiveJob {
            id: "arch-123".to_string(),
            project_id: "proj-456".to_string(),
            project_name: "Archive Test".to_string(),
            source_path: "/source/project".to_string(),
            archive_path: "/archive/project".to_string(),
            compress: false,
            compression_format: None,
            status: ArchiveStatus::Pending,
            total_files: 100,
            files_archived: 0,
            total_bytes: 1024000,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_string(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("arch-123"));
        assert!(json.contains("Archive Test"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_archive_job_with_compression() {
        let job = ArchiveJob {
            id: "arch-456".to_string(),
            project_id: "proj-789".to_string(),
            project_name: "Compressed Archive".to_string(),
            source_path: "/source".to_string(),
            archive_path: "/archive".to_string(),
            compress: true,
            compression_format: Some("zip".to_string()),
            status: ArchiveStatus::Pending,
            total_files: 50,
            files_archived: 0,
            total_bytes: 512000,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_string(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        assert!(job.compress);
        assert_eq!(job.compression_format, Some("zip".to_string()));
    }

    #[test]
    fn test_archive_progress_serialization() {
        let progress = ArchiveProgress {
            job_id: "arch-123".to_string(),
            file_name: "document.pdf".to_string(),
            current_file: 25,
            total_files: 100,
            bytes_transferred: 256000,
            total_bytes: 1024000,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("arch-123"));
        assert!(json.contains("document.pdf"));
        assert!(json.contains("25"));
    }
}
