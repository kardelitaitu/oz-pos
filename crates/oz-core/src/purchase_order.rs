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

    // ── PurchaseOrder construction ───────────────────────────────

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
    fn new_generates_uuid() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        assert_eq!(po.id.len(), 36, "UUID v4 should be 36 chars");
        assert_eq!(po.id.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn new_trims_whitespace() {
        let po = PurchaseOrder::new("  PO-002  ", "sup-1");
        assert_eq!(po.po_number, "PO-002");
    }

    #[test]
    fn new_defaults_monetary_fields_to_zero() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        assert_eq!(po.subtotal_minor, 0);
        assert_eq!(po.tax_minor, 0);
        assert_eq!(po.total_minor, 0);
    }

    #[test]
    fn new_defaults_optional_fields() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        assert_eq!(po.expected_date, "");
        assert_eq!(po.received_date, None);
        assert_eq!(po.notes, "");
        assert_eq!(po.created_by, None);
        assert_eq!(po.created_at, "");
        assert_eq!(po.updated_at, "");
    }

    #[test]
    fn new_order_date_is_rfc3339() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        assert!(po.order_date.contains('T'), "should contain 'T' separator");
        assert!(po.order_date.ends_with('Z'), "should be UTC");
    }

    #[test]
    fn unique_ids_per_instance() {
        let a = PurchaseOrder::new("PO-001", "sup-1");
        let b = PurchaseOrder::new("PO-002", "sup-2");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn debug_output() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        let debug = format!("{:?}", po);
        assert!(debug.contains("PO-001"));
        assert!(debug.contains("sup-1"));
    }

    #[test]
    fn equality_depends_on_fields() {
        let a = PurchaseOrder::new("PO-001", "sup-1");
        let mut b = a.clone();
        b.status = "approved".into();
        assert_ne!(a, b);
    }

    // ── PurchaseOrderLine ────────────────────────────────────────

    #[test]
    fn new_line() {
        let line = PurchaseOrderLine::new("po-1");
        assert_eq!(line.po_id, "po-1");
        assert_eq!(line.qty, 0);
    }

    #[test]
    fn new_line_generates_uuid() {
        let line = PurchaseOrderLine::new("po-1");
        assert_eq!(line.id.len(), 36);
        assert_eq!(line.id.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn new_line_defaults_fields() {
        let line = PurchaseOrderLine::new("po-1");
        assert_eq!(line.sku, "");
        assert_eq!(line.product_name, "");
        assert_eq!(line.qty, 0);
        assert_eq!(line.unit_cost_minor, 0);
        assert_eq!(line.line_total_minor, 0);
    }

    #[test]
    fn line_debug_output() {
        let line = PurchaseOrderLine::new("po-1");
        let debug = format!("{:?}", line);
        assert!(debug.contains("po-1"));
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        let json = serde_json::to_string(&po).unwrap();
        let back: PurchaseOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(back, po);
    }

    #[test]
    fn purchase_order_with_lines_serde() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        let line = PurchaseOrderLine::new("po-1");
        let with_lines = PurchaseOrderWithLines {
            order: po.clone(),
            lines: vec![line],
            supplier_name: Some("Acme Corp".into()),
        };
        let json = serde_json::to_string(&with_lines).unwrap();
        let back: PurchaseOrderWithLines = serde_json::from_str(&json).unwrap();
        assert_eq!(back.order, po);
        assert_eq!(back.lines.len(), 1);
        assert_eq!(back.supplier_name, Some("Acme Corp".into()));
    }

    #[test]
    fn purchase_order_with_lines_empty_lines() {
        let po = PurchaseOrder::new("PO-001", "sup-1");
        let with_lines = PurchaseOrderWithLines {
            order: po.clone(),
            lines: vec![],
            supplier_name: None,
        };
        let json = serde_json::to_string(&with_lines).unwrap();
        let back: PurchaseOrderWithLines = serde_json::from_str(&json).unwrap();
        assert_eq!(back.order, po);
        assert!(back.lines.is_empty());
        assert_eq!(back.supplier_name, None);
    }
}
