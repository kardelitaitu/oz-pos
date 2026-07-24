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
}

fn default_version() -> i64 {
    1
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
