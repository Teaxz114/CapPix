use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::sync::Mutex;
use tauri::Manager;

/// Pending screenshot data for the overlay window to pick up
pub struct PendingScreenshot(pub Mutex<Option<String>>);

/// Pending annotate image data for the annotate window to pick up
pub struct PendingAnnotateImage(pub Mutex<Option<String>>);

#[tauri::command]
pub fn get_pending_screenshot(state: tauri::State<PendingScreenshot>) -> Result<Option<String>, String> {
    let mut data = state.0.lock().map_err(|e| e.to_string())?;
    Ok(data.take())
}

#[tauri::command]
pub fn get_pending_annotate_image(state: tauri::State<PendingAnnotateImage>) -> Result<Option<String>, String> {
    let mut data = state.0.lock().map_err(|e| e.to_string())?;
    Ok(data.take())
}

#[tauri::command]
pub async fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    unsafe {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::System::DataExchange::*;
        use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
        use windows::Win32::System::Ole::CF_DIB;
        use windows::Win32::Foundation::HANDLE;

        OpenClipboard(None).map_err(|e| format!("Failed to open clipboard: {}", e))?;
        let _ = EmptyClipboard();

        // Create DIB for clipboard
        let bmi_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let pixel_data_size = (width * height * 4) as usize;
        let total_size = bmi_size + pixel_data_size;

        let h_mem = GlobalAlloc(GMEM_MOVEABLE, total_size)
            .map_err(|e| {
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
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&buf))
}

#[tauri::command]
pub fn open_screenshot_overlay(app: tauri::AppHandle) -> Result<(), String> {
    // Reuse the main window instead of creating a new webview window.
    // Creating new windows via WebviewWindowBuilder causes Edge to open because
    // the URL resolution (both App and CustomProtocol variants) fails to properly
    // register with Tauri's custom protocol handler in the new webview.
    // The main window is already correctly initialized with Tauri's protocol.
    let window = app.get_webview_window("main")
        .ok_or("Main window not found")?;

    // Transform main window into fullscreen overlay mode
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor")?;
    let pos = monitor.position();
    let size = monitor.size();

    let _ = window.set_decorations(false);
    let _ = window.set_always_on_top(true);
    let _ = window.set_size(tauri::LogicalSize::new(size.width as f64, size.height as f64));
    let _ = window.set_position(tauri::LogicalPosition::new(pos.x as f64, pos.y as f64));
    let _ = window.show();
    let _ = window.set_focus();

    // Navigate to screenshot route via JS (main window already has Vue Router)
    let _ = window.eval("window.location.hash = '/screenshot'");

    Ok(())
}

#[tauri::command]
pub fn trigger_capture(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    match mode.as_str() {
        "capture_region" | "capture_window" => {
            match crate::capture::screen::capture_screen(0) {
                Ok(result) => {
                    if let Some(state) = app.try_state::<PendingScreenshot>() {
                        if let Ok(mut data) = state.0.lock() {
                            *data = Some(result.image_base64.clone());
                        }
                    }
                    open_screenshot_overlay(app)?;
                }
                Err(e) => log::error!("Capture failed: {}", e),
            }
        }
        "capture_fullscreen" => {
            match crate::capture::screen::capture_screen(0) {
                Ok(result) => {
                    open_annotate_window(app, result.image_base64)?;
                }
                Err(e) => log::error!("Capture failed: {}", e),
            }
        }
        _ => return Err(format!("Unknown capture mode: {}", mode)),
    }
    Ok(())
}

#[tauri::command]
pub fn open_annotate_window(app: tauri::AppHandle, image_base64: String) -> Result<(), String> {
    // Reuse the main window — same reason as open_screenshot_overlay
    let window = app.get_webview_window("main")
        .ok_or("Main window not found")?;

    // Restore window to normal mode (in case coming from screenshot overlay)
    let _ = window.set_decorations(true);
    let _ = window.set_always_on_top(false);
    let _ = window.set_resizable(true);
    let _ = window.set_size(tauri::LogicalSize::new(1200.0, 800.0));
    let _ = window.center();
    let _ = window.show();
    let _ = window.set_focus();

    // Store image data for the frontend to pick up
    if let Some(state) = app.try_state::<PendingAnnotateImage>() {
        if let Ok(mut data) = state.0.lock() {
            *data = Some(image_base64);
        }
    }

    // Navigate to annotate route
    let _ = window.eval("window.location.hash = '/annotate'");

    Ok(())
}
