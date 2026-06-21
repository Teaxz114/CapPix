mod capture;
mod commands;
mod hotkey;
mod tray;

use tauri::Emitter;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            tray::setup_tray(app)?;
            hotkey::register_hotkeys(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture::get_screens,
            commands::capture::capture_fullscreen,
            commands::capture::capture_region,
            commands::capture::get_windows,
            commands::hotkey::get_hotkeys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CapPix");
}
