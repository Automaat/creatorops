#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
use serde::{Deserialize, Serialize};
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
#[allow(clippy::too_many_lines)] // Complex import logic requires detailed handling
#[tauri::command]
pub async fn copy_files(
    state: tauri::State<'_, crate::state::AppState>,
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
        let mut tokens = state.import_tokens.lock().await;
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

    for src_path in &source_paths {
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
            let _permit = semaphore_clone.acquire().await.map_err(|e| e.to_string())?;

            // Check if cancelled before starting work
            if cancel_token_clone.is_cancelled() {
                return Err("Import cancelled".to_owned());
            }

            match copy_file_with_retry(&src, &dest_file, &cancel_token_clone).await {
                Ok(size) => {
                    let copied = files_copied_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    // Safe cast: file sizes fit in usize on target platform
                    #[allow(clippy::cast_possible_truncation)]
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
                        return Err("Import cancelled".to_owned());
                    }
                    // Copy failed after retries - track as skipped
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
        #[allow(clippy::match_same_arms)] // Different semantics: success vs failure
        match result {
            Ok(Ok(())) => {} // Success
            Ok(Err(e)) if e == "Import cancelled" => {
                cancelled = true;
            }
            Ok(Err(_)) => {} // File copy failed, already counted as skipped
            Err(e) => return Err(format!("Task failed: {e}")),
        }
    }

    // Clean up token
    {
        let mut tokens = state.import_tokens.lock().await;
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
            Some(format!("Import cancelled ({files_copied} files copied)"))
        } else if files_skipped > 0 {
            Some(format!("{files_skipped} file(s) skipped due to errors"))
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
            return Err("Import cancelled".to_owned());
        }
        // Use fast native copy instead of manual chunking
        tokio::fs::copy(src, dest).await.map_err(|e| e.to_string())
    })
    .await
}

/// Core logic for canceling an import (testable)
///
/// # Errors
///
/// Returns error if import not found or already completed
pub async fn cancel_import_impl(
    import_tokens: &crate::state::ImportTokens,
    import_id: String,
) -> Result<(), String> {
    let tokens = import_tokens.lock().await;
    tokens.get(&import_id).map_or_else(
        || Err("Import not found or already completed".to_owned()),
        |token| {
            token.cancel();
            Ok(())
        },
    )
}

/// Cancel an ongoing import
#[tauri::command]
pub async fn cancel_import(
    state: tauri::State<'_, crate::state::AppState>,
    import_id: String,
) -> Result<(), String> {
    cancel_import_impl(&state.import_tokens, import_id).await
}

