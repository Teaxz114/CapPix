use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenshotRecord {
    pub id: i64,
    pub timestamp: String,
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
    pub source: String, // "region", "fullscreen", "window"
    pub ocr_text: Option<String>,
}

pub struct HistoryDb {
    conn: Mutex<rusqlite::Connection>,
}

impl HistoryDb {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let conn = rusqlite::Connection::open(db_path)
            .map_err(|e| format!("Failed to open history DB: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                image_base64 TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                source TEXT NOT NULL DEFAULT 'region',
                ocr_text TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_timestamp ON screenshots(timestamp);
            CREATE INDEX IF NOT EXISTS idx_ocr_text ON screenshots(ocr_text);"
        ).map_err(|e| format!("Failed to create tables: {}", e))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(&self, record: &ScreenshotRecord) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO screenshots (image_base64, width, height, source, ocr_text) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![record.image_base64, record.width, record.height, record.source, record.ocr_text],
        ).map_err(|e| format!("Insert failed: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    pub fn list(&self, limit: i64, offset: i64) -> Result<Vec<ScreenshotRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, image_base64, width, height, source, ocr_text FROM screenshots ORDER BY id DESC LIMIT ?1 OFFSET ?2"
            )
            .map_err(|e| format!("Prepare failed: {}", e))?;

        let rows = stmt
            .query_map(rusqlite::params![limit, offset], |row| {
                Ok(ScreenshotRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    image_base64: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    source: row.get(5)?,
                    ocr_text: row.get(6)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| e.to_string())?);
        }
        Ok(records)
    }

    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<ScreenshotRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, image_base64, width, height, source, ocr_text FROM screenshots WHERE ocr_text LIKE ?1 ORDER BY id DESC LIMIT ?2"
            )
            .map_err(|e| format!("Prepare failed: {}", e))?;

        let pattern = format!("%{}%", query);
        let rows = stmt
            .query_map(rusqlite::params![pattern, limit], |row| {
                Ok(ScreenshotRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    image_base64: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    source: row.get(5)?,
                    ocr_text: row.get(6)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| e.to_string())?);
        }
        Ok(records)
    }

    pub fn delete(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM screenshots WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("Delete failed: {}", e))?;
        Ok(())
    }

    pub fn count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT COUNT(*) FROM screenshots", [], |row| row.get(0))
            .map_err(|e| format!("Count failed: {}", e))
    }
}
