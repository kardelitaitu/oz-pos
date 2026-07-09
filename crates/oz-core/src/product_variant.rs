//! Product variant domain type — size/colour/flavour per parent product.

use foundation::Barcode;
use serde::{Deserialize, Serialize};

use crate::Money;

/// A product variant linked to a parent product via `parent_sku`.
///
/// Each variant can have its own SKU, optional price override (when `price`
/// is `None` the parent product's price is used), barcode, and sort order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductVariant {
    /// Internal row id (UUID v4).
    pub id: String,
    /// SKU of the parent product this variant belongs to.
    pub parent_sku: String,
    /// Display name of this variant (e.g., "Small", "Red", "Mint").
    pub name: String,
    /// Unique SKU for this variant.
    pub sku: String,
    /// Optional price override. `None` means use parent product's price.
    pub price: Option<Money>,
    /// Optional barcode (unique when present).
    pub barcode: Option<Barcode>,
    /// Display order within the variant list (ascending).
    pub sort_order: i64,
    /// Whether this variant is available for sale.
    pub is_active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl ProductVariant {
    /// Create a new product variant with the given parent SKU, name, and own SKU.
    ///
    /// Generates a fresh UUID for `id`. Optional fields (`price`, `barcode`)
    /// default to `None`. `sort_order` defaults to `0` and `is_active` to `true`.
    /// Timestamps are empty strings (the database layer fills them in).
    pub fn new(
        parent_sku: impl Into<String>,
        name: impl Into<String>,
        sku: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            parent_sku: parent_sku.into(),
            name: name.into(),
            sku: sku.into(),
            price: None,
            barcode: None,
            sort_order: 0,
            is_active: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Set the price override (builder-style).
    #[must_use]
    pub fn with_price(mut self, price: Money) -> Self {
        self.price = Some(price);
        self
    }

    /// Set the barcode (builder-style).
    #[must_use]
    pub fn with_barcode(mut self, barcode: Barcode) -> Self {
        self.barcode = Some(barcode);
        self
    }

    /// Set the sort order (builder-style).
    #[must_use]
    pub fn with_sort_order(mut self, order: i64) -> Self {
        self.sort_order = order;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd() -> crate::Currency {
        "USD".parse().unwrap()
    }

    fn test_price() -> Money {
        Money::from_major(12, usd()).unwrap()
    }

    #[test]
    fn new_variant_has_generated_id() {
        let v = ProductVariant::new("PARENT", "Small", "PARENT-SMALL");
        assert!(!v.id.is_empty(), "id should be generated");
        assert_eq!(v.parent_sku, "PARENT");
        assert_eq!(v.name, "Small");
        assert_eq!(v.sku, "PARENT-SMALL");
        assert!(v.price.is_none());
        assert!(v.barcode.is_none());
        assert_eq!(v.sort_order, 0);
        assert!(v.is_active);
        assert!(v.created_at.is_empty());
        assert!(v.updated_at.is_empty());
    }

    #[test]
    fn new_variant_sets_fields() {
        let v = ProductVariant::new("COFFEE", "Large", "COFFEE-LARGE");
        assert_eq!(v.parent_sku, "COFFEE");
        assert_eq!(v.name, "Large");
        assert_eq!(v.sku, "COFFEE-LARGE");
    }

    #[test]
    fn builder_methods() {
        let v = ProductVariant::new("TEA", "Green", "TEA-GREEN")
            .with_price(test_price())
            .with_barcode(Barcode::new("4901234567890").unwrap())
            .with_sort_order(1);
        assert_eq!(v.price, Some(test_price()));
        assert_eq!(v.barcode, Some(Barcode::new("4901234567890").unwrap()));
        assert_eq!(v.sort_order, 1);
    }

    #[test]
    fn serde_roundtrip() {
        let v = ProductVariant::new("TEA", "Green", "TEA-GREEN")
            .with_price(test_price())
            .with_barcode(Barcode::new("4901234567890").unwrap())
            .with_sort_order(2);
        let json = serde_json::to_string(&v).unwrap();
        let back: ProductVariant = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn variant_ids_are_unique() {
        let a = ProductVariant::new("P1", "A", "P1-A");
        let b = ProductVariant::new("P1", "B", "P1-B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn debug_output() {
        let v = ProductVariant::new("TEA", "Green", "TEA-GREEN")
            .with_price(test_price())
            .with_sort_order(1);
        let debug = format!("{v:?}");
        assert!(debug.contains("TEA-GREEN"));
        assert!(debug.contains("Green"));
    }

    #[test]
    fn serde_deserialize_minimal() {
        let json = r#"{"id":"v1","parent_sku":"TEA","name":"Oolong","sku":"T-O","price":null,"barcode":null,"sort_order":0,"is_active":true,"created_at":"","updated_at":""}"#;
        let v: ProductVariant = serde_json::from_str(json).unwrap();
        assert_eq!(v.sku, "T-O");
        assert_eq!(v.name, "Oolong");
        assert!(v.price.is_none());
        assert!(v.is_active);
    }
}
