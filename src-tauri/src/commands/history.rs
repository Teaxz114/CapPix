use crate::history::{HistoryDb, PinRecord, ScreenshotRecord};
use std::sync::Mutex;
use tauri::Manager;

pub struct HistoryState {
    pub db: Mutex<HistoryDb>,
}

#[tauri::command]
pub fn history_save(
    app: tauri::AppHandle,
    state: tauri::State<HistoryState>,
    image_base64: String,
    width: u32,
    height: u32,
    source: String,
    ocr_text: Option<String>,
) -> Result<i64, String> {
    // Save image to file first
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let image_data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let screenshots_dir = app_data.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.png", timestamp, source);
    let file_path = screenshots_dir.join(&filename);
    std::fs::write(&file_path, &image_data).map_err(|e| e.to_string())?;

    let image_path = file_path.to_string_lossy().to_string();

    // Generate thumbnail (200px wide) for faster history list loading
    let thumb_dir = app_data.join("thumbnails");
    std::fs::create_dir_all(&thumb_dir).map_err(|e| e.to_string())?;
    let thumb_path = thumb_dir.join(&filename);
    if let Ok(img) = image::load_from_memory(&image_data) {
        let thumb = img.thumbnail(200, 200);
        let _ = thumb.save(&thumb_path);
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let record = ScreenshotRecord {
        id: 0,
        timestamp: String::new(),
        image_path,
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
    // Also delete the image file
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Get the image path before deleting the record
    let image_path: Option<String> = db.get_image_path(id).ok();
    db.delete(id).map_err(|e| e.to_string())?;
    drop(db);
    // Delete the file after releasing the lock
    if let Some(path) = image_path {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

#[tauri::command]
pub fn history_count(state: tauri::State<HistoryState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_clear(state: tauri::State<HistoryState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Get all image paths before clearing
    let paths: Vec<String> = db.list(i64::MAX, 0)
        .unwrap_or_default()
        .iter()
        .map(|r| r.image_path.clone())
        .collect();
    db.clear().map_err(|e| e.to_string())?;
    drop(db);
    // Delete all image files
    for path in paths {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

/// Read a screenshot image from file and return as base64 (for on-demand loading)
#[tauri::command]
pub fn get_screenshot_image(image_path: String) -> Result<String, String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let data = std::fs::read(&image_path).map_err(|e| format!("Failed to read image: {}", e))?;
    Ok(STANDARD.encode(&data))
}

/// Read a thumbnail image for a screenshot (much faster than full image)
/// Derives thumbnail path from the image path: screenshots/X.png → thumbnails/X.png
#[tauri::command]
pub fn get_screenshot_thumbnail(image_path: String) -> Result<String, String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    // Derive thumbnail path
    let path = std::path::Path::new(&image_path);
    let filename = path.file_name().unwrap_or_default();
    let parent = path.parent().and_then(|p| p.parent()).unwrap_or(std::path::Path::new("."));
    let thumb_path = parent.join("thumbnails").join(filename);

    if thumb_path.exists() {
        let data = std::fs::read(&thumb_path).map_err(|e| format!("Failed to read thumbnail: {}", e))?;
        Ok(STANDARD.encode(&data))
    } else {
        // Fallback: generate thumbnail on-the-fly from full image
        let data = std::fs::read(&image_path).map_err(|e| format!("Failed to read image: {}", e))?;
        if let Ok(img) = image::load_from_memory(&data) {
            let thumb = img.thumbnail(200, 200);
            let mut buf = Vec::new();
            let cursor = std::io::Cursor::new(&mut buf);
            let _ = thumb.write_to(cursor, image::ImageFormat::Png);
            Ok(STANDARD.encode(&buf))
        } else {
            // Last resort: return full image
            Ok(STANDARD.encode(&data))
        }
    }
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
