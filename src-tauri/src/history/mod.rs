use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenshotRecord {
    pub id: i64,
    pub timestamp: String,
    pub image_path: String,
    pub width: u32,
    pub height: u32,
    pub source: String,
    pub ocr_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinRecord {
    pub id: String,
    pub image_path: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    pub topmost: bool,
    pub created_at: String,
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
                image_path TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                source TEXT NOT NULL DEFAULT 'region',
                ocr_text TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_history_ts ON screenshot_history(timestamp DESC);

            CREATE TABLE IF NOT EXISTS pins (
                id TEXT PRIMARY KEY,
                image_path TEXT NOT NULL,
                x REAL NOT NULL DEFAULT 100,
                y REAL NOT NULL DEFAULT 100,
                width REAL NOT NULL DEFAULT 400,
                height REAL NOT NULL DEFAULT 300,
                opacity REAL NOT NULL DEFAULT 1.0,
                topmost INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );",
        )?;

        // Migration: if old column image_base64 exists, migrate data
        let has_base64_col: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('screenshot_history') WHERE name='image_base64'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) > 0;
        let has_path_col: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('screenshot_history') WHERE name='image_path'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) > 0;

        if has_base64_col && !has_path_col {
            log::info!("Migrating screenshot_history: image_base64 → image_path");
            // Add new column, mark old records as "migrated" (path will be empty)
            conn.execute_batch(
                "ALTER TABLE screenshot_history ADD COLUMN image_path TEXT NOT NULL DEFAULT '';
                 -- Note: old base64 data is lost in migration; users should re-capture",
            )?;
            // Drop old column not easily possible in SQLite, but we'll just ignore it
        }

        Ok(Self { conn })
    }

    // --- Screenshot history methods ---

    pub fn insert(&self, record: &ScreenshotRecord) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO screenshot_history (image_path, width, height, source, ocr_text) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.image_path, record.width, record.height, record.source, record.ocr_text],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list(&self, limit: i64, offset: i64) -> SqlResult<Vec<ScreenshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, image_path, width, height, source, ocr_text FROM screenshot_history ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(ScreenshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                image_path: row.get(2)?,
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
            "SELECT id, timestamp, image_path, width, height, source, ocr_text FROM screenshot_history WHERE ocr_text LIKE ?1 ORDER BY timestamp DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![pattern, limit], |row| {
            Ok(ScreenshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                image_path: row.get(2)?,
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

    pub fn get_image_path(&self, id: i64) -> SqlResult<String> {
        self.conn.query_row(
            "SELECT image_path FROM screenshot_history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn count(&self) -> SqlResult<i64> {
        self.conn.query_row("SELECT COUNT(*) FROM screenshot_history", [], |row| row.get(0))
    }

    pub fn clear(&self) -> SqlResult<()> {
        self.conn.execute("DELETE FROM screenshot_history", [])?;
        Ok(())
    }

    // --- Pin persistence methods ---

    pub fn save_pin(&self, pin: &PinRecord) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO pins (id, image_path, x, y, width, height, opacity, topmost) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![pin.id, pin.image_path, pin.x, pin.y, pin.width, pin.height, pin.opacity, pin.topmost as i32],
        )?;
        Ok(())
    }

    pub fn list_pins(&self) -> SqlResult<Vec<PinRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, image_path, x, y, width, height, opacity, topmost, created_at FROM pins ORDER BY created_at"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PinRecord {
                id: row.get(0)?,
                image_path: row.get(1)?,
                x: row.get(2)?,
                y: row.get(3)?,
                width: row.get(4)?,
                height: row.get(5)?,
                opacity: row.get(6)?,
                topmost: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_pin(&self, id: &str) -> SqlResult<()> {
        self.conn.execute("DELETE FROM pins WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_pin_position(&self, id: &str, x: f64, y: f64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE pins SET x = ?1, y = ?2 WHERE id = ?3",
            params![x, y, id],
        )?;
        Ok(())
    }
}
