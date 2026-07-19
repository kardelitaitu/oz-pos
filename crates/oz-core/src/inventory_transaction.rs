//! Inventory audit session (ADR-18 §9a + §9b, ADR-19 §3.2).
//!
//! An [`InventoryTransaction`] is a single audit session that groups one or
//! more [`InventoryTransactionLine`] per-SKU detail rows. Every call to
//! [`Store::adjust_stock_at_location_with_reason`](crate::db::Store::adjust_stock_at_location_with_reason)
//! writes a `stock_movements` row that links back (migration 085) to the
//! session header via `inventory_transaction_id`.
//!
//! Schemas map 1:1 to migration `084_inventory_transaction_audit.sql`:
//! - `inventory_transactions`: session header (id, type, location_id, staff_id, transfer_id?, purchase_order_id?, notes, created_at)
//! - `inventory_transaction_lines`: per-SKU detail (id, transaction_id, sku, product_name, qty, barcode_scanned?, sort_order)

use serde::{Deserialize, Serialize};

/// Strongly-typed identifier for an inventory transaction audit session.
///
/// Wraps the UUID v7 string from the `inventory_transactions.id` column.
/// Newtype pattern: `String` inner, `Deref<Target=str>`, `Display`,
/// `From<&str>` / `From<String>` for ergonomic construction from migration
/// code + resolver return values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InventoryTransactionId(String);

impl InventoryTransactionId {
    /// Generate a new UUID v7 identifier for a freshly-opened session.
    #[must_use]
    pub fn new() -> Self {
        Self(crate::new_id())
    }

    /// Borrow the underlying UUID string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for InventoryTransactionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for InventoryTransactionId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for InventoryTransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for InventoryTransactionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for InventoryTransactionId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// Lifecycle / purpose of an [`InventoryTransaction`] audit session.
///
/// Stored as a kebab-case string in the `inventory_transactions.type` column.
/// Unknown values round-trip as `None` from [`Self::from_stored_str`] so a
/// future migration adding a new type fails LOUDLY rather than silently
/// truncating audit history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InventoryTransactionType {
    /// A point-of-sale deduction (sale completion per ADR-19 §2).
    Sale,
    /// A void + compensating credit (ADR-19 §5.3).
    Void,
    /// A refund of an already-completed sale (ADR-19 §5.3).
    Refund,
    /// A stock transfer (in / out / in-transit phase per ADR-18 §7).
    Transfer,
    /// A purchase-order receive (ADR-18 §8).
    PurchaseOrderReceive,
    /// A stock-count adjustment (ADR-18 §9e).
    StockCount,
    /// A manager-override manual adjustment.
    ManualAdjustment,
}

impl InventoryTransactionType {
    /// Stable string form for SQL row storage.
    #[must_use]
    pub fn as_stored_str(&self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::Void => "void",
            Self::Refund => "refund",
            Self::Transfer => "transfer",
            Self::PurchaseOrderReceive => "purchase-order-receive",
            Self::StockCount => "stock-count",
            Self::ManualAdjustment => "manual-adjustment",
        }
    }

    /// Parse from the SQL row's stored string form. Returns `None` for unknown values.
    #[must_use]
    pub fn from_stored_str(s: &str) -> Option<Self> {
        Some(match s {
            "sale" => Self::Sale,
            "void" => Self::Void,
            "refund" => Self::Refund,
            "transfer" => Self::Transfer,
            "purchase-order-receive" => Self::PurchaseOrderReceive,
            "stock-count" => Self::StockCount,
            "manual-adjustment" => Self::ManualAdjustment,
            _ => return None,
        })
    }
}

/// Inventory transaction audit session header (ADR-18 §9a).
///
/// Groups one or more [`InventoryTransactionLine`]s under a single staff-traceable
/// session for cashier / manager accountability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryTransaction {
    /// UUID v7 primary key.
    pub id: InventoryTransactionId,
    /// Type of audit session (sale / void / refund / transfer / …).
    #[serde(rename = "type")]
    pub transaction_type: InventoryTransactionType,
    /// FK to `inventory_locations.id` for the location charged (default per §13-36).
    pub location_id: String,
    /// FK to `users.id` — staff member who initiated the session.
    pub staff_id: String,
    /// Optional FK to `stock_transfers.id` for transfer-type sessions.
    #[serde(default)]
    pub transfer_id: Option<String>,
    /// Optional FK to `purchase_orders.id` for PO-receive-type sessions.
    #[serde(default)]
    pub purchase_order_id: Option<String>,
    /// Free-form auditor notes (typed in the FastPIN overlay).
    #[serde(default)]
    pub notes: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Per-SKU detail line in an [`InventoryTransaction`] session (ADR-18 §9b).
///
/// Maps to the `inventory_transaction_lines` table (migration `084_inventory_transaction_audit.sql`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryTransactionLine {
    /// UUID v7 primary key.
    pub id: String,
    /// FK to `inventory_transactions.id`.
    pub transaction_id: InventoryTransactionId,
    /// Product SKU (FK to `products.sku`).
    pub sku: String,
    /// Product display name (denormalised for sale audit).
    pub product_name: String,
    /// Quantity adjustment (strictly > 0 per schema CHECK).
    pub qty: i64,
    /// Scanned barcode value if the line was created via barcode scan.
    #[serde(default)]
    pub barcode_scanned: Option<String>,
    /// Ordinal position within the session (1-indexed).
    pub sort_order: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_transaction_id_constructors() {
        let s = InventoryTransactionId::new();
        assert!(!s.as_str().is_empty());
        let from_str = InventoryTransactionId::from("abc-123");
        assert_eq!(from_str.as_str(), "abc-123");
        let from_string = InventoryTransactionId::from(String::from("xyz-789"));
        assert_eq!(from_string.as_str(), "xyz-789");
    }

    #[test]
    fn inventory_transaction_type_roundtrip() {
        for t in [
            InventoryTransactionType::Sale,
            InventoryTransactionType::Void,
            InventoryTransactionType::Refund,
            InventoryTransactionType::Transfer,
            InventoryTransactionType::PurchaseOrderReceive,
            InventoryTransactionType::StockCount,
            InventoryTransactionType::ManualAdjustment,
        ] {
            let stored = t.as_stored_str();
            let parsed = InventoryTransactionType::from_stored_str(stored);
            assert_eq!(parsed, Some(t));
        }
    }

    #[test]
    fn inventory_transaction_type_unknown_returns_none() {
        assert_eq!(
            InventoryTransactionType::from_stored_str("legacy-future-type"),
            None
        );
    }

    #[test]
    fn inventory_transaction_serde_roundtrip() {
        let tx = InventoryTransaction {
            id: InventoryTransactionId::from("tx-001"),
            transaction_type: InventoryTransactionType::Sale,
            location_id: "loc-001".into(),
            staff_id: "user-001".into(),
            transfer_id: None,
            purchase_order_id: None,
            notes: "Cashier sale #42".into(),
            created_at: "2026-07-20T10:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&tx).unwrap();
        let back: InventoryTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tx);
    }

    #[test]
    fn inventory_transaction_line_serde_roundtrip() {
        let line = InventoryTransactionLine {
            id: "line-001".into(),
            transaction_id: InventoryTransactionId::from("tx-001"),
            sku: "CHO-001".into(),
            product_name: "Chocolate Bar".into(),
            qty: 2,
            barcode_scanned: Some("5901234123457".into()),
            sort_order: 1,
        };
        let json = serde_json::to_string(&line).unwrap();
        let back: InventoryTransactionLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back, line);
    }
}
