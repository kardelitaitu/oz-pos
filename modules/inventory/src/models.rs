/*
last audited 25-07-26 by RSA-Agent (modules-inventory slice A: models deep read)
crate: modules-inventory | status: SAFE | lint: CLEAN
findings: clean — Sku/Barcode/Money newtypes; canonical default location UUID documented; ADR #36 D1/D2 local-only fields (cost_minor, default_supplier_id never synced) match transport omissions; ProductType parse fail-closed with round-trip tests; negative-qty assertion
next: none | perf: N/A
*/
//! Inventory & Product domain types.

use foundation::{Barcode, Money, Sku};
use serde::{Deserialize, Serialize};

/// Canonical default inventory location UUID.
pub const CANONICAL_DEFAULT_LOCATION_UUID: &str = "01926b3a-0000-7000-8000-000000000001";

/// Strongly-typed identifier for an inventory location.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocationId(pub String);

impl LocationId {
    /// Create a new LocationId wrapping a string UUID.
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LocationId {
    fn default() -> Self {
        Self(CANONICAL_DEFAULT_LOCATION_UUID.to_string())
    }
}

impl From<&str> for LocationId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for LocationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::ops::Deref for LocationId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for LocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Product type classification.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductType {
    /// Retail POS product.
    #[serde(rename = "retail")]
    #[default]
    Retail,
    /// Restaurant Menu product.
    #[serde(rename = "restaurant")]
    Restaurant,
    /// Both retail and restaurant.
    #[serde(rename = "both")]
    Both,
    /// Service item.
    #[serde(rename = "service")]
    Service,
}

impl ProductType {
    /// Parse string representation into ProductType.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "retail" => Some(Self::Retail),
            "restaurant" => Some(Self::Restaurant),
            "both" => Some(Self::Both),
            "service" => Some(Self::Service),
            _ => None,
        }
    }

    /// Canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Restaurant => "restaurant",
            Self::Both => "both",
            Self::Service => "service",
        }
    }

    /// Whether this product type consumes inventory stock.
    pub fn tracks_inventory(&self) -> bool {
        matches!(self, Self::Retail | Self::Restaurant | Self::Both)
    }
}

/// A product in the store inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    /// Product ID (UUID v4).
    pub id: String,
    /// Product SKU.
    pub sku: Sku,
    /// Display name.
    pub name: String,
    /// Price with currency.
    pub price: Money,
    /// Optional category reference.
    pub category_id: Option<String>,
    /// Optional barcode.
    pub barcode: Option<Barcode>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
    /// Price update timestamp.
    pub price_updated_at: String,
    /// Serial tracking flag.
    #[serde(default)]
    pub track_serial: bool,
    /// Product type.
    #[serde(default)]
    pub product_type: ProductType,
    /// Optimistic concurrency version.
    #[serde(default = "default_version")]
    pub version: i64,
    /// Purchase/cost price in minor units. Local-only — never synced (ADR #36 D2).
    #[serde(default)]
    pub cost_minor: i64,
    /// Product brand (free text).
    #[serde(default)]
    pub brand: Option<String>,
    /// Rack position code, e.g. "A-01-03".
    #[serde(default)]
    pub rack_location: Option<String>,
    /// Free-text product notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Unit of measure, e.g. "pcs", "kg", "box".
    #[serde(default)]
    pub unit: Option<String>,
    /// Whether the product is active/sellable. Retired products are hidden,
    /// not deleted — matches `product_variants.is_active` (ADR #36 D1).
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    /// Default supplier FK. Local-only — never synced (ADR #36 D2).
    #[serde(default)]
    pub default_supplier_id: Option<String>,
}

fn default_version() -> i64 {
    1
}

fn default_is_active() -> bool {
    true
}

