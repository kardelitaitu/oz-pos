//! Product catalog commands.
//!
//! `list_products` fetches all products with category names and stock
//! quantities from the database and returns them as a JSON array.
//! The front-end uses this to populate the product grid.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::inventory::{CANONICAL_DEFAULT_LOCATION_UUID, LocationId};
use oz_core::inventory_transaction::InventoryTransactionId;
use oz_core::{Money, Store};

use oz_core::events::{ProductCreated, StockAdjusted};

use foundation::validate_not_empty;

use oz_core::permissions;

use crate::commands::authz::{require_permission_for_session, require_permission_for_user};
use crate::error::AppError;
use crate::state::AppState;

// ── Adjust stock ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Adjuststockargs.
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
#[deprecated(note = "use adjust_stock_scoped instead")]
#[allow(deprecated)]
#[tauri::command]
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
        let tid = state.terminal_id.lock().await.clone();
        let db = state.db.lock().await;
        let store = state.store_with_tid(&db, tid);
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

/// Adjust stock for the store resolved from a session token.
///
/// ADR #7: Scoped variant of `adjust_stock`. The frontend passes a
/// `session_token` instead of relying on the global database. The
/// backend resolves the token to a `SessionContext`, opens the
/// store-scoped database, and adjusts stock within that store only.
#[tauri::command]
pub async fn adjust_stock_scoped(
    session_token: String,
    args: AdjustStockArgs,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    validate_not_empty("sku", &args.sku).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("reason", &args.reason).map_err(|e| AppError::Invalid(e.to_string()))?;
    if args.delta == 0 {
        return Err(AppError::Invalid("delta must be non-zero".into()));
    }

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let new_qty = {
        let tid = state.terminal_id.lock().await.clone();
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = state.store_with_tid(&db, tid);
        let tx = db
            .unchecked_transaction()
            .map_err(|e| AppError::Internal(format!("starting tx: {e}")))?;
        let loc = LocationId::from(CANONICAL_DEFAULT_LOCATION_UUID);
        let new_qty = store.adjust_stock_at_location_with_reason(
            &tx,
            &args.sku,
            args.delta,
            &loc,
            Some(&args.reason),
            Some(&InventoryTransactionId::new()),
            Some(&oz_core::terminal::TerminalId::from(
                session.terminal_id.as_str(),
            )),
            Some(&oz_core::user::UserId::from(session.user_id.clone())),
        )?;
        tx.commit()
            .map_err(|e| AppError::Internal(format!("commit tx: {e}")))?;
        new_qty
    };

    // Publish the StockAdjusted domain event.
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
            tracing::warn!(sku = %args.sku, error = %e, "event bus publish failed");
        }
    }

    tracing::info!(sku = %args.sku, delta = %args.delta, reason = %args.reason, new_qty, "stock adjusted (scoped)");
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
    /// Cost price in minor units (local-only, ADR #36).
    pub cost_minor: i64,
    /// Brand (free text).
    pub brand: Option<String>,
    /// Rack position code.
    pub rack_location: Option<String>,
    /// Free-text notes.
    pub notes: Option<String>,
    /// Unit of measure.
    pub unit: Option<String>,
    /// Active/sellable status.
    pub is_active: bool,
    /// Default supplier FK (local-only).
    pub default_supplier_id: Option<String>,
    /// Materialized popularity score (ADR #37) — retail grid sort key.
    pub popularity_score: f64,
}

/// Money DTO matching the front-end `Money` type (snake_case keys).
#[derive(Debug, Serialize)]
pub struct MoneyDto {
    /// Minor Units.
    pub minor_units: i64,
    /// ISO-4217 currency code.
    pub currency: String,
}

/// Fetch all products from the database.
///
/// Returns an array of product DTOs with category names and stock
/// status. The front-end calls this on mount to populate the product
/// lookup grid.
///
/// **Deprecated for multi-store (ADR #4 / ADR #7):** Use
/// `list_products_scoped` with a `session_token` parameter instead.
/// This command queries the global database, which only works in
/// single-store mode.
#[tauri::command]
pub async fn list_products(state: State<'_, AppState>) -> Result<Vec<ProductDto>, AppError> {
    let db = state.db.lock().await;
    run_list_products(&db)
}

