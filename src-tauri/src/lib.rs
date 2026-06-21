mod capture;
mod commands;
mod hotkey;
mod tray;

use tauri::{Emitter, Listener, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            tray::setup_tray(app)?;
            hotkey::register_hotkeys(app.handle())?;

            // Listen for hotkey events to handle screenshot overlay
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
                                // Small delay to let window load
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
            commands::hotkey::get_hotkeys,
            commands::clipboard::crop_image,
            commands::clipboard::copy_image_to_clipboard,
            commands::clipboard::open_screenshot_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CapPix");
}
