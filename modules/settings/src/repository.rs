//! Settings Repository — key-value database persistence layer.

use rusqlite::{Connection, params};

/// Database access repository for key-value settings.
pub struct SettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsRepository<'a> {
    /// Create a new `SettingsRepository`.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve setting value by key.
    pub fn get(&self, key: &str) -> Result<Option<String>, anyhow::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Insert or update setting value by key.
    pub fn set(&self, key: &str, value: &str) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            params![key, value, now],
        )?;
        Ok(())
    }
}
