use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, PostMessageW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWLP_USERDATA,
    GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED, WS_EX_TRANSPARENT,
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
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let image_data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Create native window at center of screen
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No monitor")?;
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

    eprintln!(
        "[Pin] Native pin window created: {} (HWND {:?})",
        id, hwnd_result
    );

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

/// Restore an existing persisted pin without generating a new ID or writing a
/// second database record.  This uses the same owner-thread/message-loop path
/// as newly created pins.
pub fn restore_pin_window(app: AppHandle, record: crate::history::PinRecord) -> Result<(), String> {
    let image_data = std::fs::read(&record.image_path)
        .map_err(|e| format!("Failed to read persisted pin image: {}", e))?;
    let img = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (image_width, image_height) = rgba.dimensions();
    let rgba_vec = rgba.into_raw();
    let (tx, rx) = std::sync::mpsc::channel::<Result<isize, String>>();
    let id = record.id.clone();
    let x = record.x as i32;
    let y = record.y as i32;
    let width = record.width.max(1.0) as i32;
    let height = record.height.max(1.0) as i32;
    let app_clone = app.clone();

    std::thread::spawn(move || {
        if let Err(error) = native_window::NativePinWindow::create_with_channel(
            &rgba_vec,
            image_width as i32,
            image_height as i32,
            width,
            height,
            x,
            y,
            id,
            app_clone,
            tx,
        ) {
            log::error!("Failed to restore native pin: {}", error);
        }
    });

    let hwnd_value = rx
        .recv()
        .map_err(|_| "Pin restore thread closed before creating a window".to_string())??;
    if let Some(reg) = app.try_state::<PinRegistry>() {
        reg.0
            .lock()
            .map_err(|e| e.to_string())?
            .insert(record.id.clone(), hwnd_value);
    }

    let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
    unsafe {
        let alpha = (record.opacity.clamp(0.1, 1.0) * 255.0).round() as u8;
        SetLayeredWindowAttributes(
            hwnd,
            windows::Win32::Foundation::COLORREF(0),
            alpha,
            LWA_ALPHA,
        )
        .map_err(|e| format!("Failed to restore pin opacity: {}", e))?;
        if !record.topmost {
            windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
            )
            .map_err(|e| format!("Failed to restore pin topmost state: {}", e))?;
        }
    }
    Ok(())
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
    // Native pin HWNDs belong to their dedicated message-loop threads. Queue a
    // close request instead of calling DestroyWindow from this Tauri thread.
    if let Some(hwnd) = get_pin_hwnd(&app, &id) {
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            )
            .map_err(|e| format!("Failed to request pin close: {}", e))?;
        }
        // Registry, DB record and image are removed by the owner thread's
        // WM_DESTROY handler only after destruction is confirmed.
        return Ok(());
    }
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
        let new_width = width.round().clamp(48.0, i32::MAX as f64) as i32;
        let new_height = height.round().clamp(29.0, i32::MAX as f64) as i32;
        unsafe {
            PostMessageW(
                hwnd,
                native_window::WM_APP_RESIZE_PIN,
                WPARAM(new_width as usize),
                LPARAM(new_height as isize),
            )
            .map_err(|e| format!("Failed to request pin resize: {}", e))?;
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
