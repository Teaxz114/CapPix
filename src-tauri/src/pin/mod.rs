use tauri::{AppHandle, Emitter, Manager};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    // Generate unique ID
    let id = format!("pin-{}", uuid::Uuid::new_v4());

    // Create a new always-on-top window to display the image
    use tauri::WebviewWindowBuilder;

    let url = format!("/pin?id={}", id);
    let window_label = &id;

    let _window = WebviewWindowBuilder::new(
        &app,
        window_label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("CapPix Pin")
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .inner_size(400.0, 300.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    // Store the image data for this pin — emit after window is created so the
    // frontend can listen for it once the PinView mounts.
    let _ = app.emit("pin-image", serde_json::json!({
        "id": id,
        "image_base64": image_base64,
    }));

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
pub fn resize_pin_window(app: AppHandle, id: String, width: f64, height: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&id) {
        window.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
