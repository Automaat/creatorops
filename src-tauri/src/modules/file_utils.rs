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
    let mut buffer = vec![0u8; CHUNK_SIZE];

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
pub fn count_files_and_size(path: &str) -> Result<(usize, u64), String> {
    let files = collect_files_recursive(Path::new(path))?;
    let mut total_size = 0u64;

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
            .ok_or_else(|| "Failed to get home directory".to_string())
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
            .ok_or_else(|| "Failed to get home directory".to_string())
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err("Unsupported platform for home directory detection".to_string())
    }
}

/// Get timestamp as string
pub fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch");
    format!("{}", duration.as_secs())
}

#[tauri::command]
pub fn get_home_directory() -> Result<String, String> {
    get_home_dir().and_then(|path| {
        path.to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Failed to convert path to string".to_string())
    })
}
