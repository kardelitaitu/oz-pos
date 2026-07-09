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

use std::sync::Arc;

use crate::Money;
use crate::cache::Cache;
use crate::error::CoreError;

pub mod audit;
/// Active cart persistence (survives restarts).
pub mod cart;
pub mod cash_payouts;
pub mod customers;
/// Gift cards — issue, redeem, top-up, freeze, balance checks.
pub mod gift_cards;
pub mod kds;
pub mod loyalty;
pub mod offline;
pub mod payments;
/// CRUD for product bundles (group selling).
pub mod product_bundles;
pub mod products;
pub mod promotions;
/// CRUD for purchase orders.
pub mod purchase_orders;
pub mod recipes;
pub mod refunds;
pub mod reports;
pub mod sales;
pub mod settings;
pub mod shifts;
pub mod staff;
/// CRUD for stock counts / cycle counting.
pub mod stock_counts;
/// CRUD for stock transfers between terminals/stores.
pub mod stock_transfers;
pub mod store_profiles;
/// CRUD for suppliers.
pub mod suppliers;
/// CRUD for restaurant tables (floor plan, status management).
pub mod tables;
pub mod tax;
pub mod terminal_overrides;
pub mod terminal_profiles;
pub mod terminals;
pub mod workspaces;

// ── Re-exports ──────────────────────────────────────────────────────

pub use products::ProductWithDetails;
pub use reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    TopProductRow, WeeklyRevenueRow,
};
pub use sales::{CartLineTaxInput, DailySummaryRow, HeldCartFull, HeldCartRow, SalesByHourRow};
pub use shifts::{ShiftPaymentBreakdown, ShiftReport, ShiftSalesByHour};

// ── Store ────────────────────────────────────────────────────────────

/// Typed CRUD facade for the OZ-POS database.
///
/// All methods borrow `&self` and operate on the underlying
/// [`Connection`] directly. The caller is responsible for
/// synchronisation (e.g. `Mutex<Connection>`) and transaction
/// boundaries for multi-statement workflows.
pub struct Store<'a> {
    /// Underlying SQLite connection.
    pub conn: &'a Connection,
    /// Optional caching layer for product and inventory lookups.
    /// Uses `Arc` so multiple `Store` instances can share the same
    /// cache backend (e.g. Redis).
    pub cache: Option<Arc<dyn Cache>>,
}

impl<'a> Store<'a> {
    /// Wrap an existing connection with no cache.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn, cache: None }
    }

    /// Wrap an existing connection with a cache backend.
    pub fn with_cache(conn: &'a Connection, cache: Arc<dyn Cache>) -> Self {
        Self {
            conn,
            cache: Some(cache),
        }
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

/// Build a [`crate::Product`] from a `rusqlite::Row`. All `products` columns
/// must be present in the result set.
pub(crate) fn row_to_product(row: &rusqlite::Row) -> rusqlite::Result<crate::Product> {
    let sku_str: String = row.get("sku")?;
    let cur_str: String = row.get("currency")?;
    let barcode_raw: Option<String> = row.get("barcode")?;
    let product_type_str: Option<String> = row.get("product_type").ok();
    Ok(crate::Product {
        id: row.get("id")?,
        sku: crate::Sku::new(sku_str),
        name: row.get("name")?,
        price: Money {
            minor_units: row.get("price_minor")?,
            currency: cur_str.parse().expect("valid currency in database"),
        },
        category_id: row.get("category_id")?,
        barcode: barcode_raw.and_then(|s| foundation::Barcode::new(&s).ok()),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        price_updated_at: row.get("price_updated_at")?,
        track_serial: row.get("track_serial").unwrap_or(false),
        product_type: product_type_str
            .as_deref()
            .and_then(crate::ProductType::from_str)
            .unwrap_or_default(),
    })
}
