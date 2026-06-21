use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub struct HistoryDB {
    conn: Mutex<Connection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenshotRecord {
    pub id: i64,
    pub timestamp: String,
    pub image_base64: String,
    pub thumbnail_base64: String,
    pub width: i32,
    pub height: i32,
    pub ocr_text: Option<String>,
}

impl HistoryDB {
    pub fn new(db_path: &std::path::Path) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                image_base64 TEXT NOT NULL,
                thumbnail_base64 TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                ocr_text TEXT
            );"
        ).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

fn generate_thumbnail(image_base64: &str) -> Result<(String, i32, i32), String> {
    let data = STANDARD.decode(image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let (orig_w, orig_h) = (img.width() as i32, img.height() as i32);

    // Resize to 200px wide, maintaining aspect ratio
    let thumb_w = 200;
    let thumb_h = (orig_h as f64 * (thumb_w as f64 / orig_w as f64)).round() as u32;
    let thumbnail = img.thumbnail(thumb_w, thumb_h);

    // Encode as JPEG at 80% quality
    let mut buf = Vec::new();
    thumbnail
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| e.to_string())?;

    // Set JPEG quality to 80 by re-encoding with the image crate's JPEG encoder
    // The default write_to uses default quality; let's use the encoder explicitly
    let mut buf2 = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf2, 80);
    thumbnail
        .write_with_encoder(encoder)
        .map_err(|e| e.to_string())?;

    Ok((STANDARD.encode(&buf2), orig_w, orig_h))
}

#[tauri::command]
pub fn save_to_history(
    app: tauri::AppHandle,
    image_base64: String,
    ocr_text: Option<String>,
) -> Result<i64, String> {
    use tauri::Manager;
    let db = app.state::<HistoryDB>();
    let (thumbnail_base64, width, height) = generate_thumbnail(&image_base64)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO screenshots (image_base64, thumbnail_base64, width, height, ocr_text) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![image_base64, thumbnail_base64, width, height, ocr_text],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn get_history(
    app: tauri::AppHandle,
    limit: Option<i32>,
    offset: Option<i32>,
    search: Option<String>,
) -> Result<Vec<ScreenshotRecord>, String> {
    use tauri::Manager;
    let db = app.state::<HistoryDB>();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    let mut records = Vec::new();

    if let Some(ref search_text) = search {
        if !search_text.is_empty() {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, image_base64, thumbnail_base64, width, height, ocr_text \
                     FROM screenshots \
                     WHERE ocr_text LIKE ?1 \
                     ORDER BY id DESC LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| e.to_string())?;
            let pattern = format!("%{}%", search_text);
            let rows = stmt
                .query_map(params![pattern, limit, offset], |row| {
                    Ok(ScreenshotRecord {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        image_base64: row.get(2)?,
                        thumbnail_base64: row.get(3)?,
                        width: row.get(4)?,
                        height: row.get(5)?,
                        ocr_text: row.get(6)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                records.push(row.map_err(|e| e.to_string())?);
            }
            return Ok(records);
        }
    }

    // No search or empty search: return all
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, image_base64, thumbnail_base64, width, height, ocr_text \
             FROM screenshots \
             ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(ScreenshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                image_base64: row.get(2)?,
                thumbnail_base64: row.get(3)?,
                width: row.get(4)?,
                height: row.get(5)?,
                ocr_text: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        records.push(row.map_err(|e| e.to_string())?);
    }

    Ok(records)
}

#[tauri::command]
pub fn delete_history_item(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    use tauri::Manager;
    let db = app.state::<HistoryDB>();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM screenshots WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let db = app.state::<HistoryDB>();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM screenshots", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
