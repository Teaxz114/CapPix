use crate::history::{HistoryDb, PinRecord, ScreenshotRecord};
use std::sync::Mutex;

pub struct HistoryState {
    pub db: Mutex<HistoryDb>,
}

#[tauri::command]
pub fn history_save(
    state: tauri::State<HistoryState>,
    image_base64: String,
    width: u32,
    height: u32,
    source: String,
    ocr_text: Option<String>,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let record = ScreenshotRecord {
        id: 0,
        timestamp: String::new(),
        image_base64,
        width,
        height,
        source,
        ocr_text,
    };
    db.insert(&record).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_list(
    state: tauri::State<HistoryState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ScreenshotRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_search(
    state: tauri::State<HistoryState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<ScreenshotRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query, limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_delete(state: tauri::State<HistoryState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_count(state: tauri::State<HistoryState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count().map_err(|e| e.to_string())
}

// --- Pin persistence commands ---

#[tauri::command]
pub fn pin_save(
    state: tauri::State<HistoryState>,
    id: String,
    image_path: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    opacity: f64,
    topmost: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let record = PinRecord {
        id,
        image_path,
        x,
        y,
        width,
        height,
        opacity,
        topmost,
        created_at: String::new(),
    };
    db.save_pin(&record).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_list(state: tauri::State<HistoryState>) -> Result<Vec<PinRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_pins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_delete(state: tauri::State<HistoryState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_pin(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_update_position(
    state: tauri::State<HistoryState>,
    id: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_pin_position(&id, x, y)
        .map_err(|e| e.to_string())
}
