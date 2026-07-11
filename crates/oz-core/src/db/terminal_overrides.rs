//! Per-terminal feature override store methods.
//!
//! Provides CRUD operations for the `terminal_feature_overrides` table.
//! Each row maps a terminal + feature key to a boolean enabled state,
//! allowing terminals to deviate from the global feature set.

use rusqlite::params;

use crate::TerminalFeatureOverride;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// List all feature overrides for a given terminal.
    pub fn list_terminal_overrides(
        &self,
        terminal_id: &str,
    ) -> Result<Vec<TerminalFeatureOverride>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT terminal_id, feature, enabled, created_at, updated_at
                 FROM terminal_feature_overrides
                 WHERE terminal_id = ?1
                 ORDER BY feature ASC",
        )?;
        let rows = stmt.query_map(params![terminal_id], Self::row_to_terminal_override)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a single feature override by terminal and feature key.
    pub fn get_terminal_override(
        &self,
        terminal_id: &str,
        feature: &str,
    ) -> Result<Option<TerminalFeatureOverride>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT terminal_id, feature, enabled, created_at, updated_at
                 FROM terminal_feature_overrides
                 WHERE terminal_id = ?1 AND feature = ?2",
        )?;
        let result = stmt.query_row(
            params![terminal_id, feature],
            Self::row_to_terminal_override,
        );
        match result {
            Ok(o) => Ok(Some(o)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set (upsert) a feature override for a terminal.
    ///
    /// If an override for the same terminal_id + feature already exists,
    /// its `enabled` and `updated_at` are updated. Otherwise a new row
    /// is inserted.
    pub fn set_terminal_override(
        &self,
        terminal_id: &str,
        feature: &str,
        enabled: bool,
    ) -> Result<(), CoreError> {
        let now = format_now();
        let affected = self.conn.execute(
            "UPDATE terminal_feature_overrides
             SET enabled = ?3, updated_at = ?4
             WHERE terminal_id = ?1 AND feature = ?2",
            params![terminal_id, feature, enabled as i64, now],
        )?;
        if affected == 0 {
            self.conn.execute(
                "INSERT INTO terminal_feature_overrides (terminal_id, feature, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![terminal_id, feature, enabled as i64, now],
            )?;
        }
        Ok(())
    }

    /// Delete a single feature override for a terminal.
    pub fn delete_terminal_override(
        &self,
        terminal_id: &str,
        feature: &str,
    ) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM terminal_feature_overrides
                 WHERE terminal_id = ?1 AND feature = ?2",
            params![terminal_id, feature],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "terminal_feature_override",
                id: format!("{terminal_id}/{feature}"),
            });
        }
        Ok(())
    }

    /// Delete all feature overrides for a terminal.
    pub fn clear_terminal_overrides(&self, terminal_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM terminal_feature_overrides WHERE terminal_id = ?1",
            params![terminal_id],
        )?;
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn row_to_terminal_override(row: &rusqlite::Row) -> rusqlite::Result<TerminalFeatureOverride> {
        Ok(TerminalFeatureOverride {
            terminal_id: row.get("terminal_id")?,
            feature: row.get("feature")?,
            enabled: row.get::<_, i64>("enabled")? != 0,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

fn format_now() -> String {
    // Same format used by the SQL `strftime` default in migrations.
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_terminal(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO terminals (id, name, device_id, created_at, updated_at)
             VALUES ('term-1', 'Front Register', 'dev-001', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                    ('term-2', 'Back Office',    'dev-002', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')"
        ).unwrap();
    }

    // ── list_terminal_overrides ──────────────────────────────────────

    #[test]
    fn list_overrides_empty_when_none_exist() {
        let conn = fresh();
        seed_terminal(&conn);
        let overrides = store(&conn).list_terminal_overrides("term-1").unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn list_overrides_returns_all_for_terminal() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        s.set_terminal_override("term-1", "receipt-printing", true)
            .unwrap();
        // Different terminal should not appear.
        s.set_terminal_override("term-2", "card-payment", true)
            .unwrap();

        let overrides = s.list_terminal_overrides("term-1").unwrap();
        assert_eq!(overrides.len(), 2);
        assert_eq!(overrides[0].feature, "card-payment");
        assert!(!overrides[0].enabled);
        assert_eq!(overrides[1].feature, "receipt-printing");
        assert!(overrides[1].enabled);
    }

    // ── get_terminal_override ────────────────────────────────────────

    #[test]
    fn get_override_found() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        let o = s
            .get_terminal_override("term-1", "card-payment")
            .unwrap()
            .unwrap();
        assert_eq!(o.feature, "card-payment");
        assert!(!o.enabled);
    }

    #[test]
    fn get_override_not_found() {
        let conn = fresh();
        seed_terminal(&conn);
        let o = store(&conn)
            .get_terminal_override("term-1", "nonexistent")
            .unwrap();
        assert!(o.is_none());
    }

    // ── set_terminal_override ────────────────────────────────────────

    #[test]
    fn set_override_inserts_new_row() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        let o = s
            .get_terminal_override("term-1", "card-payment")
            .unwrap()
            .unwrap();
        assert_eq!(o.feature, "card-payment");
        assert!(!o.enabled);
    }

    #[test]
    fn set_override_updates_existing_row() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        // Update to enabled.
        s.set_terminal_override("term-1", "card-payment", true)
            .unwrap();
        let o = s
            .get_terminal_override("term-1", "card-payment")
            .unwrap()
            .unwrap();
        assert!(o.enabled);
        assert!(!o.created_at.is_empty());
        assert!(!o.updated_at.is_empty());
    }

    // ── delete_terminal_override ─────────────────────────────────────

    #[test]
    fn delete_override_removes_row() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        s.delete_terminal_override("term-1", "card-payment")
            .unwrap();
        assert!(
            s.get_terminal_override("term-1", "card-payment")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn delete_override_not_found() {
        let conn = fresh();
        seed_terminal(&conn);
        let err = store(&conn)
            .delete_terminal_override("term-1", "nope")
            .unwrap_err();
        assert!(
            matches!(err, CoreError::NotFound { entity, .. } if entity == "terminal_feature_override")
        );
    }

    // ── clear_terminal_overrides ─────────────────────────────────────

    #[test]
    fn clear_overrides_removes_all() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        s.set_terminal_override("term-1", "receipt-printing", true)
            .unwrap();
        s.clear_terminal_overrides("term-1").unwrap();
        let overrides = s.list_terminal_overrides("term-1").unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn clear_overrides_other_terminal_untouched() {
        let conn = fresh();
        seed_terminal(&conn);
        let s = store(&conn);
        s.set_terminal_override("term-1", "card-payment", false)
            .unwrap();
        s.set_terminal_override("term-2", "card-payment", true)
            .unwrap();
        s.clear_terminal_overrides("term-1").unwrap();
        let overrides = s.list_terminal_overrides("term-2").unwrap();
        assert_eq!(overrides.len(), 1);
    }
}
