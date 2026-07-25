mod commands;
mod db;
mod download;
mod error;
mod formats;
mod models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = commands::init_state(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_library_roots,
            commands::add_library_root,
            commands::remove_library_root,
            commands::list_books,
            commands::scan_library,
            commands::import_files,
            commands::ingest_paths,
            commands::get_launch_paths,
            commands::open_book,
            commands::import_from_url,
            commands::get_cover_data_url,
            commands::save_progress,
            commands::get_progress,
            commands::list_bookmarks,
            commands::add_bookmark,
            commands::remove_bookmark,
            commands::get_reader_settings,
            commands::save_reader_settings,
            commands::get_supported_formats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
