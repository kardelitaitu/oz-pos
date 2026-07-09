//! Product domain type — the core inventory item.
//!
//! A [`Product`] wraps the fields stored in the `products` table:
//! SKU (unique identifier), display name, price with currency,
//! optional category and barcode, and timestamps. The struct lives
//! in `oz-core` so every downstream crate (`oz-api`, `oz-cli`,
//! `apps/desktop-client`) uses the same definition.

use serde::{Deserialize, Serialize};

use foundation::Barcode;

use crate::{Money, Sku};

/// Product type classification that determines which workspace(s)
/// a product appears in.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductType {
    /// Appears in Retail POS only (track_serial, weight scale, etc.).
    #[serde(rename = "retail")]
    #[default]
    Retail,
    /// Appears in Restaurant Menu only (prep time, modifiers, KDS).
    #[serde(rename = "restaurant")]
    Restaurant,
    /// Appears in both workspaces.
    #[serde(rename = "both")]
    Both,
}

impl ProductType {
    /// Parse a string into a [`ProductType`]. Returns `None` for
    /// unrecognised values so callers can fall back to a default.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "retail" => Some(Self::Retail),
            "restaurant" => Some(Self::Restaurant),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// Return the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Restaurant => "restaurant",
            Self::Both => "both",
        }
    }
}

/// A product in the store's inventory.
///
/// # Schema mapping
///
/// Maps 1:1 to the `products` table (migrations `001_sales.sql`,
/// `002_products.sql`, `003_barcode.sql`). Fields use domain types
/// ([`Sku`], [`Money`]) rather than raw strings/integers so code
/// downstream benefits from validation and checked arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    /// Internal row id (UUID v4).
    pub id: String,

    /// Stock-keeping unit — the human-readable product code.
    #[serde(with = "sku_serde")]
    pub sku: Sku,

    /// Display name shown on receipts and the POS UI.
    pub name: String,

    /// Sale price with currency.
    pub price: Money,

    /// Optional reference to a [`crate::Category`] row.
    pub category_id: Option<String>,

    /// Optional machine-readable barcode (EAN-13, UPC-A, etc.).
    /// Unique when present; two products can share a `None` barcode.
    pub barcode: Option<Barcode>,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,

    /// ISO-8601 timestamp of the last price change.
    /// Used by the front-end to show a price-volatility hint.
    pub price_updated_at: String,

    /// Whether this product requires serial number capture at checkout.
    #[serde(default)]
    pub track_serial: bool,

    /// Product type classification (retail, restaurant, or both).
    #[serde(default)]
    pub product_type: ProductType,
}

impl Product {
    /// Create a new product with the given SKU, name, and price.
    ///
    /// Generates a fresh UUID for `id` and sets both timestamps to
    /// empty strings (the database layer fills them in on insert).
    /// Optional fields (`category_id`, `barcode`) default to `None`.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming.
    pub fn new(sku: impl Into<Sku>, name: impl Into<String>, price: Money) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "product name must not be empty");

        Self {
            id: uuid::Uuid::new_v4().to_string(),
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
        }
    }

    /// Set the category reference (builder-style).
    #[must_use]
    pub fn with_category(mut self, category_id: impl Into<String>) -> Self {
        self.category_id = Some(category_id.into());
        self
    }

    /// Set the barcode (builder-style).
    #[must_use]
    pub fn with_barcode(mut self, barcode: Barcode) -> Self {
        self.barcode = Some(barcode);
        self
    }

    /// Set the product type (builder-style).
    #[must_use]
    pub fn with_product_type(mut self, product_type: ProductType) -> Self {
        self.product_type = product_type;
        self
    }
}

// ── Serde helpers ────────────────────────────────────────────────

