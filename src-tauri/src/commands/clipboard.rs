use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Pending screenshot data for the overlay window to pick up
pub struct PendingScreenshot(pub Mutex<Option<String>>);

/// Pending annotate image data for the annotate window to pick up
pub struct PendingAnnotateImage(pub Mutex<Option<String>>);

#[tauri::command]
pub fn get_pending_screenshot(
    state: tauri::State<PendingScreenshot>,
) -> Result<Option<String>, String> {
    let mut data = state.0.lock().map_err(|e| e.to_string())?;
    Ok(data.take())
}

#[tauri::command]
pub fn get_pending_annotate_image(
    state: tauri::State<PendingAnnotateImage>,
) -> Result<Option<String>, String> {
    let mut data = state.0.lock().map_err(|e| e.to_string())?;
    Ok(data.take())
}

#[tauri::command]
pub fn dismiss_screenshot_overlay(
    app: tauri::AppHandle,
    pending: tauri::State<PendingScreenshot>,
) -> Result<(), String> {
    log::info!("dismiss_screenshot_overlay called");

    // Drop stale screenshot data so the next capture starts cleanly.
    if let Ok(mut data) = pending.0.lock() {
        *data = None;
    }

    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Hide immediately: cancel should exit screenshot mode, not show any app page.
    let _ = window.hide();

    restore_normal_main_window(&window);

    Ok(())
}

/// Leave the temporary screenshot-overlay state. Keep this in one place so a
/// completed capture cannot remain borderless, always-on-top, or absent from
/// the taskbar when it becomes the normal annotation window.
fn restore_normal_main_window(window: &tauri::WebviewWindow) {
    let _ = window.set_decorations(true);
    let _ = window.set_resizable(true);
    let _ = window.set_skip_taskbar(false);
    let _ = window.set_size(tauri::LogicalSize::new(1200.0, 800.0));
    // Apply HWND_NOTOPMOST after restoring visible styles. On Windows,
    // SetWindowPos can be ignored while the HWND is hidden or borderless.
    let _ = window.set_always_on_top(false);
}

#[tauri::command]
pub fn restore_normal_window_state(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    restore_normal_main_window(&window);
    Ok(())
}

#[tauri::command]
pub async fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    unsafe {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::System::DataExchange::*;
        use windows::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
        };
        use windows::Win32::System::Ole::CF_DIB;

        OpenClipboard(None).map_err(|e| format!("Failed to open clipboard: {}", e))?;
        let _ = EmptyClipboard();

        // Create DIB for clipboard
        let bmi_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let pixel_data_size = (width * height * 4) as usize;
        let total_size = bmi_size + pixel_data_size;

        let h_mem = GlobalAlloc(GMEM_MOVEABLE, total_size).map_err(|e| {
            let _ = CloseClipboard();
            format!("GlobalAlloc failed: {}", e)
        })?;

        let p_mem = GlobalLock(h_mem);
        if p_mem.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_string());
        }

        // Write BITMAPINFOHEADER
        let header = BITMAPINFOHEADER {
            biSize: bmi_size as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: pixel_data_size as u32,
            ..Default::default()
        };
        std::ptr::write(p_mem as *mut BITMAPINFOHEADER, header);

        // Write pixel data (RGBA -> BGRA for clipboard)
        let pixels = rgba.as_raw();
        let p_pixels = (p_mem as *mut u8).add(bmi_size);
        for i in 0..(width * height) as usize {
            *p_pixels.add(i * 4) = pixels[i * 4 + 2]; // B
            *p_pixels.add(i * 4 + 1) = pixels[i * 4 + 1]; // G
            *p_pixels.add(i * 4 + 2) = pixels[i * 4]; // R
            *p_pixels.add(i * 4 + 3) = pixels[i * 4 + 3]; // A
        }

        let _ = GlobalUnlock(h_mem);
        let _ = SetClipboardData(CF_DIB.0 as u32, HANDLE(h_mem.0));
        let _ = CloseClipboard();
    }

    Ok(())
}

#[tauri::command]
pub fn crop_image(
    image_base64: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let cropped = img.crop_imm(x, y, width, height);
    let mut buf = Vec::new();
    cropped
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&buf))
}