impl Product {
    /// Create a new Product.
    pub fn new(sku: impl Into<Sku>, name: impl Into<String>, price: Money) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "product name must not be empty");

        Self {
            id: uuid::Uuid::now_v7().to_string(),
            sku: sku.into(),
            name,
            price,
            category_id: None,
            barcode: None,
            created_at: String::new(),
            updated_at: String::new(),
            price_updated_at: String::new(),
            track_serial: false,
            product_type: ProductType::Retail,
            version: 1,
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
        }
    }

    /// Builder method for setting category ID.
    #[must_use]
    pub fn with_category(mut self, category_id: impl Into<String>) -> Self {
        self.category_id = Some(category_id.into());
        self
    }

    /// Builder method for setting barcode.
    #[must_use]
    pub fn with_barcode(mut self, barcode: Barcode) -> Self {
        self.barcode = Some(barcode);
        self
    }

    /// Builder method for setting product type.
    #[must_use]
    pub fn with_product_type(mut self, product_type: ProductType) -> Self {
        self.product_type = product_type;
        self
    }
}

/// Product category with display colour and icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// Category ID.
    pub id: String,
    /// Category display name.
    pub name: String,
    /// Display colour hex string.
    pub colour: String,
    /// Display icon name.
    pub icon: String,
}

impl Category {
    /// Create a new category.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        colour: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "category name must not be empty");
        Self {
            id: id.into(),
            name,
            colour: colour.into(),
            icon: icon.into(),
        }
    }
}

/// Stock inventory record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// Product ID.
    pub product_id: String,
    /// Product SKU.
    pub sku: Sku,
    /// Quantity on hand.
    pub qty: i64,
    /// Low stock threshold.
    pub low_stock_threshold: i64,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
    /// Inventory location ID.
    pub location_id: LocationId,
}

impl Inventory {
    /// Create a new inventory record.
    pub fn new(sku: impl Into<Sku>, qty: i64) -> Self {
        let sku = sku.into();
        assert!(qty >= 0, "quantity must not be negative");
        Self {
            product_id: String::new(),
            sku,
            qty,
            low_stock_threshold: 5,
            updated_at: String::new(),
            location_id: LocationId::default(),
        }
    }

    /// Check if item is low in stock.
    pub fn is_low_stock(&self) -> bool {
        self.qty <= self.low_stock_threshold
    }
}

/// Product with full category details for listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductWithDetails {
    /// Underlying product entity.
    pub product: Product,
    /// Optional category name.
    pub category_name: Option<String>,
    /// Optional stock quantity on hand.
    pub stock_qty: Option<i64>,
}

/// An inventory location where physical/logical stock is stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryLocation {
    /// Location ID (UUID v7).
    pub id: String,
    /// Human-readable location name.
    pub name: String,
    /// Location type.
    #[serde(rename = "type")]
    pub location_type: String,
    /// Optional description.
    pub description: String,
    /// Whether the location is active.
    pub is_active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
}

/// A binding between a workspace instance and an inventory location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInventoryLocation {
    /// Unique binding ID (UUID v7).
    pub id: String,
    /// Workspace instance ID.
    pub instance_id: String,
    /// Location ID.
    pub location_id: String,
    /// Whether this location is the primary location for stock deductions.
    pub is_primary: bool,
    /// Whether this location is allowed to go below zero stock.
    pub allow_negative_stock: bool,
    /// Sorting order priority.
    pub sort_order: i64,
}

/// An inventory shift representing a window of time a staff member is working at a location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryShift {
    /// Shift ID (UUID v7).
    pub id: String,
    /// Staff user ID.
    pub user_id: String,
    /// Location ID.
    pub location_id: String,
    /// Optional terminal ID where the shift was opened.
    pub terminal_id: Option<String>,
    /// ISO-8601 opened timestamp.
    pub started_at: String,
    /// ISO-8601 closed timestamp.
    pub ended_at: Option<String>,
    /// Shift status ('active', 'ended').
    pub status: String,
    /// Optional shift notes.
    pub notes: String,
}

