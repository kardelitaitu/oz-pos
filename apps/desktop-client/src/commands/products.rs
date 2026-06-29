//! Product catalog commands.
//!
//! `list_products` fetches all products with category names and stock
//! quantities from the database and returns them as a JSON array.
//! The front-end uses this to populate the product grid.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Money, Store};

use oz_core::events::{ProductCreated, StockAdjusted};

use crate::error::AppError;
use crate::state::AppState;

// ── Adjust stock ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AdjustStockArgs {
    /// SKU of the product to adjust.
    pub sku: String,
    /// Quantity change (positive = restock, negative = removal).
    pub delta: i64,
    /// Reason for the adjustment (e.g. "stock-take", "damaged", "return").
    pub reason: String,
}

/// Adjust stock for a product identified by SKU.
///
/// Positive `delta` restocks, negative `delta` removes stock.
/// Returns the new quantity on success.
#[command]
pub async fn adjust_stock(
    args: AdjustStockArgs,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    if args.sku.trim().is_empty() {
        return Err(AppError::Invalid("SKU must not be empty".into()));
    }
    if args.reason.trim().is_empty() {
        return Err(AppError::Invalid("reason must not be empty".into()));
    }
    if args.delta == 0 {
        return Err(AppError::Invalid("delta must be non-zero".into()));
    }

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    let new_qty = {
        let db = state.db.lock().await;
        let store = oz_core::db::Store::new(&db);
        store.adjust_stock(&args.sku, args.delta)?
    };

    // Publish the StockAdjusted domain event so that subscribers
    // (AuditLogHandler, etc.) fire their side effects.
    {
        let event = StockAdjusted {
            sku: args.sku.clone(),
            delta: args.delta,
            new_qty,
            reason: args.reason.clone(),
        };

        let kernel = state.kernel.lock().await;
        let bus = kernel.event_bus();
        if let Err(e) = bus.publish(&event) {
            // Logged by the bus; do not fail the command.
            tracing::warn!(sku = %args.sku, error = %e, "event bus publish failed");
        }
    }

    tracing::info!(sku = %args.sku, delta = %args.delta, reason = %args.reason, new_qty, "stock adjusted");
    Ok(new_qty)
}

/// A product DTO for the front-end, mapped from `ProductWithDetails`.
#[derive(Debug, Serialize)]
pub struct ProductDto {
    /// Stock-keeping unit — the human-readable product code.
    pub sku: String,
    /// Display name shown on receipts and the POS UI.
    pub name: String,
    /// Category display name, if the product is linked to a category.
    pub category: Option<String>,
    /// Sale price with currency.
    pub price: MoneyDto,
    /// Machine-readable barcode (EAN-13, UPC-A, etc.) if available.
    pub barcode: Option<String>,
    /// Whether the product is in stock (stock_qty > 0 or null = false).
    pub in_stock: bool,
    /// Current stock quantity, or `null` if tracking is disabled.
    pub stock_qty: Option<i64>,
    /// Tax rate IDs assigned to this product.
    pub tax_rate_ids: Vec<String>,
}

/// Money DTO matching the front-end `Money` type (snake_case keys).
#[derive(Debug, Serialize)]
pub struct MoneyDto {
    pub minor_units: i64,
    pub currency: String,
}

/// Fetch all products from the database.
///
/// Returns an array of product DTOs with category names and stock
/// status. The front-end calls this on mount to populate the product
/// lookup grid.
#[command]
pub async fn list_products(state: State<'_, AppState>) -> Result<Vec<ProductDto>, AppError> {
    let db = state.db.lock().await;
    run_list_products(&db)
}

