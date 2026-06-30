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
    pub minor_units: i64,
    pub currency: String,
}

/// Product variant DTO for the front-end.
#[derive(Debug, Serialize)]
pub struct ProductVariantDto {
    pub id: String,
    pub parent_sku: String,
    pub name: String,
    pub sku: String,
    pub price: Option<MoneyDto>,
    pub barcode: Option<String>,
    pub sort_order: i64,
    pub is_active: bool,
    pub created_at: String,
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
pub struct CreateProductVariantArgs {
    pub parent_sku: String,
    pub name: String,
    pub sku: String,
    pub price_minor: Option<i64>,
    pub currency: Option<String>,
    pub barcode: Option<String>,
    pub sort_order: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CreateProductVariantResult {
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
pub struct UpdateProductVariantArgs {
    pub sku: String,
    pub name: Option<String>,
    pub price_minor: Option<i64>,
    pub currency: Option<String>,
    pub barcode: Option<String>,
    pub sort_order: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UpdateProductVariantResult {
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
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
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
}
