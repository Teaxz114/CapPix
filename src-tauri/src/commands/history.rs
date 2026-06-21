use crate::history::{HistoryDb, ScreenshotRecord};
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
    db.list(limit.unwrap_or(50), offset.unwrap_or(0)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_search(
    state: tauri::State<HistoryState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<ScreenshotRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query, limit.unwrap_or(50)).map_err(|e| e.to_string())
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
