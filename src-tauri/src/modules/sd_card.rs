use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDCard {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub free_space: u64,
    pub file_count: usize,
}

/// Scan /Volumes/ directory for mounted SD cards
#[tauri::command]
pub async fn scan_sd_cards() -> Result<Vec<SDCard>, String> {
    let volumes_path = Path::new("/Volumes");

    if !volumes_path.exists() {
        return Ok(Vec::new());
    }

    let mut cards = Vec::new();

    if let Ok(entries) = fs::read_dir(volumes_path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip system volumes
                    if name == "Macintosh HD" || name.starts_with('.') {
                        continue;
                    }

                    // Count files
                    let file_count = count_files(&path);

                    // Get disk usage info
                    let (size, free_space) = get_disk_usage(&path);

                    cards.push(SDCard {
                        name,
                        path: path.to_string_lossy().to_string(),
                        size,
                        free_space,
                        file_count,
                    });
                }
            }
        }
    }

    Ok(cards)
}

fn count_files(path: &Path) -> usize {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

fn get_disk_usage(path: &Path) -> (u64, u64) {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::mem;

        unsafe {
            let path_cstr = CString::new(path.to_str().unwrap()).unwrap();
            let mut stats: libc::statfs = mem::zeroed();

            if libc::statfs(path_cstr.as_ptr(), &mut stats) == 0 {
                let block_size = stats.f_bsize as u64;
                let total_blocks = stats.f_blocks;
                let free_blocks = stats.f_bfree;

                let size = total_blocks * block_size;
                let free_space = free_blocks * block_size;

                return (size, free_space);
            }
        }
    }

    // Fallback for non-macOS or if statfs fails
    (0, 0)
}
