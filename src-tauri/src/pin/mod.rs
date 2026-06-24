use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
    WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

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

    // Store pin data for PinView to pick up (same pattern as PendingScreenshot)
    // We use emit since pin windows still need to be separate (they float on desktop)
    // But we store the data first so the window can pull it after mount
    if let Some(state) = app.try_state::<crate::commands::clipboard::PendingScreenshot>() {
        // Reuse PendingScreenshot as a temporary data store for pin image
        // This avoids the timing issue with emit events
        if let Ok(mut data) = state.0.lock() {
            *data = Some(image_base64.clone());
        }
    }

    // Create the pin window — use the main window as a fallback if new window fails
    use tauri::WebviewWindowBuilder;
    match WebviewWindowBuilder::new(&app, &id, tauri::WebviewUrl::App("index.html".into()))
        .title("CapPix Pin")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .inner_size(400.0, 300.0)
        .center()
        .resizable(true)
        .build()
    {
        Ok(window) => {
            let _ = window.eval(&format!("window.location.hash = '/pin?id={}'", id));
        }
        Err(e) => {
            log::warn!("Failed to create pin webview window: {}, using main window instead", e);
            // Fallback: navigate main window to pin view
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.set_decorations(false);
                let _ = main_win.set_always_on_top(true);
                let _ = main_win.set_size(tauri::LogicalSize::new(400.0, 300.0));
                let _ = main_win.center();
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

    // Also emit the event for backward compatibility
    let _ = app.emit(
        "pin-image",
        serde_json::json!({
            "id": id,
            "image_base64": image_base64,
        }),
    );

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
