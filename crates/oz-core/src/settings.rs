//! Settings store — typed access to the `settings` key-value table.
//!
//! The [`Settings`] module provides read/write helpers for the
//! `settings` table (migration `002_products.sql`). All methods take
//! a `&rusqlite::Connection` so callers control transaction boundaries.
//!
//! Feature flags are loaded/saved via [`FeatureRegistry`]'s
//! `to_settings_rows` / `from_settings_rows` methods. The settings
//! store is the bridge: it reads all rows from SQLite, extracts the
//! `feature.*` subset for the registry, and writes them back.

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::features::FeatureRegistry;

/// Typed access to the `settings` table.
pub struct Settings;

// ── Raw key-value helpers ────────────────────────────────────────────

impl Settings {
    /// Read a single setting by key. Returns `None` if the key doesn't exist.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, CoreError> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Insert or update a setting. The `updated_at` timestamp is set
    /// by SQLite's DEFAULT expression.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), CoreError> {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting. Returns `true` if the key existed.
    pub fn remove(conn: &Connection, key: &str) -> Result<bool, CoreError> {
        let n = conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(n > 0)
    }

    /// Load every row from the `settings` table as `(key, value)` pairs.
    pub fn load_all(conn: &Connection) -> Result<Vec<(String, String)>, CoreError> {
        let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Write multiple settings inside a single transaction.
    pub fn set_batch(
        conn: &Connection,
        rows: &[(String, String)],
    ) -> Result<(), CoreError> {
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
}

// ── Feature flags ─────────────────────────────────────────────────────

impl Settings {
    /// Load the feature flag registry from the `settings` table.
    pub fn load_features(conn: &Connection) -> Result<FeatureRegistry, CoreError> {
        let rows = Self::load_all(conn)?;
        Ok(FeatureRegistry::from_settings_rows(&rows))
    }

    /// Save the feature flag registry to the `settings` table.
    ///
    /// Writes all feature rows in a single transaction. Old feature rows
    /// that are no longer enabled are **not** pruned — call
    /// [`Settings::prune_stale_features`] to clean them up.
    pub fn save_features(
        conn: &Connection,
        reg: &FeatureRegistry,
    ) -> Result<(), CoreError> {
        let rows = reg.to_settings_rows();
        Self::set_batch(conn, &rows)
    }

    /// Remove stale feature rows (keys starting with `"feature."` whose
    /// value is `"1"`) that are not present in `reg`. Returns the number
    /// of pruned rows.
    pub fn prune_stale_features(
        conn: &Connection,
        reg: &FeatureRegistry,
    ) -> Result<usize, CoreError> {
        let enabled_keys: std::collections::HashSet<String> = reg
            .to_settings_rows()
            .into_iter()
            .map(|(k, _)| k)
            .collect();

        let all_features = Self::load_all(conn)?
            .into_iter()
            .filter(|(k, _)| k.starts_with("feature."))
            .filter(|(k, _)| !enabled_keys.contains(k));

        let mut removed = 0;
        for (key, _) in all_features {
            Self::remove(conn, &key)?;
            removed += 1;
        }
        Ok(removed)
    }
}

// ── Typed store configuration ─────────────────────────────────────────

/// Well-known settings keys for store configuration.
pub mod keys {
    /// Store display name. Default: `"OZ-POS Store"`.
    pub const STORE_NAME: &str = "store.name";
    /// Default ISO-4217 currency code. Default: `"USD"`.
    pub const DEFAULT_CURRENCY: &str = "store.default_currency";
    /// Store preset name (e.g., `"simple-retail"`, `"restaurant"`).
    pub const STORE_PRESET: &str = "store.preset";
    /// Whether the Setup Wizard has been completed.
    pub const SETUP_COMPLETE: &str = "store.setup_complete";
}

impl Settings {
    /// Get the store display name.
    pub fn get_store_name(conn: &Connection) -> Result<Option<String>, CoreError> {
        Self::get(conn, keys::STORE_NAME)
    }

    /// Set the store display name.
    pub fn set_store_name(conn: &Connection, name: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::STORE_NAME, name)
    }

    /// Get the default currency code (ISO-4217).
    pub fn get_default_currency(conn: &Connection) -> Result<Option<String>, CoreError> {
        Self::get(conn, keys::DEFAULT_CURRENCY)
    }

    /// Set the default currency code.
    pub fn set_default_currency(conn: &Connection, code: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::DEFAULT_CURRENCY, code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    // ── Raw get / set / remove ───────────────────────────────────

    #[test]
    fn get_returns_none_for_missing_key() {
        let conn = fresh();
        assert_eq!(Settings::get(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn set_and_get_roundtrip() {
        let conn = fresh();
        Settings::set(&conn, "test.key", "hello").unwrap();
        assert_eq!(Settings::get(&conn, "test.key").unwrap(), Some("hello".into()));
    }

    #[test]
    fn set_overwrites_existing() {
        let conn = fresh();
        Settings::set(&conn, "k", "v1").unwrap();
        Settings::set(&conn, "k", "v2").unwrap();
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("v2".into()));
    }

    #[test]
    fn remove_existing_key() {
        let conn = fresh();
        Settings::set(&conn, "k", "v").unwrap();
        assert!(Settings::remove(&conn, "k").unwrap());
        assert_eq!(Settings::get(&conn, "k").unwrap(), None);
    }

    #[test]
    fn remove_missing_key_returns_false() {
        let conn = fresh();
        assert!(!Settings::remove(&conn, "nope").unwrap());
    }

    // ── Batch ────────────────────────────────────────────────────

    #[test]
    fn set_batch_inserts_multiple() {
        let conn = fresh();
        let rows: Vec<(String, String)> = vec![
            ("a".into(), "1".into()),
            ("b".into(), "2".into()),
        ];
        Settings::set_batch(&conn, &rows).unwrap();
        assert_eq!(Settings::get(&conn, "a").unwrap(), Some("1".into()));
        assert_eq!(Settings::get(&conn, "b").unwrap(), Some("2".into()));
    }

    #[test]
    fn load_all_returns_all_rows() {
        let conn = fresh();
        Settings::set(&conn, "a", "1").unwrap();
        Settings::set(&conn, "b", "2").unwrap();
        let all = Settings::load_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&("a".into(), "1".into())));
        assert!(all.contains(&("b".into(), "2".into())));
    }

    // ── Feature flags ────────────────────────────────────────────

    #[test]
    fn load_features_from_empty_db() {
        let conn = fresh();
        let reg = Settings::load_features(&conn).unwrap();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn save_and_load_features_roundtrip() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();
        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
    }

    #[test]
    fn save_features_preserves_non_feature_settings() {
        let conn = fresh();
        Settings::set_store_name(&conn, "My Store").unwrap();
        Settings::set_default_currency(&conn, "IDR").unwrap();

        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();

        // Non-feature settings still there.
        assert_eq!(
            Settings::get_store_name(&conn).unwrap(),
            Some("My Store".into())
        );
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("IDR".into())
        );
    }

    #[test]
    fn prune_stale_features_removes_old_flags() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();
        let rows = reg.to_settings_rows();

        // Write feature rows directly, including a stale one.
        let mut all_rows = rows.clone();
        all_rows.push(("feature.old-flag".into(), "1".into()));
        Settings::set_batch(&conn, &all_rows).unwrap();

        // Prune against the current registry.
        let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
        assert_eq!(removed, 1);
        // Load back — should match simple_retail.
        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
    }

    // ── Typed store config ───────────────────────────────────────

    #[test]
    fn store_name_default_is_none() {
        let conn = fresh();
        assert_eq!(Settings::get_store_name(&conn).unwrap(), None);
    }

    #[test]
    fn set_and_get_store_name() {
        let conn = fresh();
        Settings::set_store_name(&conn, "Acme POS").unwrap();
        assert_eq!(
            Settings::get_store_name(&conn).unwrap(),
            Some("Acme POS".into())
        );
    }

    #[test]
    fn set_and_get_default_currency() {
        let conn = fresh();
        Settings::set_default_currency(&conn, "EUR").unwrap();
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("EUR".into())
        );
    }
}