/// Business logic for listing products (extracted for testing).
fn run_list_products(conn: &rusqlite::Connection) -> Result<Vec<ProductDto>, AppError> {
    let store = Store::new(conn);
    let products = store.list_products()?;

    let dtos: Vec<ProductDto> = products
        .into_iter()
        .map(|pwd| {
            let cur_str = std::str::from_utf8(&pwd.product.price.currency.0)
                .unwrap_or("USD")
                .to_owned();
            ProductDto {
                sku: pwd.product.sku.to_string(),
                name: pwd.product.name,
                category: pwd.category_name,
                price: MoneyDto {
                    minor_units: pwd.product.price.minor_units,
                    currency: cur_str,
                },
                barcode: pwd.product.barcode,
                in_stock: pwd.stock_qty.is_some_and(|q| q > 0),
                stock_qty: pwd.stock_qty,
                tax_rate_ids: store
                    .get_product_tax_rates(pwd.product.sku.as_str())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Ok(dtos)
}

// ── Lookup by barcode ────────────────────────────────────────────────

/// Look up a single product by barcode.
///
/// Returns the product DTO or `null` when no match is found.
/// Returns validation error for empty barcodes.
#[command]
pub async fn lookup_by_barcode(
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    if barcode.trim().is_empty() {
        return Err(AppError::Invalid("barcode must not be empty".into()));
    }
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let pwd = store.lookup_product_with_details_by_barcode(&barcode)?;
    let tax_rate_ids = match pwd {
        Some(ref p) => store
            .get_product_tax_rates(p.product.sku.as_str())
            .unwrap_or_default(),
        None => vec![],
    };
    drop(db);
    Ok(pwd.map(|pwd| {
        let cur_str = std::str::from_utf8(&pwd.product.price.currency.0)
            .unwrap_or("USD")
            .to_owned();
        ProductDto {
            sku: pwd.product.sku.to_string(),
            name: pwd.product.name,
            category: pwd.category_name,
            price: MoneyDto {
                minor_units: pwd.product.price.minor_units,
                currency: cur_str,
            },
            barcode: pwd.product.barcode,
            in_stock: pwd.stock_qty.is_some_and(|q| q > 0),
            stock_qty: pwd.stock_qty,
            tax_rate_ids,
        }
    }))
}

// ── Create product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProductArgs {
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub initial_stock: i64,
    pub tax_rate_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateProductResult {
    pub sku: String,
}

#[command]
pub async fn create_product(
    args: CreateProductArgs,
    state: State<'_, AppState>,
) -> Result<CreateProductResult, AppError> {
    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    {
        let db = state.db.lock().await;
        let store = Store::new(&db);

        let currency: oz_core::Currency = args
            .currency
            .parse()
            .map_err(|_| AppError::Invalid(format!("invalid currency '{}'", args.currency)))?;

        let price = Money {
            minor_units: args.price_minor,
            currency,
        };

        store.create_product(
            &args.sku,
            &args.name,
            price,
            args.category_id.as_deref(),
            args.barcode.as_deref(),
            args.initial_stock,
        )?;

        store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;
    } // db and store dropped here before .await

    // Publish the ProductCreated domain event so that subscribers
    // (AuditLogHandler, etc.) fire their side effects.
    {
        let event = ProductCreated {
            sku: args.sku.clone(),
            name: args.name.clone(),
            price_minor: args.price_minor,
            currency: args.currency.clone(),
            category_id: args.category_id.clone(),
            barcode: args.barcode.clone(),
            initial_stock: args.initial_stock,
        };

        let kernel = state.kernel.lock().await;
        let bus = kernel.event_bus();
        if let Err(e) = bus.publish(&event) {
            // Logged by the bus; do not fail the command.
            tracing::warn!(sku = %args.sku, error = %e, "event bus publish failed");
        }
    }

    tracing::info!(sku = %args.sku, name = %args.name, "product created");
    Ok(CreateProductResult { sku: args.sku })
}

// ── Update product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateProductArgs {
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub tax_rate_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateProductResult {
    pub sku: String,
}

#[command]
pub async fn update_product(
    args: UpdateProductArgs,
    state: State<'_, AppState>,
) -> Result<UpdateProductResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let currency: oz_core::Currency = args
        .currency
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency '{}'", args.currency)))?;

    let price = Money {
        minor_units: args.price_minor,
        currency,
    };

    store.update_product(
        &args.sku,
        &args.name,
        price,
        args.category_id.as_deref(),
        args.barcode.as_deref(),
    )?;

    store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;

    Ok(UpdateProductResult { sku: args.sku })
}

// ── Delete product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteProductArgs {
    pub sku: String,
}

#[command]
pub async fn delete_product(
    args: DeleteProductArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_product(&args.sku)?;
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

    #[test]
    fn list_products_empty_db() {
        let conn = fresh_conn();
        let products = run_list_products(&conn).unwrap();
        assert!(products.is_empty());
    }

    #[test]
    fn list_products_with_seeded_data() {
        let conn = fresh_conn();

        // Seed some products directly via SQL.
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour) VALUES
                ('cat-drinks', 'Drinks', '#06b6d4'),
                ('cat-food',   'Food',   '#f97316');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at) VALUES
                ('p1', 'LATTE',  'Caffè Latte',  450, 'USD', 'cat-drinks', '4901234567890', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('p2', 'BAGEL',  'Plain Bagel',   250, 'USD', 'cat-food',   NULL,           '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('p3', 'BROWNIE','Fudge Brownie', 295, 'USD', 'cat-food',   '4901234567906', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty) VALUES
                ('p1', 50),
                ('p2', 12);",
        )
        .unwrap();

        let products = run_list_products(&conn).unwrap();
        assert_eq!(products.len(), 3);

        // Check LATTE.
        let latte = products.iter().find(|p| p.sku == "LATTE").unwrap();
        assert_eq!(latte.name, "Caffè Latte");
        assert_eq!(latte.category.as_deref(), Some("Drinks"));
        assert_eq!(latte.price.minor_units, 450);
        assert_eq!(latte.barcode.as_deref(), Some("4901234567890"));
        assert!(latte.in_stock);

        // Check BROWNIE (has no inventory row).
        let brownie = products.iter().find(|p| p.sku == "BROWNIE").unwrap();
        assert!(!brownie.in_stock);
    }
}
