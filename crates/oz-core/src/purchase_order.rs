//! Purchase order domain types — stock procurement from suppliers.
//!
//! A [`PurchaseOrder`] represents a request to buy stock from a supplier.
//! Each order contains multiple [`PurchaseOrderLine`] items. The
//! [`PurchaseOrderWithLines`] composite type includes the line items and
//! supplier name for front-end display.

use serde::{Deserialize, Serialize};

/// A purchase order placed with a supplier.
///
/// # Schema mapping
///
/// Maps 1:1 to the `purchase_orders` table (migration `047_purchase_orders.sql`).
/// Monetary fields use integer minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrder {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Human-readable purchase order number (unique).
    pub po_number: String,
    /// Foreign key to [`crate::Supplier`].
    pub supplier_id: String,
    /// Order status: draft, pending, approved, received, cancelled.
    pub status: String,
    /// ISO-8601 date the order was placed.
    pub order_date: String,
    /// ISO-8601 expected delivery date.
    pub expected_date: String,
    /// ISO-8601 date the order was received (None until received).
    pub received_date: Option<String>,
    /// Subtotal in minor units (sum of line totals).
    pub subtotal_minor: i64,
    /// Tax amount in minor units.
    pub tax_minor: i64,
    /// Total amount in minor units (subtotal + tax).
    pub total_minor: i64,
    /// Free-form notes.
    pub notes: String,
    /// Optional foreign key to [`crate::User`] who created the order.
    pub created_by: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

/// A single line item within a purchase order.
///
/// # Schema mapping
///
/// Maps 1:1 to the `purchase_order_lines` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Foreign key to the parent [`PurchaseOrder`].
    pub po_id: String,
    /// SKU of the product being ordered.
    pub sku: String,
    /// Display name of the product (denormalised).
    pub product_name: String,
    /// Quantity ordered.
    pub qty: i64,
    /// Unit cost in minor units.
    pub unit_cost_minor: i64,
    /// Line total in minor units (qty × unit_cost).
    pub line_total_minor: i64,
}

/// A [`PurchaseOrder`] enriched with its line items and supplier name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderWithLines {
    /// The core purchase order fields (flattened into the parent JSON).
    #[serde(flatten)]
    pub order: PurchaseOrder,
    /// Line items for this order.
    pub lines: Vec<PurchaseOrderLine>,
    /// Display name from `suppliers.name`, if linked.
    pub supplier_name: Option<String>,
}

impl PurchaseOrder {
    /// Create a new purchase order with the given PO number and supplier.
    ///
    /// Generates a fresh UUID for `id`. Defaults status to `"draft"` and
    /// order date to the current UTC time.
    ///
    /// # Panics
    ///
    /// Panics if `po_number` is empty after trimming.
    pub fn new(po_number: impl Into<String>, supplier_id: impl Into<String>) -> Self {
        let po_number = po_number.into().trim().to_owned();
        assert!(!po_number.is_empty(), "PO number must not be empty");

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            po_number,
            supplier_id: supplier_id.into(),
            status: "draft".into(),
            order_date: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            expected_date: String::new(),
            received_date: None,
            subtotal_minor: 0,
            tax_minor: 0,
            total_minor: 0,
            notes: String::new(),
            created_by: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

impl PurchaseOrderLine {
    /// Create a new line item belonging to the given purchase order.
    pub fn new(po_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            po_id: po_id.into(),
            sku: String::new(),
            product_name: String::new(),
            qty: 0,
            unit_cost_minor: 0,
            line_total_minor: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_purchase_order() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        assert_eq!(po.po_number, "PO-001");
        assert_eq!(po.supplier_id, "sup-1");
        assert_eq!(po.status, "draft");
        assert!(!po.order_date.is_empty());
    }

    #[test]
    #[should_panic(expected = "PO number must not be empty")]
    fn new_panics_on_empty_po_number() {
        PurchaseOrder::new("  ", "sup-1");
    }

    #[test]
    fn new_line() {
        let line = PurchaseOrderLine::new("po-1");
        assert_eq!(line.po_id, "po-1");
        assert_eq!(line.qty, 0);
    }

    #[test]
    fn serde_roundtrip() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        let json = serde_json::to_string(&po).unwrap();
        let back: PurchaseOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(back, po);
    }
}