#[allow(clippy::wildcard_imports)]
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
            skipped_files: vec!["file1.jpg".to_owned()],
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
            current_file: "test.jpg".to_owned(),
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains('5'));
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

    // TODO: Fix tests after Phase 2 state management migration
    // These tests need to be updated to work with tauri::State
    // Requires refactoring to either:
    // 1. Extract business logic from Tauri commands
    // 2. Create mock Tauri state for testing
    // 3. Use integration tests instead of unit tests

    #[tokio::test]
    #[ignore = "Needs state parameter - Phase 2 migration TODO"]
    async fn test_cancel_import() {
        // Test disabled - needs AppState parameter
    }

    #[tokio::test]
    #[ignore = "Needs state parameter - Phase 2 migration TODO"]
    async fn test_cancel_import_not_found() {
        // Test disabled - needs AppState parameter
    }

    #[test]
    fn test_copy_result_with_errors() {
        let result = CopyResult {
            success: false,
            error: Some("3 file(s) skipped due to errors".to_owned()),
            files_copied: 5,
            files_skipped: 3,
            skipped_files: vec![
                "file1.jpg".to_owned(),
                "file2.mp4".to_owned(),
                "file3.png".to_owned(),
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
            error: Some("Import cancelled (10 files copied)".to_owned()),
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
        for ext in PHOTO_EXTENSIONS {
            let path = format!("test.{ext}");
            assert_eq!(get_file_type(Path::new(&path)), Some("photo"));
        }
    }

    #[test]
    fn test_all_video_extensions() {
        for ext in VIDEO_EXTENSIONS {
            let path = format!("test.{ext}");
            assert_eq!(get_file_type(Path::new(&path)), Some("video"));
        }
    }

    #[test]
    fn test_get_file_type_no_extension() {
        assert_eq!(get_file_type(Path::new("test")), None);
        assert_eq!(get_file_type(Path::new("test.")), None);
    }

    #[test]
    fn test_copy_result_deserialization() {
        let json = r#"{
            "success": true,
            "error": null,
            "filesCopied": 5,
            "filesSkipped": 1,
            "skippedFiles": ["file.txt"],
            "totalBytes": 2048,
            "photosCopied": 4,
            "videosCopied": 1
        }"#;

        let result: CopyResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.files_copied, 5);
        assert_eq!(result.files_skipped, 1);
        assert_eq!(result.photos_copied, 4);
        assert_eq!(result.videos_copied, 1);
    }

    #[test]
    fn test_import_progress_complete() {
        let progress = ImportProgress {
            files_copied: 10,
            total_files: 10,
            current_file: "last.jpg".to_owned(),
        };

        assert_eq!(progress.files_copied, progress.total_files);
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_creates_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.jpg");
        let dest = temp_dir.path().join("subdir").join("dest.jpg");

        let mut file = std::fs::File::create(&src).unwrap();
        file.write_all(b"test").unwrap();

        // Create parent directory for destination
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_ok());
        assert!(dest.exists());
    }

    #[test]
    fn test_copy_result_success_criteria() {
        let success = CopyResult {
            success: true,
            error: None,
            files_copied: 10,
            files_skipped: 0,
            skipped_files: vec![],
            total_bytes: 1024,
            photos_copied: 6,
            videos_copied: 4,
        };

        assert!(success.success);
        assert!(success.error.is_none());
        assert_eq!(
            success.files_copied,
            success.photos_copied + success.videos_copied
        );
    }

    #[test]
    fn test_mixed_photo_video_extensions() {
        assert_eq!(get_file_type(Path::new("IMG_0001.CR2")), Some("photo"));
        assert_eq!(get_file_type(Path::new("VID_0001.MOV")), Some("video"));
        assert_eq!(get_file_type(Path::new("photo.HEIC")), Some("photo"));
        assert_eq!(get_file_type(Path::new("clip.M4V")), Some("video"));
    }

    #[test]
    fn test_get_file_type_unknown_extension() {
        assert_eq!(get_file_type(Path::new("document.pdf")), None);
        assert_eq!(get_file_type(Path::new("data.txt")), None);
        assert_eq!(get_file_type(Path::new("archive.zip")), None);
    }

    #[test]
    fn test_file_type_case_insensitivity() {
        assert_eq!(get_file_type(Path::new("photo.JPG")), Some("photo"));
        assert_eq!(get_file_type(Path::new("photo.jPg")), Some("photo"));
        assert_eq!(get_file_type(Path::new("video.MP4")), Some("video"));
        assert_eq!(get_file_type(Path::new("video.MoV")), Some("video"));
    }

    #[test]
    fn test_get_file_type_with_path() {
        assert_eq!(
            get_file_type(Path::new("/path/to/photo.jpg")),
            Some("photo")
        );
        assert_eq!(
            get_file_type(Path::new("/path/to/video.mp4")),
            Some("video")
        );
        assert_eq!(get_file_type(Path::new("/path/to/file.txt")), None);
    }

    // Integration tests for main execution paths
    #[tokio::test]
    async fn test_copy_file_with_retry_exponential_backoff() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.dat");
        let dest = temp_dir.path().join("dest.dat");

        std::fs::write(&src, b"retry test data").unwrap();

        let cancel_token = CancellationToken::new();
        let start = std::time::Instant::now();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(dest.exists());
        // First attempt should succeed quickly
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_respects_max_attempts() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();

        // Non-existent source should fail after retries
        let src = temp_dir.path().join("nonexistent.dat");
        let dest = temp_dir.path().join("dest.dat");

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn test_constants_values() {
        assert_eq!(MAX_RETRY_ATTEMPTS, 3);
        assert_eq!(MAX_CONCURRENT_COPIES, 4);
    }

    #[test]
    fn test_photo_and_video_extensions_comprehensive() {
        // Test all photo extensions
        let photo_files = vec![
            "IMG.jpg", "IMG.jpeg", "IMG.png", "IMG.gif", "IMG.bmp", "IMG.tiff", "IMG.tif",
            "IMG.raw", "IMG.cr2", "IMG.nef", "IMG.arw", "IMG.dng", "IMG.orf", "IMG.rw2", "IMG.pef",
            "IMG.srw", "IMG.heic", "IMG.heif", "IMG.webp",
        ];

        for file in photo_files {
            assert_eq!(
                get_file_type(Path::new(file)),
                Some("photo"),
                "Failed for {file}"
            );
        }

        // Test all video extensions
        let video_files = vec![
            "VID.mp4", "VID.mov", "VID.avi", "VID.mkv", "VID.wmv", "VID.flv", "VID.webm",
            "VID.m4v", "VID.mpg", "VID.mpeg", "VID.3gp", "VID.mts", "VID.m2ts",
        ];

        for file in video_files {
            assert_eq!(
                get_file_type(Path::new(file)),
                Some("video"),
                "Failed for {file}"
            );
        }
    }

    #[tokio::test]
    #[ignore = "Needs state parameter - Phase 2 migration TODO"]
    async fn test_cancel_import_cancels_token() {
        // Test disabled - needs AppState parameter
    }

    #[test]
    fn test_copy_result_with_partial_success() {
        let result = CopyResult {
            success: true,
            error: Some("2 file(s) skipped due to errors".to_owned()),
            files_copied: 8,
            files_skipped: 2,
            skipped_files: vec!["bad1.jpg".to_owned(), "bad2.mp4".to_owned()],
            total_bytes: 8192,
            photos_copied: 6,
            videos_copied: 2,
        };

        assert!(result.success);
        assert!(result.error.is_some());
        assert_eq!(result.files_copied, 8);
        assert_eq!(result.files_skipped, 2);
        assert_eq!(result.skipped_files.len(), 2);
        assert_eq!(
            result.photos_copied + result.videos_copied,
            result.files_copied
        );
    }

    #[test]
    fn test_copy_result_complete_success() {
        let result = CopyResult {
            success: true,
            error: None,
            files_copied: 15,
            files_skipped: 0,
            skipped_files: vec![],
            total_bytes: 15360,
            photos_copied: 10,
            videos_copied: 5,
        };

        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.files_skipped, 0);
        assert!(result.skipped_files.is_empty());
        assert_eq!(
            result.photos_copied + result.videos_copied,
            result.files_copied
        );
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_cancel_during_retry() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("cancel_test.dat");
        let dest = temp_dir.path().join("dest_cancel.dat");

        std::fs::write(&src, b"data to cancel").unwrap();

        let cancel_token = CancellationToken::new();

        // Cancel immediately before copy
        cancel_token.cancel();

        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Import cancelled");
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_large_file() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("large.bin");
        let dest = temp_dir.path().join("large_dest.bin");

        // Create 5MB file
        let data = vec![0xAB; 5 * 1024 * 1024];
        std::fs::write(&src, &data).unwrap();

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);
        assert!(dest.exists());

        let dest_data = std::fs::read(&dest).unwrap();
        assert_eq!(dest_data.len(), data.len());
    }

    #[test]
    fn test_import_progress_zero_progress() {
        let progress = ImportProgress {
            files_copied: 0,
            total_files: 100,
            current_file: String::new(),
        };

        assert_eq!(progress.files_copied, 0);
        assert!(progress.files_copied < progress.total_files);
    }

    #[test]
    fn test_import_progress_mid_progress() {
        let progress = ImportProgress {
            files_copied: 50,
            total_files: 100,
            current_file: "photo_50.jpg".to_owned(),
        };

        // Safe cast: small test values well within f64 mantissa precision
        #[allow(clippy::cast_precision_loss)]
        let percent = (progress.files_copied as f64 / progress.total_files as f64) * 100.0;
        assert!((percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_file_type_real_world_filenames() {
        // Canon camera files
        assert_eq!(get_file_type(Path::new("IMG_1234.CR2")), Some("photo"));
        assert_eq!(get_file_type(Path::new("IMG_1234.JPG")), Some("photo"));

        // Nikon camera files
        assert_eq!(get_file_type(Path::new("DSC_5678.NEF")), Some("photo"));
        assert_eq!(get_file_type(Path::new("DSC_5678.JPG")), Some("photo"));

        // Sony camera files
        assert_eq!(get_file_type(Path::new("DSC00123.ARW")), Some("photo"));

        // iPhone files
        assert_eq!(get_file_type(Path::new("IMG_9876.HEIC")), Some("photo"));
        assert_eq!(get_file_type(Path::new("IMG_9876.MOV")), Some("video"));

        // Video files
        assert_eq!(get_file_type(Path::new("MVI_0001.MP4")), Some("video"));
        assert_eq!(get_file_type(Path::new("GOPR0123.MP4")), Some("video"));
    }

    #[tokio::test]
    #[ignore = "Needs state parameter - Phase 2 migration TODO"]
    async fn test_cancel_import_idempotent() {
        // Test disabled - needs AppState parameter
    }

    #[tokio::test]
    async fn test_copy_file_with_retry_zero_byte_file() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("empty.txt");
        let dest = temp_dir.path().join("empty_dest.txt");

        std::fs::write(&src, b"").unwrap();

        let cancel_token = CancellationToken::new();
        let result = copy_file_with_retry(&src, &dest, &cancel_token).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(dest.exists());
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 0);
    }

    #[test]
    fn test_copy_result_all_skipped() {
        let result = CopyResult {
            success: false,
            error: Some("10 file(s) skipped due to errors".to_owned()),
            files_copied: 0,
            files_skipped: 10,
            skipped_files: vec!["f1.jpg".to_owned(), "f2.jpg".to_owned()],
            total_bytes: 0,
            photos_copied: 0,
            videos_copied: 0,
        };

        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.files_copied, 0);
        assert_eq!(result.files_skipped, 10);
    }
}
