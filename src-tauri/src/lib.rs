mod capture;
mod commands;
mod history;
mod hotkey;
mod ocr;
mod pin;
mod tray;

use tauri::{Emitter, Listener, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tray::setup_tray(app)?;
            hotkey::register_hotkeys(app.handle())?;

            // Initialize history database
            let db_path = app.path().app_data_dir().unwrap().join("history.db");
            std::fs::create_dir_all(db_path.parent().unwrap()).ok();
            let db = history::HistoryDB::new(&db_path)?;
            app.manage(db);

            // Listen for hotkey events
            let app_handle = app.handle().clone();
            app.listen("hotkey", move |event| {
                let payload = event.payload().to_string();

                if payload.contains("capture_region") {
                    // Region capture: take screenshot, open overlay for selection
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                if let Err(e) = commands::clipboard::open_screenshot_overlay(app.clone()) {
                                    log::error!("Failed to open overlay: {}", e);
                                    return;
                                }
                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                let _ = app.emit("screenshot-ready", result.image_base64);
                            }
                            Err(e) => log::error!("Capture failed: {}", e),
                        }
                    });
                } else if payload.contains("capture_fullscreen") {
                    // Fullscreen capture: take screenshot, open annotate editor directly
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                if let Err(e) = commands::clipboard::open_annotate_window(app.clone(), result.image_base64) {
                                    log::error!("Failed to open annotate window: {}", e);
                                }
                            }
                            Err(e) => log::error!("Capture failed: {}", e),
                        }
                    });
                } else if payload.contains("capture_window") {
                    // Window capture: same as fullscreen for now (future: capture specific window)
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                if let Err(e) = commands::clipboard::open_annotate_window(app.clone(), result.image_base64) {
                                    log::error!("Failed to open annotate window: {}", e);
                                }
                            }
                            Err(e) => log::error!("Capture failed: {}", e),
                        }
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture::get_screens,
            commands::capture::capture_fullscreen,
            commands::capture::capture_region,
            commands::capture::get_windows,
            commands::capture::get_window_at_point,
            commands::hotkey::get_hotkeys,
            commands::clipboard::crop_image,
            commands::clipboard::copy_image_to_clipboard,
            commands::clipboard::open_screenshot_overlay,
            commands::clipboard::open_annotate_window,
            commands::save::save_image_to_file,
            ocr::ocr_image,
            pin::create_pin_window,
            pin::close_pin_window,
            pin::resize_pin_window,
            history::save_to_history,
            history::get_history,
            history::delete_history_item,
            history::clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CapPix");
}
