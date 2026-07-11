//! Product variant Tauri commands.
//!
//! CRUD operations for product variants (size, colour, flavour).
//! Each variant is linked to a parent product via `parent_sku` and has
//! its own SKU, optional price override, and barcode.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Money, ProductVariant, Store};

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Money DTO matching the front-end `Money` type (snake_case keys).
#[derive(Debug, Serialize)]
pub struct MoneyDto {
    /// Minor Units.
    pub minor_units: i64,
    /// ISO-4217 currency code.
    pub currency: String,
}

/// Product variant DTO for the front-end.
#[derive(Debug, Serialize)]
pub struct ProductVariantDto {
    /// Unique identifier.
    pub id: String,
    /// Parent Sku.
    pub parent_sku: String,
    /// Display name.
    pub name: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Unit price in minor currency units.
    pub price: Option<MoneyDto>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Display sort order.
    pub sort_order: i64,
    /// Whether this is active.
    pub is_active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl From<ProductVariant> for ProductVariantDto {
    fn from(v: ProductVariant) -> Self {
        Self {
            id: v.id,
            parent_sku: v.parent_sku,
            name: v.name,
            sku: v.sku,
            price: v.price.map(|m| {
                let cur_str = std::str::from_utf8(&m.currency.0)
                    .unwrap_or("USD")
                    .to_owned();
                MoneyDto {
                    minor_units: m.minor_units,
                    currency: cur_str,
                }
            }),
            barcode: v.barcode.map(|b| b.to_string()),
            sort_order: v.sort_order,
            is_active: v.is_active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// ── List ──────────────────────────────────────────────────────────────

/// List all variants for a given parent product SKU.
#[command]
pub async fn list_product_variants(
    parent_sku: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProductVariantDto>, AppError> {
    validate_not_empty("parent_sku", &parent_sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let variants = store.list_product_variants(&parent_sku)?;
    drop(db);

    let dtos: Vec<ProductVariantDto> = variants.into_iter().map(ProductVariantDto::from).collect();
    Ok(dtos)
}

// ── Get by SKU ────────────────────────────────────────────────────────

/// Get a single variant by its own SKU.
#[command]
pub async fn get_product_variant(
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductVariantDto>, AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let variant = store.get_product_variant(&sku)?;
    drop(db);

    Ok(variant.map(ProductVariantDto::from))
}

// ── Create ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Createproductvariantargs.
pub struct CreateProductVariantArgs {
    /// Parent Sku.
    pub parent_sku: String,
    /// Display name.
    pub name: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Price Minor.
    pub price_minor: Option<i64>,
    /// ISO-4217 currency code.
    pub currency: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Display sort order.
    pub sort_order: Option<i64>,
    /// Whether this is active.
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
/// Createproductvariantresult.
pub struct CreateProductVariantResult {
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Create a new product variant.
#[command]
pub async fn create_product_variant(
    args: CreateProductVariantArgs,
    state: State<'_, AppState>,
) -> Result<CreateProductVariantResult, AppError> {
    validate_not_empty("parent_sku", &args.parent_sku)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("sku", &args.sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let price = match (args.price_minor, args.currency) {
        (Some(minor), Some(cur_str)) => {
            let currency: oz_core::Currency = cur_str
                .parse()
                .map_err(|_| AppError::Invalid(format!("invalid currency '{cur_str}'")))?;
            Some(Money {
                minor_units: minor,
                currency,
            })
        }
        _ => None,
    };

    let mut variant = ProductVariant::new(args.parent_sku, args.name, args.sku);
    if let Some(p) = price {
        variant = variant.with_price(p);
    }
    if let Some(ref barcode) = args.barcode {
        let parsed = foundation::Barcode::new(barcode)
            .map_err(|e| AppError::Invalid(e.message.to_string()))?;
        variant = variant.with_barcode(parsed);
    }
    if let Some(order) = args.sort_order {
        variant = variant.with_sort_order(order);
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.create_product_variant(&variant)?;
    drop(db);

    tracing::info!(sku = %variant.sku, parent_sku = %variant.parent_sku, "product variant created");
    Ok(CreateProductVariantResult { sku: variant.sku })
}

// ── Update ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Updateproductvariantargs.
pub struct UpdateProductVariantArgs {
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Display name.
    pub name: Option<String>,
    /// Price Minor.
    pub price_minor: Option<i64>,
    /// ISO-4217 currency code.
    pub currency: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Display sort order.
    pub sort_order: Option<i64>,
    /// Whether this is active.
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
/// Updateproductvariantresult.
pub struct UpdateProductVariantResult {
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Update an existing product variant (matched by SKU).
#[command]
pub async fn update_product_variant(
    args: UpdateProductVariantArgs,
    state: State<'_, AppState>,
) -> Result<UpdateProductVariantResult, AppError> {
    validate_not_empty("sku", &args.sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Fetch existing variant first.
    let mut variant = store
        .get_product_variant(&args.sku)?
        .ok_or_else(|| AppError::Invalid(format!("variant '{}' not found", args.sku)))?;

    if let Some(name) = args.name {
        validate_not_empty("name", &name).map_err(|e| AppError::Invalid(e.to_string()))?;
        variant.name = name;
    }
    if let (Some(minor), Some(cur_str)) = (args.price_minor, args.currency) {
        let currency: oz_core::Currency = cur_str
            .parse()
            .map_err(|_| AppError::Invalid(format!("invalid currency '{cur_str}'")))?;
        variant.price = Some(Money {
            minor_units: minor,
            currency,
        });
    }
    if let Some(ref barcode) = args.barcode {
        let parsed = foundation::Barcode::new(barcode)
            .map_err(|e| AppError::Invalid(e.message.to_string()))?;
        variant.barcode = Some(parsed);
    }
    if let Some(order) = args.sort_order {
        variant.sort_order = order;
    }
    if let Some(active) = args.is_active {
        variant.is_active = active;
    }

    store.update_product_variant(&variant)?;
    drop(db);

    tracing::info!(sku = %variant.sku, "product variant updated");
    Ok(UpdateProductVariantResult { sku: variant.sku })
}

// ── Delete ────────────────────────────────────────────────────────────

/// Delete a product variant by its own SKU.
#[command]
pub async fn delete_product_variant(
    sku: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_product_variant(&sku)?;
    drop(db);

    tracing::info!(sku, "product variant deleted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    fn seed_product(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
             VALUES ('p1', 'TEA', 'Tea', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        )
        .unwrap();
    }

    #[test]
    fn list_product_variants_empty_db() {
        let conn = fresh_conn();
        seed_product(&conn);
        let store = Store::new(&conn);
        let variants = store.list_product_variants("TEA").unwrap();
        assert!(variants.is_empty());
    }

    #[test]
    fn list_product_variants_with_seeded_data() {
        let conn = fresh_conn();
        seed_product(&conn);

        let store = Store::new(&conn);
        let v = ProductVariant::new("TEA", "Green", "TEA-GREEN").with_sort_order(1);
        store.create_product_variant(&v).unwrap();
        let v = ProductVariant::new("TEA", "Black", "TEA-BLACK").with_sort_order(2);
        store.create_product_variant(&v).unwrap();

        let variants = store.list_product_variants("TEA").unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].sku, "TEA-GREEN");
        assert_eq!(variants[1].sku, "TEA-BLACK");
    }

    // ── Barcode validation ─────────────────────────────────────────

    #[test]
    fn barcode_empty_is_rejected() {
        let err = foundation::Barcode::new("").unwrap_err();
        assert_eq!(err.field, "barcode");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn barcode_whitespace_only_is_rejected() {
        let err = foundation::Barcode::new("   ").unwrap_err();
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn barcode_valid_ean13_passes() {
        let bc = foundation::Barcode::new("5901234123457").unwrap();
        assert_eq!(bc.as_str(), "5901234123457");
    }

    #[test]
    fn barcode_valid_upca_passes() {
        let bc = foundation::Barcode::new("012345678905").unwrap();
        assert_eq!(bc.as_str(), "012345678905");
    }

    #[test]
    fn barcode_valid_alphanumeric_passes() {
        let bc = foundation::Barcode::new("CODE128-ABC").unwrap();
        assert_eq!(bc.as_str(), "CODE128-ABC");
    }

    #[test]
    fn barcode_trims_whitespace() {
        let bc = foundation::Barcode::new("  4901234567890  ").unwrap();
        assert_eq!(bc.as_str(), "4901234567890");
    }

    #[test]
    fn barcode_optional_when_none_is_ok() {
        // The barcode field is optional in CreateProductVariantArgs and
        // is only validated via foundation::Barcode::new() when Some.
        let args = CreateProductVariantArgs {
            parent_sku: "TEA".into(),
            name: "Green".into(),
            sku: "TEA-GREEN".into(),
            price_minor: None,
            currency: None,
            barcode: None,
            sort_order: None,
            is_active: None,
        };
        // When None, no Barcode::new() is called, so validation passes.
        assert!(args.barcode.is_none());
    }

    // ── DTO struct tests ──────────────────────────────────────────

    #[test]
    fn product_variant_dto_from() {
        let variant = ProductVariant {
            id: "v1".into(),
            parent_sku: "TEA".into(),
            name: "Green".into(),
            sku: "TEA-GREEN".into(),
            price: None,
            barcode: Some(foundation::Barcode::new("123").unwrap()),
            sort_order: 1,
            is_active: true,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let dto = ProductVariantDto::from(variant);
        assert_eq!(dto.sku, "TEA-GREEN");
        assert_eq!(dto.parent_sku, "TEA");
        assert_eq!(dto.name, "Green");
        assert!(dto.price.is_none());
        assert_eq!(dto.barcode.as_deref(), Some("123"));
        assert_eq!(dto.sort_order, 1);
        assert!(dto.is_active);
    }

    #[test]
    fn product_variant_dto_from_with_price() {
        let variant = ProductVariant {
            id: "v2".into(),
            parent_sku: "TEA".into(),
            name: "Black".into(),
            sku: "TEA-BLACK".into(),
            price: Some(oz_core::Money {
                minor_units: 400,
                currency: oz_core::Currency([85, 83, 68]),
            }),
            barcode: None,
            sort_order: 2,
            is_active: false,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let dto = ProductVariantDto::from(variant);
        let price = dto.price.unwrap();
        assert_eq!(price.minor_units, 400);
        assert_eq!(price.currency, "USD");
    }

    #[test]
    fn product_variant_dto_debug() {
        let dto = ProductVariantDto {
            id: "v1".into(),
            parent_sku: "TEA".into(),
            name: "Green".into(),
            sku: "TEA-GREEN".into(),
            price: None,
            barcode: None,
            sort_order: 0,
            is_active: true,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("TEA-GREEN"));
    }

    #[test]
    fn money_dto_serialize() {
        let dto = MoneyDto {
            minor_units: 500,
            currency: "IDR".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["minor_units"], 500);
        assert_eq!(json["currency"], "IDR");
    }

    #[test]
    fn create_product_variant_args_deserialize() {
        let json = r#"{"parent_sku":"TEA","name":"Green","sku":"TEA-GREEN","price_minor":400,"currency":"USD","barcode":null,"sort_order":1,"is_active":true}"#;
        let args: CreateProductVariantArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.parent_sku, "TEA");
        assert_eq!(args.sku, "TEA-GREEN");
        assert_eq!(args.price_minor, Some(400));
        assert_eq!(args.sort_order, Some(1));
    }

    #[test]
    fn create_product_variant_args_deserialize_minimal() {
        let json = r#"{"parent_sku":"TEA","name":"Oolong","sku":"TEA-OOLONG"}"#;
        let args: CreateProductVariantArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Oolong");
        assert_eq!(args.price_minor, None);
        assert_eq!(args.sort_order, None);
    }

    #[test]
    fn create_product_variant_result_serialize() {
        let result = CreateProductVariantResult {
            sku: "TEA-GREEN".into(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["sku"], "TEA-GREEN");
    }

    #[test]
    fn update_product_variant_args_deserialize() {
        let json = r#"{"sku":"TEA-GREEN","name":"Green XL","price_minor":450,"currency":"USD","barcode":null,"sort_order":2,"is_active":true}"#;
        let args: UpdateProductVariantArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "TEA-GREEN");
        assert_eq!(args.name, Some("Green XL".into()));
        assert_eq!(args.price_minor, Some(450));
    }

    #[test]
    fn update_product_variant_args_deserialize_minimal() {
        let json = r#"{"sku":"TEA-BLACK"}"#;
        let args: UpdateProductVariantArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "TEA-BLACK");
        assert_eq!(args.name, None);
        assert_eq!(args.is_active, None);
    }

    #[test]
    fn update_product_variant_result_serialize() {
        let result = UpdateProductVariantResult {
            sku: "TEA-GREEN".into(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["sku"], "TEA-GREEN");
    }
}
