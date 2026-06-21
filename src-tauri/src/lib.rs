mod capture;
mod commands;
mod history;
mod hotkey;
mod ocr;
mod pin;
mod recording;
mod tray;

use commands::history::HistoryState;
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
            let db = history::HistoryDb::new(&db_path.to_string_lossy()).map_err(|e| e.to_string())?;
            app.manage(HistoryState {
                db: std::sync::Mutex::new(db),
            });

            // Initialize recording manager
            app.manage(recording::RecordingManager::new());

            // Listen for hotkey events
            let app_handle = app.handle().clone();
            app.listen("hotkey", move |event| {
                let payload = event.payload().to_string();

                if payload.contains("capture_region") {
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
            commands::color::pick_color_at_point,
            commands::history::history_save,
            commands::history::history_list,
            commands::history::history_search,
            commands::history::history_delete,
            commands::history::history_count,
            ocr::ocr_image,
            pin::create_pin_window,
            pin::close_pin_window,
            pin::resize_pin_window,
            recording::start_recording,
            recording::stop_recording,
            recording::get_recording_state,
            recording::pause_recording,
            recording::resume_recording,
            recording::record_to_gif,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CapPix");
}
