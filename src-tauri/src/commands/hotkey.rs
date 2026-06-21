#[tauri::command]
pub fn get_hotkeys() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({"id": "capture_region", "name": "区域截图", "shortcut": "Ctrl+Shift+A"}),
        serde_json::json!({"id": "capture_fullscreen", "name": "全屏截图", "shortcut": "Ctrl+Shift+S"}),
        serde_json::json!({"id": "capture_window", "name": "窗口截图", "shortcut": "Ctrl+Shift+Q"}),
    ])
}
