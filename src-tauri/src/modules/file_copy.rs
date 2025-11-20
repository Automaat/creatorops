use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tokio_util::sync::CancellationToken;

const MAX_RETRY_ATTEMPTS: usize = 3;
const MAX_CONCURRENT_COPIES: usize = 4; // Parallel file copies

// Global map of active import cancellation tokens
lazy_static::lazy_static! {
    static ref IMPORT_TOKENS: Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub skipped_files: Vec<String>,
    pub total_bytes: u64,
}

/// Copy files from source to destination with parallel processing
#[tauri::command]
pub async fn copy_files(
    import_id: String,
    source_paths: Vec<String>,
    destination: String,
) -> Result<CopyResult, String> {
    let dest_path = PathBuf::from(&destination);

    // Create destination directory if it doesn't exist
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
    }

    // Create cancellation token and register it
    let cancel_token = CancellationToken::new();
    {
        let mut tokens = IMPORT_TOKENS.lock().await;
        tokens.insert(import_id.clone(), cancel_token.clone());
    }

    let files_copied = Arc::new(AtomicUsize::new(0));
    let files_skipped = Arc::new(AtomicUsize::new(0));
    let total_bytes = Arc::new(AtomicUsize::new(0));
    let skipped_files = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_COPIES));

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
        let total_bytes_clone = total_bytes.clone();
        let skipped_files_clone = skipped_files.clone();
        let semaphore_clone = semaphore.clone();
        let cancel_token_clone = cancel_token.clone();

        let task = tokio::spawn(async move {
            // Check if cancelled before starting
            if cancel_token_clone.is_cancelled() {
                return Err("Import cancelled".to_string());
            }

            let _permit = semaphore_clone.acquire().await.unwrap();

            // Check again after acquiring semaphore
            if cancel_token_clone.is_cancelled() {
                return Err("Import cancelled".to_string());
            }

            match copy_file_with_retry(&src, &dest_file, &cancel_token_clone).await {
                Ok(size) => {
                    files_copied_clone.fetch_add(1, Ordering::SeqCst);
                    total_bytes_clone.fetch_add(size as usize, Ordering::SeqCst);
                    Ok(())
                }
                Err(e) => {
                    if cancel_token_clone.is_cancelled() {
                        return Err("Import cancelled".to_string());
                    }
                    eprintln!("Failed to copy {} after retries: {}", file_name, e);
                    files_skipped_clone.fetch_add(1, Ordering::SeqCst);
                    skipped_files_clone.lock().await.push(file_name);
                    Err(e)
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks and handle errors
    let mut cancelled = false;
    for result in futures::future::join_all(tasks).await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) if e == "Import cancelled" => {
                cancelled = true;
            }
            Ok(Err(_)) => {} // File copy failed, already counted as skipped
            Err(e) => return Err(format!("Task failed: {}", e)),
        }
    }

    // Clean up token
    {
        let mut tokens = IMPORT_TOKENS.lock().await;
        tokens.remove(&import_id);
    }

    let files_copied = files_copied.load(Ordering::SeqCst);
    let files_skipped = files_skipped.load(Ordering::SeqCst);
    let total_bytes = total_bytes.load(Ordering::SeqCst) as u64;
    let skipped_files = skipped_files.lock().await.clone();

    Ok(CopyResult {
        success: !cancelled && files_copied > 0,
        error: if cancelled {
            Some(format!("Import cancelled ({} files copied)", files_copied))
        } else if files_skipped > 0 {
            Some(format!("{} file(s) skipped due to errors", files_skipped))
        } else {
            None
        },
        files_copied,
        files_skipped,
        skipped_files,
        total_bytes,
    })
}

/// Copy file with retry logic using fast native copy
async fn copy_file_with_retry(
    src: &Path,
    dest: &Path,
    cancel_token: &CancellationToken,
) -> Result<u64, String> {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter)
        .take(MAX_RETRY_ATTEMPTS);

    Retry::spawn(retry_strategy, || async {
        if cancel_token.is_cancelled() {
            return Err("Import cancelled".to_string());
        }
        // Use fast native copy instead of manual chunking
        tokio::fs::copy(src, dest).await.map_err(|e| e.to_string())
    })
    .await
}

/// Cancel an ongoing import
#[tauri::command]
pub async fn cancel_import(import_id: String) -> Result<(), String> {
    let tokens = IMPORT_TOKENS.lock().await;
    if let Some(token) = tokens.get(&import_id) {
        token.cancel();
        Ok(())
    } else {
        Err("Import not found or already completed".to_string())
    }
}
