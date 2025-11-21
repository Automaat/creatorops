mod modules;

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
    open_in_aftershoot, open_in_davinci_resolve, open_in_lightroom, reveal_in_finder,
};
use modules::import_history::{
    get_import_history, get_project_import_history, save_import_history,
};
use modules::project::{
    create_project, delete_project, list_projects, refresh_projects, update_project_deadline,
    update_project_status,
};
use modules::sd_card::{list_sd_card_files, scan_sd_cards};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_sd_cards,
            list_sd_card_files,
            copy_files,
            cancel_import,
            create_project,
            list_projects,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
