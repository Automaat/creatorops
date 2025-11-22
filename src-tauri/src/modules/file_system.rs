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
/// This function assumes the standard CreatorOps project structure:
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
            "{} directory not found. Expected RAW/{} subdirectory.",
            subfolder, subfolder
        ));
    }

    let media_path_str = media_path
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())?;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg(app_name)
            .arg(media_path_str)
            .spawn()
            .map_err(|e| format!("Failed to open in {}: {}", app_name, e))?;
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
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
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
            return Err("Failed to get parent directory".to_string());
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
