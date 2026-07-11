//! Settings delegation — store settings, currencies, exchange rates.

use crate::Settings;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// Read a single setting.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, CoreError> {
        Settings::get(self.conn, key)
    }

    /// Write a single setting.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), CoreError> {
        Settings::set(self.conn, key, value)
    }

    /// Load the feature flag registry.
    pub fn load_features(&self) -> Result<crate::FeatureRegistry, CoreError> {
        Settings::load_features(self.conn)
    }

    /// Save the feature flag registry.
    pub fn save_features(&self, reg: &crate::FeatureRegistry) -> Result<(), CoreError> {
        Settings::save_features(self.conn, reg)
    }

    /// Prune stale feature rows.
    pub fn prune_stale_features(&self, reg: &crate::FeatureRegistry) -> Result<usize, CoreError> {
        Settings::prune_stale_features(self.conn, reg)
    }

    /// Get the store display name.
    pub fn get_store_name(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_name(self.conn)
    }

    /// Set the store display name.
    pub fn set_store_name(&self, name: &str) -> Result<(), CoreError> {
        Settings::set_store_name(self.conn, name)
    }

    /// Get the store address.
    pub fn get_store_address(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_address(self.conn)
    }

    /// Set the store address.
    pub fn set_store_address(&self, addr: &str) -> Result<(), CoreError> {
        Settings::set_store_address(self.conn, addr)
    }

    /// Get the store tax / VAT number.
    pub fn get_store_tax_id(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_tax_id(self.conn)
    }

    /// Set the store tax / VAT number.
    pub fn set_store_tax_id(&self, id: &str) -> Result<(), CoreError> {
        Settings::set_store_tax_id(self.conn, id)
    }

    /// Get the default currency.
    pub fn get_default_currency(&self) -> Result<Option<String>, CoreError> {
        Settings::get_default_currency(self.conn)
    }

    /// Set the default currency.
    pub fn set_default_currency(&self, code: &str) -> Result<(), CoreError> {
        Settings::set_default_currency(self.conn, code)
    }

    /// List all currencies from the ISO-4217 table.
    pub fn list_currencies(&self) -> Result<Vec<(String, String, u32, String)>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT code, name, minor_exponent, symbol FROM currencies ORDER BY code")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// List all exchange rates.
    pub fn list_exchange_rates(
        &self,
    ) -> Result<Vec<crate::exchange_rate::ExchangeRateRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate, source, effective_date, created_at
             FROM exchange_rates ORDER BY from_currency, to_currency",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::exchange_rate::ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Create a new exchange rate entry.
    pub fn create_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate: f64,
        source: &str,
        effective_date: &str,
    ) -> Result<crate::exchange_rate::ExchangeRateRow, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO exchange_rates (id, from_currency, to_currency, rate, source, effective_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from_currency, to_currency, rate, source, effective_date],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate, source, effective_date, created_at FROM exchange_rates WHERE id = ?1"
        )?;
        let row = stmt.query_row(rusqlite::params![id], |row| {
            Ok(crate::exchange_rate::ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(row)
    }

    /// Insert or replace an exchange rate.
    ///
    /// Uses `INSERT OR REPLACE` so that a rate with the same
    /// `(from_currency, to_currency, effective_date)` is replaced
    /// with a new row and a fresh id.
    pub fn upsert_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate: f64,
        source: &str,
        effective_date: &str,
    ) -> Result<crate::exchange_rate::ExchangeRateRow, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT OR REPLACE INTO exchange_rates (id, from_currency, to_currency, rate, source, effective_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from_currency, to_currency, rate, source, effective_date],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate, source, effective_date, created_at FROM exchange_rates WHERE id = ?1"
        )?;
        let row = stmt.query_row(rusqlite::params![id], |row| {
            Ok(crate::exchange_rate::ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(row)
    }

    /// Delete an exchange rate by ID.
    pub fn delete_exchange_rate(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM exchange_rates WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "exchange_rate",
                id: id.to_string(),
            });
        }
        Ok(())
    }
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

    fn usd() -> crate::money::Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> crate::Money {
        crate::Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    #[test]
    fn store_get_set_setting() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_setting("my.key").unwrap(), None);
        s.set_setting("my.key", "hello").unwrap();
        assert_eq!(s.get_setting("my.key").unwrap(), Some("hello".into()));
    }

    #[test]
    fn store_features_roundtrip() {
        let conn = fresh();
        let s = store(&conn);
        let reg = crate::FeatureRegistry::simple_retail();
        s.save_features(&reg).unwrap();
        let loaded = s.load_features().unwrap();
        assert_eq!(loaded, reg);
    }

    #[test]
    fn store_name_get_set() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_store_name().unwrap(), None);
        s.set_store_name("Acme").unwrap();
        assert_eq!(s.get_store_name().unwrap(), Some("Acme".into()));
    }

    #[test]
    fn store_default_currency_get_set() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_default_currency().unwrap(), None);
        s.set_default_currency("EUR").unwrap();
        assert_eq!(s.get_default_currency().unwrap(), Some("EUR".into()));
    }

    #[test]
    fn store_conn_returns_underlying_connection() {
        let conn = fresh();
        let s = store(&conn);
        let p = s
            .create_product("T1", "Test", price(1), None, None, 0, None)
            .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM products WHERE sku = 'T1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        drop(p);
    }

    #[test]
    fn backup_creates_snapshot_file() {
        let conn = fresh();
        // seed some data
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour) VALUES ('cat-test', 'Test', '#000')",
        )
        .unwrap();
        let s = store(&conn);

        let tmp = std::env::temp_dir().join("oz-test-backup.db");
        let _ = std::fs::remove_file(&tmp);

        s.backup(tmp.to_str().unwrap()).unwrap();

        let backup_conn = Connection::open(&tmp).unwrap();
        let count: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let _ = std::fs::remove_file(&tmp);
    }

    // ── Store Tax ID ───────────────────────────────────────────────────

    #[test]
    fn store_tax_id_default_none() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_store_tax_id().unwrap(), None);
    }

    #[test]
    fn store_tax_id_set_and_get() {
        let conn = fresh();
        let s = store(&conn);
        s.set_store_tax_id("12-3456789").unwrap();
        assert_eq!(s.get_store_tax_id().unwrap(), Some("12-3456789".into()));
    }

    #[test]
    fn store_tax_id_overwrites() {
        let conn = fresh();
        let s = store(&conn);
        s.set_store_tax_id("OLD").unwrap();
        s.set_store_tax_id("NEW").unwrap();
        assert_eq!(s.get_store_tax_id().unwrap(), Some("NEW".into()));
    }

    // ── Exchange Rates ─────────────────────────────────────────────────

    fn seed_currency(
        conn: &Connection,
        code: &str,
        numeric_code: &str,
        name: &str,
        exp: i32,
        sym: &str,
    ) {
        conn.execute(
            "INSERT OR IGNORE INTO currencies (code, numeric_code, name, minor_exponent, symbol) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![code, numeric_code, name, exp, sym],
        ).unwrap();
    }

    #[test]
    fn list_exchange_rates_empty() {
        let conn = fresh();
        let s = store(&conn);
        let rates = s.list_exchange_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn create_exchange_rate_and_find_in_list() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        seed_currency(&conn, "JPY", "392", "Japanese Yen", 0, "\u{a5}");
        let s = store(&conn);
        s.create_exchange_rate("USD", "EUR", 0.92, "ecb", "2026-06-28")
            .unwrap();
        s.create_exchange_rate("USD", "JPY", 149.50, "ecb", "2026-06-28")
            .unwrap();

        let rates = s.list_exchange_rates().unwrap();
        assert_eq!(rates.len(), 2);
        assert!(rates.iter().any(|r| r.to_currency == "EUR"));
        assert!(rates.iter().any(|r| r.to_currency == "JPY"));
    }

    #[test]
    fn create_exchange_rate_returns_row() {
        let conn = fresh();
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        seed_currency(&conn, "GBP", "826", "Pound", 2, "\u{a3}");
        let s = store(&conn);
        let row = s
            .create_exchange_rate("EUR", "GBP", 0.86, "ecb", "2026-06-28")
            .unwrap();
        assert_eq!(row.from_currency, "EUR");
        assert_eq!(row.to_currency, "GBP");
        assert!((row.rate - 0.86).abs() < 0.001);
    }

    #[test]
    fn delete_exchange_rate_removes() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "CAD", "124", "Canadian Dollar", 2, "CA$");
        let s = store(&conn);
        let row = s
            .create_exchange_rate("USD", "CAD", 1.36, "manual", "2026-06-28")
            .unwrap();
        s.delete_exchange_rate(&row.id).unwrap();
        let rates = s.list_exchange_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn upsert_exchange_rate_replaces_existing() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        let s = store(&conn);
        let first = s
            .create_exchange_rate("USD", "EUR", 0.90, "manual", "2026-07-01")
            .unwrap();
        let second = s
            .upsert_exchange_rate("USD", "EUR", 0.92, "auto-sync", "2026-07-01")
            .unwrap();
        // Same (from, to, date) but different id and updated rate
        assert_ne!(first.id, second.id);
        assert!((second.rate - 0.92).abs() < 0.001);
        assert_eq!(second.source, "auto-sync");
        // Only one row in the table
        let rates = s.list_exchange_rates().unwrap();
        assert_eq!(rates.len(), 1);
    }

    #[test]
    fn delete_exchange_rate_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.delete_exchange_rate("bad-id");
        assert!(matches!(result, Err(CoreError::NotFound { .. })));
    }
}
