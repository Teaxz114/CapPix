use crate::history::{HistoryDb, PinRecord, ScreenshotRecord, MAX_HISTORY_RECORDS};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Manager;
use uuid::Uuid;

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
    let image_data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let screenshots_dir = app_data.join("screenshots");
    let thumbnails_dir = app_data.join("thumbnails");
    fs::create_dir_all(&screenshots_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&thumbnails_dir).map_err(|e| e.to_string())?;

    // UUID prevents same-second overwrite. create_new + rename prevents partially-written files.
    let filename = format!("{}.png", Uuid::new_v4());
    let image_path = screenshots_dir.join(&filename);
    let thumbnail_path = thumbnails_dir.join(&filename);
    atomic_write(&image_path, &image_data).map_err(|e| format!("Failed to save screenshot: {e}"))?;

    if let Err(error) = write_thumbnail(&thumbnail_path, &image_data) {
        remove_history_media(&app_data, &image_path.to_string_lossy());
        return Err(format!("Failed to save screenshot thumbnail: {error}"));
    }

    let record = ScreenshotRecord {
        id: 0,
        timestamp: String::new(),
        image_path: image_path.to_string_lossy().to_string(),
        width,
        height,
        source,
        ocr_text,
    };

    let insert_result = match state.db.lock() {
        Ok(db) => db.insert(&record).map_err(|e| e.to_string()),
        Err(error) => Err(error.to_string()),
    };
    let id = match insert_result {
        Ok(id) => id,
        Err(error) => {
            // No database row owns these files, so always clean them up on DB/lock failure.
            remove_history_media(&app_data, &record.image_path);
            return Err(error);
        }
    };

    let pruned_paths = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.prune_to_limit(MAX_HISTORY_RECORDS)
            .map_err(|e| format!("History retention cleanup failed: {e}"))?
    };
    for path in pruned_paths {
        remove_history_media(&app_data, &path);
    }

    Ok(id)
}

