use serde::{Deserialize, Serialize};

/// A product bundle — a single SKU that contains multiple sub-items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBundle {
    /// Internal row id (UUID v4).
    pub id: String,
    /// The bundle's own SKU (must match an existing product).
    pub bundle_sku: String,
    /// Display name for the bundle.
    pub name: String,
    /// Optional description shown on receipts / UI.
    pub description: String,
    /// Bundle-level price in minor units, if overridden.
    pub bundle_price_minor: Option<i64>,
    /// ISO currency code (e.g. "USD").
    pub currency: String,
    /// Whether the bundle is available for sale.
    pub active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

/// An item within a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to [`ProductBundle::id`].
    pub bundle_id: String,
    /// SKU of the component product.
    pub sku: String,
    /// Quantity of this component in the bundle.
    pub qty: i64,
    /// Per-unit price override (None = use product's price).
    pub unit_price_minor: Option<i64>,
}

/// A bundle with its items (returned by list/get).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWithItems {
    /// The bundle metadata.
    pub bundle: ProductBundle,
    /// Component items in the bundle.
    pub items: Vec<BundleItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_bundle_serde_roundtrip() {
        let pb = ProductBundle {
            id: "b1".into(),
            bundle_sku: "GIFT-BOX".into(),
            name: "Gift Box".into(),
            description: "A curated gift box".into(),
            bundle_price_minor: Some(25000),
            currency: "IDR".into(),
            active: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-07-01T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&pb).unwrap();
        let back: ProductBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, pb.id);
        assert_eq!(back.bundle_sku, pb.bundle_sku);
        assert_eq!(back.name, pb.name);
        assert_eq!(back.description, pb.description);
        assert_eq!(back.bundle_price_minor, pb.bundle_price_minor);
        assert_eq!(back.currency, pb.currency);
        assert_eq!(back.active, pb.active);
    }

    #[test]
    fn product_bundle_null_price() {
        let pb = ProductBundle {
            id: "b2".into(),
            bundle_sku: "SIMPLE".into(),
            name: "Simple".into(),
            description: String::new(),
            bundle_price_minor: None,
            currency: "USD".into(),
            active: false,
            created_at: "2026-06-01T00:00:00Z".into(),
            updated_at: "2026-06-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&pb).unwrap();
        assert!(json.contains("\"bundle_price_minor\":null"));
        let back: ProductBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bundle_price_minor, None);
        assert!(!back.active);
    }

    #[test]
    fn bundle_item_serde_roundtrip() {
        let item = BundleItem {
            id: "i1".into(),
            bundle_id: "b1".into(),
            sku: "SKU-001".into(),
            qty: 3,
            unit_price_minor: Some(1500),
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: BundleItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, item.id);
        assert_eq!(back.bundle_id, item.bundle_id);
        assert_eq!(back.sku, item.sku);
        assert_eq!(back.qty, item.qty);
        assert_eq!(back.unit_price_minor, item.unit_price_minor);
    }

    #[test]
    fn bundle_item_null_unit_price() {
        let item = BundleItem {
            id: "i2".into(),
            bundle_id: "b2".into(),
            sku: "SKU-002".into(),
            qty: 1,
            unit_price_minor: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"unit_price_minor\":null"));
        let back: BundleItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.unit_price_minor, None);
    }

    #[test]
    fn bundle_with_items_serde_roundtrip() {
        let bundle = ProductBundle {
            id: "b3".into(),
            bundle_sku: "COMBO".into(),
            name: "Combo Deal".into(),
            description: String::new(),
            bundle_price_minor: Some(50000),
            currency: "IDR".into(),
            active: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let items = vec![
            BundleItem {
                id: "ci1".into(),
                bundle_id: "b3".into(),
                sku: "A".into(),
                qty: 2,
                unit_price_minor: Some(10000),
            },
            BundleItem {
                id: "ci2".into(),
                bundle_id: "b3".into(),
                sku: "B".into(),
                qty: 1,
                unit_price_minor: None,
            },
        ];
        let bwi = BundleWithItems {
            bundle: bundle.clone(),
            items: items.clone(),
        };
        let json = serde_json::to_string(&bwi).unwrap();
        let back: BundleWithItems = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bundle.id, bundle.id);
        assert_eq!(back.items.len(), 2);
        assert_eq!(back.items[0].sku, "A");
        assert_eq!(back.items[1].sku, "B");
    }

    #[test]
    fn bundle_with_items_empty_items() {
        let bundle = ProductBundle {
            id: "b4".into(),
            bundle_sku: "EMPTY".into(),
            name: "Empty".into(),
            description: String::new(),
            bundle_price_minor: None,
            currency: "IDR".into(),
            active: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let bwi = BundleWithItems {
            bundle,
            items: vec![],
        };
        let json = serde_json::to_string(&bwi).unwrap();
        let back: BundleWithItems = serde_json::from_str(&json).unwrap();
        assert!(back.items.is_empty());
    }
}
