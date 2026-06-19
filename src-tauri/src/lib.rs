mod commands;
mod page;
mod pack;
mod render;
mod shape;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_image_path,
            commands::load_image_bytes,
            commands::pack,
            commands::export_png,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
