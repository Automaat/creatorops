#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
use crate::modules::file_utils::{get_home_dir, get_timestamp};
use crate::modules::project::Project;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryJob {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub selected_files: Vec<String>,
    pub delivery_path: String,
    pub naming_template: Option<String>,
    pub status: DeliveryStatus,
    pub total_files: usize,
    pub files_copied: usize,
    pub total_bytes: u64,
    pub bytes_transferred: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub manifest_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryProgress {
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
pub struct ProjectFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: String,
    pub file_type: String,
    pub relative_path: String,
}

// Global delivery queue state
type DeliveryQueue = Arc<Mutex<HashMap<String, DeliveryJob>>>;

lazy_static::lazy_static! {
    static ref DELIVERY_QUEUE: DeliveryQueue = Arc::new(Mutex::new(HashMap::new()));
}

/// List all files in a project directory
#[tauri::command]
pub async fn list_project_files(project_id: String) -> Result<Vec<ProjectFile>, String> {
    // Load project to get folder path
    let home_dir = get_home_dir()?;
    let projects_path = home_dir.join("CreatorOps").join("Projects");

    // Find project by scanning directories
    let mut project_path: Option<PathBuf> = None;
    if let Ok(entries) = fs::read_dir(&projects_path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let metadata_path = path.join("project.json");
            if metadata_path.exists() {
                if let Ok(json_data) = fs::read_to_string(&metadata_path) {
                    if let Ok(project) = serde_json::from_str::<Project>(&json_data) {
                        if project.id == project_id {
                            project_path = Some(path);
                            break;
                        }
                    }
                }
            }
        }
    }

    let project_path = project_path.ok_or("Project not found")?;

    // Recursively list all files
    let mut files = Vec::new();
    collect_project_files(&project_path, &project_path, &mut files)?;

    Ok(files)
}

fn collect_project_files(
    base_path: &Path,
    current_path: &Path,
    files: &mut Vec<ProjectFile>,
) -> Result<(), String> {
    for entry in fs::read_dir(current_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        if metadata.is_dir() {
            // Skip project.json metadata file directory
            if path.file_name().and_then(|n| n.to_str()) == Some("project.json") {
                continue;
            }
            collect_project_files(base_path, &path, files)?;
        } else if metadata.is_file() {
            // Skip project.json metadata file
            if path.file_name().and_then(|n| n.to_str()) == Some("project.json") {
                continue;
            }

            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();

            let file_type = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_uppercase();

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or_else(|| "0".to_owned(), |d| d.as_secs().to_string());

            files.push(ProjectFile {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown").to_owned(),
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                modified,
                file_type,
                relative_path,
            });
        }
    }

    Ok(())
}

/// Create a delivery job
#[tauri::command]
pub async fn create_delivery(
    project_id: String,
    project_name: String,
    selected_files: Vec<String>,
    delivery_path: String,
    naming_template: Option<String>,
) -> Result<DeliveryJob, String> {
    let id = Uuid::new_v4().to_string();
    let now = get_timestamp();

    // Calculate total size
    let mut total_bytes = 0_u64;
    for file_path in &selected_files {
        if let Ok(metadata) = fs::metadata(file_path) {
            total_bytes += metadata.len();
        }
    }

    let job = DeliveryJob {
        id: id.clone(),
        project_id,
        project_name,
        selected_files: selected_files.clone(),
        delivery_path,
        naming_template,
        status: DeliveryStatus::Pending,
        total_files: selected_files.len(),
        files_copied: 0,
        total_bytes,
        bytes_transferred: 0,
        created_at: now,
        started_at: None,
        completed_at: None,
        error_message: None,
        manifest_path: None,
    };

    // Add to queue
    {
        let mut queue = DELIVERY_QUEUE.lock().map_err(|e| e.to_string())?;
        queue.insert(id, job.clone());
    }

    Ok(job)
}