#[tauri::command]
pub fn open_screenshot_overlay(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("open_screenshot_overlay called");

    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // The overlay's (0, 0) must match the captured image's (0, 0): the
    // virtual desktop origin, not the primary monitor origin. This includes
    // displays placed left of or above the primary display.
    let virtual_screen = crate::capture::screen::virtual_screen_bounds();

    // Resize and reposition while still hidden (no visual artifacts).
    let _ = window.set_size(tauri::PhysicalSize::new(
        virtual_screen.width as u32,
        virtual_screen.height as u32,
    ));
    let _ = window.set_position(tauri::PhysicalPosition::new(
        virtual_screen.x,
        virtual_screen.y,
    ));

    // Remove window chrome (title bar + border) and stay on top for the overlay.
    // Without this the screenshot capture shows the app's own window frame.
    let _ = window.set_decorations(false);
    let _ = window.set_always_on_top(true);

    // Show the window FIRST — WebView2 may not execute JS or deliver events
    // while the window is hidden (SW_HIDE state).
    let _ = window.show();

    // Now navigate to screenshot route via Tauri event.
    // The frontend App.vue listens for "navigate" events and calls router.push().
    // Using emit instead of eval because eval with __vue_app__ is fragile.
    let _ = app.emit("navigate", "screenshot");

    let _ = window.set_focus();

    Ok(())
}

#[tauri::command]
pub async fn trigger_capture(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    log::info!("trigger_capture called with mode: {}", mode);

    // Hide main window BEFORE capturing — otherwise we capture ourselves.
    // CRITICAL: win.hide() posts a message to the window's message queue,
    // but it won't be processed until the main thread's message loop runs.
    // Since this is now an async command (runs on Tokio worker thread),
    // the main thread is free to process the SW_HIDE message during our sleep.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    // Wait for: main thread processes SW_HIDE → DWM recomposites desktop (1 vsync ~16ms)
    // 200ms gives ample margin for the hide to fully take effect.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    match mode.as_str() {
        "capture_region" | "capture_window" => {
            match crate::capture::screen::capture_virtual_screen() {
                Ok(result) => {
                    if let Some(state) = app.try_state::<PendingScreenshot>() {
                        if let Ok(mut data) = state.0.lock() {
                            *data = Some(result.image_base64.clone());
                        }
                    }
                    open_screenshot_overlay(app)?;
                }
                Err(e) => {
                    log::error!("Capture failed: {}", e);
                    // Restore window on failure
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                    }
                }
            }
        }
        "capture_fullscreen" => match crate::capture::screen::capture_virtual_screen() {
            Ok(result) => {
                open_annotate_window(app, result.image_base64)?;
            }
            Err(e) => {
                log::error!("Capture failed: {}", e);
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                }
            }
        },
        _ => return Err(format!("Unknown capture mode: {}", mode)),
    }
    Ok(())
}

#[tauri::command]
pub fn open_annotate_window(app: tauri::AppHandle, image_base64: String) -> Result<(), String> {
    eprintln!(
        "[open_annotate_window] called, image_base64 length: {}",
        image_base64.len()
    );

    // Close screenshot overlay if it exists (from dedicated window attempt)
    if let Some(overlay) = app.get_webview_window("screenshot-overlay") {
        let _ = overlay.close();
    }

    // Store image data for the frontend to pick up
    if let Some(state) = app.try_state::<PendingAnnotateImage>() {
        if let Ok(mut data) = state.0.lock() {
            *data = Some(image_base64);
            eprintln!("[open_annotate_window] PendingAnnotateImage stored");
        }
    } else {
        eprintln!("[open_annotate_window] ERROR: PendingAnnotateImage state not found!");
    }

    // Reuse main window for annotate — restore to normal state
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    eprintln!(
        "[open_annotate_window] main window found, current visible: {}",
        window.is_visible().unwrap_or(false)
    );

    let _ = window.show();
    restore_normal_main_window(&window);
    let _ = window.center();

    // Navigate via Tauri event (same pattern as open_screenshot_overlay)
    let _ = app.emit("navigate", "annotate");
    eprintln!("[open_annotate_window] navigate event emitted");

    let _ = window.set_focus();

    Ok(())
}
