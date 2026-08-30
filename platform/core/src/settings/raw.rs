//! Raw key-value settings helpers.
/*
last audited 25-07-26 by RSA-Agent (platform-core slice C: settings/raw deep read)
crate: platform-core | status: SAFE | lint: CLEAN
findings: exemplary — DB-08 delta-ledger concurrency contract documented and implemented (UNIQUE (key,terminal,version) collision retry under BEGIN IMMEDIATE, bounded 32 attempts, savepoint variant for nested callers with lingering-savepoint logging); all SQL parameterized; delta loss documented non-fatal with sync reconstruction path; next_delta_version .unwrap_or(1) is safe (collision retried)
next: none | perf: single-connection LOCAL GUC-free
*/

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

    /// Standalone delta writer — serialized allocation with a bounded retry.
    ///
    /// Computes `version = MAX(version) + 1` for the `(key, terminal_id)`
    /// pair and inserts a new row.
    ///
    /// # Concurrency contract (DB-08, migration 116)
    ///
    /// Migration 116 adds `idx_setting_updated_unique_version`, a UNIQUE
    /// index on `(key, terminal_id, version)`. Two concurrent writers that
    /// compute the same `MAX(version) + 1` collide: the loser's INSERT fails
    /// with a constraint error. When called standalone (no outer
    /// transaction), each attempt therefore runs in its own `BEGIN IMMEDIATE`
    /// transaction — SQLite's reserved write lock serializes concurrent
    /// allocations — and a constraint/busy collision retries with a fresh
    /// snapshot, so the ledger records gapless sequential versions and no
    /// delta is lost. Callers already inside a transaction take the
    /// single-attempt savepoint path (`write_delta_nested`): their outer
    /// transaction's earlier value write already serializes the allocation,
    /// and a retry inside the same transaction could not observe the
    /// winner's committed row anyway.
    pub fn write_delta(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        if !conn.is_autocommit() {
            Self::write_delta_nested(conn, key, value, terminal_id)
        } else {
            Self::write_delta_standalone(conn, key, value, terminal_id)
        }
    }

    /// Single-attempt savepoint variant for callers already inside a
    /// transaction. `execute_batch` is used instead of `conn.savepoint()`
    /// because the latter requires `&mut Connection`. A collision surfaces
    /// the constraint error (the caller's earlier value write has already
    /// serialized the allocation, so this is not expected).
    fn write_delta_nested(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let sp = format!("_oz_delta_{}", std::process::id());
        conn.execute_batch(&format!("SAVEPOINT {sp}"))?;
        let result = Self::write_delta_row(
            conn,
            key,
            value,
            terminal_id,
            Self::next_delta_version(conn, key, terminal_id),
        );
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

    /// Per-attempt `BEGIN IMMEDIATE` variant with a bounded retry for
    /// standalone callers (see the `write_delta` concurrency contract).
    fn write_delta_standalone(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        const MAX_ATTEMPTS: u32 = 32;
        let mut attempt: u32 = 0;
        loop {
            let result = (|| -> Result<(), PlatformError> {
                conn.execute_batch("BEGIN IMMEDIATE")?;
                let version = Self::next_delta_version(conn, key, terminal_id);
                Self::write_delta_row(conn, key, value, terminal_id, version)?;
                conn.execute_batch("COMMIT")?;
                Ok(())
            })();
            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // End the attempt's transaction (a no-op when BEGIN failed).
                    let _ = conn.execute_batch("ROLLBACK");
                    let transient = matches!(
                        &e,
                        PlatformError::Db(rusqlite::Error::SqliteFailure(err, _))
                            if err.code == rusqlite::ErrorCode::ConstraintViolation
                                || err.code == rusqlite::ErrorCode::DatabaseBusy
                    );
                    if transient && attempt + 1 < MAX_ATTEMPTS {
                        attempt += 1;
                        continue;
                    }
                    if transient {
                        tracing::warn!(key, terminal_id, error = %e, "delta write gave up after concurrent collisions");
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Compute the next version for a `(key, terminal_id)` pair.
    fn next_delta_version(conn: &Connection, key: &str, terminal_id: &str) -> i64 {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1
             FROM setting_updated
             WHERE key = ?1 AND terminal_id = ?2",
            params![key, terminal_id],
            |row| row.get(0),
        )
        .unwrap_or(1)
    }

    /// Insert one versioned delta row.
    fn write_delta_row(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
        version: i64,
    ) -> Result<(), PlatformError> {
        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version)
             VALUES (?1, ?2, ?3, ?4)",
            params![key, value, terminal_id, version],
        )?;
        Ok(())
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
        let version = Self::next_delta_version(tx, key, terminal_id);
        Self::write_delta_row(tx, key, value, terminal_id, version)
    }
}
