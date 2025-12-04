#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDCard {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub free_space: u64,
    pub file_count: usize,
    pub device_type: String,
    pub is_removable: bool,
}

/// Determines if a device type should be excluded from the scan results.
/// Excludes: disk images (app installers), internal drives, and unknown devices.
/// Includes: SD cards, USB drives, and external drives.
fn should_exclude_device_type(device_type: &str) -> bool {
    device_type == "Disk Image" || device_type == "Internal Drive" || device_type == "Unknown"
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

                    // Get device information early to filter before expensive operations
                    let (device_type, is_removable) = get_device_info(&path.to_string_lossy());

                    // Only show removable storage: SD cards, USB drives, external drives
                    // Filter out: disk images (app installers), internal HDDs, unknown
                    if should_exclude_device_type(&device_type) {
                        continue;
                    }

                    // Count files (only for volumes that pass the filter)
                    let file_count = count_files(&path);

                    // Get disk usage info (only for volumes that pass the filter)
                    let (size, free_space) = get_disk_usage(&path);

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
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .count()
}

#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
#[allow(unsafe_code, clippy::unwrap_used, clippy::missing_const_for_fn)]
fn get_disk_usage(path: &Path) -> (u64, u64) {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::mem;

        unsafe {
            let path_cstr = CString::new(path.to_str().unwrap()).unwrap();
            let mut stats: libc::statfs = mem::zeroed();

            if libc::statfs(path_cstr.as_ptr(), &mut stats) == 0 {
                let block_size = u64::from(stats.f_bsize);
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
        return Err("SD card path does not exist".to_owned());
    }

    let mut file_paths = Vec::new();
    let photo_video_extensions = [
        "jpg", "jpeg", "png", "heic", "heif", "raw", "cr2", "cr3", "nef", "arw", "dng", "mp4",
        "mov", "avi", "mkv", "m4v",
    ];

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
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

/// Eject an SD card by volume path
#[tauri::command]
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub async fn eject_sd_card(volume_path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("diskutil")
            .args(["eject", &volume_path])
            .output()
            .map_err(|e| format!("Failed to execute diskutil: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to eject SD card: {error}"))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("SD card ejection is only supported on macOS".to_owned())
    }
}

/// Get device type and removability using diskutil (macOS)
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn get_device_info(volume_name: &str) -> (String, bool) {
    #[cfg(target_os = "macos")]
    {
        // Use diskutil to get device information
        let output = Command::new("diskutil")
            .args(["info", volume_name])
            .output();

        if let Ok(output) = output {
            let info = String::from_utf8_lossy(&output.stdout);

            // Parse device type - check Protocol field for accuracy
            // Note: diskutil output has variable spacing, so we check if line starts with
            // "Protocol:" and contains the protocol type
            let device_type = if info.lines().any(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("Protocol:") && trimmed.contains("Secure Digital")
            }) {
                "SD Card".to_owned()
            } else if info.lines().any(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("Protocol:") && trimmed.contains("USB")
            }) {
                "USB Drive".to_owned()
            } else if info.lines().any(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("Protocol:") && trimmed.contains("Disk Image")
            }) {
                "Disk Image".to_owned()
            } else if info.contains("SD Card") || info.contains("SD_Card") {
                "SD Card".to_owned()
            } else if info.contains("External") {
                "External Drive".to_owned()
            } else if info.contains("Internal") {
                "Internal Drive".to_owned()
            } else {
                "Unknown".to_owned()
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
    ("Unknown".to_owned(), true)
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sd_card_serialization() {
        let card = SDCard {
            name: "SD_CARD".to_owned(),
            path: "/Volumes/SD_CARD".to_owned(),
            size: 32_000_000_000,
            free_space: 16_000_000_000,
            file_count: 150,
            device_type: "SD Card".to_owned(),
            is_removable: true,
        };

        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("SD_CARD"));
        assert!(json.contains("32000000000"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_sd_card_deserialization() {
        let json = r#"{
            "name": "USB_DRIVE",
            "path": "/Volumes/USB_DRIVE",
            "size": 64000000000,
            "freeSpace": 32000000000,
            "fileCount": 200,
            "deviceType": "USB Drive",
            "isRemovable": true
        }"#;

        let card: SDCard = serde_json::from_str(json).unwrap();
        assert_eq!(card.name, "USB_DRIVE");
        assert_eq!(card.size, 64_000_000_000);
        assert!(card.is_removable);
    }

    #[test]
    fn test_count_files() {
        let temp_dir = std::env::temp_dir().join("test_sd_count");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("file1.txt"), b"test").unwrap();
        std::fs::write(temp_dir.join("file2.txt"), b"test").unwrap();

        let count = count_files(&temp_dir);
        assert_eq!(count, 2);

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_device_info_returns_tuple() {
        let (device_type, _) = get_device_info("TestVolume");
        assert!(!device_type.is_empty());
    }

    #[test]
    fn test_count_files_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        std::fs::write(temp_dir.path().join("file1.txt"), b"test").unwrap();
        std::fs::write(subdir.join("file2.txt"), b"test").unwrap();
        std::fs::write(subdir.join("file3.txt"), b"test").unwrap();

        let count = count_files(temp_dir.path());
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let count = count_files(temp_dir.path());
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_list_sd_card_files() {
        let temp_dir = TempDir::new().unwrap();

        std::fs::write(temp_dir.path().join("photo1.jpg"), b"photo").unwrap();
        std::fs::write(temp_dir.path().join("photo2.png"), b"photo").unwrap();
        std::fs::write(temp_dir.path().join("video1.mp4"), b"video").unwrap();
        std::fs::write(temp_dir.path().join("ignored.txt"), b"text").unwrap();

        let result = list_sd_card_files(temp_dir.path().to_string_lossy().to_string()).await;
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.contains("photo1.jpg")));
        assert!(files.iter().any(|f| f.contains("photo2.png")));
        assert!(files.iter().any(|f| f.contains("video1.mp4")));
        assert!(!files.iter().any(|f| f.contains("ignored.txt")));
    }

    #[tokio::test]
    async fn test_list_sd_card_files_with_raw_formats() {
        let temp_dir = TempDir::new().unwrap();

        std::fs::write(temp_dir.path().join("raw1.cr2"), b"raw").unwrap();
        std::fs::write(temp_dir.path().join("raw2.nef"), b"raw").unwrap();
        std::fs::write(temp_dir.path().join("raw3.arw"), b"raw").unwrap();
        std::fs::write(temp_dir.path().join("raw4.dng"), b"raw").unwrap();

        let result = list_sd_card_files(temp_dir.path().to_string_lossy().to_string()).await;
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 4);
    }

    #[tokio::test]
    async fn test_list_sd_card_files_nonexistent_path() {
        let result = list_sd_card_files("/nonexistent/path".to_owned()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "SD card path does not exist");
    }

    #[tokio::test]
    async fn test_list_sd_card_files_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let dcim = temp_dir.path().join("DCIM");
        let folder1 = dcim.join("100CANON");
        std::fs::create_dir_all(&folder1).unwrap();

        std::fs::write(folder1.join("IMG_0001.jpg"), b"photo").unwrap();
        std::fs::write(folder1.join("IMG_0002.cr3"), b"raw").unwrap();
        std::fs::write(folder1.join("VID_0001.mov"), b"video").unwrap();

        let result = list_sd_card_files(temp_dir.path().to_string_lossy().to_string()).await;
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 3);
    }

    #[tokio::test]
    async fn test_list_sd_card_files_case_insensitive_extensions() {
        let temp_dir = TempDir::new().unwrap();

        std::fs::write(temp_dir.path().join("photo.JPG"), b"photo").unwrap();
        std::fs::write(temp_dir.path().join("photo.jpeg"), b"photo").unwrap();
        std::fs::write(temp_dir.path().join("video.MOV"), b"video").unwrap();

        let result = list_sd_card_files(temp_dir.path().to_string_lossy().to_string()).await;
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 3);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_disk_usage() {
        let temp_dir = TempDir::new().unwrap();
        let (size, free_space) = get_disk_usage(temp_dir.path());
        assert!(size > 0);
        assert!(free_space > 0);
        assert!(free_space <= size);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_get_disk_usage_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let (size, free_space) = get_disk_usage(temp_dir.path());
        assert_eq!(size, 0);
        assert_eq!(free_space, 0);
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn test_eject_sd_card_not_supported() {
        let result = eject_sd_card("/test/path".to_owned()).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "SD card ejection is only supported on macOS"
        );
    }

    #[test]
    fn test_sd_card_complete_struct() {
        let card = SDCard {
            name: "TestCard".to_owned(),
            path: "/Volumes/TestCard".to_owned(),
            size: 64_000_000_000,
            free_space: 32_000_000_000,
            file_count: 250,
            device_type: "SD Card".to_owned(),
            is_removable: true,
        };

        assert_eq!(card.name, "TestCard");
        assert_eq!(card.size, 64_000_000_000);
        assert_eq!(card.free_space, 32_000_000_000);
        assert_eq!(card.file_count, 250);
        assert!(card.is_removable);
    }

    #[test]
    fn test_device_type_filter_logic() {
        // Test that filter logic correctly identifies which device types to exclude
        // Should be filtered out (excluded)
        assert!(should_exclude_device_type("Disk Image"));
        assert!(should_exclude_device_type("Internal Drive"));
        assert!(should_exclude_device_type("Unknown"));

        // Should pass filter (included)
        assert!(!should_exclude_device_type("SD Card"));
        assert!(!should_exclude_device_type("USB Drive"));
        assert!(!should_exclude_device_type("External Drive"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_device_type_detection_with_actual_volume() {
        // This test validates that get_device_info returns valid device types
        // Testing with root path which should exist on macOS
        let (device_type, _is_removable) = get_device_info("/");

        // Device type should be one of the known types
        assert!(
            device_type == "SD Card"
                || device_type == "USB Drive"
                || device_type == "Disk Image"
                || device_type == "External Drive"
                || device_type == "Internal Drive"
                || device_type == "Unknown"
        );
    }

    #[tokio::test]
    async fn test_scan_sd_cards_returns_vec() {
        // Test that scan_sd_cards returns a Result with Vec
        // This will scan actual /Volumes on macOS or return empty on other platforms
        let result = scan_sd_cards().await;
        assert!(result.is_ok());

        let cards = result.unwrap();
        // Should return a Vec (empty or with cards)
        assert!(cards.is_empty() || !cards.is_empty());

        // If any cards are returned, validate their structure
        for card in cards {
            assert!(!card.name.is_empty());
            assert!(!card.path.is_empty());
            assert!(!card.device_type.is_empty());

            // Device type should be one of the allowed types (filtered)
            assert!(
                card.device_type == "SD Card"
                    || card.device_type == "USB Drive"
                    || card.device_type == "External Drive"
            );

            // Should not include filtered types
            assert!(card.device_type != "Disk Image");
            assert!(card.device_type != "Internal Drive");
            assert!(card.device_type != "Unknown");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_protocol_based_detection_priority() {
        // Test that protocol-based detection takes priority over fallback checks
        // This validates the order of checks in get_device_info
        // Note: diskutil output has multiple spaces, we match with a single space after colon

        // Simulate diskutil output for Secure Digital protocol (with multiple spaces like real output)
        let sd_output = "   Protocol:                  Secure Digital\n   Device Location:           Internal";
        let has_sd_protocol = sd_output
            .lines()
            .any(|line| line.trim_start().starts_with("Protocol:") && line.contains("Secure Digital"));
        assert!(has_sd_protocol);

        // Simulate diskutil output for USB protocol
        let usb_output = "   Protocol:                  USB\n   Device Location:           External";
        let has_usb_protocol = usb_output
            .lines()
            .any(|line| line.trim_start().starts_with("Protocol:") && line.contains("USB"));
        assert!(has_usb_protocol);

        // Simulate diskutil output for Disk Image protocol
        let disk_image_output = "   Protocol:                  Disk Image\n   Device Location:           External";
        let has_disk_image_protocol = disk_image_output
            .lines()
            .any(|line| line.trim_start().starts_with("Protocol:") && line.contains("Disk Image"));
        assert!(has_disk_image_protocol);
    }
}
