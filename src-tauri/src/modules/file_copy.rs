use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tokio_util::sync::CancellationToken;

const MAX_RETRY_ATTEMPTS: usize = 3;
const MAX_CONCURRENT_COPIES: usize = 4; // Parallel file copies

// Photo extensions
const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "raw", "cr2", "nef", "arw", "dng", "orf",
    "rw2", "pef", "srw", "heic", "heif", "webp",
];

// Video extensions
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp", "mts", "m2ts",
];

/// Detect if file is a photo or video based on extension
fn get_file_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    if PHOTO_EXTENSIONS.contains(&ext.as_str()) {
        Some("photo")
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some("video")
    } else {
        None
    }
}

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
    pub photos_copied: usize,
    pub videos_copied: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub files_copied: usize,
    pub total_files: usize,
    pub current_file: String,
}

/// Copy files from source to destination with parallel processing
#[tauri::command]
pub async fn copy_files(
    app: AppHandle,
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
    let photos_copied = Arc::new(AtomicUsize::new(0));
    let videos_copied = Arc::new(AtomicUsize::new(0));
    let skipped_files = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_COPIES));
    let total_files = source_paths.len();

    let mut tasks = Vec::new();

    for src_path in source_paths.iter() {
        let src = PathBuf::from(src_path);
        let file_name = src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let file_type = get_file_type(&src);

        // Route to Photos or Videos subdirectory based on file type
        let dest_file = match file_type {
            Some("photo") => dest_path.join("Photos").join(&file_name),
            Some("video") => dest_path.join("Videos").join(&file_name),
            _ => dest_path.join(&file_name), // Fallback to root if unknown type
        };

        let files_copied_clone = files_copied.clone();
        let files_skipped_clone = files_skipped.clone();
        let total_bytes_clone = total_bytes.clone();
        let photos_copied_clone = photos_copied.clone();
        let videos_copied_clone = videos_copied.clone();
        let skipped_files_clone = skipped_files.clone();
        let semaphore_clone = semaphore.clone();
        let cancel_token_clone = cancel_token.clone();
        let app_clone = app.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            // Check if cancelled before starting work
            if cancel_token_clone.is_cancelled() {
                return Err("Import cancelled".to_string());
            }

            match copy_file_with_retry(&src, &dest_file, &cancel_token_clone).await {
                Ok(size) => {
                    let copied = files_copied_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    total_bytes_clone.fetch_add(size as usize, Ordering::SeqCst);

                    // Track photo/video counts
                    match file_type {
                        Some("photo") => {
                            photos_copied_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Some("video") => {
                            videos_copied_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }

                    // Emit progress event
                    let _ = app_clone.emit(
                        "import-progress",
                        ImportProgress {
                            files_copied: copied,
                            total_files,
                            current_file: file_name.clone(),
                        },
                    );

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
    let photos_copied = photos_copied.load(Ordering::SeqCst);
    let videos_copied = videos_copied.load(Ordering::SeqCst);
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
        photos_copied,
        videos_copied,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_get_file_type_photos() {
        assert_eq!(get_file_type(Path::new("test.jpg")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.JPG")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.png")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.raw")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.cr2")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.nef")), Some("photo"));
        assert_eq!(get_file_type(Path::new("test.heic")), Some("photo"));
    }

    #[test]
    fn test_get_file_type_videos() {
        assert_eq!(get_file_type(Path::new("test.mp4")), Some("video"));
        assert_eq!(get_file_type(Path::new("test.MP4")), Some("video"));
        assert_eq!(get_file_type(Path::new("test.mov")), Some("video"));
        assert_eq!(get_file_type(Path::new("test.avi")), Some("video"));
        assert_eq!(get_file_type(Path::new("test.mkv")), Some("video"));
    }

    #[test]
    fn test_get_file_type_unknown() {
        assert_eq!(get_file_type(Path::new("test.txt")), None);
        assert_eq!(get_file_type(Path::new("test.pdf")), None);
        assert_eq!(get_file_type(Path::new("test")), None);
        assert_eq!(get_file_type(Path::new("test.unknown")), None);
    }

    #[test]
    fn test_copy_result_serialization() {
        let result = CopyResult {
            success: true,
            error: None,
            files_copied: 10,
            files_skipped: 2,
            skipped_files: vec!["file1.jpg".to_string()],
            total_bytes: 1024,
            photos_copied: 8,
            videos_copied: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("10"));
        assert!(json.contains("file1.jpg"));
    }

    #[test]
    fn test_import_progress_serialization() {
        let progress = ImportProgress {
            files_copied: 5,
            total_files: 10,
            current_file: "test.jpg".to_string(),
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("5"));
        assert!(json.contains("10"));
        assert!(json.contains("test.jpg"));
    }

    #[test]
    fn test_photo_extensions() {
        assert!(PHOTO_EXTENSIONS.contains(&"jpg"));
        assert!(PHOTO_EXTENSIONS.contains(&"png"));
        assert!(PHOTO_EXTENSIONS.contains(&"raw"));
        assert!(PHOTO_EXTENSIONS.contains(&"heic"));
    }

    #[test]
    fn test_video_extensions() {
        assert!(VIDEO_EXTENSIONS.contains(&"mp4"));
        assert!(VIDEO_EXTENSIONS.contains(&"mov"));
        assert!(VIDEO_EXTENSIONS.contains(&"avi"));
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_success() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.jpg");
        let dest = temp_dir.path().join("dest.jpg");

        let mut file = std::fs::File::create(&src).unwrap();
        file.write_all(b"test photo data").unwrap();

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 15);
        assert!(dest.exists());
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_cancelled() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.mp4");
        let dest = temp_dir.path().join("dest.mp4");

        let mut file = std::fs::File::create(&src).unwrap();
        file.write_all(b"test video").unwrap();

        let cancel_token = CancellationToken::new();
        cancel_token.cancel();

        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Import cancelled");
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_missing_source() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("nonexistent.jpg");
        let dest = temp_dir.path().join("dest.jpg");

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_import() {
        let import_id = "test-import-123".to_string();
        let cancel_token = CancellationToken::new();

        {
            let mut tokens = IMPORT_TOKENS.lock().await;
            tokens.insert(import_id.clone(), cancel_token.clone());
        }

        let result = cancel_import(import_id.clone()).await;
        assert!(result.is_ok());
        assert!(cancel_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_import_not_found() {
        let result = cancel_import("nonexistent-import".to_string()).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Import not found or already completed"
        );
    }

    #[test]
    fn test_copy_result_with_errors() {
        let result = CopyResult {
            success: false,
            error: Some("3 file(s) skipped due to errors".to_string()),
            files_copied: 5,
            files_skipped: 3,
            skipped_files: vec![
                "file1.jpg".to_string(),
                "file2.mp4".to_string(),
                "file3.png".to_string(),
            ],
            total_bytes: 2048,
            photos_copied: 4,
            videos_copied: 1,
        };

        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.files_skipped, 3);
        assert_eq!(result.skipped_files.len(), 3);
    }

    #[test]
    fn test_copy_result_cancelled() {
        let result = CopyResult {
            success: false,
            error: Some("Import cancelled (10 files copied)".to_string()),
            files_copied: 10,
            files_skipped: 0,
            skipped_files: vec![],
            total_bytes: 5120,
            photos_copied: 8,
            videos_copied: 2,
        };

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("cancelled"));
        assert_eq!(result.files_copied, 10);
    }

    #[test]
    fn test_all_photo_extensions() {
        for ext in PHOTO_EXTENSIONS.iter() {
            let path = format!("test.{}", ext);
            assert_eq!(get_file_type(Path::new(&path)), Some("photo"));
        }
    }

    #[test]
    fn test_all_video_extensions() {
        for ext in VIDEO_EXTENSIONS.iter() {
            let path = format!("test.{}", ext);
            assert_eq!(get_file_type(Path::new(&path)), Some("video"));
        }
    }
}
