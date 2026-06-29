//! Database facade — typed CRUD for every domain entity.
//!
//! The [`Store`] is a lightweight borrow-wrapper around a
//! `&rusqlite::Connection`. It holds no state of its own; callers
//! create a `Store` on the fly and call methods that map directly to
//! SQL queries. All writes that touch more than one row use
//! `unchecked_transaction` for atomicity.
//!
//! Domain methods are organised into sub-modules, each one implementing
//! `impl Store<'_>` for a logical domain (products, sales, customers, etc.).

use rusqlite::Connection;

use crate::Money;
use crate::error::CoreError;

pub mod audit;
pub mod cash_payouts;
pub mod customers;
pub mod offline;
pub mod payments;
pub mod products;
pub mod refunds;
pub mod sales;
pub mod settings;
pub mod shifts;
pub mod staff;
pub mod store_profiles;
pub mod tax;
pub mod terminals;

// ── Re-exports ──────────────────────────────────────────────────────

pub use products::ProductWithDetails;
pub use sales::{DailySummaryRow, HeldCartFull, HeldCartRow, SalesByHourRow};
pub use shifts::{ShiftPaymentBreakdown, ShiftReport, ShiftSalesByHour};

// ── Store ────────────────────────────────────────────────────────────

/// Typed CRUD facade for the OZ-POS database.
///
/// All methods borrow `&self` and operate on the underlying
/// [`Connection`] directly. The caller is responsible for
/// synchronisation (e.g. `Mutex<Connection>`) and transaction
/// boundaries for multi-statement workflows.
pub struct Store<'a> {
    conn: &'a Connection,
}

impl<'a> Store<'a> {
    /// Wrap an existing connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Return a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        self.conn
    }
}

// ── Backup / Export ────────────────────────────────────────────────────

impl Store<'_> {
    /// Create a snapshot of the database to a file at `output_path`.
    ///
    /// Uses SQLite's online backup API so the source connection can
    /// remain in use during the copy.
    pub fn backup(&self, output_path: &str) -> Result<(), CoreError> {
        // VACUUM INTO creates a clean, optimized database snapshot.
        let escaped = output_path.replace('\'', "''");
        let sql = format!("VACUUM INTO '{escaped}'");
        self.conn.execute_batch(&sql)?;
        Ok(())
    }
}

// ── Default helpers for row mapping ──────────────────────────────────

/// Build a [`crate::Product`] from a `rusqlite::Row`. All 9 `products` columns
/// must be present in the result set.
pub(crate) fn row_to_product(row: &rusqlite::Row) -> rusqlite::Result<crate::Product> {
    let sku_str: String = row.get("sku")?;
    let cur_str: String = row.get("currency")?;
    Ok(crate::Product {
        id: row.get("id")?,
        sku: crate::Sku::new(sku_str),
        name: row.get("name")?,
        price: Money {
            minor_units: row.get("price_minor")?,
            currency: cur_str.parse().expect("valid currency in database"),
        },
        category_id: row.get("category_id")?,
        barcode: row.get("barcode")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}
