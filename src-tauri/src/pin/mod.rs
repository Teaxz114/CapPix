use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GetWindowRect, MoveWindow, SetLayeredWindowAttributes, SetWindowLongPtrW,
    GWLP_USERDATA, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

mod native_window;

/// Global registry: pin_id → HWND (native windows are not in Tauri's window manager)
pub struct PinRegistry(pub Mutex<HashMap<String, isize>>);

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

    // Decode base64 image
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

    // CRITICAL: Create the window on a dedicated thread that runs its own
    // message loop. Win32 windows must have a GetMessage/DispatchMessage loop
    // on the SAME thread that created them, otherwise WM_PAINT and other
    // messages are never processed (window won't render or respond to input).
    //
    // We use a channel to get the HWND back; NativePinWindow::create sends
    // the HWND via the channel right after ShowWindow, BEFORE entering its
    // message loop. The thread stays alive running the loop until the window
    // is destroyed.
    let (tx, rx) = std::sync::mpsc::channel::<Result<isize, String>>();
    let id_clone = id.clone();
    let app_clone = app.clone();
    let rgba_vec = rgba.as_raw().to_vec();
    std::thread::spawn(move || {
        let result = native_window::NativePinWindow::create_with_channel(
            &rgba_vec,
            width as i32,
            height as i32,
            x,
            y,
            id_clone,
            app_clone,
            tx,
        );
        // If create_with_channel returned an error before sending, send it now.
        if let Err(e) = result {
            eprintln!("[Pin] create_with_channel error: {}", e);
        }
        // Thread stays alive inside create_with_channel's message loop.
        // When the window is destroyed, the loop exits and the thread ends.
    });

    let hwnd_result = rx
        .recv()
        .map_err(|_| "Pin window channel closed".to_string())?
        .map_err(|e| e)?;

    eprintln!("[Pin] Native pin window created: {} (HWND {:?})", id, hwnd_result);

    // Register HWND in the pin registry
    if let Some(reg) = app.try_state::<PinRegistry>() {
        if let Ok(mut map) = reg.0.lock() {
            map.insert(id.clone(), hwnd_result);
        }
    }

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

    // Decode base64 image and create a native Win32 pin window (no webview!)
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let image_data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (img_w, img_h) = rgba.dimensions();

    let hwnd = native_window::NativePinWindow::create(
        rgba.as_raw(),
        img_w as i32,
        img_h as i32,
        x as i32,
        y as i32,
        id.clone(),
        app.clone(),
    )?;

    log::info!("Native pin window (at) created: {} (HWND {:?})", id, hwnd);

    // Register HWND in the pin registry
    if let Some(reg) = app.try_state::<PinRegistry>() {
        if let Ok(mut map) = reg.0.lock() {
            map.insert(id.clone(), hwnd);
        }
    }

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
                x,
                y,
                width,
                height,
                opacity: 1.0,
                topmost: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    Ok(id)
}

/// Helper: get HWND from pin registry by id
fn get_pin_hwnd(app: &AppHandle, id: &str) -> Option<HWND> {
    let reg = app.try_state::<PinRegistry>()?;
    let map = reg.0.lock().ok()?;
    let h = map.get(id).copied()?;
    Some(HWND(h as *mut std::ffi::c_void))
}

#[tauri::command]
pub fn close_pin_window(app: AppHandle, id: String) -> Result<(), String> {
    // Try native HWND registry first
    if let Some(reg) = app.try_state::<PinRegistry>() {
        if let Ok(mut map) = reg.0.lock() {
            if let Some(h) = map.get(&id) {
                let hwnd = HWND(*h as *mut std::ffi::c_void);
                // DestroyWindow triggers WM_DESTROY which will remove from registry via pin_id
                let result = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd)
                };
                if result.is_err() {
                    // If DestroyWindow failed, manually remove from registry
                    map.remove(&id);
                }
                return Ok(());
            }
        }
    }
    // Fallback: try Tauri webview window (shouldn't happen for native pins)
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
    if let Some(hwnd) = get_pin_hwnd(&app, &id) {
        unsafe {
            let mut rect = std::mem::zeroed();
            let _ = GetWindowRect(hwnd, &mut rect);
            let _ = MoveWindow(hwnd, rect.left, rect.top, width as i32, height as i32, true);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_pin_opacity(app: AppHandle, id: String, opacity: f64) -> Result<(), String> {
    if let Some(hwnd) = get_pin_hwnd(&app, &id) {
        let alpha = (opacity.clamp(0.1, 1.0) * 255.0) as u8;
        unsafe {
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
pub fn set_pin_clickthrough(app: AppHandle, id: String, clickthrough: bool) -> Result<(), String> {
    if let Some(hwnd) = get_pin_hwnd(&app, &id) {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if clickthrough {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
            } else {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_TRANSPARENT.0 as isize));
            }
        }
    }
    Ok(())
}
