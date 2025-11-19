use crate::modules::file_utils::verify_checksum;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks
const MAX_RETRY_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub file_name: String,
    pub current_file: usize,
    pub total_files: usize,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub eta: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub skipped_files: Vec<String>,
}

/// Context for tracking copy progress across multiple files
struct CopyContext {
    current_file: usize,
    total_files: usize,
    bytes_transferred: u64,
    total_bytes: u64,
    start_time: std::time::Instant,
}

/// Copy files from source to destination with progress tracking and checksum verification
#[tauri::command]
pub async fn copy_files(
    window: tauri::Window,
    source_paths: Vec<String>,
    destination: String,
) -> Result<CopyResult, String> {
    let dest_path = PathBuf::from(&destination);

    // Create destination directory if it doesn't exist
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
    }

    let total_files = source_paths.len();
    let mut total_bytes = 0u64;

    // Calculate total size
    for src in &source_paths {
        if let Ok(metadata) = fs::metadata(src) {
            total_bytes += metadata.len();
        }
    }

    let mut bytes_transferred = 0u64;
    let start_time = std::time::Instant::now();
    let mut files_copied = 0;
    let mut files_skipped = 0;
    let mut skipped_files = Vec::new();

    for (index, src_path) in source_paths.iter().enumerate() {
        let src = Path::new(src_path);
        let file_name = src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let dest_file = dest_path.join(&file_name);

        let context = CopyContext {
            current_file: index + 1,
            total_files,
            bytes_transferred,
            total_bytes,
            start_time,
        };

        // Try copying with retries
        match copy_file_with_retry(src, &dest_file, &window, &context).await {
            Ok(size) => {
                bytes_transferred += size;
                files_copied += 1;
            }
            Err(e) => {
                eprintln!("Failed to copy {} after retries: {}", file_name, e);
                files_skipped += 1;
                skipped_files.push(file_name);
                // Continue with next file instead of failing entire operation
            }
        }
    }

    Ok(CopyResult {
        success: files_copied > 0,
        error: if files_skipped > 0 {
            Some(format!("{} file(s) skipped due to errors", files_skipped))
        } else {
            None
        },
        files_copied,
        files_skipped,
        skipped_files,
    })
}

/// Copy file with retry logic and checksum verification
async fn copy_file_with_retry(
    src: &Path,
    dest: &Path,
    window: &tauri::Window,
    context: &CopyContext,
) -> Result<u64, String> {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter)
        .take(MAX_RETRY_ATTEMPTS);

    Retry::spawn(retry_strategy, || async {
        // Attempt copy
        let size = copy_file_with_progress(src, dest, window, context).await?;

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

async fn copy_file_with_progress(
    src: &Path,
    dest: &Path,
    window: &tauri::Window,
    context: &CopyContext,
) -> Result<u64, String> {
    let file_name = src
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut src_file = tokio::fs::File::open(src)
        .await
        .map_err(|e| e.to_string())?;

    let mut dest_file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| e.to_string())?;

    let file_size = src_file.metadata().await.map_err(|e| e.to_string())?.len();

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut file_bytes_transferred = 0u64;

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

        file_bytes_transferred += bytes_read as u64;

        let elapsed = context.start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            (context.bytes_transferred + file_bytes_transferred) as f64 / elapsed
        } else {
            0.0
        };

        let remaining_bytes =
            context.total_bytes - (context.bytes_transferred + file_bytes_transferred);
        let eta = if speed > 0.0 {
            (remaining_bytes as f64 / speed) as u64
        } else {
            0
        };

        let progress = ImportProgress {
            file_name: file_name.clone(),
            current_file: context.current_file,
            total_files: context.total_files,
            bytes_transferred: context.bytes_transferred + file_bytes_transferred,
            total_bytes: context.total_bytes,
            speed,
            eta,
        };

        let _ = window.emit("import-progress", progress);
    }

    Ok(file_size)
}
