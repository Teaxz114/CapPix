use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
    WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

mod native_window;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PinWindow {
    pub id: String,
    pub image_base64: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[tauri::command]
pub fn create_pin_window(app: AppHandle, image_base64: String) -> Result<String, String> {
    let id = format!("pin-{}", uuid::Uuid::new_v4());

        // Decode base64 image and create a native Win32 pin window (no webview!)
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let image_data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Create native window at center of screen
        let monitor = app.primary_monitor().map_err(|e| e.to_string())?.ok_or("No monitor")?;
        let mon_size = monitor.size();
        let x = (mon_size.width as i32 - width as i32) / 2;
        let y = (mon_size.height as i32 - height as i32) / 2;

        let hwnd = native_window::NativePinWindow::create(
            rgba.as_raw(),
            width as i32,
            height as i32,
            x,
            y,
        )?;

        // Store hwnd in the pin record for future reference
        log::info!("Native pin window created: {} (HWND {:?})", id, hwnd);

        // Save to database for persistence
        if let Some(state) = app.try_state::<crate::commands::history::HistoryState>() {
            if let Ok(db) = state.db.lock() {
                let image_path = format!("pins/{}/image.png", id);
                let app_data = app.path().app_data_dir().unwrap();
                let full_path = app_data.join(&image_path);
                std::fs::create_dir_all(full_path.parent().unwrap()).ok();
                std::fs::write(&full_path, &image_data).ok();
                let _ = db.save_pin(&crate::history::PinRecord {
                    id: id.clone(),
                    image_path: full_path.to_string_lossy().to_string(),
                    x: x as f64,
                    y: y as f64,
                    width: width as f64,
                    height: height as f64,
                    opacity: 1.0,
                    topmost: true,
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }

        Ok(id)
    }

pub fn create_pin_window_at(
    app: AppHandle,
    image_base64: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<String, String> {
    let id = format!("pin-{}", uuid::Uuid::new_v4());
    use tauri::WebviewWindowBuilder;

    // Store pin data for PinView to pick up
    if let Some(state) = app.try_state::<crate::commands::clipboard::PendingScreenshot>() {
        if let Ok(mut data) = state.0.lock() {
            *data = Some(image_base64.clone());
        }
    }

    // Try creating a new webview window, fallback to main window
    match WebviewWindowBuilder::new(&app, &id, tauri::WebviewUrl::App("index.html".into()))
        .title("CapPix Pin")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .inner_size(width, height)
        .position(x, y)
        .resizable(true)
        .build()
    {
        Ok(window) => {
            let _ = window.eval(&format!("window.location.hash = '/pin?id={}'", id));
        }
        Err(e) => {
            log::warn!("Failed to create pin webview window at ({},{}): {}, using main window", x, y, e);
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.set_decorations(false);
                let _ = main_win.set_always_on_top(true);
                let _ = main_win.set_size(tauri::LogicalSize::new(width, height));
                let _ = main_win.set_position(tauri::LogicalPosition::new(x, y));
                let _ = main_win.show();
                let _ = main_win.set_focus();
                let _ = main_win.eval(&format!("window.location.hash = '/pin?id={}'", id));
            }
        }
    }

    // Enable layered window for opacity support
    if let Some(w) = app.get_webview_window(&id) {
        if let Ok(raw_hwnd) = w.hwnd() {
            let hwnd = HWND(raw_hwnd.0);
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
            }
        }
    }

    let _ = app.emit(
        "pin-image",
        serde_json::json!({
            "id": id,
            "image_base64": image_base64,
        }),
    );

    Ok(id)
}

#[tauri::command]
pub fn close_pin_window(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&id) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn resize_pin_window(
    app: AppHandle,
    id: String,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&id) {
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_pin_opacity(app: AppHandle, id: String, opacity: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&id) {
        let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let hwnd = HWND(raw_hwnd.0);
        let alpha = (opacity.clamp(0.1, 1.0) * 255.0) as u8;
        unsafe {
            // SetLayeredWindowAttributes requires WS_EX_LAYERED (set at creation)
            let _ = SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                alpha,
                LWA_ALPHA,
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_pin_clickthrough(
    app: AppHandle,
    id: String,
    clickthrough: bool,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&id) {
        let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let hwnd = HWND(raw_hwnd.0);
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if clickthrough {
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    style | WS_EX_TRANSPARENT.0 as isize,
                );
            } else {
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    style & !(WS_EX_TRANSPARENT.0 as isize),
                );
            }
        }
    }
    Ok(())
}
