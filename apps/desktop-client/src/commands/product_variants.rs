//! Product variant Tauri commands.
//!
//! CRUD operations for product variants (size, colour, flavour).
//! Each variant is linked to a parent product via `parent_sku` and has
//! its own SKU, optional price override, and barcode.

use serde::{Deserialize, Serialize};
use tauri::State;

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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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

#[cfg(test)] #[path = "product_variants_tests.rs"] mod tests;
