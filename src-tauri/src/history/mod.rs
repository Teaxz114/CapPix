use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// Fixed retention ceiling. Keeping this bounded prevents unbounded database/media growth.
pub const MAX_HISTORY_RECORDS: i64 = 1_000;

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

        // Migration: old databases retain their original data/schema and gain image_path.
        let has_base64_col: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('screenshot_history') WHERE name='image_base64'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        let has_path_col: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('screenshot_history') WHERE name='image_path'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if has_base64_col && !has_path_col {
            log::info!("Migrating screenshot_history: image_base64 → image_path");
            conn.execute_batch(
                "ALTER TABLE screenshot_history ADD COLUMN image_path TEXT NOT NULL DEFAULT '';",
            )?;
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
            "SELECT id, timestamp, image_path, width, height, source, ocr_text
             FROM screenshot_history ORDER BY timestamp DESC, id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], row_to_screenshot)?;
        rows.collect()
    }

    pub fn search(&self, query: &str, limit: i64) -> SqlResult<Vec<ScreenshotRecord>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, image_path, width, height, source, ocr_text
             FROM screenshot_history WHERE ocr_text LIKE ?1 ORDER BY timestamp DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit], row_to_screenshot)?;
        rows.collect()
    }

    /// Deletes a row and returns its recorded media path for best-effort cleanup.
    pub fn delete_and_get_path(&self, id: i64) -> SqlResult<Option<String>> {
        let path = self
            .conn
            .query_row(
                "SELECT image_path FROM screenshot_history WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        self.conn
            .execute("DELETE FROM screenshot_history WHERE id = ?1", params![id])?;
        Ok(path)
    }

    pub fn count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM screenshot_history", [], |row| row.get(0))
    }

    /// Clears rows and returns every media path so callers can remove files afterwards.
    pub fn clear_and_get_paths(&self) -> SqlResult<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT image_path FROM screenshot_history")?;
        let paths = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        drop(stmt);
        self.conn.execute("DELETE FROM screenshot_history", [])?;
        Ok(paths)
    }

    /// Removes the oldest records above `max_records` and returns their media paths.
    pub fn prune_to_limit(&self, max_records: i64) -> SqlResult<Vec<String>> {
        let count = self.count()?;
        let excess = count.saturating_sub(max_records.max(0));
        if excess == 0 {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT image_path FROM screenshot_history ORDER BY timestamp ASC, id ASC LIMIT ?1",
        )?;
        let paths = stmt
            .query_map(params![excess], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        drop(stmt);
        self.conn.execute(
            "DELETE FROM screenshot_history WHERE id IN (
                SELECT id FROM screenshot_history ORDER BY timestamp ASC, id ASC LIMIT ?1
            )",
            params![excess],
        )?;
        Ok(paths)
    }

    pub fn get_image_path(&self, id: i64) -> SqlResult<String> {
        self.conn.query_row(
            "SELECT image_path FROM screenshot_history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
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
            "SELECT id, image_path, x, y, width, height, opacity, topmost, created_at FROM pins ORDER BY created_at",
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

fn row_to_screenshot(row: &rusqlite::Row<'_>) -> SqlResult<ScreenshotRecord> {
    Ok(ScreenshotRecord {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        image_path: row.get(2)?,
        width: row.get(3)?,
        height: row.get(4)?,
        source: row.get(5)?,
        ocr_text: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> HistoryDb {
        HistoryDb::new(":memory:").expect("in-memory history database")
    }

    fn record(path: &str) -> ScreenshotRecord {
        ScreenshotRecord {
            id: 0,
            timestamp: String::new(),
            image_path: path.to_owned(),
            width: 10,
            height: 10,
            source: "region".to_owned(),
            ocr_text: None,
        }
    }

    #[test]
    fn deleting_returns_path_and_removes_record() {
        let db = test_db();
        let id = db.insert(&record("one.png")).unwrap();

        assert_eq!(db.delete_and_get_path(id).unwrap(), Some("one.png".to_owned()));
        assert_eq!(db.count().unwrap(), 0);
        assert_eq!(db.delete_and_get_path(id).unwrap(), None);
    }

    #[test]
    fn clear_returns_all_paths() {
        let db = test_db();
        db.insert(&record("one.png")).unwrap();
        db.insert(&record("two.png")).unwrap();

        assert_eq!(db.clear_and_get_paths().unwrap(), vec!["one.png", "two.png"]);
        assert_eq!(db.count().unwrap(), 0);
    }

    #[test]
    fn pruning_removes_oldest_records_and_keeps_limit() {
        let db = test_db();
        db.insert(&record("one.png")).unwrap();
        db.insert(&record("two.png")).unwrap();
        db.insert(&record("three.png")).unwrap();

        assert_eq!(db.prune_to_limit(2).unwrap(), vec!["one.png"]);
        assert_eq!(db.list(10, 0).unwrap().len(), 2);
        assert_eq!(db.prune_to_limit(2).unwrap(), Vec::<String>::new());
    }
}