/// Start a delivery job
#[tauri::command]
pub async fn start_delivery(job_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // Get job from queue
    let job = {
        let mut queue = DELIVERY_QUEUE.lock().map_err(|e| e.to_string())?;
        let job = queue.get_mut(&job_id).ok_or("Job not found")?;

        if job.status != DeliveryStatus::Pending {
            return Err("Job is not in pending status".to_owned());
        }

        job.status = DeliveryStatus::InProgress;
        job.started_at = Some(get_timestamp());
        let job_clone = job.clone();
        drop(queue);
        job_clone
    };

    // Spawn background task
    tokio::spawn(async move {
        let result = process_delivery(job.clone(), app_handle.clone()).await;

        // Update job status
        if let Ok(mut queue) = DELIVERY_QUEUE.lock() {
            if let Some(job) = queue.get_mut(&job_id) {
                match result {
                    Ok(()) => {
                        job.status = DeliveryStatus::Completed;
                        job.completed_at = Some(get_timestamp());
                    }
                    Err(e) => {
                        job.status = DeliveryStatus::Failed;
                        job.error_message = Some(e);
                        job.completed_at = Some(get_timestamp());
                    }
                }
            }
        }
    });

    Ok(())
}

async fn process_delivery(
    mut job: DeliveryJob,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Create delivery directory
    let delivery_path = Path::new(&job.delivery_path);
    fs::create_dir_all(delivery_path).map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let mut manifest_entries = Vec::new();

    for (index, source_file) in job.selected_files.iter().enumerate() {
        let source_path = Path::new(source_file);
        let file_name = source_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_string_lossy()
            .to_string();

        // Apply naming template if provided
        let dest_name = job.naming_template.as_ref().map_or_else(
            || file_name.clone(),
            |template| apply_naming_template(template, &file_name, index),
        );

        let dest_path = delivery_path.join(&dest_name);

        let file_size = fs::metadata(source_path).map_err(|e| e.to_string())?.len();

        copy_file_with_progress(
            source_path,
            &dest_path,
            &job.id,
            index + 1,
            job.total_files,
            &mut job.bytes_transferred,
            job.total_bytes,
            start_time,
            &app_handle,
        )
        .await?;

        job.files_copied += 1;

        // Add to manifest
        manifest_entries.push(format!("{file_name} -> {dest_name} ({file_size})"));

        // Update queue
        {
            if let Ok(mut queue) = DELIVERY_QUEUE.lock() {
                if let Some(q_job) = queue.get_mut(&job.id) {
                    q_job.files_copied = job.files_copied;
                    q_job.bytes_transferred = job.bytes_transferred;
                }
            }
        }
    }

    // Generate manifest file
    let manifest_path = delivery_path.join("delivery_manifest.txt");
    let manifest_content = format!(
        "Delivery Manifest\n\
         Project: {}\n\
         Date: {}\n\
         Total Files: {}\n\
         Total Size: {} bytes\n\
         \n\
         Files:\n{}",
        job.project_name,
        get_timestamp(),
        job.total_files,
        job.total_bytes,
        manifest_entries.join("\n")
    );

    fs::write(&manifest_path, manifest_content).map_err(|e| e.to_string())?;

    // Update job with manifest path
    {
        if let Ok(mut queue) = DELIVERY_QUEUE.lock() {
            if let Some(q_job) = queue.get_mut(&job.id) {
                q_job.manifest_path = Some(manifest_path.to_string_lossy().to_string());
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn copy_file_with_progress(
    source: &Path,
    dest: &Path,
    job_id: &str,
    current_file: usize,
    total_files: usize,
    bytes_transferred: &mut u64,
    total_bytes: u64,
    start_time: std::time::Instant,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let mut source_file = tokio::fs::File::open(source)
        .await
        .map_err(|e| e.to_string())?;

    let mut dest_file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| e.to_string())?;

    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let file_name = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    loop {
        let bytes_read = source_file
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

        *bytes_transferred += bytes_read as u64;

        // Calculate speed and ETA
        let elapsed = start_time.elapsed().as_secs_f64();
        // Safe cast: bytes_transferred used for progress display, precision loss acceptable
        #[allow(clippy::cast_precision_loss)]
        let speed = if elapsed > 0.0 {
            *bytes_transferred as f64 / elapsed
        } else {
            0.0
        };

        let remaining_bytes = total_bytes.saturating_sub(*bytes_transferred);
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let eta = if speed > 0.0 {
            // Safe: ETA calculation for display, truncation acceptable
            (remaining_bytes as f64 / speed) as u64
        } else {
            0
        };

        // Emit progress event
        let progress = DeliveryProgress {
            job_id: job_id.to_owned(),
            file_name: file_name.clone(),
            current_file,
            total_files,
            bytes_transferred: *bytes_transferred,
            total_bytes,
            speed,
            eta,
        };

        let _ = app_handle.emit("delivery-progress", &progress);
    }

    dest_file.flush().await.map_err(|e| e.to_string())?;

    Ok(())
}

fn apply_naming_template(template: &str, original_name: &str, index: usize) -> String {
    // Simple template replacement
    // Supports: {index}, {name}, {ext}
    let path = Path::new(original_name);
    let name_without_ext = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let index_str = format!("{:03}", index + 1);

    #[allow(clippy::literal_string_with_formatting_args)] // Template placeholders, not format args
    {
        template
            .replace("{index}", &index_str)
            .replace("{name}", &name_without_ext)
            .replace("{ext}", &ext)
    }
}

/// Get delivery queue
#[tauri::command]
pub async fn get_delivery_queue() -> Result<Vec<DeliveryJob>, String> {
    let queue = DELIVERY_QUEUE.lock().map_err(|e| e.to_string())?;
    Ok(queue.values().cloned().collect())
}

/// Remove a delivery job from queue
#[tauri::command]
pub async fn remove_delivery_job(job_id: String) -> Result<(), String> {
    {
        let mut queue = DELIVERY_QUEUE.lock().map_err(|e| e.to_string())?;
        queue.remove(&job_id);
    }
    Ok(())
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_status_serialization() {
        assert_eq!(
            serde_json::to_string(&DeliveryStatus::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&DeliveryStatus::InProgress).unwrap(),
            r#""inprogress""#
        );
        assert_eq!(
            serde_json::to_string(&DeliveryStatus::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&DeliveryStatus::Failed).unwrap(),
            r#""failed""#
        );
    }

    #[test]
    fn test_delivery_job_serialization() {
        let job = DeliveryJob {
            id: "del-123".to_owned(),
            project_id: "proj-456".to_owned(),
            project_name: "Delivery Test".to_owned(),
            selected_files: vec!["/file1.jpg".to_owned(), "/file2.jpg".to_owned()],
            delivery_path: "/delivery".to_owned(),
            naming_template: Some("{index}_{name}.{ext}".to_owned()),
            status: DeliveryStatus::Pending,
            total_files: 2,
            files_copied: 0,
            total_bytes: 2048,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_owned(),
            started_at: None,
            completed_at: None,
            error_message: None,
            manifest_path: None,
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("del-123"));
        assert!(json.contains("Delivery Test"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_project_file_serialization() {
        let file = ProjectFile {
            name: "photo.jpg".to_owned(),
            path: "/project/photo.jpg".to_owned(),
            size: 1024,
            modified: "1640000000".to_owned(),
            file_type: "JPG".to_owned(),
            relative_path: "RAW/Photos/photo.jpg".to_owned(),
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("photo.jpg"));
        assert!(json.contains("1024"));
        assert!(json.contains("JPG"));
    }

    #[test]
    fn test_delivery_progress_serialization() {
        let progress = DeliveryProgress {
            job_id: "del-123".to_owned(),
            file_name: "photo.jpg".to_owned(),
            current_file: 1,
            total_files: 5,
            bytes_transferred: 512,
            total_bytes: 2560,
            speed: 100.5,
            eta: 20,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("del-123"));
        assert!(json.contains("photo.jpg"));
    }

    #[test]
    fn test_apply_naming_template() {
        assert_eq!(
            apply_naming_template("{index}_{name}.{ext}", "photo.jpg", 0),
            "001_photo.jpg"
        );
        assert_eq!(
            apply_naming_template("{index}_{name}.{ext}", "photo.jpg", 9),
            "010_photo.jpg"
        );
        assert_eq!(
            apply_naming_template("{name}_final.{ext}", "document.pdf", 0),
            "document_final.pdf"
        );
        assert_eq!(
            apply_naming_template("image_{index}.{ext}", "test.png", 99),
            "image_100.png"
        );
    }

    #[test]
    fn test_apply_naming_template_no_extension() {
        assert_eq!(
            apply_naming_template("{index}_{name}", "file", 5),
            "006_file"
        );
    }

    #[tokio::test]
    async fn test_create_delivery() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.jpg");
        let file2 = temp_dir.path().join("test2.jpg");

        // Create temp files
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(b"test data 1").unwrap();
        let mut f2 = std::fs::File::create(&file2).unwrap();
        f2.write_all(b"test data 2").unwrap();

        let result = create_delivery(
            "proj-123".to_owned(),
            "Test Project".to_owned(),
            vec![
                file1.to_string_lossy().to_string(),
                file2.to_string_lossy().to_string(),
            ],
            "/delivery".to_owned(),
            Some("{index}_{name}.{ext}".to_owned()),
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");
        assert_eq!(job.project_name, "Test Project");
        assert_eq!(job.status, DeliveryStatus::Pending);
        assert_eq!(job.total_files, 2);
        assert_eq!(job.files_copied, 0);
        assert!(job.total_bytes > 0);
    }

    #[tokio::test]
    async fn test_get_delivery_queue() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test.jpg");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(b"test").unwrap();

        // Create a delivery job
        let job = create_delivery(
            "proj-456".to_owned(),
            "Queue Test".to_owned(),
            vec![file1.to_string_lossy().to_string()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        // Get queue
        let queue = get_delivery_queue().await.unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        // Clean up
        let _ = remove_delivery_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_delivery_job() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test.jpg");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(b"test").unwrap();

        // Create and remove
        let job = create_delivery(
            "proj-789".to_owned(),
            "Remove Test".to_owned(),
            vec![file1.to_string_lossy().to_string()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        let result = remove_delivery_job(job.id.clone()).await;
        assert!(result.is_ok());

        // Verify removed
        let queue = get_delivery_queue().await.unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }

    #[tokio::test]
    async fn test_delivery_job_calculates_total_bytes() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("large.jpg");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(&vec![0_u8; 1024]).unwrap(); // 1KB file

        let job = create_delivery(
            "proj-size".to_owned(),
            "Size Test".to_owned(),
            vec![file1.to_string_lossy().to_string()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.total_bytes, 1024);
        let _ = remove_delivery_job(job.id).await;
    }

    #[test]
    fn test_collect_project_files_helper() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test structure
        std::fs::create_dir_all(base.join("subdir")).unwrap();
        std::fs::write(base.join("file1.txt"), "test1").unwrap();
        std::fs::write(base.join("subdir/file2.txt"), "test2").unwrap();

        let mut files = Vec::new();
        let result = collect_project_files(base, base, &mut files);

        assert!(result.is_ok());
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.name == "file1.txt"));
        assert!(files.iter().any(|f| f.name == "file2.txt"));
    }

    #[test]
    fn test_collect_project_files_skips_project_json() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        std::fs::write(base.join("project.json"), "{}").unwrap();
        std::fs::write(base.join("normal.txt"), "data").unwrap();

        let mut files = Vec::new();
        let result = collect_project_files(base, base, &mut files);

        assert!(result.is_ok());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "normal.txt");
    }

    #[test]
    fn test_delivery_status_all_variants() {
        let statuses = vec![
            DeliveryStatus::Pending,
            DeliveryStatus::InProgress,
            DeliveryStatus::Completed,
            DeliveryStatus::Failed,
        ];

        for status in statuses {
            let job = DeliveryJob {
                id: "test".to_owned(),
                project_id: "proj".to_owned(),
                project_name: "Test".to_owned(),
                selected_files: vec![],
                delivery_path: "/delivery".to_owned(),
                naming_template: None,
                status: status.clone(),
                total_files: 0,
                files_copied: 0,
                total_bytes: 0,
                bytes_transferred: 0,
                created_at: "2024-01-01".to_owned(),
                started_at: None,
                completed_at: None,
                error_message: None,
                manifest_path: None,
            };
            assert_eq!(job.status, status);
        }
    }

    #[test]
    fn test_delivery_status_deserialization() {
        assert!(matches!(
            serde_json::from_str::<DeliveryStatus>(r#""pending""#).unwrap(),
            DeliveryStatus::Pending
        ));
        assert!(matches!(
            serde_json::from_str::<DeliveryStatus>(r#""inprogress""#).unwrap(),
            DeliveryStatus::InProgress
        ));
        assert!(matches!(
            serde_json::from_str::<DeliveryStatus>(r#""completed""#).unwrap(),
            DeliveryStatus::Completed
        ));
        assert!(matches!(
            serde_json::from_str::<DeliveryStatus>(r#""failed""#).unwrap(),
            DeliveryStatus::Failed
        ));
    }

    #[test]
    fn test_delivery_progress_calculation() {
        let progress = DeliveryProgress {
            job_id: "delivery-123".to_owned(),
            file_name: "image.jpg".to_owned(),
            current_file: 30,
            total_files: 100,
            bytes_transferred: 307_200,
            total_bytes: 1_024_000,
            speed: 102_400.0,
            eta: 7,
        };

        // Safe cast: small test values well within f64 mantissa precision
        #[allow(clippy::cast_precision_loss)]
        let progress_percent = (progress.current_file as f64 / progress.total_files as f64) * 100.0;
        assert!((progress_percent - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_delivery_constants() {
        assert_eq!(CHUNK_SIZE, 4 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_create_delivery_job() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project = temp_dir.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let file = project.join("photo.jpg");
        std::fs::write(&file, "photo data").unwrap();

        let delivery_path = temp_dir.path().join("delivery");
        std::fs::create_dir(&delivery_path).unwrap();

        let job = create_delivery(
            "proj-del".to_owned(),
            "Delivery Test".to_owned(),
            vec![file.to_string_lossy().to_string()],
            delivery_path.to_string_lossy().to_string(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.status, DeliveryStatus::Pending);
        assert_eq!(job.total_files, 1);
        assert!(job.total_bytes > 0);

        let _ = remove_delivery_job(job.id).await;
    }

    #[tokio::test]
    async fn test_remove_nonexistent_delivery_job() {
        // remove_delivery_job returns Ok even for nonexistent jobs
        let result = remove_delivery_job("nonexistent-id".to_owned()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_project_file_complete_struct() {
        let file = ProjectFile {
            name: "photo.jpg".to_owned(),
            path: "/path/to/photo.jpg".to_owned(),
            size: 2_048_000,
            modified: "2024-01-01T10:00:00Z".to_owned(),
            file_type: "image/jpeg".to_owned(),
            relative_path: "Selects/photo.jpg".to_owned(),
        };

        assert_eq!(file.name, "photo.jpg");
        assert_eq!(file.size, 2_048_000);
        assert_eq!(file.file_type, "image/jpeg");
    }

    #[test]
    fn test_apply_naming_template_with_index() {
        let template = "{name}_{index}";
        // index is 1-based and zero-padded to 3 digits: index 5 becomes "006"
        let result = apply_naming_template(template, "photo.jpg", 5);
        assert_eq!(result, "photo_006");
    }

    #[test]
    fn test_apply_naming_template_with_ext() {
        let template = "{name}.{ext}";
        let result = apply_naming_template(template, "photo.jpg", 1);
        assert_eq!(result, "photo.jpg");
    }

    #[tokio::test]
    async fn test_delivery_job_with_naming_template() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project = temp_dir.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let file = project.join("original.jpg");
        std::fs::write(&file, "data").unwrap();

        let delivery_path = temp_dir.path().join("delivery");
        std::fs::create_dir(&delivery_path).unwrap();

        let job = create_delivery(
            "proj-template".to_owned(),
            "Template Test".to_owned(),
            vec![file.to_string_lossy().to_string()],
            delivery_path.to_string_lossy().to_string(),
            Some("{name}_{index}".to_owned()),
        )
        .await
        .unwrap();

        assert!(job.naming_template.is_some());
        assert_eq!(job.naming_template.unwrap(), "{name}_{index}");

        let _ = remove_delivery_job(job.id).await;
    }

    #[test]
    fn test_delivery_job_deserialization() {
        let json = r#"{
            "id": "del-123",
            "projectId": "proj-456",
            "projectName": "Wedding",
            "selectedFiles": ["/file1.jpg", "/file2.jpg"],
            "deliveryPath": "/delivery",
            "namingTemplate": null,
            "status": "pending",
            "totalFiles": 2,
            "filesCopied": 0,
            "totalBytes": 2048,
            "bytesTransferred": 0,
            "createdAt": "2024-01-01",
            "startedAt": null,
            "completedAt": null,
            "errorMessage": null,
            "manifestPath": null
        }"#;

        let job: DeliveryJob = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, "del-123");
        assert_eq!(job.project_id, "proj-456");
        assert_eq!(job.total_files, 2);
    }

    #[test]
    fn test_collect_project_files_with_subdirs() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        std::fs::create_dir_all(base.join("dir1/dir2")).unwrap();
        std::fs::write(base.join("file1.txt"), "1").unwrap();
        std::fs::write(base.join("dir1/file2.txt"), "2").unwrap();
        std::fs::write(base.join("dir1/dir2/file3.txt"), "3").unwrap();

        let mut files = Vec::new();
        let result = collect_project_files(base, base, &mut files);

        assert!(result.is_ok());
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.name == "file1.txt"));
        assert!(files.iter().any(|f| f.name == "file2.txt"));
        assert!(files.iter().any(|f| f.name == "file3.txt"));
    }

    // Integration tests for main execution paths
    #[tokio::test]
    async fn test_apply_naming_template_in_workflow() {
        let template = "{name}_{index}.{ext}";

        let result1 = apply_naming_template(template, "photo.jpg", 0);
        assert_eq!(result1, "photo_001.jpg");

        let result2 = apply_naming_template(template, "video.mp4", 9);
        assert_eq!(result2, "video_010.mp4");

        let result3 = apply_naming_template(template, "document.pdf", 99);
        assert_eq!(result3, "document_100.pdf");
    }

    #[tokio::test]
    async fn test_create_delivery_calculates_sizes_correctly() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("large1.bin");
        let file2 = temp_dir.path().join("large2.bin");
        let file3 = temp_dir.path().join("small.txt");

        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(&vec![0_u8; 1024]).unwrap();

        let mut f2 = std::fs::File::create(&file2).unwrap();
        f2.write_all(&vec![0_u8; 2048]).unwrap();

        let mut f3 = std::fs::File::create(&file3).unwrap();
        f3.write_all(b"test").unwrap();

        let job = create_delivery(
            "proj-size".to_owned(),
            "Size Test".to_owned(),
            vec![
                file1.to_string_lossy().to_string(),
                file2.to_string_lossy().to_string(),
                file3.to_string_lossy().to_string(),
            ],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.total_files, 3);
        assert_eq!(job.total_bytes, 1024 + 2048 + 4);
        assert_eq!(job.bytes_transferred, 0);
        assert_eq!(job.files_copied, 0);

        let _ = remove_delivery_job(job.id).await;
    }

    #[tokio::test]
    async fn test_collect_project_files_calculates_metadata() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        std::fs::create_dir_all(base.join("photos")).unwrap();
        std::fs::write(base.join("photo1.jpg"), b"photo1").unwrap();
        std::fs::write(base.join("photos/photo2.jpg"), b"photo2data").unwrap();
        std::fs::write(base.join("document.txt"), b"text").unwrap();

        let mut files = Vec::new();
        collect_project_files(base, base, &mut files).unwrap();

        assert_eq!(files.len(), 3);

        let photo1 = files.iter().find(|f| f.name == "photo1.jpg").unwrap();
        assert_eq!(photo1.size, 6);
        assert_eq!(photo1.file_type, "JPG");
        assert_eq!(photo1.relative_path, "photo1.jpg");

        let photo2 = files.iter().find(|f| f.name == "photo2.jpg").unwrap();
        assert_eq!(photo2.size, 10);
        assert_eq!(photo2.file_type, "JPG");
        assert!(photo2.relative_path.contains("photos"));

        let doc = files.iter().find(|f| f.name == "document.txt").unwrap();
        assert_eq!(doc.size, 4);
        assert_eq!(doc.file_type, "TXT");
    }

    #[tokio::test]
    async fn test_delivery_status_transitions() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test.jpg");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        f1.write_all(b"data").unwrap();

        let job = create_delivery(
            "status-test".to_owned(),
            "Status Test".to_owned(),
            vec![file1.to_string_lossy().to_string()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(job.status, DeliveryStatus::Pending);
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());

        let _ = remove_delivery_job(job.id).await;
    }

    #[tokio::test]
    async fn test_delivery_queue_operations() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("f1.jpg");
        let file2 = temp_dir.path().join("f2.jpg");

        std::fs::File::create(&file1)
            .unwrap()
            .write_all(b"1")
            .unwrap();
        std::fs::File::create(&file2)
            .unwrap()
            .write_all(b"2")
            .unwrap();

        // Create two jobs
        let job1 = create_delivery(
            "q1".to_owned(),
            "Queue1".to_owned(),
            vec![file1.to_string_lossy().to_string()],
            "/del1".to_owned(),
            None,
        )
        .await
        .unwrap();

        let job2 = create_delivery(
            "q2".to_owned(),
            "Queue2".to_owned(),
            vec![file2.to_string_lossy().to_string()],
            "/del2".to_owned(),
            None,
        )
        .await
        .unwrap();

        let job1_id = job1.id.clone();
        let job2_id = job2.id.clone();

        // Get queue
        let queue = get_delivery_queue().await.unwrap();
        assert!(queue.len() >= 2);
        assert!(queue.iter().any(|j| j.id == job1_id));
        assert!(queue.iter().any(|j| j.id == job2_id));

        // Remove jobs
        let _ = remove_delivery_job(job1.id).await;
        let _ = remove_delivery_job(job2.id).await;

        let queue_after = get_delivery_queue().await.unwrap();
        assert!(!queue_after.iter().any(|j| j.id == job1_id));
        assert!(!queue_after.iter().any(|j| j.id == job2_id));
    }

    #[test]
    fn test_apply_naming_template_edge_cases() {
        // Empty template
        let result = apply_naming_template("", "file.jpg", 0);
        assert_eq!(result, "");

        // No placeholders
        let result = apply_naming_template("static_name.jpg", "original.png", 5);
        assert_eq!(result, "static_name.jpg");

        // Only index
        let result = apply_naming_template("{index}", "file.jpg", 42);
        assert_eq!(result, "043");

        // File without extension
        let result = apply_naming_template("{name}_{index}", "README", 10);
        assert_eq!(result, "README_011");
    }

    #[test]
    fn test_project_file_with_no_extension() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        std::fs::write(base.join("Makefile"), "build").unwrap();
        std::fs::write(base.join("README"), "readme").unwrap();

        let mut files = Vec::new();
        collect_project_files(base, base, &mut files).unwrap();

        assert_eq!(files.len(), 2);

        let makefile = files.iter().find(|f| f.name == "Makefile").unwrap();
        assert_eq!(makefile.file_type, "UNKNOWN");

        let readme = files.iter().find(|f| f.name == "README").unwrap();
        assert_eq!(readme.file_type, "UNKNOWN");
    }

    #[tokio::test]
    async fn test_create_delivery_with_nonexistent_files() {
        let result = create_delivery(
            "nonexist".to_owned(),
            "Nonexistent".to_owned(),
            vec!["/nonexistent/file.jpg".to_owned()],
            "/delivery".to_owned(),
            None,
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();

        // Should create job with 0 bytes since file doesn't exist
        assert_eq!(job.total_bytes, 0);

        let _ = remove_delivery_job(job.id).await;
    }

    #[test]
    fn test_collect_project_files_empty_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let mut files = Vec::new();
        let result = collect_project_files(base, base, &mut files);

        assert!(result.is_ok());
        assert_eq!(files.len(), 0);
    }
}
