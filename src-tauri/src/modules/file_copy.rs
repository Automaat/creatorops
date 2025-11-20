use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;

const MAX_RETRY_ATTEMPTS: usize = 3;
const MAX_CONCURRENT_COPIES: usize = 4; // Parallel file copies

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub skipped_files: Vec<String>,
}

/// Copy files from source to destination with parallel processing
#[tauri::command]
pub async fn copy_files(
    _window: tauri::Window,
    source_paths: Vec<String>,
    destination: String,
) -> Result<CopyResult, String> {
    let dest_path = PathBuf::from(&destination);

    // Create destination directory if it doesn't exist
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
    }

    let files_copied = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let files_skipped = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let skipped_files = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    // Process files in parallel batches
    let mut tasks = Vec::new();

    for src_path in source_paths.iter() {
        let src = PathBuf::from(src_path);
        let file_name = src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dest_file = dest_path.join(&file_name);

        let files_copied_clone = files_copied.clone();
        let files_skipped_clone = files_skipped.clone();
        let skipped_files_clone = skipped_files.clone();

        let task = tokio::spawn(async move {
            match copy_file_with_retry(&src, &dest_file).await {
                Ok(_size) => {
                    let mut fc = files_copied_clone.lock().unwrap();
                    *fc += 1;
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to copy {} after retries: {}", file_name, e);
                    let mut fs = files_skipped_clone.lock().unwrap();
                    *fs += 1;
                    let mut sf = skipped_files_clone.lock().unwrap();
                    sf.push(file_name);
                    Err(e)
                }
            }
        });

        tasks.push(task);

        // Limit concurrent copies
        if tasks.len() >= MAX_CONCURRENT_COPIES {
            futures::future::join_all(tasks.drain(..)).await;
        }
    }

    // Wait for remaining tasks
    futures::future::join_all(tasks).await;

    let files_copied = *files_copied.lock().unwrap();
    let files_skipped = *files_skipped.lock().unwrap();
    let skipped_files = skipped_files.lock().unwrap().clone();

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

/// Copy file with retry logic using fast native copy
async fn copy_file_with_retry(src: &Path, dest: &Path) -> Result<u64, String> {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter)
        .take(MAX_RETRY_ATTEMPTS);

    Retry::spawn(retry_strategy, || async {
        // Use fast native copy instead of manual chunking
        tokio::fs::copy(src, dest).await.map_err(|e| e.to_string())
    })
    .await
}
