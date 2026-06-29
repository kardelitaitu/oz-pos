//! Settings store — key-value operations and feature flag integration.
//!
//! The raw key-value operations and typed store-configuration helpers
//! are delegated to [`platform_core::settings::Settings`]. This module
//! adds feature-flag integration on top via [`FeatureRegistry`].

use rusqlite::Connection;

use crate::error::CoreError;
use crate::features::FeatureRegistry;

/// Well-known settings keys for store configuration.
pub mod keys {
    pub use platform_core::settings::keys::*;
}

/// Typed access to the `settings` table.
///
/// Raw get/set/remove/batch operations are delegated to
/// `platform-core`; feature-flag methods are implemented here.
pub struct Settings;

// ── Raw key-value helpers (delegated to platform-core) ──────────────

impl Settings {
    /// Read a single setting by key. Returns `None` if the key doesn't exist.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get(conn, key)?)
    }

    /// Insert or update a setting.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set(conn, key, value)?)
    }

    /// Delete a setting. Returns `true` if the key existed.
    pub fn remove(conn: &Connection, key: &str) -> Result<bool, CoreError> {
        Ok(platform_core::settings::Settings::remove(conn, key)?)
    }

    /// Load every row from the `settings` table as `(key, value)` pairs.
    pub fn load_all(conn: &Connection) -> Result<Vec<(String, String)>, CoreError> {
        Ok(platform_core::settings::Settings::load_all(conn)?)
    }

    /// Write multiple settings inside a single transaction.
    pub fn set_batch(conn: &Connection, rows: &[(String, String)]) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_batch(conn, rows)?)
    }

    /// Get the store display name.
    pub fn get_store_name(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_store_name(conn)?)
    }

    /// Set the store display name.
    pub fn set_store_name(conn: &Connection, name: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_store_name(
            conn, name,
        )?)
    }

    /// Get the store address (printed on receipts).
    pub fn get_store_address(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_store_address(conn)?)
    }

    /// Set the store address.
    pub fn set_store_address(conn: &Connection, addr: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_store_address(
            conn, addr,
        )?)
    }

    /// Get the store tax / VAT registration number.
    pub fn get_store_tax_id(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_store_tax_id(conn)?)
    }

    /// Set the store tax / VAT registration number.
    pub fn set_store_tax_id(conn: &Connection, id: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_store_tax_id(
            conn, id,
        )?)
    }

    /// Get the default currency code (ISO-4217).
    pub fn get_default_currency(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_default_currency(
            conn,
        )?)
    }

    /// Set the default currency code.
    pub fn set_default_currency(conn: &Connection, code: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_default_currency(
            conn, code,
        )?)
    }

    /// Whether to show the currency symbol prefix on receipt amounts.
    pub fn get_receipt_show_currency(conn: &Connection) -> Result<bool, CoreError> {
        Ok(platform_core::settings::Settings::get_receipt_show_currency(conn)?)
    }

    /// Set whether to show the currency symbol prefix.
    pub fn set_receipt_show_currency(conn: &Connection, on: bool) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_receipt_show_currency(conn, on)?)
    }

    /// Decimal separator style: `"dot"`, `"comma"`, or `"none"`.
    pub fn get_receipt_decimal_separator(conn: &Connection) -> Result<String, CoreError> {
        Ok(platform_core::settings::Settings::get_receipt_decimal_separator(conn)?)
    }

    /// Set the decimal separator style.
    pub fn set_receipt_decimal_separator(conn: &Connection, val: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_receipt_decimal_separator(conn, val)?)
    }

    /// Whether to show the tax line on receipts.
    pub fn get_receipt_show_tax(conn: &Connection) -> Result<bool, CoreError> {
        Ok(platform_core::settings::Settings::get_receipt_show_tax(
            conn,
        )?)
    }

    /// Set whether to show the tax line.
    pub fn set_receipt_show_tax(conn: &Connection, on: bool) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_receipt_show_tax(
            conn, on,
        )?)
    }

    /// Get the receipt footer text (empty = no footer).
    pub fn get_receipt_footer(conn: &Connection) -> Result<String, CoreError> {
        Ok(platform_core::settings::Settings::get_receipt_footer(conn)?)
    }

    /// Set the receipt footer text.
    pub fn set_receipt_footer(conn: &Connection, text: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_receipt_footer(
            conn, text,
        )?)
    }

    /// Paper width: `"standard"` (80 mm) or `"narrow"` (58 mm).
    pub fn get_receipt_paper_width(conn: &Connection) -> Result<String, CoreError> {
        Ok(platform_core::settings::Settings::get_receipt_paper_width(
            conn,
        )?)
    }

    /// Set the paper width.
    pub fn set_receipt_paper_width(conn: &Connection, val: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_receipt_paper_width(
            conn, val,
        )?)
    }

    /// Get the configured sync server URL.
    pub fn get_sync_server_url(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_sync_server_url(
            conn,
        )?)
    }

    /// Set the sync server URL.
    pub fn set_sync_server_url(conn: &Connection, url: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_sync_server_url(
            conn, url,
        )?)
    }

    /// Get the sync API key.
    pub fn get_sync_api_key(conn: &Connection) -> Result<Option<String>, CoreError> {
        Ok(platform_core::settings::Settings::get_sync_api_key(conn)?)
    }

    /// Set the sync API key.
    pub fn set_sync_api_key(conn: &Connection, key: &str) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_sync_api_key(
            conn, key,
        )?)
    }

    /// Check if sync is enabled.
    pub fn is_sync_enabled(conn: &Connection) -> Result<bool, CoreError> {
        Ok(platform_core::settings::Settings::is_sync_enabled(conn)?)
    }

    /// Enable or disable sync.
    pub fn set_sync_enabled(conn: &Connection, enabled: bool) -> Result<(), CoreError> {
        Ok(platform_core::settings::Settings::set_sync_enabled(
            conn, enabled,
        )?)
    }
}

// ── Feature flags (oz-core specific) ─────────────────────────────────

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
    pub fn save_features(conn: &Connection, reg: &FeatureRegistry) -> Result<(), CoreError> {
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
        let enabled_keys: std::collections::HashSet<String> =
            reg.to_settings_rows().into_iter().map(|(k, _)| k).collect();

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
        assert_eq!(
            Settings::get(&conn, "test.key").unwrap(),
            Some("hello".into())
        );
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
        let rows: Vec<(String, String)> = vec![("a".into(), "1".into()), ("b".into(), "2".into())];
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

        let mut all_rows = rows.clone();
        all_rows.push(("feature.old-flag".into(), "1".into()));
        Settings::set_batch(&conn, &all_rows).unwrap();

        let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
        assert_eq!(removed, 1);
        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
    }

    #[test]
    fn prune_stale_features_noop_when_no_stale() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();

        let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
        assert_eq!(removed, 0);
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
