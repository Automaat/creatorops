use crate::modules::file_utils::{get_home_dir, get_timestamp};
use crate::modules::project::{invalidate_project_cache, Project, ProjectStatus};
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

fn count_files_and_size(path: &str) -> Result<(usize, u64), String> {
    let mut total_files = 0;
    let mut total_bytes = 0u64;

    fn count_recursive(path: &Path, files: &mut usize, bytes: &mut u64) -> Result<(), String> {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let metadata = entry.metadata().map_err(|e| e.to_string())?;

            if metadata.is_dir() {
                count_recursive(&entry.path(), files, bytes)?;
            } else if metadata.is_file() {
                *files += 1;
                *bytes += metadata.len();
            }
        }
        Ok(())
    }

    count_recursive(Path::new(path), &mut total_files, &mut total_bytes)?;

    Ok((total_files, total_bytes))
}

fn update_project_status(project_id: &str, new_status: ProjectStatus) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let projects_path = home_dir.join("CreatorOps").join("Projects");

    // Find project metadata file
    for entry in fs::read_dir(&projects_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata_path = path.join("project.json");

        if metadata_path.exists() {
            if let Ok(json_data) = fs::read_to_string(&metadata_path) {
                if let Ok(mut project) = serde_json::from_str::<Project>(&json_data) {
                    if project.id == project_id {
                        // Update status
                        project.status = new_status;
                        project.updated_at = get_timestamp();

                        // Save back to file
                        let updated_json =
                            serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
                        fs::write(&metadata_path, updated_json).map_err(|e| e.to_string())?;

                        // Invalidate project cache since we updated project status
                        invalidate_project_cache();

                        return Ok(());
                    }
                }
            }
        }
    }

    Err("Project not found".to_string())
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
