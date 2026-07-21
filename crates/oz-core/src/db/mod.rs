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
use crate::money::Currency;

/// Audit log queries (read / write).
pub mod audit;
/// Active cart persistence (survives restarts).
pub mod cart;
/// Cash payout CRUD (open / close / list).
pub mod cash_payouts;
/// Customer CRUD and lookups.
pub mod customers;
/// Gift cards — issue, redeem, top-up, freeze, balance checks.
pub mod gift_cards;
/// Inventory management CRUD (locations, shifts, thresholds, transaction logs).
pub mod inventory;
/// Kitchen Display System order CRUD.
pub mod kds;
/// Loyalty points / rewards CRUD.
pub mod loyalty;
/// Offline queue and sync state.
pub mod offline;
/// Payment CRUD (tenders, transactions).
pub mod payments;
/// CRUD for product bundles (group selling).
pub mod product_bundles;
/// Product CRUD and search.
pub mod products;
/// Promotion / discount CRUD.
pub mod promotions;
/// CRUD for purchase orders.
pub mod purchase_orders;
/// Recipe / modifier CRUD.
pub mod recipes;
/// Refund CRUD.
pub mod refunds;
/// Report generation queries.
pub mod reports;
/// Sale CRUD (transactions, lines, taxes).
pub mod sales;
/// Settings key/value CRUD.
pub mod settings;
/// Shift CRUD (open, close, reports).
pub mod shifts;
/// Staff / employee CRUD.
pub mod staff;
/// CRUD for stock counts / cycle counting.
pub mod stock_counts;
/// CRUD for stock transfers between terminals/stores.
pub mod stock_transfers;
/// Store profile CRUD.
pub mod store_profiles;
/// CRUD for suppliers.
pub mod suppliers;
/// CRUD for restaurant tables (floor plan, status management).
pub mod tables;
/// Tax rate CRUD.
pub mod tax;
/// Terminal override CRUD.
pub mod terminal_overrides;
/// Terminal profile CRUD.
pub mod terminal_profiles;
/// Terminal CRUD (registration, status).
pub mod terminals;
/// Workspace CRUD.
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
    /// Terminal ID for pub/sub message tagging (multi-terminal).
    /// Passed through to `Cache::publish_inventory_change` so other
    /// terminals can skip their own messages.
    pub terminal_id: Option<String>,
}

impl<'a> Store<'a> {
    /// Wrap an existing connection with no cache.
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            cache: None,
            terminal_id: None,
        }
    }

    /// Wrap an existing connection with a cache backend.
    pub fn with_cache(conn: &'a Connection, cache: Arc<dyn Cache>) -> Self {
        Self {
            conn,
            cache: Some(cache),
            terminal_id: None,
        }
    }

    /// Set the terminal ID for pub/sub message tagging.
    pub fn with_terminal_id(mut self, terminal_id: Option<String>) -> Self {
        self.terminal_id = terminal_id;
        self
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
            currency: cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?,
        },
        category_id: row.get("category_id")?,
        barcode: barcode_raw.and_then(|s| foundation::Barcode::new(&s).ok()),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        price_updated_at: row.get("price_updated_at")?,
        track_serial: row.get("track_serial").unwrap_or(false),
        product_type: product_type_str
            .as_deref()
            .and_then(crate::ProductType::parse_str)
            .unwrap_or_default(),
        version: row.get("version").unwrap_or(1),
    })
}
