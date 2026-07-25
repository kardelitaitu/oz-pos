//! Currency/Exchange repository — database persistence for exchange rates.

use rusqlite::Connection;

use crate::error::CurrencyError;
use crate::models::ExchangeRateRow;

/// Database access repository for currency and exchange-rate data.
pub struct CurrencyRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CurrencyRepository<'a> {
    /// Create a new `CurrencyRepository` borrowing a SQLite connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all exchange rates ordered by `(from_currency, to_currency)`.
    pub fn list_exchange_rates(&self) -> Result<Vec<ExchangeRateRow>, CurrencyError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate_millionths, source, effective_date, created_at
             FROM exchange_rates ORDER BY from_currency, to_currency",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate_millionths: row.get(3)?,
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
    ///
    /// `rate_millionths` is the fixed-point exchange rate at a 6-decimal
    /// scale (e.g. `0.92` → `920_000`). Strictly positive — zero and
    /// negative rates are rejected at this layer.
    pub fn create_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate_millionths: i64,
        source: &str,
        effective_date: &str,
    ) -> Result<ExchangeRateRow, CurrencyError> {
        if rate_millionths <= 0 {
            return Err(CurrencyError::validation(
                "rate_millionths",
                "rate must be strictly positive; zero and negative exchange rates are not valid",
            ));
        }
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO exchange_rates (id, from_currency, to_currency, rate_millionths, source, effective_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from_currency, to_currency, rate_millionths, source, effective_date],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate_millionths, source, effective_date, created_at FROM exchange_rates WHERE id = ?1"
        )?;
        let row = stmt.query_row(rusqlite::params![id], |row| {
            Ok(ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate_millionths: row.get(3)?,
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
    /// with a new row and a fresh id. `rate_millionths` is at the 6-decimal
    /// scale. Zero and negative rates are rejected.
    pub fn upsert_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate_millionths: i64,
        source: &str,
        effective_date: &str,
    ) -> Result<ExchangeRateRow, CurrencyError> {
        if rate_millionths <= 0 {
            return Err(CurrencyError::validation(
                "rate_millionths",
                "rate must be strictly positive; zero and negative exchange rates are not valid",
            ));
        }
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT OR REPLACE INTO exchange_rates (id, from_currency, to_currency, rate_millionths, source, effective_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from_currency, to_currency, rate_millionths, source, effective_date],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate_millionths, source, effective_date, created_at FROM exchange_rates WHERE id = ?1"
        )?;
        let row = stmt.query_row(rusqlite::params![id], |row| {
            Ok(ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate_millionths: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(row)
    }

    /// Delete an exchange rate by ID.
    pub fn delete_exchange_rate(&self, id: &str) -> Result<(), CurrencyError> {
        let affected = self.conn.execute(
            "DELETE FROM exchange_rates WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if affected == 0 {
            return Err(CurrencyError::NotFound {
                entity: "exchange_rate",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        oz_core::migrations::fresh_db()
    }

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
        )
        .unwrap();
    }

    #[test]
    fn list_exchange_rates_empty() {
        let conn = fresh();
        let repo = CurrencyRepository::new(&conn);
        let rates = repo.list_exchange_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn create_exchange_rate_and_find_in_list() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        seed_currency(&conn, "JPY", "392", "Japanese Yen", 0, "\u{a5}");
        let repo = CurrencyRepository::new(&conn);
        repo.create_exchange_rate("USD", "EUR", 920_000, "ecb", "2026-06-28")
            .unwrap();
        repo.create_exchange_rate("USD", "JPY", 149_500_000, "ecb", "2026-06-28")
            .unwrap();

        let rates = repo.list_exchange_rates().unwrap();
        assert_eq!(rates.len(), 2);
        assert!(rates.iter().any(|r| r.to_currency == "EUR"));
        assert!(rates.iter().any(|r| r.to_currency == "JPY"));
    }

    #[test]
    fn create_exchange_rate_rejects_zero_rate() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        let repo = CurrencyRepository::new(&conn);
        let result = repo.create_exchange_rate("USD", "EUR", 0, "manual", "2026-01-01");
        assert!(result.is_err(), "zero rate must be rejected");
    }

    #[test]
    fn create_exchange_rate_rejects_negative_rate() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        let repo = CurrencyRepository::new(&conn);
        let result = repo.create_exchange_rate("USD", "EUR", -500_000, "manual", "2026-01-01");
        assert!(result.is_err(), "negative rate must be rejected");
    }

    #[test]
    fn upsert_exchange_rate_replaces_existing() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "EUR", "978", "Euro", 2, "\u{20ac}");
        let repo = CurrencyRepository::new(&conn);
        let first = repo
            .create_exchange_rate("USD", "EUR", 900_000, "manual", "2026-07-01")
            .unwrap();
        let second = repo
            .upsert_exchange_rate("USD", "EUR", 920_000, "auto-sync", "2026-07-01")
            .unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(second.rate_millionths, 920_000);
        assert_eq!(second.source, "auto-sync");
        let rates = repo.list_exchange_rates().unwrap();
        assert_eq!(rates.len(), 1);
    }

    #[test]
    fn delete_exchange_rate_removes() {
        let conn = fresh();
        seed_currency(&conn, "USD", "840", "US Dollar", 2, "$");
        seed_currency(&conn, "CAD", "124", "Canadian Dollar", 2, "CA$");
        let repo = CurrencyRepository::new(&conn);
        let row = repo
            .create_exchange_rate("USD", "CAD", 1_360_000, "manual", "2026-06-28")
            .unwrap();
        repo.delete_exchange_rate(&row.id).unwrap();
        let rates = repo.list_exchange_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn delete_exchange_rate_not_found() {
        let conn = fresh();
        let repo = CurrencyRepository::new(&conn);
        let result = repo.delete_exchange_rate("bad-id");
        assert!(matches!(result, Err(CurrencyError::NotFound { .. })));
    }
}
