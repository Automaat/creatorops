use crate::modules::file_utils::{count_files_and_size, get_timestamp};
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

    #[tokio::test]
    async fn test_create_archive() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file1.txt"), "test data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let result = create_archive(
            "proj-123".to_string(),
            "Archive Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");
        assert_eq!(job.project_name, "Archive Test");
        assert_eq!(job.status, ArchiveStatus::Pending);
        assert_eq!(job.total_files, 1);
        assert!(job.total_bytes > 0);
        assert!(!job.compress);

        // Clean up
        let _ = remove_archive_job(job.id).await;
    }

    #[tokio::test]
    async fn test_create_archive_with_compression() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file1.txt"), "test data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let result = create_archive(
            "proj-456".to_string(),
            "Compressed Archive".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            true,
            Some("zip".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert!(job.compress);
        assert_eq!(job.compression_format, Some("zip".to_string()));

        let _ = remove_archive_job(job.id).await;
    }

    #[tokio::test]
    async fn test_get_archive_queue() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive(
            "proj-789".to_string(),
            "Queue Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        let queue = get_archive_queue().await.unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        let _ = remove_archive_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_archive_job() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive(
            "proj-remove".to_string(),
            "Remove Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        let result = remove_archive_job(job.id.clone()).await;
        assert!(result.is_ok());

        // Verify removed
        let queue = get_archive_queue().await.unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }

    #[tokio::test]
    async fn test_archive_job_calculates_total_bytes() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();

        let file1 = source.join("large.dat");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(&vec![0u8; 2048]).unwrap(); // 2KB file

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive(
            "proj-size".to_string(),
            "Size Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.total_bytes, 2048);
        let _ = remove_archive_job(job.id).await;
    }

    #[tokio::test]
    async fn test_archive_counts_multiple_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file1.txt"), "data1").unwrap();
        std::fs::write(source.join("file2.txt"), "data2").unwrap();
        std::fs::write(source.join("file3.txt"), "data3").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive(
            "proj-multi".to_string(),
            "Multi File Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.total_files, 3);
        let _ = remove_archive_job(job.id).await;
    }

    #[test]
    fn test_archive_status_all_variants() {
        let statuses = vec![
            ArchiveStatus::Pending,
            ArchiveStatus::InProgress,
            ArchiveStatus::Completed,
            ArchiveStatus::Failed,
        ];

        for status in statuses {
            let job = ArchiveJob {
                id: "test".to_string(),
                project_id: "proj".to_string(),
                project_name: "Test".to_string(),
                source_path: "/src".to_string(),
                archive_path: "/arch".to_string(),
                compress: false,
                compression_format: None,
                status: status.clone(),
                total_files: 0,
                files_archived: 0,
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
    fn test_archive_status_deserialization() {
        assert!(matches!(
            serde_json::from_str::<ArchiveStatus>(r#""pending""#).unwrap(),
            ArchiveStatus::Pending
        ));
        assert!(matches!(
            serde_json::from_str::<ArchiveStatus>(r#""inprogress""#).unwrap(),
            ArchiveStatus::InProgress
        ));
        assert!(matches!(
            serde_json::from_str::<ArchiveStatus>(r#""completed""#).unwrap(),
            ArchiveStatus::Completed
        ));
        assert!(matches!(
            serde_json::from_str::<ArchiveStatus>(r#""failed""#).unwrap(),
            ArchiveStatus::Failed
        ));
    }

    #[test]
    fn test_archive_job_with_error() {
        let job = ArchiveJob {
            id: "arch-error".to_string(),
            project_id: "proj-123".to_string(),
            project_name: "Failed Archive".to_string(),
            source_path: "/source".to_string(),
            archive_path: "/archive".to_string(),
            compress: false,
            compression_format: None,
            status: ArchiveStatus::Failed,
            total_files: 10,
            files_archived: 5,
            total_bytes: 1024,
            bytes_transferred: 512,
            created_at: "2024-01-01".to_string(),
            started_at: Some("2024-01-01T10:00:00Z".to_string()),
            completed_at: Some("2024-01-01T10:05:00Z".to_string()),
            error_message: Some("Disk full".to_string()),
        };

        assert_eq!(job.status, ArchiveStatus::Failed);
        assert_eq!(job.error_message, Some("Disk full".to_string()));
        assert!(job.files_archived < job.total_files);
    }

    #[test]
    fn test_archive_progress_calculation() {
        let progress = ArchiveProgress {
            job_id: "arch-456".to_string(),
            file_name: "file.txt".to_string(),
            current_file: 50,
            total_files: 100,
            bytes_transferred: 512000,
            total_bytes: 1024000,
        };

        let progress_percent = (progress.current_file as f64 / progress.total_files as f64) * 100.0;
        assert_eq!(progress_percent, 50.0);

        let bytes_percent =
            (progress.bytes_transferred as f64 / progress.total_bytes as f64) * 100.0;
        assert_eq!(bytes_percent, 50.0);
    }

    #[tokio::test]
    async fn test_create_archive_with_subdirectories() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        let subdir = source.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(source.join("file1.txt"), "data1").unwrap();
        std::fs::write(subdir.join("file2.txt"), "data2").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive(
            "proj-subdir".to_string(),
            "Subdir Test".to_string(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.total_files, 2);
        assert!(job.total_bytes > 0);

        let _ = remove_archive_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_nonexistent_archive_job() {
        // remove_archive_job returns Ok even for nonexistent jobs
        let result = remove_archive_job("nonexistent-id".to_string()).await;
        assert!(result.is_ok());
    }
}
