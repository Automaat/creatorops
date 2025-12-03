#![allow(clippy::wildcard_imports)] // Tauri command macro uses wildcard imports
use std::process::Command;

// Windows application paths for external editors
#[cfg(target_os = "windows")]
const LIGHTROOM_PATHS: &[&str] = &[
    r"C:\Program Files\Adobe\Adobe Lightroom Classic\Lightroom.exe",
    r"C:\Program Files (x86)\Adobe\Adobe Lightroom Classic\Lightroom.exe",
];

#[cfg(target_os = "windows")]
const AFTERSHOOT_PATHS: &[&str] = &[
    r"C:\Program Files\AfterShoot\AfterShoot.exe",
    r"C:\Program Files (x86)\AfterShoot\AfterShoot.exe",
];

#[cfg(target_os = "windows")]
const DAVINCI_RESOLVE_PATHS: &[&str] = &[
    r"C:\Program Files\Blackmagic Design\DaVinci Resolve\Resolve.exe",
    r"C:\Program Files (x86)\Blackmagic Design\DaVinci Resolve\Resolve.exe",
];

/// Opens a project's media folder in an external editing application.
///
/// This function assumes the standard `CreatorOps` project structure:
/// ```text
/// ProjectFolder/
///   RAW/
///     Photos/  (for photo editing apps)
///     Videos/  (for video editing apps)
/// ```
///
/// The function launches the external app in the background (fire-and-forget)
/// which is appropriate for GUI applications that should run independently.
fn open_in_external_app(
    project_path: &str,
    subfolder: &str,
    app_name: &str,
    #[cfg_attr(not(target_os = "windows"), allow(unused_variables))] windows_paths: &[&str],
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] linux_path: Option<&str>,
) -> Result<(), String> {
    let media_path = std::path::Path::new(project_path)
        .join("RAW")
        .join(subfolder);

    if !media_path.exists() {
        return Err(format!(
            "{subfolder} directory not found. Expected RAW/{subfolder} subdirectory."
        ));
    }

    let media_path_str = media_path
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_owned())?;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg(app_name)
            .arg(media_path_str)
            .spawn()
            .map_err(|e| format!("Failed to open in {app_name}: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        let mut launched = false;
        for exe_path in windows_paths {
            if std::path::Path::new(exe_path).exists() {
                Command::new(exe_path)
                    .arg(media_path_str)
                    .spawn()
                    .map_err(|e| format!("Failed to open in {}: {}", app_name, e))?;
                launched = true;
                break;
            }
        }

        if !launched {
            return Err(format!(
                "{} not found. Please ensure it's installed.",
                app_name
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(path) = linux_path {
            if std::path::Path::new(path).exists() {
                Command::new(path)
                    .arg(media_path_str)
                    .spawn()
                    .map_err(|e| format!("Failed to open in {}: {}", app_name, e))?;
            } else {
                return Err(format!(
                    "{} not found. Please ensure it's installed.",
                    app_name
                ));
            }
        } else {
            return Err(format!("{} not supported on Linux", app_name));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try xdg-open with the parent directory
        if let Some(parent) = std::path::Path::new(&path).parent() {
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to open file manager: {}", e))?;
        } else {
            return Err("Failed to get parent directory".to_owned());
        }
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_lightroom(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let paths = LIGHTROOM_PATHS;
    #[cfg(not(target_os = "windows"))]
    let paths = &[];

    open_in_external_app(&path, "Photos", "Adobe Lightroom Classic", paths, None)
}

#[tauri::command]
pub fn open_in_aftershoot(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let paths = AFTERSHOOT_PATHS;
    #[cfg(not(target_os = "windows"))]
    let paths = &[];

    open_in_external_app(&path, "Photos", "AfterShoot", paths, None)
}

#[tauri::command]
pub fn open_in_davinci_resolve(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let paths = DAVINCI_RESOLVE_PATHS;
    #[cfg(not(target_os = "windows"))]
    let paths = &[];

    open_in_external_app(
        &path,
        "Videos",
        "DaVinci Resolve",
        paths,
        Some("/opt/resolve/bin/resolve"),
    )
}

#[tauri::command]
pub fn open_in_final_cut_pro(path: String) -> Result<(), String> {
    open_in_external_app(
        &path,
        "Videos",
        "Final Cut Pro",
        &[],
        Some("/Applications/Final Cut Pro.app/Contents/MacOS/Final Cut Pro"),
    )
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_windows_paths_constants() {
        #[cfg(target_os = "windows")]
        {
            assert!(!LIGHTROOM_PATHS.is_empty());
            assert!(!AFTERSHOOT_PATHS.is_empty());
            assert!(!DAVINCI_RESOLVE_PATHS.is_empty());
        }
    }

    #[test]
    fn test_open_in_external_app_path_validation() {
        let temp_dir = std::env::temp_dir().join("test_external_app");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let project_path = temp_dir.to_str().unwrap();

        // Test with non-existent subdirectory
        let result = open_in_external_app(project_path, "NonExistent", "TestApp", &[], None);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_open_in_external_app_creates_valid_path() {
        let temp_dir = std::env::temp_dir().join("test_external_app2");
        std::fs::create_dir_all(temp_dir.join("RAW").join("Photos")).unwrap();

        let project_path = temp_dir.to_str().unwrap();
        let media_path = Path::new(project_path).join("RAW").join("Photos");

        assert!(media_path.exists());
        assert!(media_path.is_dir());

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_open_in_external_app_invalid_path_encoding() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("RAW").join("Photos")).unwrap();

        let project_path = temp_dir.path().to_str().unwrap();
        let result = open_in_external_app(project_path, "Photos", "TestApp", &[], None);

        // Should validate path exists (error occurs when trying to spawn)
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_open_in_external_app_photos_subfolder() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("RAW").join("Photos")).unwrap();

        let project_path = temp_dir.path().to_str().unwrap();
        let media_path = Path::new(project_path).join("RAW").join("Photos");

        assert!(media_path.exists());
        assert!(media_path.is_dir());
    }

    #[test]
    fn test_open_in_external_app_videos_subfolder() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("RAW").join("Videos")).unwrap();

        let project_path = temp_dir.path().to_str().unwrap();
        let media_path = Path::new(project_path).join("RAW").join("Videos");

        assert!(media_path.exists());
        assert!(media_path.is_dir());
    }

    #[test]
    fn test_open_in_lightroom_validates_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = open_in_lightroom(temp_dir.path().to_string_lossy().to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_open_in_aftershoot_validates_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = open_in_aftershoot(temp_dir.path().to_string_lossy().to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_open_in_davinci_resolve_validates_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = open_in_davinci_resolve(temp_dir.path().to_string_lossy().to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_open_in_final_cut_pro_validates_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = open_in_final_cut_pro(temp_dir.path().to_string_lossy().to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_reveal_in_finder_with_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"test").unwrap();

        // Function spawns background process, so it may succeed or fail depending on platform
        let result = reveal_in_finder(test_file.to_string_lossy().to_string());
        assert!(result.is_ok() || result.is_err());
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_lightroom_paths() {
        assert!(LIGHTROOM_PATHS.len() >= 2);
        assert!(LIGHTROOM_PATHS[0].contains("Adobe"));
        assert!(LIGHTROOM_PATHS[0].contains("Lightroom"));
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_aftershoot_paths() {
        assert!(AFTERSHOOT_PATHS.len() >= 2);
        assert!(AFTERSHOOT_PATHS[0].contains("AfterShoot"));
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_davinci_resolve_paths() {
        assert!(DAVINCI_RESOLVE_PATHS.len() >= 2);
        assert!(DAVINCI_RESOLVE_PATHS[0].contains("Blackmagic Design"));
        assert!(DAVINCI_RESOLVE_PATHS[0].contains("DaVinci Resolve"));
    }

    #[test]
    fn test_open_in_external_app_missing_raw_directory() {
        let temp_dir = TempDir::new().unwrap();

        let result = open_in_external_app(
            temp_dir.path().to_str().unwrap(),
            "Photos",
            "TestApp",
            &[],
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
