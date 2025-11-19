mod modules;

use modules::file_copy::copy_files;
use modules::project::create_project;
use modules::sd_card::scan_sd_cards;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_sd_cards,
            copy_files,
            create_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