#[tauri::command]
pub fn history_list(
    state: tauri::State<HistoryState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ScreenshotRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list(limit.unwrap_or(50).clamp(1, 100), offset.unwrap_or(0).max(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_search(
    state: tauri::State<HistoryState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<ScreenshotRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query, limit.unwrap_or(50).clamp(1, 100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_delete(
    app: tauri::AppHandle,
    state: tauri::State<HistoryState>,
    id: i64,
) -> Result<(), String> {
    let path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.delete_and_get_path(id).map_err(|e| e.to_string())?
    };
    if let Some(path) = path {
        let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
        remove_history_media(&app_data, &path);
    }
    Ok(())
}

#[tauri::command]
pub fn history_count(state: tauri::State<HistoryState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_clear(app: tauri::AppHandle, state: tauri::State<HistoryState>) -> Result<(), String> {
    let paths = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.clear_and_get_paths().map_err(|e| e.to_string())?
    };
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    for path in paths {
        remove_history_media(&app_data, &path);
    }
    Ok(())
}

/// Reads a screenshot by database record ID; callers never supply filesystem paths.
#[tauri::command]
pub fn get_screenshot_image(
    app: tauri::AppHandle,
    state: tauri::State<HistoryState>,
    id: i64,
) -> Result<String, String> {
    read_history_image(&app, &state, id, false)
}

/// Reads a screenshot thumbnail by database record ID; falls back to a generated in-memory thumbnail.
#[tauri::command]
pub fn get_screenshot_thumbnail(
    app: tauri::AppHandle,
    state: tauri::State<HistoryState>,
    id: i64,
) -> Result<String, String> {
    read_history_image(&app, &state, id, true)
}

fn read_history_image(
    app: &tauri::AppHandle,
    state: &tauri::State<HistoryState>,
    id: i64,
    thumbnail: bool,
) -> Result<String, String> {
    let image_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_image_path(id).map_err(|_| "Screenshot record not found".to_owned())?
    };
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let image_path = validated_media_path(&app_data, &image_path, "screenshots")?;

    if thumbnail {
        let thumbnail_path = app_data
            .join("thumbnails")
            .join(image_path.file_name().ok_or("Invalid screenshot filename")?);
        if let Ok(thumbnail_path) = validated_media_path(&app_data, &thumbnail_path.to_string_lossy(), "thumbnails") {
            if thumbnail_path.exists() {
                return fs::read(thumbnail_path)
                    .map(|data| STANDARD.encode(data))
                    .map_err(|e| format!("Failed to read thumbnail: {e}"));
            }
        }
    }

    let data = fs::read(&image_path).map_err(|e| format!("Failed to read image: {e}"))?;
    if thumbnail {
        if let Ok(image) = image::load_from_memory(&data) {
            let mut buffer = Vec::new();
            image
                .thumbnail(200, 200)
                .write_to(std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
                .map_err(|e| format!("Failed to generate thumbnail: {e}"))?;
            return Ok(STANDARD.encode(buffer));
        }
    }
    Ok(STANDARD.encode(data))
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| std::io::Error::other("missing media directory"))?;
    let temp_path = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;
    if let Err(error) = (|| -> std::io::Result<()> {
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temp_path, path)
    })() {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn write_thumbnail(path: &Path, image_data: &[u8]) -> Result<(), String> {
    let image = image::load_from_memory(image_data).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    image
        .thumbnail(200, 200)
        .write_to(std::io::Cursor::new(&mut data), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    atomic_write(path, &data).map_err(|e| e.to_string())
}

/// Resolves paths only under a named app-data media directory, blocking absolute/path traversal DB values.
fn validated_media_path(app_data: &Path, candidate: &str, media_dir: &str) -> Result<PathBuf, String> {
    let root_canonical = app_data
        .join(media_dir)
        .canonicalize()
        .map_err(|_| "History media directory is unavailable".to_owned())?;
    let candidate = Path::new(candidate);
    let candidate_parent = candidate
        .parent()
        .ok_or("Invalid history media path")?
        .canonicalize()
        .map_err(|_| "Invalid history media path")?;
    let filename = candidate.file_name().ok_or("Invalid history media filename")?;
    if candidate_parent != root_canonical {
        return Err("Refusing to access a path outside managed history storage".to_owned());
    }

    let managed_path = root_canonical.join(filename);
    // Existing symlinks must also resolve inside the managed directory before reading/removing.
    if managed_path.exists() {
        let resolved = managed_path
            .canonicalize()
            .map_err(|_| "History media file is unavailable".to_owned())?;
        if resolved.parent() != Some(root_canonical.as_path()) {
            return Err("Refusing to access a path outside managed history storage".to_owned());
        }
    }
    Ok(managed_path)
}

fn remove_history_media(app_data: &Path, image_path: &str) {
    let Ok(image_path) = validated_media_path(app_data, image_path, "screenshots") else {
        log::warn!("Refusing to delete unmanaged history path: {image_path}");
        return;
    };
    // This is derived solely from an already validated screenshot filename, never from caller input.
    let thumbnail_path = app_data
        .join("thumbnails")
        .join(image_path.file_name().unwrap_or_default());
    let _ = fs::remove_file(image_path);
    let _ = fs::remove_file(thumbnail_path);
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
    let record = PinRecord { id, image_path, x, y, width, height, opacity, topmost, created_at: String::new() };
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
pub fn pin_update_position(state: tauri::State<HistoryState>, id: String, x: f64, y: f64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_pin_position(&id, x, y).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_paths_must_stay_in_the_expected_managed_directory() {
        let temp = std::env::temp_dir().join(format!("cappix-history-test-{}", Uuid::new_v4()));
        let screenshots = temp.join("screenshots");
        fs::create_dir_all(&screenshots).unwrap();
        let media = screenshots.join("safe.png");
        fs::write(&media, b"test").unwrap();
        let outside = temp.join("outside.png");
        fs::write(&outside, b"test").unwrap();

        assert_eq!(validated_media_path(&temp, &media.to_string_lossy(), "screenshots").unwrap(), media.canonicalize().unwrap());
        assert!(validated_media_path(&temp, &outside.to_string_lossy(), "screenshots").is_err());
        assert!(validated_media_path(&temp, "../outside.png", "screenshots").is_err());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn cleanup_removes_screenshot_and_thumbnail_even_if_screenshot_is_already_missing() {
        let temp = std::env::temp_dir().join(format!("cappix-history-test-{}", Uuid::new_v4()));
        let screenshots = temp.join("screenshots");
        let thumbnails = temp.join("thumbnails");
        fs::create_dir_all(&screenshots).unwrap();
        fs::create_dir_all(&thumbnails).unwrap();
        let screenshot = screenshots.join("capture.png");
        let thumbnail = thumbnails.join("capture.png");
        fs::write(&screenshot, b"image").unwrap();
        fs::write(&thumbnail, b"thumbnail").unwrap();

        remove_history_media(&temp, &screenshot.to_string_lossy());
        assert!(!screenshot.exists());
        assert!(!thumbnail.exists());
        fs::write(&thumbnail, b"thumbnail").unwrap();
        remove_history_media(&temp, &screenshot.to_string_lossy());
        assert!(!thumbnail.exists());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn atomic_write_creates_complete_unique_files() {
        let temp = std::env::temp_dir().join(format!("cappix-history-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp).unwrap();
        let first = temp.join("first.png");
        let second = temp.join("second.png");
        atomic_write(&first, b"first").unwrap();
        atomic_write(&second, b"second").unwrap();
        assert_eq!(fs::read(first).unwrap(), b"first");
        assert_eq!(fs::read(second).unwrap(), b"second");
        fs::remove_dir_all(temp).unwrap();
    }
}
