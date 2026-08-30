//! Product variant Tauri commands.
//!
//! CRUD operations for product variants (size, colour, flavour).
//! Each variant is linked to a parent product via `parent_sku` and has
//! its own SKU, optional price override, and barcode.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::{Money, ProductVariant, Store};

use foundation::validate_not_empty;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;
use oz_core::permissions;

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

// ── Get by SKU ────────────────────────────────────────────────────────

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

// ── Delete ────────────────────────────────────────────────────────────

// ── Scoped variants (ADR #7) ────────────────────────────────────

/// Scoped variant of `list_product_variants` (ADR #7).
#[tauri::command]
pub async fn list_product_variants_scoped(
    parent_sku: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProductVariantDto>, AppError> {
    validate_not_empty("parent_sku", &parent_sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let (session, _conn) = state.resolve_scope(&session_token)?;

    // F-017: enforce per-domain permission on this scoped command.

    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let variants = store.list_product_variants(&parent_sku)?;
    drop(db);

    let dtos: Vec<ProductVariantDto> = variants.into_iter().map(ProductVariantDto::from).collect();
    Ok(dtos)
}

/// Scoped variant of `get_product_variant` (ADR #7).
#[tauri::command]
pub async fn get_product_variant_scoped(
    sku: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductVariantDto>, AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let (session, _conn) = state.resolve_scope(&session_token)?;

    // F-017: enforce per-domain permission on this scoped command.

    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let variant = store.get_product_variant(&sku)?;
    drop(db);

    Ok(variant.map(ProductVariantDto::from))
}

/// Scoped variant of `delete_product_variant` (ADR #7).
#[tauri::command]
pub async fn delete_product_variant_scoped(
    sku: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let (session, _conn) = state.resolve_scope(&session_token)?;

    // F-017: enforce per-domain permission on this scoped command.

    require_permission_for_session(&state, &session, permissions::PRODUCTS_DELETE).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.delete_product_variant(&sku)?;
    drop(db);

    tracing::info!(sku, "product variant deleted");
    Ok(())
}

/// Create a product variant (scoped).
#[tauri::command]
pub async fn create_product_variant_scoped(
    args: CreateProductVariantArgs,
    session_token: String,
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

    let (session, conn) = state.resolve_scope(&session_token)?;

    // F-017: enforce per-domain permission on this scoped command.

    require_permission_for_session(&state, &session, permissions::PRODUCTS_CREATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.create_product_variant(&variant)?;
    drop(db);

    tracing::info!(sku = %variant.sku, parent_sku = %variant.parent_sku, "product variant created (scoped)");
    Ok(CreateProductVariantResult { sku: variant.sku })
}

/// Update an existing product variant (scoped).
#[tauri::command]
pub async fn update_product_variant_scoped(
    args: UpdateProductVariantArgs,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<UpdateProductVariantResult, AppError> {
    validate_not_empty("sku", &args.sku).map_err(|e| AppError::Invalid(e.to_string()))?;

    let (session, conn) = state.resolve_scope(&session_token)?;

    // F-017: enforce per-domain permission on this scoped command.

    require_permission_for_session(&state, &session, permissions::PRODUCTS_UPDATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

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
    if let Some(barcode) = args.barcode {
        if barcode.is_empty() {
            variant.barcode = None;
        } else {
            let parsed = foundation::Barcode::new(&barcode)
                .map_err(|e| AppError::Invalid(e.message.to_string()))?;
            variant.barcode = Some(parsed);
        }
    }
    if let Some(order) = args.sort_order {
        variant.sort_order = order;
    }
    if let Some(active) = args.is_active {
        variant.is_active = active;
    }

    variant.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    store.update_product_variant(&variant)?;
    drop(db);

    tracing::info!(sku = %variant.sku, "product variant updated (scoped)");
    Ok(UpdateProductVariantResult { sku: variant.sku })
}

#[cfg(test)]
#[path = "product_variants_tests.rs"]
mod tests;
