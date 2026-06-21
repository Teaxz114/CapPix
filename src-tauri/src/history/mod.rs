use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenshotRecord {
    pub id: i64,
    pub timestamp: String,
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
    pub source: String,
    pub ocr_text: Option<String>,
}

pub struct HistoryDb {
    conn: Connection,
}

impl HistoryDb {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS screenshot_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                image_base64 TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                source TEXT NOT NULL DEFAULT 'region',
                ocr_text TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_history_ts ON screenshot_history(timestamp DESC);",
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, record: &ScreenshotRecord) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO screenshot_history (image_base64, width, height, source, ocr_text) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.image_base64, record.width, record.height, record.source, record.ocr_text],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list(&self, limit: i64, offset: i64) -> SqlResult<Vec<ScreenshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, image_base64, width, height, source, ocr_text FROM screenshot_history ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(ScreenshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                image_base64: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                source: row.get(5)?,
                ocr_text: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn search(&self, query: &str, limit: i64) -> SqlResult<Vec<ScreenshotRecord>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, image_base64, width, height, source, ocr_text FROM screenshot_history WHERE ocr_text LIKE ?1 ORDER BY timestamp DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![pattern, limit], |row| {
            Ok(ScreenshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                image_base64: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                source: row.get(5)?,
                ocr_text: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM screenshot_history WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn count(&self) -> SqlResult<i64> {
        self.conn.query_row("SELECT COUNT(*) FROM screenshot_history", [], |row| row.get(0))
    }
}