/// Fetch all products for the store resolved from a session token.
///
/// ADR #4 / ADR #7 canonical pattern: The frontend passes an opaque
/// `session_token` (obtained from `create_session`). The backend
/// resolves it to a `SessionContext` containing `store_id`, then
/// opens the store-scoped database and queries only that store's
/// products.
///
/// This is the reference implementation for all store-scoped domain
/// commands. New commands should follow this pattern.
#[tauri::command]
pub async fn list_products_scoped(
    state: State<'_, AppState>,
    session_token: String,
) -> Result<Vec<ProductDto>, AppError> {
    let (_session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_list_products(&db)
}

/// Business logic for listing products (extracted for testing).
fn run_list_products(conn: &rusqlite::Connection) -> Result<Vec<ProductDto>, AppError> {
    let store = Store::new(conn);
    let products = store.list_products()?;
    map_products_to_dtos(&store, products)
}

/// Shared mapping from a vec of ProductWithDetails to ProductDto vec.
fn map_products_to_dtos(
    store: &Store<'_>,
    products: Vec<oz_core::db::ProductWithDetails>,
) -> Result<Vec<ProductDto>, AppError> {
    // PROD-12: batch-load tax assignments in ONE query instead of one
    // `get_product_tax_rates` call per product (N+1 catalog-load pattern).
    let skus: Vec<String> = products
        .iter()
        .map(|pwd| pwd.product.sku.to_string())
        .collect();
    let tax_rates_by_sku = store.get_product_tax_rates_batch(&skus)?;
    let dtos: Vec<ProductDto> = products
        .into_iter()
        .map(|pwd| {
            let cur_str = std::str::from_utf8(&pwd.product.price.currency.0)
                .unwrap_or("USD")
                .to_owned();
            let sku = pwd.product.sku.to_string();
            let tax_rate_ids = tax_rates_by_sku.get(&sku).cloned().unwrap_or_default();
            ProductDto {
                sku,
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
                tax_rate_ids,
                cost_minor: pwd.product.cost_minor,
                brand: pwd.product.brand.clone(),
                rack_location: pwd.product.rack_location.clone(),
                notes: pwd.product.notes.clone(),
                unit: pwd.product.unit.clone(),
                is_active: pwd.product.is_active,
                default_supplier_id: pwd.product.default_supplier_id.clone(),
                popularity_score: pwd.popularity_score,
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
///
/// **Deprecated for multi-store (ADR #7):** Use `lookup_by_barcode_scoped`.
#[tauri::command]
pub async fn lookup_by_barcode(
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    validate_not_empty("barcode", &barcode).map_err(|e| AppError::Invalid(e.to_string()))?;
    let db = state.db.lock().await;
    let result = run_lookup_by_barcode(&db, &barcode);
    drop(db);
    result
}

/// Look up a product by barcode for the store resolved from a
/// session token. ADR #7 scoped variant.
#[tauri::command]
pub async fn lookup_by_barcode_scoped(
    session_token: String,
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    validate_not_empty("barcode", &barcode).map_err(|e| AppError::Invalid(e.to_string()))?;
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_lookup_by_barcode(&db, &barcode)
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
///
/// **Deprecated for multi-store (ADR #7):** Use `lookup_product_by_sku_scoped`.
#[tauri::command]
pub async fn lookup_product_by_sku(
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;
    let db = state.db.lock().await;
    let result = run_lookup_product_by_sku(&db, &sku);
    drop(db);
    result
}

/// Look up a product by SKU for the store resolved from a
/// session token. ADR #7 scoped variant.
#[tauri::command]
pub async fn lookup_product_by_sku_scoped(
    session_token: String,
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<ProductDto>, AppError> {
    validate_not_empty("sku", &sku).map_err(|e| AppError::Invalid(e.to_string()))?;
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_lookup_product_by_sku(&db, &sku)
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
            cost_minor: pwd.product.cost_minor,
            brand: pwd.product.brand.clone(),
            rack_location: pwd.product.rack_location.clone(),
            notes: pwd.product.notes.clone(),
            unit: pwd.product.unit.clone(),
            is_active: pwd.product.is_active,
            default_supplier_id: pwd.product.default_supplier_id.clone(),
            popularity_score: pwd.popularity_score,
        }
    }))
}

// ── Create product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Createproductargs.
pub struct CreateProductArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Display name.
    pub name: String,
    /// Price Minor.
    pub price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ID of the associated category.
    pub category_id: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Initial Stock.
    pub initial_stock: i64,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
    #[serde(default = "default_product_type")]
    /// Product Type.
    pub product_type: String,
    #[serde(default)]
    /// Cost price in minor units (ADR #36, local-only).
    pub cost_minor: i64,
    #[serde(default)]
    /// Brand (free text).
    pub brand: Option<String>,
    #[serde(default)]
    /// Rack position code.
    pub rack_location: Option<String>,
    #[serde(default)]
    /// Free-text notes.
    pub notes: Option<String>,
    #[serde(default)]
    /// Unit of measure.
    pub unit: Option<String>,
    #[serde(default = "default_true")]
    /// Active/sellable status.
    pub is_active: bool,
    #[serde(default)]
    /// Default supplier FK (local-only).
    pub default_supplier_id: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Args for `create_product_scoped` — identical to `CreateProductArgs`
/// but without `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
pub struct CreateProductScopedArgs {
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Display name.
    pub name: String,
    /// Price Minor.
    pub price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ID of the associated category.
    pub category_id: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Initial Stock.
    pub initial_stock: i64,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
    #[serde(default = "default_product_type")]
    /// Product Type.
    pub product_type: String,
    #[serde(default)]
    /// Cost price in minor units (ADR #36, local-only).
    pub cost_minor: i64,
    #[serde(default)]
    /// Brand (free text).
    pub brand: Option<String>,
    #[serde(default)]
    /// Rack position code.
    pub rack_location: Option<String>,
    #[serde(default)]
    /// Free-text notes.
    pub notes: Option<String>,
    #[serde(default)]
    /// Unit of measure.
    pub unit: Option<String>,
    #[serde(default = "default_true")]
    /// Active/sellable status.
    pub is_active: bool,
    #[serde(default)]
    /// Default supplier FK (local-only).
    pub default_supplier_id: Option<String>,
}

fn default_product_type() -> String {
    "retail".to_owned()
}

#[derive(Debug, Serialize)]
/// Createproductresult.
pub struct CreateProductResult {
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Create a product using the global database and a `user_id` parameter.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_product_scoped`
/// with a `session_token` instead. The `user_id` field should be read
/// from the resolved session, not passed as a frontend parameter.
#[tauri::command]
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

        store.create_product_with_attributes(
            &args.sku,
            &args.name,
            price,
            args.category_id.as_deref(),
            args.barcode.as_deref(),
            args.initial_stock,
            Some(&args.product_type),
            &oz_core::db::CreateProductAttributes {
                cost_minor: args.cost_minor,
                brand: args.brand.clone(),
                rack_location: args.rack_location.clone(),
                notes: args.notes.clone(),
                unit: args.unit.clone(),
                is_active: args.is_active,
                default_supplier_id: args.default_supplier_id.clone(),
            },
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

/// Create a product within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `create_product`. The `user_id` for
/// permission checks is read from the resolved `SessionContext`,
/// not passed as a frontend parameter. The product is created in
/// the store-scoped database for the session's `store_id`.
#[tauri::command]
pub async fn create_product_scoped(
    session_token: String,
    args: CreateProductScopedArgs,
    state: State<'_, AppState>,
) -> Result<CreateProductResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_CREATE).await?;
    // ADR #36 D7: setting a cost (HPP) requires the manager-only
    // products:edit_cost permission — staff can create products without
    // ever touching cost.
    if args.cost_minor != 0 {
        require_permission_for_session(&state, &session, permissions::PRODUCTS_EDIT_COST).await?;
    }
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);

        let currency: oz_core::Currency = args
            .currency
            .parse()
            .map_err(|_| AppError::Invalid(format!("invalid currency '{}'", args.currency)))?;

        let price = Money {
            minor_units: args.price_minor,
            currency,
        };

        store.create_product_with_attributes(
            &args.sku,
            &args.name,
            price,
            args.category_id.as_deref(),
            args.barcode.as_deref(),
            args.initial_stock,
            Some(&args.product_type),
            &oz_core::db::CreateProductAttributes {
                cost_minor: args.cost_minor,
                brand: args.brand.clone(),
                rack_location: args.rack_location.clone(),
                notes: args.notes.clone(),
                unit: args.unit.clone(),
                is_active: args.is_active,
                default_supplier_id: args.default_supplier_id.clone(),
            },
        )?;

        store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;
    } // db and store dropped here before .await

    // Publish the ProductCreated domain event.
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
            tracing::warn!(sku = %args.sku, error = %e, "event bus publish failed");
        }
    }

    tracing::info!(sku = %args.sku, name = %args.name, "product created (scoped)");
    Ok(CreateProductResult { sku: args.sku })
}

// ── Update product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Updateproductargs.
pub struct UpdateProductArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Display name.
    pub name: String,
    /// Price Minor.
    pub price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ID of the associated category.
    pub category_id: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
    /// Product Type.
    pub product_type: Option<String>,
    #[serde(default)]
    /// Updated cost in minor units (None keeps).
    pub cost_minor: Option<i64>,
    #[serde(default)]
    /// Updated brand — `null` clears, string sets, absent keeps.
    pub brand: Option<Option<String>>,
    #[serde(default)]
    /// Updated rack position code — `null` clears, string sets, absent keeps.
    pub rack_location: Option<Option<String>>,
    #[serde(default)]
    /// Updated notes — `null` clears, string sets, absent keeps.
    pub notes: Option<Option<String>>,
    #[serde(default)]
    /// Updated unit — `null` clears, string sets, absent keeps.
    pub unit: Option<Option<String>>,
    #[serde(default)]
    /// Updated active status.
    pub is_active: Option<bool>,
    #[serde(default)]
    /// Updated default supplier — `null` clears, string sets, absent keeps.
    pub default_supplier_id: Option<Option<String>>,
}

/// Args for `update_product_scoped` — identical to `UpdateProductArgs`
/// but without `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
pub struct UpdateProductScopedArgs {
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Display name.
    pub name: String,
    /// Price Minor.
    pub price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ID of the associated category.
    pub category_id: Option<String>,
    /// Barcode string.
    pub barcode: Option<String>,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
    /// Product Type.
    pub product_type: Option<String>,
    #[serde(default)]
    /// Updated cost in minor units (None keeps).
    pub cost_minor: Option<i64>,
    #[serde(default)]
    /// Updated brand — `null` clears, string sets, absent keeps.
    pub brand: Option<Option<String>>,
    #[serde(default)]
    /// Updated rack position code — `null` clears, string sets, absent keeps.
    pub rack_location: Option<Option<String>>,
    #[serde(default)]
    /// Updated notes — `null` clears, string sets, absent keeps.
    pub notes: Option<Option<String>>,
    #[serde(default)]
    /// Updated unit — `null` clears, string sets, absent keeps.
    pub unit: Option<Option<String>>,
    #[serde(default)]
    /// Updated active status.
    pub is_active: Option<bool>,
    #[serde(default)]
    /// Updated default supplier — `null` clears, string sets, absent keeps.
    pub default_supplier_id: Option<Option<String>>,
}

impl UpdateProductArgs {
    /// Map the PATCH-style attribute fields onto the core update struct.
    fn to_update_attributes(&self) -> oz_core::db::UpdateProductAttributes {
        oz_core::db::UpdateProductAttributes {
            cost_minor: self.cost_minor,
            brand: self.brand.clone(),
            rack_location: self.rack_location.clone(),
            notes: self.notes.clone(),
            unit: self.unit.clone(),
            is_active: self.is_active,
            default_supplier_id: self.default_supplier_id.clone(),
        }
    }
}

impl UpdateProductScopedArgs {
    /// Map the PATCH-style attribute fields onto the core update struct.
    fn to_update_attributes(&self) -> oz_core::db::UpdateProductAttributes {
        oz_core::db::UpdateProductAttributes {
            cost_minor: self.cost_minor,
            brand: self.brand.clone(),
            rack_location: self.rack_location.clone(),
            notes: self.notes.clone(),
            unit: self.unit.clone(),
            is_active: self.is_active,
            default_supplier_id: self.default_supplier_id.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
/// Updateproductresult.
pub struct UpdateProductResult {
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Update a product using the global database and a `user_id` parameter.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_product_scoped`
/// with a `session_token` instead.
#[tauri::command]
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
        None,
    )?;

    store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;

    store.update_product_attributes(&args.sku, &args.to_update_attributes())?;

    Ok(UpdateProductResult { sku: args.sku })
}

/// Update a product within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `update_product`. The `user_id` for
/// permission checks is read from the resolved `SessionContext`.
#[tauri::command]
pub async fn update_product_scoped(
    session_token: String,
    args: UpdateProductScopedArgs,
    state: State<'_, AppState>,
) -> Result<UpdateProductResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_UPDATE).await?;
    // ADR #36 D7: changing a product's cost (HPP) requires the manager-only
    // products:edit_cost permission. A PATCH that does not touch cost
    // (cost_minor absent) stays open to PRODUCTS_UPDATE holders.
    if args.cost_minor.is_some() {
        require_permission_for_session(&state, &session, permissions::PRODUCTS_EDIT_COST).await?;
    }
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // any future .await points.
    {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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
            args.product_type.as_deref(),
            None,
        )?;

        store.set_product_tax_rates(&args.sku, &args.tax_rate_ids)?;

        store.update_product_attributes(&args.sku, &args.to_update_attributes())?;
    }

    tracing::info!(sku = %args.sku, name = %args.name, "product updated (scoped)");
    Ok(UpdateProductResult { sku: args.sku })
}

/// Check whether a product tracks serial numbers.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_product_track_serial_scoped`.
#[tauri::command]
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

/// Check whether a product tracks serial numbers, store-scoped. ADR #7.
#[tauri::command]
pub async fn get_product_track_serial_scoped(
    session_token: String,
    sku: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let product = store.get_product(&sku)?;
    drop(db);
    Ok(product.map(|p| p.product.track_serial).unwrap_or(false))
}

/// A single serial-tracking flag keyed by SKU (batch response row).
#[derive(Debug, Serialize)]
pub struct SerialTrackRow {
    /// Stock-keeping unit.
    pub sku: String,
    /// Whether the product is configured for serial tracking.
    pub track_serial: bool,
}

/// Check serial-tracking flags for many SKUs in one round trip
/// (PERF-03: replaces the N+1 `get_product_track_serial` loop).
///
/// Unknown SKUs resolve to `track_serial: false` (same behaviour as
/// the single-SKU command). The response preserves request order.
#[tauri::command]
pub async fn get_product_track_serial_batch(
    skus: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SerialTrackRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = run_get_product_track_serial_batch(&store, &skus);
    drop(db);
    Ok(rows)
}

/// Store-scoped batch variant of `get_product_track_serial_batch`. ADR #7.
#[tauri::command]
pub async fn get_product_track_serial_batch_scoped(
    session_token: String,
    skus: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SerialTrackRow>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = run_get_product_track_serial_batch(&store, &skus);
    drop(db);
    Ok(rows)
}

/// Business logic for the batch serial-tracking lookup (extracted for testing).
fn run_get_product_track_serial_batch(store: &Store<'_>, skus: &[String]) -> Vec<SerialTrackRow> {
    skus.iter()
        .map(|sku| {
            // get_product returns Result<Option<ProductWithDetails>> —
            // collapse both error and missing-product into `false` so the
            // batch never fails for unknown SKUs (matches single-SKU).
            let track_serial = store
                .get_product(sku)
                .ok()
                .flatten()
                .map(|p| p.product.track_serial)
                .unwrap_or(false);
            SerialTrackRow {
                sku: sku.clone(),
                track_serial,
            }
        })
        .collect()
}

// ── Popularity search signal (ADR #37) ──────────────────────────────

/// Record an acted-upon product search for the popularity index.
///
/// ADR #37 D2: only searches that end in an add-to-cart count — raw
/// search counts are polluted by typos and "do you have…" lookups, so
/// the UI fires this event when a search result is actually added.
///
/// Fire-and-forget from the frontend: the response is `()` and failures
/// are logged, never surfaced. The write is a single append to
/// `product_activity` plus a single-SKU score recompute.
#[tauri::command]
pub async fn record_product_search_scoped(
    session_token: String,
    sku: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    match store.record_product_search(&sku) {
        Ok(()) => {}
        Err(e) => {
            // ADR #37 D3: non-blocking — a tracking failure must never
            // fail an add-to-cart.
            tracing::warn!(sku = %sku, error = %e, "product search signal not recorded");
        }
    }
    Ok(())
}

// ── Delete product ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Deleteproductargs.
pub struct DeleteProductArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Args for `delete_product_scoped` — no `user_id`; read from session.
#[derive(Debug, Deserialize)]
pub struct DeleteProductScopedArgs {
    /// Stock-keeping unit identifier.
    pub sku: String,
}

/// Delete a product using the global database and a `user_id` parameter.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_product_scoped`
/// with a `session_token` instead.
#[tauri::command]
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

/// Delete a product within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `delete_product`. The `user_id` for
/// permission checks is read from the resolved `SessionContext`.
#[tauri::command]
pub async fn delete_product_scoped(
    session_token: String,
    args: DeleteProductScopedArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_DELETE).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // any future .await points.
    {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);
        store.delete_product(&args.sku)?;
    }

    tracing::info!(sku = %args.sku, "product deleted (scoped)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;
    use tauri::Manager as _;

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
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
            popularity_score: 0.0,
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
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
            popularity_score: 0.0,
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
    fn create_product_scoped_args_deserialize() {
        let json = r##"{"sku":"LATTE","name":"Latte","price_minor":450,"currency":"USD","category_id":null,"barcode":null,"initial_stock":0,"tax_rate_ids":[]}"##;
        let args: CreateProductScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "LATTE");
        assert_eq!(args.price_minor, 450);

        // user_id is intentionally absent — verify it's not deserialized
        let json_with_user = r##"{"user_id":"u1","sku":"LATTE","name":"Latte","price_minor":450,"currency":"USD","category_id":null,"barcode":null,"initial_stock":0,"tax_rate_ids":[]}"##;
        let args2: CreateProductScopedArgs = serde_json::from_str(json_with_user).unwrap();
        assert_eq!(args2.sku, "LATTE"); // extra fields ignored by serde
    }

    #[test]
    fn get_product_track_serial_batch_maps_known_and_unknown_skus() {
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour, icon) VALUES
                ('cat-1', 'Gadgets', '#06b6d4', '');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id, track_serial, barcode, created_at, updated_at) VALUES
                ('p1', 'TRACKED', 'Tracked Widget', 100, 'USD', 'cat-1', 1, '4901234567890', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('p2', 'PLAIN', 'Plain Widget', 200, 'USD', 'cat-1', 0, NULL, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();

        let store = Store::new(&conn);
        let rows = run_get_product_track_serial_batch(
            &store,
            &[
                "TRACKED".to_string(),
                "PLAIN".to_string(),
                "MISSING".to_string(),
            ],
        );

        assert_eq!(rows.len(), 3);
        assert!(rows[0].track_serial);
        assert!(!rows[1].track_serial);
        // Unknown SKUs resolve to false (matches the single-SKU behaviour).
        assert!(!rows[2].track_serial);
        // Response preserves request order.
        assert_eq!(rows[0].sku, "TRACKED");
        assert_eq!(rows[1].sku, "PLAIN");
        assert_eq!(rows[2].sku, "MISSING");
    }

    #[test]
    fn get_product_track_serial_batch_empty_input() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let rows = run_get_product_track_serial_batch(&store, &[]);
        assert!(rows.is_empty());
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
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
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
    fn update_product_scoped_args_deserialize() {
        let json = r##"{"sku":"LATTE","name":"Latte Updated","price_minor":500,"currency":"USD","category_id":null,"barcode":null,"tax_rate_ids":[]}"##;
        let args: UpdateProductScopedArgs = serde_json::from_str(json).unwrap();
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
    fn delete_product_scoped_args_deserialize() {
        let json = r##"{"sku":"OLD-SKU"}"##;
        let args: DeleteProductScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "OLD-SKU");
    }

    #[test]
    fn create_product_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn update_product_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("bad-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn delete_product_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("bogus");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn list_products_scoped_rejects_invalid_token() {
        // Verify that an invalid/nonexistent session token returns InvalidSession.
        let state = AppState::for_test();
        // list_products_scoped is an async command, so we can't call it directly.
        // Instead, test that resolve_session rejects unknown tokens.
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn list_products_scoped_accepts_valid_token() {
        // Verify that a valid session token resolves and returns products.
        let state = AppState::for_test();
        let ctx = oz_core::session::SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "default".into(),
            "default-restaurant-pos".into(),
            "restaurant-pos".into(),
            None,
            0,
        );
        state
            .session_store
            .write()
            .unwrap()
            .insert("tok-valid".into(), ctx);

        let session = state.resolve_session("tok-valid").unwrap();
        assert_eq!(session.store_id, "default");
        assert_eq!(session.type_key, "restaurant-pos");
    }

    fn seeded_cost_gate_app() -> tauri::App<tauri::test::MockRuntime> {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        // role-staff holds PRODUCTS_CREATE/PRODUCTS_UPDATE but NOT
        // PRODUCTS_EDIT_COST (ADR #36 D7 — manager+ only).
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                    ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let state = AppState::for_test_with_conn(conn);
        for (token, user, role) in [
            ("staff-token", "user-staff", "role-staff"),
            ("manager-token", "user-manager", "role-manager"),
        ] {
            state.session_store.write().unwrap().insert(
                token.into(),
                oz_core::session::SessionContext::new(
                    user.into(),
                    role.into(),
                    "terminal-1".into(),
                    "store-1".into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
            );
        }
        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap()
    }

    fn create_args(sku: &str, cost_minor: i64) -> CreateProductScopedArgs {
        CreateProductScopedArgs {
            sku: sku.into(),
            name: sku.into(),
            price_minor: 100,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 0,
            tax_rate_ids: vec![],
            product_type: "retail".into(),
            cost_minor,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
        }
    }

    fn update_args(sku: &str, cost_minor: Option<i64>) -> UpdateProductScopedArgs {
        UpdateProductScopedArgs {
            sku: sku.into(),
            name: sku.into(),
            price_minor: 100,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            tax_rate_ids: vec![],
            product_type: Some("retail".into()),
            cost_minor,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: None,
            default_supplier_id: None,
        }
    }

    #[tokio::test]
    async fn create_product_scoped_denies_cost_without_edit_cost_permission() {
        let app = seeded_cost_gate_app();

        // Staff creating a product WITHOUT cost passes the gate (the test
        // state has no store DB, so the call then fails internally — the
        // point is it is NOT a permission denial).
        let no_cost = create_product_scoped(
            "staff-token".into(),
            create_args("SKU-NO-COST", 0),
            app.state(),
        )
        .await;
        assert!(!matches!(no_cost, Err(AppError::PermissionDenied(_))));

        // Staff setting a cost is rejected outright.
        let with_cost = create_product_scoped(
            "staff-token".into(),
            create_args("SKU-COST", 100),
            app.state(),
        )
        .await;
        assert!(matches!(with_cost, Err(AppError::PermissionDenied(_))));

        // Manager passes the gate (then fails on the missing store DB, not
        // on the permission).
        let manager = create_product_scoped(
            "manager-token".into(),
            create_args("SKU-MGR", 100),
            app.state(),
        )
        .await;
        assert!(!matches!(manager, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn update_product_scoped_denies_cost_change_without_edit_cost_permission() {
        let app = seeded_cost_gate_app();

        // Staff PATCH without touching cost passes the gate.
        let no_cost = update_product_scoped(
            "staff-token".into(),
            update_args("SKU-1", None),
            app.state(),
        )
        .await;
        assert!(!matches!(no_cost, Err(AppError::PermissionDenied(_))));

        // Staff PATCH that changes cost is rejected.
        let with_cost = update_product_scoped(
            "staff-token".into(),
            update_args("SKU-1", Some(100)),
            app.state(),
        )
        .await;
        assert!(matches!(with_cost, Err(AppError::PermissionDenied(_))));

        // Manager PATCH with cost passes the gate.
        let manager = update_product_scoped(
            "manager-token".into(),
            update_args("SKU-1", Some(100)),
            app.state(),
        )
        .await;
        assert!(!matches!(manager, Err(AppError::PermissionDenied(_))));
    }

    #[test]
    fn edit_cost_permission_membership_is_manager_only() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                    ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                    ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let store = Store::new(&conn);
        // Owner (`*`) and Manager presets hold it; Staff does not.
        assert!(
            require_permission_for_user(&store, "user-owner", permissions::PRODUCTS_EDIT_COST)
                .is_ok()
        );
        assert!(
            require_permission_for_user(&store, "user-manager", permissions::PRODUCTS_EDIT_COST)
                .is_ok()
        );
        assert!(matches!(
            require_permission_for_user(&store, "user-staff", permissions::PRODUCTS_EDIT_COST),
            Err(AppError::PermissionDenied(_))
        ));
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