/// Custom serde module so `Sku` serializes as a bare string within the
/// containing struct. Without this, `#[serde(transparent)]` on `Sku`
/// would work on its own, but nesting inside `Product` requires the
/// `#[serde(with = "...")]` indirection.
mod sku_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sku: &super::Sku, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(sku.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<super::Sku, D::Error> {
        let s = String::deserialize(de)?;
        super::Sku::try_new(s).ok_or_else(|| serde::de::Error::custom("SKU must not be empty"))
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
    fn new_product_has_generated_id() {
        let p = Product::new("COFFEE", "Espresso", test_price());
        assert!(!p.id.is_empty(), "id should be generated");
        assert_eq!(p.sku.as_str(), "COFFEE");
        assert_eq!(p.name, "Espresso");
        assert_eq!(p.price, test_price());
        assert!(p.category_id.is_none());
        assert!(p.barcode.is_none());
        assert!(p.created_at.is_empty());
        assert!(p.updated_at.is_empty());
    }

    #[test]
    fn product_ids_are_unique() {
        let a = Product::new("A", "Alpha", test_price());
        let b = Product::new("B", "Beta", test_price());
        assert_ne!(a.id, b.id);
    }

    #[test]
    #[should_panic(expected = "product name must not be empty")]
    fn new_product_panics_on_empty_name() {
        Product::new("SKU", "   ", test_price());
    }

    #[test]
    fn builder_sets_category() {
        let p = Product::new("SKU", "Widget", test_price()).with_category("cat-tools");
        assert_eq!(p.category_id, Some("cat-tools".into()));
    }

    #[test]
    fn builder_sets_barcode() {
        let p = Product::new("SKU", "Widget", test_price())
            .with_barcode(Barcode::new("5901234123457").unwrap());
        assert_eq!(p.barcode, Some(Barcode::new("5901234123457").unwrap()));
    }

    #[test]
    fn builder_chains() {
        let p = Product::new("SKU", "Widget", test_price())
            .with_category("cat-tools")
            .with_barcode(Barcode::new("5901234123457").unwrap());
        assert_eq!(p.category_id, Some("cat-tools".into()));
        assert_eq!(p.barcode, Some(Barcode::new("5901234123457").unwrap()));
    }

    #[test]
    fn serde_roundtrip() {
        let p = Product::new("COFFEE", "Espresso", test_price())
            .with_category("cat-drinks")
            .with_barcode(Barcode::new("5901234123457").unwrap());
        let json = serde_json::to_string(&p).unwrap();
        let back: Product = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn serde_deserialize_minimal() {
        let json = r#"{
            "id": "abc",
            "sku": "COFFEE",
            "name": "Espresso",
            "price": { "minor_units": 350, "currency": "USD" },
            "category_id": null,
            "barcode": null,
            "created_at": "",
            "updated_at": "",
            "price_updated_at": ""
        }"#;
        let p: Product = serde_json::from_str(json).unwrap();
        assert_eq!(p.sku.as_str(), "COFFEE");
        assert_eq!(p.name, "Espresso");
        assert_eq!(p.price.minor_units, 350);
        assert_eq!(p.price.currency, usd());
        // Missing product_type defaults to Retail for backward compat.
        assert_eq!(p.product_type, ProductType::Retail);
    }

    #[test]
    fn serde_deserialize_rejects_empty_sku() {
        let json = r#"{
            "id": "abc",
            "sku": "",
            "name": "Espresso",
            "price": { "minor_units": 350, "currency": "USD" },
            "category_id": null,
            "barcode": null,
            "created_at": "",
            "updated_at": ""
        }"#;
        let result: Result<Product, _> = serde_json::from_str(json);
        assert!(result.is_err(), "empty SKU should fail deserialization");
    }

    #[test]
    fn new_product_defaults_to_retail() {
        let p = Product::new("P", "Test", test_price());
        assert_eq!(p.product_type, ProductType::Retail);
    }

    #[test]
    fn builder_sets_product_type() {
        let p = Product::new("P", "Test", test_price()).with_product_type(ProductType::Restaurant);
        assert_eq!(p.product_type, ProductType::Restaurant);
    }

    #[test]
    fn builder_sets_product_type_both() {
        let p = Product::new("P", "Test", test_price()).with_product_type(ProductType::Both);
        assert_eq!(p.product_type, ProductType::Both);
    }

    #[test]
    fn product_type_roundtrip_serde() {
        for &(s, expected) in &[
            ("retail", ProductType::Retail),
            ("restaurant", ProductType::Restaurant),
            ("both", ProductType::Both),
        ] {
            assert_eq!(ProductType::parse_str(s), Some(expected));
            assert_eq!(expected.as_str(), s);
            let json = serde_json::to_value(expected).unwrap();
            assert_eq!(json, serde_json::json!(s));
            let back: ProductType = serde_json::from_value(json).unwrap();
            assert_eq!(back, expected);
        }
    }

    #[test]
    fn product_type_from_str_unknown_returns_none() {
        assert_eq!(ProductType::parse_str("unknown"), None);
        assert_eq!(ProductType::parse_str(""), None);
    }

    #[test]
    fn product_type_default_is_retail() {
        assert_eq!(ProductType::default(), ProductType::Retail);
    }

    #[test]
    fn sku_serializes_as_bare_string() {
        let p = Product::new("COFFEE", "Espresso", test_price());
        let json = serde_json::to_string(&p).unwrap();
        // Sku field should be "COFFEE", not {"0": "COFFEE"} or similar.
        assert!(
            json.contains(r#""sku":"COFFEE""#),
            "unexpected JSON: {json}"
        );
    }
}
