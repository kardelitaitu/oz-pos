//! Raw key-value settings helpers.

use super::Settings;
use crate::error::PlatformError;
use rusqlite::{Connection, params};

impl Settings {
    /// Read a single setting by key. Returns `None` if the key doesn't exist.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, PlatformError> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Insert or update a setting.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), PlatformError> {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting. Returns `true` if the key existed.
    pub fn remove(conn: &Connection, key: &str) -> Result<bool, PlatformError> {
        let n = conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(n > 0)
    }

    /// Load every row from the `settings` table as `(key, value)` pairs.
    pub fn load_all(conn: &Connection) -> Result<Vec<(String, String)>, PlatformError> {
        let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Write multiple settings inside a single transaction.
    pub fn set_batch(conn: &Connection, rows: &[(String, String)]) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        for (key, value) in rows {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![key, value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // ── Delta ledger methods ──────────────────────────────────────

    /// Standalone delta writer — uses a savepoint for nesting safety.
    ///
    /// Computes `version = MAX(version) + 1` for the `(key, terminal_id)`
    /// pair and inserts a new row. Uses a savepoint so the SELECT MAX +
    /// INSERT are atomic and the call is safe from within an existing
    /// transaction (no nested `BEGIN` error).
    pub fn write_delta(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        // Use a savepoint so this works both standalone and when called
        // from within an existing transaction (e.g. set_tracked).
        // `execute_batch` is used instead of `conn.savepoint()` because
        // the latter requires `&mut Connection`.
        let sp = format!("_oz_delta_{}", std::process::id());
        conn.execute_batch(&format!("SAVEPOINT {sp}"))?;
        let result = (|| -> Result<(), PlatformError> {
            let version: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) + 1
                     FROM setting_updated
                     WHERE key = ?1 AND terminal_id = ?2",
                    params![key, terminal_id],
                    |row| row.get(0),
                )
                .unwrap_or(1);

            conn.execute(
                "INSERT INTO setting_updated (key, value, terminal_id, version)
                 VALUES (?1, ?2, ?3, ?4)",
                params![key, value, terminal_id, version],
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute_batch(&format!("RELEASE {sp}"))?;
                Ok(())
            }
            Err(e) => {
                tracing::warn!(key, terminal_id, error = %e, "delta write failed, rolling back savepoint");
                if let Err(rollback_err) = conn.execute_batch(&format!("ROLLBACK TO {sp}")) {
                    tracing::error!(key, terminal_id, error = %rollback_err, "ROLLBACK TO savepoint failed — savepoint may linger");
                }
                Err(e)
            }
        }
    }

    /// Get the latest version number for a `(key, terminal_id)` pair.
    ///
    /// Returns `None` if no deltas exist for that pair. Used by shared
    /// settings cards to detect concurrent edits (compare known version
    /// against the stored version before writing).
    pub fn get_version(
        conn: &Connection,
        key: &str,
        terminal_id: &str,
    ) -> Result<Option<i64>, PlatformError> {
        let mut stmt = conn.prepare(
            "SELECT version FROM setting_updated
             WHERE key = ?1 AND terminal_id = ?2
             ORDER BY version DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![key, terminal_id], |row| row.get::<_, i64>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Set a value AND write a delta record — both in a single transaction.
    ///
    /// This is the recommended method for Tauri command handlers that have
    /// access to a terminal ID. Calls `Settings::set()` for the value and
    /// `Settings::write_delta()` for the versioned audit trail, both
    /// within a single transaction. Since `write_delta()` uses a nested
    /// savepoint, the delta write failure does not roll back the `set()`.
    ///
    /// Delta write failures are logged but do not roll back the `set()` —
    /// delta loss is non-fatal; the sync layer can reconstruct from the
    /// settings table.
    pub fn set_tracked(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        Self::set(conn, key, value)?;
        // Inline delta write within the existing transaction to avoid
        // nested BEGIN (SQLite does not support nested transactions).
        if let Err(e) = Self::write_delta_on_tx(&tx, key, value, terminal_id) {
            tracing::warn!(key, terminal_id, error = %e, "delta write failed (non-fatal)");
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch write with delta tracking for every row.
    ///
    /// Like `set_batch()`, but also writes a delta row for each key/value
    /// pair. All operations run in a single transaction.
    pub fn set_batch_tracked(
        conn: &Connection,
        rows: &[(String, String)],
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        for (key, value) in rows {
            Self::set(conn, key, value)?;
            if let Err(e) = Self::write_delta_on_tx(&tx, key, value, terminal_id) {
                tracing::warn!(key, terminal_id, error = %e, "delta batch write failed (non-fatal)");
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Write a delta row using an existing transaction (no nested BEGIN).
    fn write_delta_on_tx(
        tx: &rusqlite::Transaction,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let version: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1
                 FROM setting_updated
                 WHERE key = ?1 AND terminal_id = ?2",
                params![key, terminal_id],
                |row| row.get(0),
            )
            .unwrap_or(1);
        tx.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version)
             VALUES (?1, ?2, ?3, ?4)",
            params![key, value, terminal_id, version],
        )?;
        Ok(())
    }
}
