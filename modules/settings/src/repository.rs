/*
last audited 25-07-26 by RSA-Agent (modules-settings slice A: repository verified)
crate: modules-settings | status: SAFE | lint: CLEAN
findings: MSL-5 INFO — SettingsRepository.set writes the settings table directly WITHOUT the DB-08 delta ledger (no versioned delta row) and without platform-core typed.rs encrypted-at-rest handling; currently a thin shell (no secret/tracked-key callers found), but any future adopter would silently skip sync deltas and encryption; prefer platform-core Settings::set_tracked for tracked keys
next: route tracked keys through platform-core Settings | perf: N/A
*/
//! Settings Repository — key-value database persistence layer.

use crate::error::SettingsError;
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
    pub fn get(&self, key: &str) -> Result<Option<String>, SettingsError> {
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
    pub fn set(&self, key: &str, value: &str) -> Result<(), SettingsError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            params![key, value, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
