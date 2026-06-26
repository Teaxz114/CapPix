mod capture;
mod commands;
mod history;
mod hotkey;
mod ocr;
mod pin;
mod recording;
mod tray;

use commands::history::HistoryState;
use commands::save::SaveSeqState;
use std::sync::Mutex;
use tauri::{Listener, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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

            // Initialize save sequence state
            app.manage(SaveSeqState { seq: Mutex::new(0) });

            // Initialize pending screenshot store
            app.manage(commands::clipboard::PendingScreenshot(Mutex::new(None)));
            // Initialize pending annotate image store
            app.manage(commands::clipboard::PendingAnnotateImage(Mutex::new(None)));

            // Restore pinned windows from database
            {
                let state = app.state::<HistoryState>();
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let pins = db.list_pins().map_err(|e| e.to_string())?;

                for pin_record in &pins {
                    let image_path = std::path::PathBuf::from(&pin_record.image_path);
                    if !image_path.exists() {
                        // Image file no longer exists, remove the pin record
                        let _ = db.delete_pin(&pin_record.id);
                    }
                }
                drop(db); // Release lock before creating windows

                for pin_record in pins {
                    let image_path = std::path::PathBuf::from(&pin_record.image_path);
                    if image_path.exists() {
                        if let Ok(image_data) = std::fs::read(&image_path) {
                            use base64::Engine;
                            use base64::engine::general_purpose::STANDARD;
                            let image_base64 = STANDARD.encode(&image_data);
                            if let Ok(window_id) = crate::pin::create_pin_window_at(
                                app.handle().clone(),
                                image_base64,
                                pin_record.x,
                                pin_record.y,
                                pin_record.width,
                                pin_record.height,
                            ) {
                                log::info!("Restored pin: {} at ({}, {})", window_id, pin_record.x, pin_record.y);
                            }
                        }
                    }
                }
            }

            // Listen for hotkey events
            let app_handle = app.handle().clone();
            app.listen("hotkey", move |event| {
                let payload = event.payload().to_string();
                log::info!("Hotkey event received: {}", payload);

                if payload.contains("capture_region") {
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Hide main window BEFORE capturing — otherwise we capture ourselves
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.hide();
                        }
                        // Pump the Windows message loop to ensure the hide is processed.
                        // win.hide() dispatches to the main thread, but from a Tokio worker
                        // the actual SW_HIDE may not execute until the main thread processes it.
                        // We need to wait long enough for: dispatch → main thread processes →
                        // DWM recomposites the desktop (one vsync ~16ms).
                        //
                        // 200ms gives ample time for the hide to take effect and the DWM to
                        // recompose the desktop without the CapPix window.
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                // Store screenshot data for overlay to pick up
                                if let Some(state) = app.try_state::<commands::clipboard::PendingScreenshot>() {
                                    if let Ok(mut data) = state.0.lock() {
                                        *data = Some(result.image_base64.clone());
                                    }
                                }
                                if let Err(e) = commands::clipboard::open_screenshot_overlay(app.clone()) {
                                    log::error!("Failed to open overlay: {}", e);
                                    return;
                                }
                            }
                            Err(e) => {
                                log::error!("Capture failed: {}", e);
                                // Restore window on failure
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                }
                            }
                        }
                    });
                } else if payload.contains("capture_fullscreen") {
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Hide main window before capturing
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.hide();
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                if let Err(e) = commands::clipboard::open_annotate_window(app.clone(), result.image_base64) {
                                    log::error!("Failed to open annotate window: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Capture failed: {}", e);
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                }
                            }
                        }
                    });
                } else if payload.contains("capture_window") {
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Hide main window before capturing
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.hide();
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                        match crate::capture::screen::capture_screen(0) {
                            Ok(result) => {
                                // Store screenshot data for overlay to pick up
                                if let Some(state) = app.try_state::<commands::clipboard::PendingScreenshot>() {
                                    if let Ok(mut data) = state.0.lock() {
                                        *data = Some(result.image_base64.clone());
                                    }
                                }
                                if let Err(e) = commands::clipboard::open_screenshot_overlay(app.clone()) {
                                    log::error!("Failed to open overlay: {}", e);
                                    return;
                                }
                            }
                            Err(e) => {
                                log::error!("Capture failed: {}", e);
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                }
                            }
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
            commands::hotkey::set_hotkey,
            commands::hotkey::toggle_game_mode,
            commands::hotkey::get_game_mode,
            commands::clipboard::crop_image,
            commands::clipboard::copy_image_to_clipboard,
            commands::clipboard::open_screenshot_overlay,
            commands::clipboard::open_annotate_window,
            commands::clipboard::get_pending_screenshot,
            commands::clipboard::get_pending_annotate_image,
            commands::clipboard::trigger_capture,
            commands::save::save_image_to_file,
            commands::save::save_image_to_path,
            commands::save::prepare_save_path,
            commands::color::pick_color_at_point,
            commands::history::history_save,
            commands::history::history_list,
            commands::history::history_search,
            commands::history::history_delete,
            commands::history::history_count,
            commands::history::history_clear,
            commands::history::get_screenshot_image,
            commands::history::pin_save,
            commands::history::pin_list,
            commands::history::pin_delete,
            commands::history::pin_update_position,
            ocr::ocr_image,
            ocr::ocr_translate,
            pin::create_pin_window,
            pin::close_pin_window,
            pin::resize_pin_window,
            pin::set_pin_opacity,
            pin::set_pin_clickthrough,
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
