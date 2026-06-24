use crate::hotkey;

#[tauri::command]
pub fn get_hotkeys(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    hotkey::get_hotkeys(app)
}

#[tauri::command]
pub fn set_hotkey(app: tauri::AppHandle, id: String, shortcut: String) -> Result<(), String> {
    hotkey::set_hotkey(app, id, shortcut)
}
