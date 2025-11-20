use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDCard {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub free_space: u64,
    pub file_count: usize,
    pub device_type: String,
    pub is_removable: bool,
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
                    if name == "Macintosh HD"
                        || name == "Data"
                        || name == "Preboot"
                        || name == "Recovery"
                        || name == "VM"
                        || name == "Update"
                        || name.starts_with('.')
                    {
                        continue;
                    }

                    // Count files
                    let file_count = count_files(&path);

                    // Get disk usage info
                    let (size, free_space) = get_disk_usage(&path);

                    // Get device information
                    let (device_type, is_removable) = get_device_info(&name);

                    cards.push(SDCard {
                        name,
                        path: path.to_string_lossy().to_string(),
                        size,
                        free_space,
                        file_count,
                        device_type,
                        is_removable,
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

/// List all files from an SD card path (photo/video files)
#[tauri::command]
pub async fn list_sd_card_files(card_path: String) -> Result<Vec<String>, String> {
    let path = Path::new(&card_path);

    if !path.exists() {
        return Err("SD card path does not exist".to_string());
    }

    let mut file_paths = Vec::new();
    let photo_video_extensions = [
        "jpg", "jpeg", "png", "heic", "heif", "raw", "cr2", "cr3", "nef", "arw", "dng", "mp4",
        "mov", "avi", "mkv", "m4v",
    ];

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        if let Some(ext) = file_path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if photo_video_extensions.contains(&ext_lower.as_str()) {
                file_paths.push(file_path.to_string_lossy().to_string());
            }
        }
    }

    Ok(file_paths)
}

/// Get device type and removability using diskutil (macOS)
fn get_device_info(volume_name: &str) -> (String, bool) {
    #[cfg(target_os = "macos")]
    {
        // Use diskutil to get device information
        let output = Command::new("diskutil")
            .args(["info", volume_name])
            .output();

        if let Ok(output) = output {
            let info = String::from_utf8_lossy(&output.stdout);

            // Parse device type
            let device_type = if info.contains("SD Card") || info.contains("SD_Card") {
                "SD Card".to_string()
            } else if info.contains("USB") {
                "USB Drive".to_string()
            } else if info.contains("Disk Image") {
                "Disk Image".to_string()
            } else if info.contains("External") {
                "External Drive".to_string()
            } else if info.contains("Internal") {
                "Internal Drive".to_string()
            } else {
                "Unknown".to_string()
            };

            // Check if removable
            let is_removable = info.contains("Removable Media: Yes")
                || info.contains("Ejectable: Yes")
                || device_type == "SD Card"
                || device_type == "USB Drive";

            return (device_type, is_removable);
        }
    }

    // Fallback for non-macOS or if diskutil fails
    ("Unknown".to_string(), true)
}
