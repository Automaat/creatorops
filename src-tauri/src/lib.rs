//! `CreatorOps` - Photography workflow management application
//!
//! This is the core library for the `CreatorOps` Tauri application.

mod error;
mod modules;
mod state;

/// Result type for application-level operations
pub type AppResult = Result<(), Box<dyn std::error::Error>>;

use modules::archive::{create_archive, get_archive_queue, remove_archive_job, start_archive};
use modules::backup::{
    cancel_backup, get_backup_history, get_backup_queue, get_project_backup_history, queue_backup,
    remove_backup_job, start_backup,
};
use modules::delivery::{
    create_delivery, get_delivery_queue, list_project_files, remove_delivery_job, start_delivery,
};
use modules::file_copy::{cancel_import, copy_files};
use modules::file_system::{
    open_in_aftershoot, open_in_davinci_resolve, open_in_final_cut_pro, open_in_lightroom,
    reveal_in_finder,
};
use modules::file_utils::get_home_directory;
use modules::google_drive::{
    complete_google_drive_auth, get_google_drive_account, remove_google_drive_account,
    set_drive_parent_folder, start_google_drive_auth, test_google_drive_connection,
    upload_to_google_drive,
};
use modules::import_history::{
    get_import_history, get_project_import_history, save_import_history,
};
use modules::project::{
    create_project, delete_project, get_project, list_projects, refresh_projects,
    update_project_deadline, update_project_status,
};
use modules::sd_card::{eject_sd_card, list_sd_card_files, scan_sd_cards};

/// Run the Tauri application
///
/// # Errors
///
/// Returns error if database initialization or Tauri runtime fails
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::exit)] // Tauri's run() internally uses process::exit
pub fn run() -> AppResult {
    // Initialize logger (safe to call multiple times)
    let _ = env_logger::try_init();

    // Load .env file if present (for Google OAuth credentials in development)
    let _ = dotenvy::dotenv();

    // Initialize database with dependency injection
    let db =
        modules::db::Database::new().map_err(|e| format!("Failed to initialize database: {e}"))?;

    // Initialize application state
    let app_state = state::AppState::default();

    tauri::Builder::default()
        .manage(db)
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_sd_cards,
            list_sd_card_files,
            eject_sd_card,
            copy_files,
            cancel_import,
            create_project,
            list_projects,
            get_project,
            refresh_projects,
            update_project_status,
            update_project_deadline,
            delete_project,
            save_import_history,
            get_import_history,
            get_project_import_history,
            queue_backup,
            get_backup_queue,
            start_backup,
            cancel_backup,
            remove_backup_job,
            get_backup_history,
            get_project_backup_history,
            list_project_files,
            create_delivery,
            start_delivery,
            get_delivery_queue,
            remove_delivery_job,
            create_archive,
            start_archive,
            get_archive_queue,
            remove_archive_job,
            reveal_in_finder,
            open_in_lightroom,
            open_in_aftershoot,
            open_in_davinci_resolve,
            open_in_final_cut_pro,
            get_home_directory,
            start_google_drive_auth,
            complete_google_drive_auth,
            get_google_drive_account,
            set_drive_parent_folder,
            remove_google_drive_account,
            test_google_drive_connection,
            upload_to_google_drive,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
