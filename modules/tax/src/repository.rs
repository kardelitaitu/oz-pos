//! Tax Repository — database persistence layer for tax configuration.

use crate::models::TaxRate;
use rusqlite::{Connection, params};

/// Database access repository for tax rates.
pub struct TaxRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TaxRepository<'a> {
    /// Create a new `TaxRepository`.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a tax rate by ID.
    pub fn get_tax_rate(&self, id: &str) -> Result<Option<TaxRate>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        Ok(Some(TaxRate {
            id: row.get(0)?,
            name: row.get(1)?,
            rate_bps: row.get(2)?,
            is_default: row.get::<_, i64>(3)? != 0,
            is_inclusive: row.get::<_, i64>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        }))
    }
}
