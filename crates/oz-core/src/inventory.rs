//! Inventory domain type — stock tracking for products.
//!
//! An [`Inventory`] row ties a product to its current stock level.
//! The `qty` field is non-negative (enforced by the schema's `CHECK`
//! constraint and the [`Inventory::adjust_qty`] helper).

use serde::{Deserialize, Serialize};

/// Canonical default inventory location UUID.
///
/// **Frozen invariant** (ADR-18 §13-36): this exact UUID is seeded by
/// `crates/oz-core/migrations/078_inventory_locations.sql` and is the
/// column DEFAULT on `inventory.location_id` (migration 079),
/// `stock_movements.location_id` (migration 080), and
/// `stock_summary.location_id` (composite-PK rebuild in migration 089).
///
/// Use this constant in all Rust code that needs to reference the
/// canonical default location — never copy-paste the UUID literal.
/// Future SQL migrations that touch routing/backfill must continue
/// to use the same UUID and may comment-reference this constant
/// location for discoverability.
///
/// The companion canonical transit UUID (`01926b3a-0000-7000-8000-000000000002`)
/// intentionally has no Rust const — transit routing is invariant-free
/// and should remain a SQL-only concern under migration 078.
///
/// **Test assertion exception**: the LITERAL `01926b3a-0000-7000-8000-000000000001`
/// may remain hardcoded inside
/// `crates/oz-core/src/migrations.rs::migration_078_seeds_canonical_default_and_transit_locations`
/// — that test asserts the SQL seed value matches the canonical UUID on disk, so
/// substituting this const there would make the assertion circular / self-referential
/// (`const == seeded-const`). Future automated cleanup passes must NOT
/// "fix" that test by importing this constant. An inline sibling comment at the
/// test site provides the layered defence.
pub const CANONICAL_DEFAULT_LOCATION_UUID: &str = "01926b3a-0000-7000-8000-000000000001";

/// Stock level for a single product.
///
/// # Schema mapping
///
/// Maps 1:1 to the `inventory` table (migration `002_products.sql`).
/// The `product_id` is the primary key — there is at most one
/// inventory row per product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// FK to `products.id`.
    pub product_id: String,

    /// Current quantity on hand (≥ 0).
    pub qty: i64,

    /// ISO-8601 timestamp of the last stock adjustment.
    pub updated_at: String,
}

impl Inventory {
    /// Create a new inventory row for a product.
    ///
    /// # Panics
    ///
    /// Panics if `qty` is negative.
    pub fn new(product_id: impl Into<String>, qty: i64) -> Self {
        assert!(qty >= 0, "initial stock qty must be ≥ 0, got {qty}");
        Self {
            product_id: product_id.into(),
            qty,
            updated_at: String::new(),
        }
    }

    /// True when there is at least one unit in stock.
    #[must_use]
    pub fn is_in_stock(&self) -> bool {
        self.qty > 0
    }

    /// Adjust stock by `delta` (positive to restock, negative to sell).
    ///
    /// Returns `Some(new_qty)` on success, or `None` if the adjustment
    /// would cause the stock level to go negative.
    #[must_use]
    pub fn adjust_qty(&self, delta: i64) -> Option<i64> {
        self.qty.checked_add(delta).filter(|&v| v >= 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_inventory() {
        let inv = Inventory::new("prod-1", 50);
        assert_eq!(inv.product_id, "prod-1");
        assert_eq!(inv.qty, 50);
        assert!(inv.updated_at.is_empty());
    }

    #[test]
    fn zero_qty_is_allowed() {
        let inv = Inventory::new("prod-1", 0);
        assert_eq!(inv.qty, 0);
        assert!(!inv.is_in_stock());
    }

    #[test]
    #[should_panic(expected = "initial stock qty must be ≥ 0")]
    fn new_panics_on_negative_qty() {
        Inventory::new("prod-1", -1);
    }

    #[test]
    fn is_in_stock_when_qty_positive() {
        assert!(Inventory::new("p", 1).is_in_stock());
        assert!(Inventory::new("p", 100).is_in_stock());
    }

    #[test]
    fn is_not_in_stock_when_qty_zero() {
        assert!(!Inventory::new("p", 0).is_in_stock());
    }

    #[test]
    fn adjust_qty_sell() {
        let inv = Inventory::new("p", 10);
        assert_eq!(inv.adjust_qty(-3), Some(7));
        assert_eq!(inv.adjust_qty(-7), Some(3));
        // Selling exactly all stock is allowed.
        assert_eq!(inv.adjust_qty(-10), Some(0));
    }

    #[test]
    fn adjust_qty_restock() {
        let inv = Inventory::new("p", 10);
        assert_eq!(inv.adjust_qty(5), Some(15));
        assert_eq!(inv.adjust_qty(0), Some(10));
    }

    #[test]
    fn adjust_qty_rejects_oversell() {
        let inv = Inventory::new("p", 3);
        assert_eq!(inv.adjust_qty(-4), None);
    }

    #[test]
    fn adjust_qty_handles_large_values() {
        let inv = Inventory::new("p", 1_000_000);
        assert_eq!(inv.adjust_qty(500_000), Some(1_500_000));
        assert_eq!(inv.adjust_qty(-999_999), Some(1));
        assert_eq!(inv.adjust_qty(-1_000_001), None);
    }

    #[test]
    fn serde_roundtrip() {
        let inv = Inventory::new("prod-1", 42);
        let json = serde_json::to_string(&inv).unwrap();
        let back: Inventory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, inv);
    }

    #[test]
    fn adjust_qty_is_pure() {
        // adjust_qty returns a new value without mutating self.
        let inv = Inventory::new("p", 10);
        let _ = inv.adjust_qty(-5);
        assert_eq!(inv.qty, 10, "original should be unchanged");
    }

    #[test]
    fn debug_output() {
        let inv = Inventory::new("prod-1", 25);
        let debug = format!("{inv:?}");
        assert!(debug.contains("prod-1"));
        assert!(debug.contains("25"));
    }
}
