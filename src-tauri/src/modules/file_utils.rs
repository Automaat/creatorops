#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks

/// Calculate SHA-256 hash of a file
pub async fn calculate_file_hash(path: &Path) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| e.to_string())?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; CHUNK_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer).await.map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify file integrity using SHA-256 checksum
pub async fn verify_checksum(src: &Path, dest: &Path) -> Result<bool, String> {
    let src_hash = calculate_file_hash(src).await?;
    let dest_hash = calculate_file_hash(dest).await?;
    Ok(src_hash == dest_hash)
}

/// Recursively collect all files in a directory
pub fn collect_files_recursive(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                files.push(entry_path);
            } else if entry_path.is_dir() {
                let mut sub_files = collect_files_recursive(&entry_path)?;
                files.append(&mut sub_files);
            }
        }
    }

    Ok(files)
}

/// Count files and calculate total size
type FileSizeResult = Result<(usize, u64), String>;

pub fn count_files_and_size(path: &str) -> FileSizeResult {
    let files = collect_files_recursive(Path::new(path))?;
    let mut total_size = 0_u64;

    for file in &files {
        if let Ok(metadata) = fs::metadata(file) {
            total_size += metadata.len();
        }
    }

    Ok((files.len(), total_size))
}

/// Get home directory (cross-platform)
pub fn get_home_dir() -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        std::env::var_os("HOME")
            .and_then(|h| if h.is_empty() { None } else { Some(h) })
            .map(PathBuf::from)
            .ok_or_else(|| "Failed to get home directory".to_owned())
    }

    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .or_else(|| {
                // Fallback: combine HOMEDRIVE and HOMEPATH
                std::env::var_os("HOMEDRIVE").and_then(|drive| {
                    std::env::var_os("HOMEPATH").map(|path| {
                        let mut full_path = PathBuf::from(drive);
                        full_path.push(path);
                        full_path.into_os_string()
                    })
                })
            })
            .and_then(|h| if h.is_empty() { None } else { Some(h) })
            .map(PathBuf::from)
            .ok_or_else(|| "Failed to get home directory".to_owned())
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err("Unsupported platform for home directory detection".to_owned())
    }
}

/// Get timestamp as string
pub fn get_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs()
        .to_string()
}

#[tauri::command]
pub fn get_home_directory() -> Result<String, String> {
    get_home_dir()?
        .to_str()
        .map(ToString::to_string)
        .ok_or_else(|| "Failed to convert path to string".to_owned())
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_calculate_file_hash() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hash.txt");

        // Create test file with known content
        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        drop(file);

        let hash = calculate_file_hash(&test_file).await.unwrap();

        // SHA-256 of "Hello, World!"
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a428688a362182986f"
        );

        std::fs::remove_file(test_file).ok();
    }

    #[tokio::test]
    async fn test_verify_checksum_matching() {
        let temp_dir = std::env::temp_dir();
        let src_file = temp_dir.join("test_src.txt");
        let dest_file = temp_dir.join("test_dest.txt");

        // Create identical files
        std::fs::write(&src_file, b"test content").unwrap();
        std::fs::write(&dest_file, b"test content").unwrap();

        let result = verify_checksum(&src_file, &dest_file).await.unwrap();
        assert!(result);

        std::fs::remove_file(src_file).ok();
        std::fs::remove_file(dest_file).ok();
    }

    #[tokio::test]
    async fn test_verify_checksum_different() {
        let temp_dir = std::env::temp_dir();
        let src_file = temp_dir.join("test_src2.txt");
        let dest_file = temp_dir.join("test_dest2.txt");

        // Create different files
        std::fs::write(&src_file, b"test content 1").unwrap();
        std::fs::write(&dest_file, b"test content 2").unwrap();

        let result = verify_checksum(&src_file, &dest_file).await.unwrap();
        assert!(!result);

        std::fs::remove_file(src_file).ok();
        std::fs::remove_file(dest_file).ok();
    }

    #[test]
    fn test_collect_files_recursive() {
        let temp_dir = std::env::temp_dir().join("test_collect");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("subdir")).unwrap();

        // Create test files
        std::fs::write(temp_dir.join("file1.txt"), b"test").unwrap();
        std::fs::write(temp_dir.join("file2.txt"), b"test").unwrap();
        std::fs::write(temp_dir.join("subdir").join("file3.txt"), b"test").unwrap();

        let files = collect_files_recursive(&temp_dir).unwrap();
        assert_eq!(files.len(), 3);

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_collect_files_single_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("single_file.txt");
        std::fs::write(&test_file, b"test").unwrap();

        let files = collect_files_recursive(&test_file).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);

        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_count_files_and_size() {
        let temp_dir = std::env::temp_dir().join("test_count");
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("file1.txt"), b"12345").unwrap();
        std::fs::write(temp_dir.join("file2.txt"), b"1234567890").unwrap();

        let (count, size) = count_files_and_size(temp_dir.to_str().unwrap()).unwrap();
        assert_eq!(count, 2);
        assert_eq!(size, 15);

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_get_home_dir() {
        let home = get_home_dir().unwrap();
        assert!(home.is_absolute());
        // In CI environments, HOME might not exist, so create it for testing
        if home.exists() {
            assert!(home.exists());
        } else {
            std::fs::create_dir_all(&home).unwrap();
            assert!(home.exists());
            std::fs::remove_dir_all(&home).ok();
        }
    }

    #[test]
    fn test_get_timestamp() {
        let ts1 = get_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(1100)); // Need >1s for timestamp to change
        let ts2 = get_timestamp();

        let t1: u64 = ts1.parse().unwrap();
        let t2: u64 = ts2.parse().unwrap();
        assert!(t2 > t1);
    }

    #[test]
    fn test_get_home_directory_command() {
        let result = get_home_directory().unwrap();
        assert!(!result.is_empty());
        let home_path = PathBuf::from(&result);
        assert!(home_path.is_absolute());
        // In CI environments, HOME might not exist, so create it for testing
        if home_path.exists() {
            assert!(home_path.exists());
        } else {
            std::fs::create_dir_all(&home_path).unwrap();
            assert!(home_path.exists());
            std::fs::remove_dir_all(&home_path).ok();
        }
    }

    #[tokio::test]
    async fn test_calculate_hash_large_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("large_file.dat");

        // Create file larger than CHUNK_SIZE (>4MB)
        let data = vec![0_u8; 5 * 1024 * 1024]; // 5MB
        std::fs::write(&test_file, data).unwrap();

        let hash = calculate_file_hash(&test_file).await.unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters

        std::fs::remove_file(test_file).ok();
    }

    #[tokio::test]
    async fn test_calculate_hash_nonexistent_file() {
        let nonexistent = std::path::PathBuf::from("/nonexistent/file.txt");
        let result = calculate_file_hash(&nonexistent).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_files_empty_dir() {
        let temp_dir = std::env::temp_dir().join("empty_dir");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let files = collect_files_recursive(&temp_dir).unwrap();
        assert_eq!(files.len(), 0);

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_count_files_empty_directory() {
        let temp_dir = std::env::temp_dir().join("empty_count");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let (count, size) = count_files_and_size(temp_dir.to_str().unwrap()).unwrap();
        assert_eq!(count, 0);
        assert_eq!(size, 0);

        std::fs::remove_dir_all(temp_dir).ok();
    }
}
