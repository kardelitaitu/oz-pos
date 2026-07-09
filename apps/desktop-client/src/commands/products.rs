//! Product catalog commands.
//!
//! `list_products` fetches all products with category names and stock
//! quantities from the database and returns them as a JSON array.
//! The front-end uses this to populate the product grid.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Money, Store};

use oz_core::events::{ProductCreated, StockAdjusted};

use foundation::validate_not_empty;

use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
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
    validate_not_empty("sku", &args.sku).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("reason", &args.reason).map_err(|e| AppError::Invalid(e.to_string()))?;
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
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 timestamp of the last price change.
    pub price_updated_at: String,
    /// Product type: "retail", "restaurant", or "both".
    pub product_type: String,
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
                barcode: pwd.product.barcode.as_ref().map(|b| b.to_string()),
                in_stock: pwd.stock_qty.is_some_and(|q| q > 0),
                stock_qty: pwd.stock_qty,
                created_at: pwd.product.created_at,
                price_updated_at: pwd.product.price_updated_at,
                product_type: pwd.product.product_type.as_str().to_owned(),
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
    validate_not_empty("barcode", &barcode).map_err(|e| AppError::Invalid(e.to_string()))?;
    let db = state.db.lock().await;
    let _store = Store::new(&db);
    let result = run_lookup_by_barcode(&db, &barcode);
    drop(db);
    result
}

/// Business logic for barcode lookup (extracted for testing).
fn run_lookup_by_barcode(
    conn: &rusqlite::Connection,
    barcode: &str,
) -> Result<Option<ProductDto>, AppError> {
    let store = Store::new(conn);
    let pwd = store.lookup_product_with_details_by_barcode(barcode)?;
    map_pwd_to_dto(&store, pwd)
}

/// Look up a single product by SKU.
///
/// Returns the product DTO or `null` when no match is found.
#[command]
pub async fn lookup_product_by_sku(
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;
    let db = state.db.lock().await;
    let _store = Store::new(&db);
    let result = run_lookup_product_by_sku(&db, &sku);
    drop(db);
    result
}

/// Business logic for SKU lookup (extracted for testing).
fn run_lookup_product_by_sku(
    conn: &rusqlite::Connection,
    sku: &str,
) -> Result<Option<ProductDto>, AppError> {
    let store = Store::new(conn);
    let pwd = store.get_product(sku)?;
    map_pwd_to_dto(&store, pwd)
}

/// Shared mapping from `ProductWithDetails` to `ProductDto`.
fn map_pwd_to_dto(
    store: &Store<'_>,
    pwd: Option<oz_core::db::ProductWithDetails>,
) -> Result<Option<ProductDto>, AppError> {
    let tax_rate_ids = match pwd {
        Some(ref p) => store
            .get_product_tax_rates(p.product.sku.as_str())
            .unwrap_or_default(),
        None => vec![],
    };
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
            barcode: pwd.product.barcode.as_ref().map(|b| b.to_string()),
            in_stock: pwd.stock_qty.is_some_and(|q| q > 0),
            stock_qty: pwd.stock_qty,
            tax_rate_ids,
            product_type: pwd.product.product_type.as_str().to_owned(),
            created_at: pwd.product.created_at,
            price_updated_at: pwd.product.price_updated_at,
        }
    }))
}

// ── Create product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProductArgs {
    pub user_id: String,
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub initial_stock: i64,
    pub tax_rate_ids: Vec<String>,
    #[serde(default = "default_product_type")]
    pub product_type: String,
}

fn default_product_type() -> String {
    "retail".to_owned()
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

        require_permission_for_user(&store, &args.user_id, permissions::PRODUCTS_CREATE)?;

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
            Some(&args.product_type),
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
            barcode: args
                .barcode
                .as_ref()
                .and_then(|s| foundation::Barcode::new(s).ok()),
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
    pub user_id: String,
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub tax_rate_ids: Vec<String>,
    pub product_type: Option<String>,
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

    require_permission_for_user(&store, &args.user_id, permissions::PRODUCTS_UPDATE)?;

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
        args.product_type.as_deref(),
    )?;

    store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;

    Ok(UpdateProductResult { sku: args.sku })
}

/// Check whether a product tracks serial numbers.
#[command]
pub async fn get_product_track_serial(
    sku: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let product = store.get_product(&sku)?;
    drop(db);
    Ok(product.map(|p| p.product.track_serial).unwrap_or(false))
}

