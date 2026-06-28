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
    /// Store street address (printed on receipts).
    pub const STORE_ADDRESS: &str = "store.address";
    /// Store tax / VAT registration number.
    pub const STORE_TAX_ID: &str = "store.tax_id";
    /// Default ISO-4217 currency code. Default: `"USD"`.
    pub const DEFAULT_CURRENCY: &str = "store.default_currency";
    /// Store preset name (e.g., `"simple-retail"`, `"restaurant"`).
    pub const STORE_PRESET: &str = "store.preset";
    /// Whether the Setup Wizard has been completed.
    pub const SETUP_COMPLETE: &str = "store.setup_complete";

    // ── Receipt display settings ───────────────────────────────────
    /// Show currency symbol prefix on amounts. `"1"` or `"0"`. Default `"0"`.
    pub const RECEIPT_SHOW_CURRENCY: &str = "receipt.show_currency";
    /// Decimal separator style: `"dot"`, `"comma"`, or `"none"`. Default `"dot"`.
    pub const RECEIPT_DECIMAL_SEP: &str = "receipt.decimal_separator";
    /// Show tax line on receipts. `"1"` or `"0"`. Default `"1"`.
    pub const RECEIPT_SHOW_TAX: &str = "receipt.show_tax";
    /// Receipt footer text. Empty string means no footer.
    pub const RECEIPT_FOOTER: &str = "receipt.footer";
    /// Paper width: `"standard"` (80 mm) or `"narrow"` (58 mm). Default `"standard"`.
    pub const RECEIPT_PAPER_WIDTH: &str = "receipt.paper_width";
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

    /// Get the store address (printed on receipts).
    pub fn get_store_address(conn: &Connection) -> Result<Option<String>, CoreError> {
        Self::get(conn, keys::STORE_ADDRESS)
    }

    /// Set the store address.
    pub fn set_store_address(conn: &Connection, addr: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::STORE_ADDRESS, addr)
    }

    /// Get the store tax / VAT registration number.
    pub fn get_store_tax_id(conn: &Connection) -> Result<Option<String>, CoreError> {
        Self::get(conn, keys::STORE_TAX_ID)
    }

    /// Set the store tax / VAT registration number.
    pub fn set_store_tax_id(conn: &Connection, id: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::STORE_TAX_ID, id)
    }

    /// Get the default currency code (ISO-4217).
    pub fn get_default_currency(conn: &Connection) -> Result<Option<String>, CoreError> {
        Self::get(conn, keys::DEFAULT_CURRENCY)
    }

    /// Set the default currency code.
    pub fn set_default_currency(conn: &Connection, code: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::DEFAULT_CURRENCY, code)
    }

    // ── Receipt display settings ───────────────────────────────────

    /// Whether to show the currency symbol prefix on receipt amounts.
    pub fn get_receipt_show_currency(conn: &Connection) -> Result<bool, CoreError> {
        Ok(Self::get(conn, keys::RECEIPT_SHOW_CURRENCY)?
            .as_deref()
            .unwrap_or("0")
            == "1")
    }

    /// Set whether to show the currency symbol prefix.
    pub fn set_receipt_show_currency(conn: &Connection, on: bool) -> Result<(), CoreError> {
        Self::set(conn, keys::RECEIPT_SHOW_CURRENCY, if on { "1" } else { "0" })
    }

    /// Decimal separator style: `"dot"`, `"comma"`, or `"none"`.
    pub fn get_receipt_decimal_separator(conn: &Connection) -> Result<String, CoreError> {
        Ok(Self::get(conn, keys::RECEIPT_DECIMAL_SEP)?
            .unwrap_or_else(|| "dot".into()))
    }

    /// Set the decimal separator style.
    pub fn set_receipt_decimal_separator(conn: &Connection, val: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::RECEIPT_DECIMAL_SEP, val)
    }

    /// Whether to show the tax line on receipts.
    pub fn get_receipt_show_tax(conn: &Connection) -> Result<bool, CoreError> {
        Ok(Self::get(conn, keys::RECEIPT_SHOW_TAX)?
            .as_deref()
            .unwrap_or("1")
            == "1")
    }

    /// Set whether to show the tax line.
    pub fn set_receipt_show_tax(conn: &Connection, on: bool) -> Result<(), CoreError> {
        Self::set(conn, keys::RECEIPT_SHOW_TAX, if on { "1" } else { "0" })
    }

    /// Get the receipt footer text (empty = no footer).
    pub fn get_receipt_footer(conn: &Connection) -> Result<String, CoreError> {
        Ok(Self::get(conn, keys::RECEIPT_FOOTER)?.unwrap_or_default())
    }

    /// Set the receipt footer text.
    pub fn set_receipt_footer(conn: &Connection, text: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::RECEIPT_FOOTER, text)
    }

    /// Paper width: `"standard"` (80 mm) or `"narrow"` (58 mm).
    pub fn get_receipt_paper_width(conn: &Connection) -> Result<String, CoreError> {
        Ok(Self::get(conn, keys::RECEIPT_PAPER_WIDTH)?
            .unwrap_or_else(|| "standard".into()))
    }

    /// Set the paper width.
    pub fn set_receipt_paper_width(conn: &Connection, val: &str) -> Result<(), CoreError> {
        Self::set(conn, keys::RECEIPT_PAPER_WIDTH, val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::Feature;

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

    #[test]
    fn prune_stale_features_noop_when_no_stale() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();

        let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
        assert_eq!(removed, 0);
        // Registry unchanged.
        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
    }

    #[test]
    fn prune_stale_features_empty_registry_removes_all() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();

        let empty_reg = FeatureRegistry::new();
        let removed = Settings::prune_stale_features(&conn, &empty_reg).unwrap();
        assert_eq!(removed, reg.count());

        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded.count(), 0);
    }

    #[test]
    fn prune_stale_features_preserves_non_feature_keys() {
        let conn = fresh();
        Settings::set_store_name(&conn, "My Store").unwrap();
        Settings::set_default_currency(&conn, "EUR").unwrap();

        // Write one feature flag that will be pruned.
        let reg = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &reg).unwrap();

        // Now prune with a different registry (restaurant) — only feature
        // rows that differ should be removed; store.name and default_currency
        // should survive.
        let restaurant = FeatureRegistry::restaurant();
        let removed = Settings::prune_stale_features(&conn, &restaurant).unwrap();
        assert!(removed > 0);

        assert_eq!(
            Settings::get_store_name(&conn).unwrap(),
            Some("My Store".into())
        );
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("EUR".into())
        );
    }

    #[test]
    fn prune_stale_features_multiple_stale() {
        let conn = fresh();
        // Write a bunch of stale feature flags.
        let stale_rows: Vec<(String, String)> = vec![
            ("feature.old-1".into(), "1".into()),
            ("feature.old-2".into(), "1".into()),
            ("feature.old-3".into(), "1".into()),
            ("feature.old-4".into(), "1".into()),
            ("feature.old-5".into(), "1".into()),
        ];
        Settings::set_batch(&conn, &stale_rows).unwrap();

        let reg = FeatureRegistry::simple_retail();
        let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
        assert_eq!(removed, 5);
    }

    // ── save_features edge cases ─────────────────────────────────

    #[test]
    fn save_features_empty_registry() {
        let conn = fresh();
        let reg = FeatureRegistry::new();
        Settings::save_features(&conn, &reg).unwrap();

        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded.count(), 0);
    }

    #[test]
    fn save_features_overwrites_shared_keys() {
        let conn = fresh();

        // Save simple retail.
        let retail = FeatureRegistry::simple_retail();
        Settings::save_features(&conn, &retail).unwrap();

        // Overwrite with restaurant — shared keys (e.g. cash-payment,
        // receipt-printing) are upserted, but retail-only keys
        // (simple-retail, barcode-scanning) are NOT removed.
        let restaurant = FeatureRegistry::restaurant();
        Settings::save_features(&conn, &restaurant).unwrap();

        let loaded = Settings::load_features(&conn).unwrap();

        // Restaurant features are present.
        assert!(loaded.is_enabled(Feature::Restaurant));
        assert!(loaded.is_enabled(Feature::CashPayment));
        assert!(loaded.is_enabled(Feature::DiscountEngine));

        // Stale retail-only keys are still present because
        // save_features does NOT prune.
        assert!(loaded.is_enabled(Feature::SimpleRetail));
        assert!(loaded.is_enabled(Feature::BarcodeScanning));

        // After pruning, stale keys are gone and only restaurant remains.
        Settings::prune_stale_features(&conn, &restaurant).unwrap();
        let loaded = Settings::load_features(&conn).unwrap();
        assert!(!loaded.is_enabled(Feature::SimpleRetail));
        assert!(!loaded.is_enabled(Feature::BarcodeScanning));
        assert!(loaded.is_enabled(Feature::Restaurant));
        assert_eq!(loaded, restaurant);
    }

    #[test]
    fn save_features_idempotent() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();

        // Save twice.
        Settings::save_features(&conn, &reg).unwrap();
        Settings::save_features(&conn, &reg).unwrap();

        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
        assert_eq!(loaded.count(), reg.count());
    }

    #[test]
    fn save_features_stale_rows_remain_until_pruned() {
        let conn = fresh();
        let reg = FeatureRegistry::simple_retail();

        // Write a stale feature row with a KNOWN key that isn't in
        // simple_retail (card-payment). This will be picked up by
        // load_features because from_settings_rows parses known keys.
        Settings::set(&conn, "feature.card-payment", "1").unwrap();

        // Save features does NOT prune — stale rows stay.
        Settings::save_features(&conn, &reg).unwrap();

        let loaded = Settings::load_features(&conn).unwrap();
        // card-payment is not in simple_retail, so count = reg + 1.
        assert_eq!(loaded.count(), reg.count() + 1);
        assert!(loaded.is_enabled(Feature::CardPayment));

        // After pruning, card-payment is removed.
        Settings::prune_stale_features(&conn, &reg).unwrap();
        let loaded = Settings::load_features(&conn).unwrap();
        assert_eq!(loaded, reg);
        assert!(!loaded.is_enabled(Feature::CardPayment));
    }

    // ── Batch edge cases ─────────────────────────────────────────

    #[test]
    fn set_batch_empty_vec() {
        let conn = fresh();
        Settings::set_batch(&conn, &[]).unwrap();
        // No crash, no rows.
        assert_eq!(Settings::load_all(&conn).unwrap().len(), 0);
    }

    #[test]
    fn set_batch_overwrites_existing_keys() {
        let conn = fresh();
        Settings::set(&conn, "k", "old").unwrap();

        let rows = vec![("k".into(), "new".into())];
        Settings::set_batch(&conn, &rows).unwrap();

        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("new".into()));
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

    #[test]
    fn remove_idempotent() {
        let conn = fresh();
        Settings::set(&conn, "k", "v").unwrap();
        assert!(Settings::remove(&conn, "k").unwrap());
        assert!(!Settings::remove(&conn, "k").unwrap());
    }
}