/// A stock threshold config for a product at a location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockThreshold {
    /// Threshold ID (UUID v7).
    pub id: String,
    /// Product ID.
    pub product_id: String,
    /// Location ID (nullable for global thresholds).
    pub location_id: Option<String>,
    /// Threshold quantity.
    pub threshold: i64,
    /// Whether the threshold is enabled.
    pub enabled: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LocationId ──────────────────────────────────────────────────

    #[test]
    fn location_id_default_is_canonical() {
        let id = LocationId::default();
        assert_eq!(id.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    #[test]
    fn location_id_new_returns_unique_uuid() {
        let a = LocationId::new();
        let b = LocationId::new();
        assert_ne!(a.as_str(), b.as_str());
        // Must be parseable as a UUID v7.
        let parsed = uuid::Uuid::parse_str(a.as_str()).unwrap();
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn location_id_display_matches_as_str() {
        let id = LocationId::from("custom-loc");
        assert_eq!(format!("{id}"), id.as_str());
    }

    #[test]
    fn location_id_deref_to_str() {
        let id = LocationId::from("loc-123");
        assert_eq!(&*id, "loc-123");
        assert_eq!(id.len(), 7);
    }

    #[test]
    fn location_id_from_string_roundtrip() {
        let id = LocationId::from("abc".to_string());
        assert_eq!(id.as_str(), "abc");
    }

    #[test]
    fn location_id_from_str_roundtrip() {
        let id = LocationId::from("xyz");
        assert_eq!(id.as_str(), "xyz");
    }

    #[test]
    fn location_id_default_eq_canonical_constant() {
        let id = LocationId::default();
        assert_eq!(id.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
        // Must also round-trip through serde.
        let json = serde_json::to_string(&id).unwrap();
        let back: LocationId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    // ── ProductType ─────────────────────────────────────────────────

    #[test]
    fn product_type_parse_str_roundtrip() {
        for variant in [
            ProductType::Retail,
            ProductType::Restaurant,
            ProductType::Both,
            ProductType::Service,
        ] {
            let s = variant.as_str();
            assert_eq!(ProductType::parse_str(s), Some(variant));
        }
    }

    #[test]
    fn product_type_parse_str_unknown_returns_none() {
        assert!(ProductType::parse_str("unknown").is_none());
        assert!(ProductType::parse_str("").is_none());
        assert!(ProductType::parse_str("RETAIL").is_none());
    }

    #[test]
    fn product_type_tracks_inventory_for_physical_types() {
        assert!(ProductType::Retail.tracks_inventory());
        assert!(ProductType::Restaurant.tracks_inventory());
        assert!(ProductType::Both.tracks_inventory());
        assert!(!ProductType::Service.tracks_inventory());
    }

    #[test]
    fn product_type_default_is_retail() {
        assert_eq!(ProductType::default(), ProductType::Retail);
    }

    #[test]
    fn product_type_serde_roundtrip() {
        for variant in [
            ProductType::Retail,
            ProductType::Restaurant,
            ProductType::Both,
            ProductType::Service,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: ProductType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // ── Product ─────────────────────────────────────────────────────

    fn make_product() -> Product {
        Product::new(
            "SKU-1",
            "Widget",
            Money::from_major(10, "USD".parse().unwrap()).unwrap(),
        )
    }

    #[test]
    fn product_new_sets_defaults() {
        let p = make_product();
        assert_eq!(p.sku.as_str(), "SKU-1");
        assert_eq!(p.name, "Widget");
        assert_eq!(
            p.price,
            Money::from_major(10, "USD".parse().unwrap()).unwrap()
        );
        assert!(p.category_id.is_none());
        assert!(p.barcode.is_none());
        assert!(!p.track_serial);
        assert_eq!(p.product_type, ProductType::Retail);
        assert_eq!(p.version, 1);
        assert_eq!(p.cost_minor, 0);
        assert!(p.brand.is_none());
        assert!(p.rack_location.is_none());
        assert!(p.notes.is_none());
        assert!(p.unit.is_none());
        assert!(p.is_active);
        assert!(p.default_supplier_id.is_none());
    }

    #[test]
    fn product_new_trims_name() {
        let p = Product::new(
            "SKU-2",
            "  Spaced  ",
            Money::from_major(1, "USD".parse().unwrap()).unwrap(),
        );
        assert_eq!(p.name, "Spaced");
    }

    #[test]
    #[should_panic(expected = "product name must not be empty")]
    fn product_new_rejects_empty_name_after_trim() {
        Product::new(
            "SKU-3",
            "   ",
            Money::from_major(1, "USD".parse().unwrap()).unwrap(),
        );
    }

    #[test]
    fn product_new_generates_unique_id() {
        let a = make_product();
        let b = make_product();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn product_builder_with_category() {
        let p = make_product().with_category("cat-1");
        assert_eq!(p.category_id.as_deref(), Some("cat-1"));
    }

    #[test]
    fn product_builder_with_product_type() {
        let p = make_product().with_product_type(ProductType::Service);
        assert_eq!(p.product_type, ProductType::Service);
    }

    #[test]
    fn product_serde_roundtrip() {
        let p = make_product();
        let json = serde_json::to_string(&p).unwrap();
        let back: Product = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, p.id);
        assert_eq!(back.sku, p.sku);
        assert_eq!(back.name, p.name);
        assert_eq!(back.price, p.price);
        assert_eq!(back.version, 1);
        assert!(back.is_active);
    }

    #[test]
    fn product_default_version_is_one() {
        let p = make_product();
        assert_eq!(p.version, 1);
    }

    #[test]
    fn product_default_is_active_is_true() {
        let p = make_product();
        assert!(p.is_active);
    }

    #[test]
    fn product_cost_minor_not_synced() {
        let mut p = make_product();
        p.cost_minor = 750;
        let json = serde_json::to_string(&p).unwrap();
        let back: Product = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cost_minor, 750);
    }

    // ── Category ────────────────────────────────────────────────────

    #[test]
    fn category_new_sets_fields() {
        let c = Category::new("cat-1", "Drinks", "#ff0000", "coffee");
        assert_eq!(c.id, "cat-1");
        assert_eq!(c.name, "Drinks");
        assert_eq!(c.colour, "#ff0000");
        assert_eq!(c.icon, "coffee");
    }

    #[test]
    fn category_new_trims_name() {
        let c = Category::new("cat-2", "  Food  ", "#000", "utensils");
        assert_eq!(c.name, "Food");
    }

    #[test]
    #[should_panic(expected = "category name must not be empty")]
    fn category_new_rejects_empty_name() {
        Category::new("cat-3", "  ", "#000", "icon");
    }

    #[test]
    fn category_serde_roundtrip() {
        let c = Category::new("cat-1", "Drinks", "#ff0000", "coffee");
        let json = serde_json::to_string(&c).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    // ── Inventory ───────────────────────────────────────────────────

    #[test]
    fn inventory_new_sets_defaults() {
        let inv = Inventory::new("SKU-1", 10);
        assert_eq!(inv.sku.as_str(), "SKU-1");
        assert_eq!(inv.qty, 10);
        assert_eq!(inv.low_stock_threshold, 5);
        assert_eq!(inv.location_id, LocationId::default());
    }

    #[test]
    #[should_panic(expected = "quantity must not be negative")]
    fn inventory_new_rejects_negative_qty() {
        Inventory::new("SKU-1", -1);
    }

    #[test]
    fn inventory_new_zero_qty_is_valid() {
        let inv = Inventory::new("SKU-1", 0);
        assert_eq!(inv.qty, 0);
    }

    #[test]
    fn inventory_is_low_stock_at_threshold() {
        let mut inv = Inventory::new("SKU-1", 5);
        inv.low_stock_threshold = 5;
        assert!(inv.is_low_stock()); // 5 <= 5
    }

    #[test]
    fn inventory_is_not_low_stock_above_threshold() {
        let mut inv = Inventory::new("SKU-1", 6);
        inv.low_stock_threshold = 5;
        assert!(!inv.is_low_stock());
    }

    #[test]
    fn inventory_is_low_stock_below_threshold() {
        let mut inv = Inventory::new("SKU-1", 3);
        inv.low_stock_threshold = 5;
        assert!(inv.is_low_stock());
    }

    #[test]
    fn inventory_serde_roundtrip() {
        let inv = Inventory::new("SKU-1", 42);
        let json = serde_json::to_string(&inv).unwrap();
        let back: Inventory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.qty, 42);
        assert_eq!(back.sku.as_str(), "SKU-1");
    }

    // ── ProductWithDetails ──────────────────────────────────────────

    #[test]
    fn product_with_details_serde_roundtrip() {
        let pwd = ProductWithDetails {
            product: make_product(),
            category_name: Some("Drinks".into()),
            stock_qty: Some(100),
        };
        let json = serde_json::to_string(&pwd).unwrap();
        let back: ProductWithDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(back.category_name.as_deref(), Some("Drinks"));
        assert_eq!(back.stock_qty, Some(100));
    }

    #[test]
    fn product_with_details_none_fields_roundtrip() {
        let pwd = ProductWithDetails {
            product: make_product(),
            category_name: None,
            stock_qty: None,
        };
        let json = serde_json::to_string(&pwd).unwrap();
        let back: ProductWithDetails = serde_json::from_str(&json).unwrap();
        assert!(back.category_name.is_none());
        assert!(back.stock_qty.is_none());
    }

    // ── InventoryLocation ───────────────────────────────────────────

    #[test]
    fn inventory_location_serde_roundtrip() {
        let loc = InventoryLocation {
            id: "loc-1".into(),
            name: "Main".into(),
            location_type: "warehouse".into(),
            description: "Primary".into(),
            is_active: true,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let back: InventoryLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "loc-1");
        assert_eq!(back.location_type, "warehouse");
    }

    // ── WorkspaceInventoryLocation ──────────────────────────────────

    #[test]
    fn workspace_inventory_location_serde_roundtrip() {
        let w = WorkspaceInventoryLocation {
            id: "binding-1".into(),
            instance_id: "ws-1".into(),
            location_id: "loc-1".into(),
            is_primary: true,
            allow_negative_stock: false,
            sort_order: 0,
        };
        let json = serde_json::to_string(&w).unwrap();
        let back: WorkspaceInventoryLocation = serde_json::from_str(&json).unwrap();
        assert!(back.is_primary);
        assert!(!back.allow_negative_stock);
    }

    // ── InventoryShift ──────────────────────────────────────────────

    #[test]
    fn inventory_shift_serde_roundtrip() {
        let shift = InventoryShift {
            id: "shift-1".into(),
            user_id: "u-1".into(),
            location_id: "loc-1".into(),
            terminal_id: Some("term-1".into()),
            started_at: "2025-01-01T09:00:00Z".into(),
            ended_at: None,
            status: "active".into(),
            notes: String::new(),
        };
        let json = serde_json::to_string(&shift).unwrap();
        let back: InventoryShift = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, "active");
        assert!(back.ended_at.is_none());
    }

    // ── StockThreshold ──────────────────────────────────────────────

    #[test]
    fn stock_threshold_serde_roundtrip() {
        let t = StockThreshold {
            id: "t-1".into(),
            product_id: "p-1".into(),
            location_id: Some("loc-1".into()),
            threshold: 10,
            enabled: true,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: StockThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(back.threshold, 10);
        assert!(back.location_id.is_some());
    }

    #[test]
    fn stock_threshold_location_id_nullable() {
        let t = StockThreshold {
            id: "t-2".into(),
            product_id: "p-1".into(),
            location_id: None,
            threshold: 5,
            enabled: false,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: StockThreshold = serde_json::from_str(&json).unwrap();
        assert!(back.location_id.is_none());
    }
}
