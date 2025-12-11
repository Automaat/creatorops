/// Optimized file operations using `spawn_blocking` for sync I/O
///
/// Per Phase 3 optimization:
/// - Use `spawn_blocking` for simple file operations (copy, remove, metadata)
/// - Use `tokio::fs` only for operations needing cancellation or progress tracking
use std::path::Path;

/// Copy file using `spawn_blocking` (more efficient for simple copies)
///
/// Use this for fire-and-forget copies without progress tracking.
/// For progress tracking, use `tokio::fs` with chunked copying.
pub async fn copy_file(source: &Path, dest: &Path) -> Result<u64, String> {
    let source = source.to_path_buf();
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || std::fs::copy(&source, &dest))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
        .map_err(|e| format!("Copy failed: {e}"))
}

/// Remove file using `spawn_blocking` (more efficient than `tokio::fs`)
pub async fn remove_file(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || std::fs::remove_file(&path))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
        .map_err(|e| format!("Remove failed: {e}"))
}

/// Get file metadata using `spawn_blocking`
#[allow(dead_code)] // Created for future use in Phase 3
pub async fn metadata(path: &Path) -> Result<std::fs::Metadata, String> {
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || std::fs::metadata(&path))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
        .map_err(|e| format!("Metadata failed: {e}"))
}

/// Create directory using `spawn_blocking`
#[allow(dead_code)] // Created for future use in Phase 3
pub async fn create_dir_all(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || std::fs::create_dir_all(&path))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
        .map_err(|e| format!("Create dir failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_copy_file() {
        let temp_dir = std::env::temp_dir();
        let src = temp_dir.join("test_copy_src.txt");
        let dest = temp_dir.join("test_copy_dest.txt");

        std::fs::write(&src, b"test content").unwrap();

        let size = copy_file(&src, &dest).await.unwrap();
        assert_eq!(size, 12);
        assert!(dest.exists());

        std::fs::remove_file(src).ok();
        std::fs::remove_file(dest).ok();
    }

    #[tokio::test]
    async fn test_remove_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_remove.txt");

        std::fs::write(&test_file, b"test").unwrap();
        assert!(test_file.exists());

        remove_file(&test_file).await.unwrap();
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_metadata() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_metadata.txt");

        std::fs::write(&test_file, b"12345").unwrap();

        let meta = metadata(&test_file).await.unwrap();
        assert_eq!(meta.len(), 5);

        std::fs::remove_file(test_file).ok();
    }

    #[tokio::test]
    async fn test_create_dir_all() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("test_create_dir").join("nested");

        create_dir_all(&test_dir).await.unwrap();
        assert!(test_dir.exists());

        std::fs::remove_dir_all(temp_dir.join("test_create_dir")).ok();
    }
}
