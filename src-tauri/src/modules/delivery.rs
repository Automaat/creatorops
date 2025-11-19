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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|| "0".to_string());

            files.push(ProjectFile {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
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
    let mut total_bytes = 0u64;
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
        let mut queue = DELIVERY_QUEUE.lock().unwrap();
        queue.insert(id.clone(), job.clone());
    }

    Ok(job)
}

/// Start a delivery job
#[tauri::command]
pub async fn start_delivery(job_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // Get job from queue
    let job = {
        let mut queue = DELIVERY_QUEUE.lock().unwrap();
        let job = queue.get_mut(&job_id).ok_or("Job not found")?;

        if job.status != DeliveryStatus::Pending {
            return Err("Job is not in pending status".to_string());
        }

        job.status = DeliveryStatus::InProgress;
        job.started_at = Some(get_timestamp());
        job.clone()
    };

    // Spawn background task
    tokio::spawn(async move {
        let result = process_delivery(job.clone(), app_handle.clone()).await;

        // Update job status
        let mut queue = DELIVERY_QUEUE.lock().unwrap();
        if let Some(job) = queue.get_mut(&job_id) {
            match result {
                Ok(_) => {
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
        let dest_name = if let Some(template) = &job.naming_template {
            apply_naming_template(template, &file_name, index)
        } else {
            file_name.clone()
        };

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
        manifest_entries.push(format!("{} -> {} ({})", file_name, dest_name, file_size));

        // Update queue
        {
            let mut queue = DELIVERY_QUEUE.lock().unwrap();
            if let Some(q_job) = queue.get_mut(&job.id) {
                q_job.files_copied = job.files_copied;
                q_job.bytes_transferred = job.bytes_transferred;
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
        let mut queue = DELIVERY_QUEUE.lock().unwrap();
        if let Some(q_job) = queue.get_mut(&job.id) {
            q_job.manifest_path = Some(manifest_path.to_string_lossy().to_string());
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

    let mut buffer = vec![0u8; CHUNK_SIZE];
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
        let speed = if elapsed > 0.0 {
            *bytes_transferred as f64 / elapsed
        } else {
            0.0
        };

        let remaining_bytes = total_bytes.saturating_sub(*bytes_transferred);
        let eta = if speed > 0.0 {
            (remaining_bytes as f64 / speed) as u64
        } else {
            0
        };

        // Emit progress event
        let progress = DeliveryProgress {
            job_id: job_id.to_string(),
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

    template
        .replace("{index}", &format!("{:03}", index + 1))
        .replace("{name}", &name_without_ext)
        .replace("{ext}", &ext)
}

/// Get delivery queue
#[tauri::command]
pub async fn get_delivery_queue() -> Result<Vec<DeliveryJob>, String> {
    let queue = DELIVERY_QUEUE.lock().unwrap();
    Ok(queue.values().cloned().collect())
}

/// Remove a delivery job from queue
#[tauri::command]
pub async fn remove_delivery_job(job_id: String) -> Result<(), String> {
    let mut queue = DELIVERY_QUEUE.lock().unwrap();
    queue.remove(&job_id);
    Ok(())
}