// ── Delete product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteProductArgs {
    pub user_id: String,
    pub sku: String,
}

#[command]
pub async fn delete_product(
    args: DeleteProductArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &args.user_id, permissions::PRODUCTS_DELETE)?;
    store.delete_product(&args.sku)?;
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
            "INSERT INTO categories (id, name, colour, icon) VALUES
                ('cat-drinks', 'Drinks', '#06b6d4', ''),
                ('cat-food',   'Food',   '#f97316', '');
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
        // Also verify BROWNIE has a barcode.
        let brownie = products.iter().find(|p| p.sku == "BROWNIE").unwrap();
        assert_eq!(brownie.barcode.as_deref(), Some("4901234567906"));
        assert!(latte.in_stock);

        // Check BROWNIE (has no inventory row).
        let brownie = products.iter().find(|p| p.sku == "BROWNIE").unwrap();
        assert!(!brownie.in_stock);
    }

    // ── Barcode lookup integration tests ─────────────────────────────

    #[test]
    fn lookup_by_barcode_found() {
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour, icon) VALUES
                ('cat-drinks', 'Drinks', '#06b6d4', '');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at) VALUES
                ('p1', 'LATTE', 'Caffè Latte', 450, 'USD', 'cat-drinks', '4901234567890', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty) VALUES ('p1', 50);",
        )
        .unwrap();

        let result = run_lookup_by_barcode(&conn, "4901234567890").unwrap();
        let dto = result.expect("expected product for known barcode");
        assert_eq!(dto.sku, "LATTE");
        assert_eq!(dto.name, "Caffè Latte");
        assert_eq!(dto.category.as_deref(), Some("Drinks"));
        assert_eq!(dto.price.minor_units, 450);
        assert_eq!(dto.barcode.as_deref(), Some("4901234567890"));
        assert!(dto.in_stock);
        assert_eq!(dto.stock_qty, Some(50));
    }

    #[test]
    fn lookup_by_barcode_not_found() {
        let conn = fresh_conn();
        let result = run_lookup_by_barcode(&conn, "0000000000000").unwrap();
        assert!(result.is_none(), "unknown barcode should return None");
    }

    #[test]
    fn lookup_by_barcode_returns_product_without_barcode() {
        // A product with no barcode stored (NULL in DB) should NOT be
        // returned when looking up a barcode — confirm the DB query works.
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, barcode, created_at, updated_at) VALUES
                ('p1', 'TEA', 'Green Tea', 275, 'USD', NULL, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();

        let result = run_lookup_by_barcode(&conn, "2750000000000").unwrap();
        assert!(result.is_none(), "no match for random barcode");
    }

    // ── SKU lookup integration tests ─────────────────────────────────

    #[test]
    fn lookup_product_by_sku_found() {
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour, icon) VALUES
                ('cat-drinks', 'Drinks', '#06b6d4', '');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at) VALUES
                ('p1', 'LATTE', 'Caffè Latte', 450, 'USD', 'cat-drinks', '4901234567890', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty) VALUES ('p1', 50);",
        )
        .unwrap();

        let result = run_lookup_product_by_sku(&conn, "LATTE").unwrap();
        let dto = result.expect("expected product for known SKU");
        assert_eq!(dto.sku, "LATTE");
        assert_eq!(dto.name, "Caffè Latte");
        assert_eq!(dto.category.as_deref(), Some("Drinks"));
        assert_eq!(dto.price.minor_units, 450);
        assert_eq!(dto.barcode.as_deref(), Some("4901234567890"));
        assert!(dto.in_stock);
        assert_eq!(dto.stock_qty, Some(50));
    }

    #[test]
    fn lookup_product_by_sku_not_found() {
        let conn = fresh_conn();
        let result = run_lookup_product_by_sku(&conn, "NO-SUCH-SKU").unwrap();
        assert!(result.is_none(), "unknown SKU should return None");
    }

    #[test]
    fn lookup_product_by_sku_without_stock() {
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, barcode, created_at, updated_at) VALUES
                ('p1', 'UNSTOCKED', 'Unstocked Item', 199, 'USD', NULL, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();

        let result = run_lookup_product_by_sku(&conn, "UNSTOCKED").unwrap();
        let dto = result.expect("expected product for known SKU without stock");
        assert_eq!(dto.sku, "UNSTOCKED");
        assert_eq!(dto.name, "Unstocked Item");
        assert_eq!(dto.price.minor_units, 199);
        assert!(!dto.in_stock);
        assert_eq!(dto.stock_qty, None);
    }

    // -- DTO struct tests --

    #[test]
    fn product_dto_serialize() {
        let dto = ProductDto {
            sku: "COFFEE".into(),
            name: "Caffe Latte".into(),
            category: Some("Drinks".into()),
            price: MoneyDto {
                minor_units: 450,
                currency: "USD".into(),
            },
            barcode: Some("4901234567890".into()),
            in_stock: true,
            stock_qty: Some(50),
            tax_rate_ids: vec!["t1".into()],
            created_at: "2025-01-01".into(),
            price_updated_at: "2025-01-01".into(),
            product_type: "retail".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["sku"], "COFFEE");
        assert_eq!(json["price"]["minor_units"], 450);
    }

    #[test]
    fn product_dto_debug() {
        let dto = ProductDto {
            sku: "TEA".into(),
            name: "Green Tea".into(),
            category: None,
            price: MoneyDto {
                minor_units: 275,
                currency: "USD".into(),
            },
            barcode: None,
            in_stock: false,
            stock_qty: None,
            tax_rate_ids: vec![],
            created_at: "2025-01-01".into(),
            price_updated_at: "2025-01-01".into(),
            product_type: "retail".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Green Tea"));
    }

    #[test]
    fn money_dto_serialize() {
        let dto = MoneyDto {
            minor_units: 1550,
            currency: "IDR".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["minor_units"], 1550);
        assert_eq!(json["currency"], "IDR");
    }

    #[test]
    fn money_dto_debug() {
        let dto = MoneyDto {
            minor_units: 100,
            currency: "EUR".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("EUR"));
    }

    #[test]
    fn adjust_stock_args_deserialize() {
        let json = r##"{"sku":"COFFEE","delta":10,"reason":"restock"}"##;
        let args: AdjustStockArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "COFFEE");
        assert_eq!(args.delta, 10);
    }

    #[test]
    fn adjust_stock_args_debug() {
        let args = AdjustStockArgs {
            sku: "S".into(),
            delta: -5,
            reason: "damaged".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("damaged"));
    }

    #[test]
    fn create_product_args_deserialize() {
        let json = r##"{"user_id":"u1","sku":"LATTE","name":"Latte","price_minor":450,"currency":"USD","category_id":null,"barcode":null,"initial_stock":0,"tax_rate_ids":[]}"##;
        let args: CreateProductArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "LATTE");
        assert_eq!(args.price_minor, 450);
    }

    #[test]
    fn create_product_args_debug() {
        let args = CreateProductArgs {
            user_id: "u".into(),
            sku: "S".into(),
            name: "N".into(),
            price_minor: 100,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 0,
            tax_rate_ids: vec![],
            product_type: "retail".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("N"));
    }

    #[test]
    fn create_product_result_serialize() {
        let result = CreateProductResult {
            sku: "NEW-SKU".into(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["sku"], "NEW-SKU");
    }

    #[test]
    fn create_product_result_debug() {
        let result = CreateProductResult { sku: "X".into() };
        let d = format!("{result:?}");
        assert!(d.contains("X"));
    }

    #[test]
    fn update_product_args_deserialize() {
        let json = r##"{"user_id":"u1","sku":"LATTE","name":"Latte Updated","price_minor":500,"currency":"USD","category_id":null,"barcode":null,"tax_rate_ids":[]}"##;
        let args: UpdateProductArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Latte Updated");
        assert_eq!(args.price_minor, 500);
    }

    #[test]
    fn update_product_result_serialize() {
        let result = UpdateProductResult {
            sku: "UPD-SKU".into(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["sku"], "UPD-SKU");
    }

    #[test]
    fn delete_product_args_deserialize() {
        let json = r##"{"user_id":"u1","sku":"OLD-SKU"}"##;
        let args: DeleteProductArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "OLD-SKU");
    }

    #[test]
    fn delete_product_args_debug() {
        let args = DeleteProductArgs {
            user_id: "u".into(),
            sku: "S".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("S"));
    }
}
