//! Postgres data layer for the oz-api REST handlers (Phase 1.2 of
//! `unify-auth-and-sync.md`).
//!
//! The desktop/tablet/cloud POS share one SQLite data layer
//! ([`oz_core::Store`]), which cannot be rewritten to Postgres. The cloud
//! server therefore gets a **parallel async data layer** for the REST
//! surface, exactly like `apps/cloud-server/src/sync_store.rs` is for the
//! sync function. Each handler dispatches on `AppState::pg`:
//!
//! - `Some(pool)` → the cloud server's Postgres branch: this module runs the
//!   query against `deadpool_postgres::Pool`.
//! - `None` → local dev / tests / SQLite branch: the existing
//!   `oz_core::Store` path runs unchanged.
//!
//! The SQL is written natively for Postgres (`$n` parameters). The port
//! schema (`20260813_init.pg.sql`) stores boolean-ish columns as `BIGINT`
//! (0/1), so reads go through [`pg_bool`] and writes pass `i64`. The
//! behaviour mirrors the SQLite `Store` methods the handlers used before:
//! validation errors, unique-constraint conflicts, the `stock_movements` +
//! `stock_summary` + `inventory` ledger writes on product/stock changes, and
//! the sale header + line insert inside one transaction.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use deadpool_postgres::Pool;
use tokio_postgres::error::SqlState;

use oz_core::tax_rate::TaxRate;
use oz_core::{
    Category, Currency, Money, Product, ProductWithDetails, Sale, SaleLine, SaleStatus, Sku,
    TenantPlan, User,
};

use crate::routes::terminals::RegisteredTerminal;

/// Default inventory location UUID (must match the port schema's default).
const CANONICAL_DEFAULT_LOCATION_UUID: &str = "01926b3a-0000-7000-8000-000000000001";

/// Error from the Postgres REST data layer, mapped to HTTP statuses the same
/// way the SQLite `Store` errors were.
#[derive(Debug)]
pub enum PgError {
    /// Unique-constraint violation → 409.
    Conflict,
    /// Missing row → 404.
    NotFound,
    /// Input validation failed → 400.
    Validation(String),
    /// Backend / connection failure → 500.
    Db(String),
}

impl std::fmt::Display for PgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgError::Conflict => write!(f, "resource already exists"),
            PgError::NotFound => write!(f, "not found"),
            PgError::Validation(m) => write!(f, "{m}"),
            PgError::Db(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PgError {}

impl PgError {
    /// Convert into an axum [`Response`] with the matching status code.
    pub fn into_response(self) -> Response {
        match self {
            PgError::Conflict => (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "resource already exists"})),
            )
                .into_response(),
            PgError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "not found"})),
            )
                .into_response(),
            PgError::Validation(message) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": message})),
            )
                .into_response(),
            PgError::Db(e) => {
                tracing::error!("postgres REST data layer error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "internal error"})),
                )
                    .into_response()
            }
        }
    }
}

/// Read a `BIGINT` boolean-ish column as `bool` (0 → false, else true).
fn pg_bool(row: &tokio_postgres::Row, column: &str) -> Result<bool, PgError> {
    let v: i64 = row
        .try_get(column)
        .map_err(|e| PgError::Db(e.to_string()))?;
    Ok(v != 0)
}

/// Check whether a Postgres error is a unique-constraint violation.
fn is_unique_violation(e: &tokio_postgres::Error) -> bool {
    e.as_db_error()
        .map(|d| d.code() == &SqlState::UNIQUE_VIOLATION)
        .unwrap_or(false)
}

/// Check whether a Postgres error is a foreign-key violation.
fn is_fk_violation(e: &tokio_postgres::Error) -> bool {
    e.as_db_error()
        .map(|d| d.code() == &SqlState::FOREIGN_KEY_VIOLATION)
        .unwrap_or(false)
}

/// Current UTC timestamp in the same format the SQLite path uses.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn currency_str(currency: &Currency) -> Result<String, PgError> {
    std::str::from_utf8(&currency.0)
        .map(str::to_owned)
        .map_err(|e| PgError::Validation(format!("invalid UTF-8 in currency bytes: {e}")))
}

// ── Tenant plans ──────────────────────────────────────────────────────

/// Read a tenant's sync plan, or `None` when the tenant has no row.
pub async fn get_tenant_plan(pool: &Pool, tenant_id: &str) -> Result<Option<TenantPlan>, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let row = client
        .query_opt(
            "SELECT plan FROM tenant_plans WHERE tenant_id = $1",
            &[&tenant_id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    row.map(|r| {
        let plan: String = r.try_get(0).map_err(|e| PgError::Db(e.to_string()))?;
        Ok(TenantPlan::from_db(&plan))
    })
    .transpose()
}

/// Assign or change a tenant's plan (upsert).
pub async fn set_tenant_plan(
    pool: &Pool,
    tenant_id: &str,
    plan: TenantPlan,
) -> Result<(), PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    client
        .execute(
            "INSERT INTO tenant_plans (tenant_id, plan, updated_at) VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id) DO UPDATE SET plan = excluded.plan, updated_at = excluded.updated_at",
            &[&tenant_id, &plan.as_db_str(), &now_rfc3339()],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    Ok(())
}

// ── Categories ────────────────────────────────────────────────────────

/// List all categories, ordered by name.
pub async fn list_categories(pool: &Pool) -> Result<Vec<Category>, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let rows = client
        .query(
            "SELECT id, name, colour, icon FROM categories ORDER BY name",
            &[],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    rows.iter()
        .map(|r| {
            Ok(Category {
                id: r.try_get("id").map_err(|e| PgError::Db(e.to_string()))?,
                name: r.try_get("name").map_err(|e| PgError::Db(e.to_string()))?,
                colour: r
                    .try_get("colour")
                    .map_err(|e| PgError::Db(e.to_string()))?,
                icon: r.try_get("icon").map_err(|e| PgError::Db(e.to_string()))?,
            })
        })
        .collect()
}

// ── Tax rates ─────────────────────────────────────────────────────────

/// Create a tax rate, scoped to `tenant_id`, mirroring `Store::create_tax_rate`
/// including the TAX-02 default-flag swap inside one transaction.
pub async fn create_tax_rate(
    pool: &Pool,
    tenant_id: &str,
    name: &str,
    rate_bps: i64,
    is_default: bool,
    is_inclusive: bool,
) -> Result<TaxRate, PgError> {
    if name.trim().is_empty() {
        return Err(PgError::Validation(
            "tax rate name must not be empty".into(),
        ));
    }
    if rate_bps < 0 {
        return Err(PgError::Validation("rate_bps must be non-negative".into()));
    }

    let mut client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    if is_default {
        tx.execute(
            "UPDATE tax_rates SET is_default = 0 WHERE is_default = 1",
            &[],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    }

    let id = uuid::Uuid::now_v7().to_string();
    let now = now_rfc3339();
    tx.execute(
        "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at, tenant_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &id,
            &name.trim(),
            &rate_bps,
            &(is_default as i64),
            &(is_inclusive as i64),
            &now,
            &now,
            &tenant_id,
        ],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;

    tx.commit().await.map_err(|e| PgError::Db(e.to_string()))?;

    Ok(TaxRate {
        id,
        name: name.trim().to_owned(),
        rate_bps,
        is_default,
        is_inclusive,
        created_at: now.clone(),
        updated_at: now,
    })
}

// ── Users ─────────────────────────────────────────────────────────────

/// Create a user, scoped to `tenant_id`, mirroring `Store::create_user`
/// (validation, the role FK check, the default `assignments` row).
pub async fn create_user(
    pool: &Pool,
    tenant_id: &str,
    username: &str,
    pin_hash: &str,
    display_name: &str,
    role_id: &str,
) -> Result<User, PgError> {
    let username = username.trim().to_lowercase();
    if username.is_empty() {
        return Err(PgError::Validation("username must not be empty".into()));
    }
    if username.len() > 100 {
        return Err(PgError::Validation(format!(
            "username must not exceed 100 characters, got {}",
            username.len()
        )));
    }
    if display_name.trim().is_empty() {
        return Err(PgError::Validation("display name must not be empty".into()));
    }
    if display_name.len() > 255 {
        return Err(PgError::Validation(format!(
            "display name must not exceed 255 characters, got {}",
            display_name.len()
        )));
    }

    let mut client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    // The SQLite path surfaces an unknown role as a constraint violation;
    // here we fail closed with a clear validation error instead.
    let role_exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1)",
            &[&role_id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?
        .get(0);
    if !role_exists {
        return Err(PgError::Validation(format!(
            "role_id '{role_id}' does not reference an existing role"
        )));
    }

    let id = uuid::Uuid::now_v7().to_string();
    let now = now_rfc3339();
    if let Err(e) = tx
        .execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at, tenant_id)
             VALUES ($1, $2, $3, $4, $5, 1, $6, $7, $8)",
            &[
                &id,
                &username,
                &pin_hash,
                &display_name.trim(),
                &role_id,
                &now,
                &now,
                &tenant_id,
            ],
        )
        .await
    {
        if is_unique_violation(&e) {
            return Err(PgError::Conflict);
        }
        if is_fk_violation(&e) {
            return Err(PgError::Validation(format!(
                "role_id '{role_id}' does not reference an existing role"
            )));
        }
        return Err(PgError::Db(e.to_string()));
    }

    // Every user gets their single effective assignment (ADR #35 D5).
    tx.execute(
        "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
         VALUES ($1, $2, 'global', 'all', 'all')",
        &[&id, &role_id],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;

    tx.commit().await.map_err(|e| PgError::Db(e.to_string()))?;

    Ok(User {
        id,
        username,
        pin_hash: pin_hash.to_owned(),
        display_name: display_name.trim().to_owned(),
        role_id: role_id.to_owned(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

// ── Terminals ─────────────────────────────────────────────────────────

/// Register (or rotate the secret of) a sync terminal.
pub async fn register_terminal(
    pool: &Pool,
    terminal_id: &str,
    secret_hash: &str,
    label: &str,
    tenant_id: Option<&str>,
) -> Result<(), PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    client
        .execute(
            "INSERT INTO sync_terminals (terminal_id, secret_hash, label, tenant_id, created_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (terminal_id) DO UPDATE SET
                secret_hash = excluded.secret_hash,
                label = excluded.label,
                tenant_id = excluded.tenant_id",
            &[
                &terminal_id,
                &secret_hash,
                &label,
                &tenant_id,
                &now_rfc3339(),
            ],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    Ok(())
}

/// Resolve a terminal from client credentials, or `None` on mismatch.
pub async fn verify_terminal_credentials(
    pool: &Pool,
    client_id: &str,
    client_secret: &str,
) -> Result<Option<RegisteredTerminal>, PgError> {
    let digest = crate::routes::terminals::hash_secret(client_secret);
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let row = client
        .query_opt(
            "SELECT terminal_id, tenant_id FROM sync_terminals WHERE terminal_id = $1 AND secret_hash = $2",
            &[&client_id, &digest],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    row.map(|r| {
        Ok(RegisteredTerminal {
            terminal_id: r.try_get(0).map_err(|e| PgError::Db(e.to_string()))?,
            tenant_id: r.try_get(1).map_err(|e| PgError::Db(e.to_string()))?,
        })
    })
    .transpose()
}

// ── Products ──────────────────────────────────────────────────────────

const PRODUCT_SELECT: &str = "SELECT p.id, p.sku, p.name, p.price_minor, p.currency, \
     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at, \
     p.track_serial, p.product_type, p.version, \
     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit, \
     p.is_active, p.default_supplier_id, p.popularity_score, \
     c.name AS category_name, \
     COALESCE((SELECT SUM(ss.qty)::bigint FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty \
     FROM products p \
     LEFT JOIN categories c ON p.category_id = c.id \
     LEFT JOIN inventory i ON p.id = i.product_id";

/// Build a [`ProductWithDetails`] from a Postgres row (mirrors the SQLite
/// `row_to_product_with_details` mapper, including the `BIGINT` → `bool`
/// conversion for the boolean-ish columns).
fn pg_row_to_product_with_details(
    row: &tokio_postgres::Row,
) -> Result<ProductWithDetails, PgError> {
    let sku_str: String = row.try_get("sku").map_err(|e| PgError::Db(e.to_string()))?;
    let cur_str: String = row
        .try_get("currency")
        .map_err(|e| PgError::Db(e.to_string()))?;
    let barcode_raw: Option<String> = row
        .try_get("barcode")
        .map_err(|e| PgError::Db(e.to_string()))?;
    let product_type_str: String = row
        .try_get("product_type")
        .map_err(|e| PgError::Db(e.to_string()))?;

    let product = Product {
        id: row.try_get("id").map_err(|e| PgError::Db(e.to_string()))?,
        sku: Sku::new(sku_str),
        name: row
            .try_get("name")
            .map_err(|e| PgError::Db(e.to_string()))?,
        price: Money {
            minor_units: row
                .try_get("price_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency: cur_str
                .parse::<Currency>()
                .map_err(|e| PgError::Db(e.to_string()))?,
        },
        category_id: row
            .try_get("category_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        barcode: barcode_raw.and_then(|s| foundation::Barcode::new(&s).ok()),
        created_at: row
            .try_get("created_at")
            .map_err(|e| PgError::Db(e.to_string()))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| PgError::Db(e.to_string()))?,
        price_updated_at: row
            .try_get("price_updated_at")
            .map_err(|e| PgError::Db(e.to_string()))?,
        track_serial: pg_bool(row, "track_serial")?,
        product_type: oz_core::ProductType::parse_str(&product_type_str).unwrap_or_default(),
        version: row
            .try_get("version")
            .map_err(|e| PgError::Db(e.to_string()))?,
        cost_minor: row
            .try_get("cost_minor")
            .map_err(|e| PgError::Db(e.to_string()))?,
        brand: row
            .try_get("brand")
            .map_err(|e| PgError::Db(e.to_string()))?,
        rack_location: row
            .try_get("rack_location")
            .map_err(|e| PgError::Db(e.to_string()))?,
        notes: row
            .try_get("notes")
            .map_err(|e| PgError::Db(e.to_string()))?,
        unit: row
            .try_get("unit")
            .map_err(|e| PgError::Db(e.to_string()))?,
        is_active: pg_bool(row, "is_active")?,
        default_supplier_id: row
            .try_get("default_supplier_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
    };

    Ok(ProductWithDetails {
        product,
        category_name: row
            .try_get("category_name")
            .map_err(|e| PgError::Db(e.to_string()))?,
        stock_qty: row
            .try_get("stock_qty")
            .map_err(|e| PgError::Db(e.to_string()))?,
        popularity_score: row
            .try_get("popularity_score")
            .map_err(|e| PgError::Db(e.to_string()))?,
    })
}

/// List a tenant's products, ordered by name, with category name and stock.
pub async fn list_products(
    pool: &Pool,
    tenant_id: &str,
) -> Result<Vec<ProductWithDetails>, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let rows = client
        .query(
            &format!("{PRODUCT_SELECT} WHERE p.tenant_id = $1 ORDER BY p.name"),
            &[&tenant_id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    rows.iter().map(pg_row_to_product_with_details).collect()
}

/// Get a single product by SKU (tenant-scoped), including category name and
/// stock. SKUs are unique per tenant, so the lookup must be scoped.
pub async fn get_product(
    pool: &Pool,
    tenant_id: &str,
    sku: &str,
) -> Result<Option<ProductWithDetails>, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let row = client
        .query_opt(
            &format!("{PRODUCT_SELECT} WHERE p.tenant_id = $1 AND p.sku = $2"),
            &[&tenant_id, &sku],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    row.map(|r| pg_row_to_product_with_details(&r)).transpose()
}

/// Create a product (scoped to `tenant_id`), mirroring the SQLite path:
/// the product row plus — for `initial_stock > 0` — the `inventory`,
/// `stock_movements` ledger, and `stock_summary` rows in one transaction.
pub async fn create_product(
    pool: &Pool,
    tenant_id: &str,
    sku: &str,
    name: &str,
    price: Money,
    category_id: Option<&str>,
    barcode: Option<&str>,
    initial_stock: i64,
) -> Result<ProductWithDetails, PgError> {
    if sku.trim().is_empty() {
        return Err(PgError::Validation("SKU must not be empty".into()));
    }
    if sku.len() > 50 {
        return Err(PgError::Validation(format!(
            "SKU must not exceed 50 characters, got {}",
            sku.len()
        )));
    }
    if name.trim().is_empty() {
        return Err(PgError::Validation("name must not be empty".into()));
    }
    if price.minor_units < 0 {
        return Err(PgError::Validation("price must be ≥ 0".into()));
    }
    if initial_stock < 0 {
        return Err(PgError::Validation("initial_stock must be ≥ 0".into()));
    }

    let mut client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    let id = uuid::Uuid::now_v7().to_string();
    let now = now_rfc3339();
    let cur_str = currency_str(&price.currency)?;

    if let Err(e) = tx
        .execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, \
             created_at, updated_at, price_updated_at, track_serial, product_type, version, \
             cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, tenant_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $8, 0, 'retail', 1, 0, NULL, NULL, NULL, NULL, 1, NULL, $9)",
            &[
                &id,
                &sku.trim(),
                &name.trim(),
                &price.minor_units,
                &cur_str,
                &category_id,
                &barcode,
                &now,
                &tenant_id,
            ],
        )
        .await
    {
        if is_unique_violation(&e) {
            return Err(PgError::Conflict);
        }
        return Err(PgError::Db(e.to_string()));
    }

    if initial_stock > 0 {
        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES ($1, $2, $3)",
            &[&id, &initial_stock, &now],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
        let movement_id = uuid::Uuid::now_v7().to_string();
        tx.execute(
            "INSERT INTO stock_movements (id, item_id, delta, reason, source_terminal_id, source_user_id, created_at)
             VALUES ($1, $2, $3, 'initial-stock', NULL, NULL, $4)",
            &[&movement_id, &id, &initial_stock, &now],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
        tx.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES ($1, $2, $3, $4)
             ON CONFLICT (item_id, location_id) DO UPDATE SET qty = excluded.qty, updated_at = excluded.updated_at",
            &[&id, &CANONICAL_DEFAULT_LOCATION_UUID, &initial_stock, &now],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    }

    tx.commit().await.map_err(|e| PgError::Db(e.to_string()))?;

    Ok(ProductWithDetails {
        product: Product {
            id,
            sku: Sku::new(sku.trim()),
            name: name.trim().to_owned(),
            price,
            category_id: category_id.map(str::to_owned),
            barcode: barcode.and_then(|s| foundation::Barcode::new(s).ok()),
            created_at: now.clone(),
            updated_at: now.clone(),
            price_updated_at: now,
            track_serial: false,
            product_type: oz_core::ProductType::Retail,
            version: 1,
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
        },
        category_name: None,
        stock_qty: if initial_stock > 0 {
            Some(initial_stock)
        } else {
            None
        },
        popularity_score: 0.0,
    })
}

/// Outcome of a stock adjustment.
#[derive(Debug, Clone, Copy)]
pub struct StockAdjustment {
    /// Stock before the adjustment.
    pub previous_qty: i64,
    /// Stock after the adjustment.
    pub new_qty: i64,
}

/// Adjust stock by SKU, mirroring the SQLite `adjust_stock` path: read the
/// previous quantity, reject negative stock, and write the `stock_movements`
/// ledger + `inventory` + `stock_summary` rows in one transaction.
///
/// The whole read-modify-write runs inside the transaction with the product
/// row locked (`SELECT … FOR UPDATE`), so concurrent adjustments to the same
/// SKU serialize instead of losing updates — SQLite's single-writer
/// semantics made the read-outside-tx shape safe there, Postgres does not.
pub async fn adjust_stock(
    pool: &Pool,
    tenant_id: &str,
    sku: &str,
    delta: i64,
) -> Result<StockAdjustment, PgError> {
    let mut client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    // Lock the product row first: every adjustment to this SKU contends on
    // the same row lock, so the inventory read below always sees the latest
    // committed quantity (and the `inventory` row, when missing, is created
    // by the first locker before the second reads it).
    let product_id: Option<String> = tx
        .query_opt(
            "SELECT id FROM products WHERE tenant_id = $1 AND sku = $2 FOR UPDATE",
            &[&tenant_id, &sku],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?
        .map(|r| r.get(0));
    let product_id = match product_id {
        Some(id) => id,
        None => return Err(PgError::NotFound),
    };

    let previous_qty: i64 = tx
        .query_opt(
            "SELECT qty FROM inventory WHERE product_id = $1",
            &[&product_id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?
        .map(|r| r.get::<_, i64>(0))
        .unwrap_or(0);

    let new_qty = previous_qty
        .checked_add(delta)
        .filter(|&v| v >= 0)
        .ok_or_else(|| {
            PgError::Validation(format!(
                "adjustment would cause negative stock (previous: {previous_qty}, delta: {delta})"
            ))
        })?;

    let now = now_rfc3339();
    tx.execute(
        "INSERT INTO stock_movements (id, item_id, delta, reason, source_terminal_id, source_user_id, created_at)
         VALUES ($1, $2, $3, NULL, NULL, NULL, $4)",
        &[&uuid::Uuid::now_v7().to_string(), &product_id, &delta, &now],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;
    tx.execute(
        "INSERT INTO inventory (product_id, qty, updated_at) VALUES ($1, $2, $3)
         ON CONFLICT (product_id) DO UPDATE SET qty = excluded.qty, updated_at = excluded.updated_at",
        &[&product_id, &new_qty, &now],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;
    tx.execute(
        "INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES ($1, $2, $3, $4)
         ON CONFLICT (item_id, location_id) DO UPDATE SET qty = excluded.qty, updated_at = excluded.updated_at",
        &[&product_id, &CANONICAL_DEFAULT_LOCATION_UUID, &new_qty, &now],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;
    tx.commit().await.map_err(|e| PgError::Db(e.to_string()))?;

    Ok(StockAdjustment {
        previous_qty,
        new_qty,
    })
}

// ── Sales ─────────────────────────────────────────────────────────────

/// Persist a sale header + lines in one transaction, mirroring the SQLite
/// `Store::create_sale` (including the same negative-value rejections and
/// the frozen `cost_minor` snapshot on each line). The per-line cost freeze
/// looks the product up within `tenant_id` (SKUs are unique per tenant).
pub async fn create_sale(pool: &Pool, tenant_id: &str, sale: &Sale) -> Result<(), PgError> {
    for line in &sale.lines {
        if line.qty < 0 {
            return Err(PgError::Validation(format!(
                "sale line quantity must be positive, got {}",
                line.qty
            )));
        }
        if line.line_total.minor_units < 0 {
            return Err(PgError::Validation(
                "sale line total must be non-negative".into(),
            ));
        }
        if line.tax_amount.minor_units < 0 {
            return Err(PgError::Validation(
                "sale line tax must be non-negative".into(),
            ));
        }
    }
    if sale.total.minor_units < 0 || sale.subtotal.minor_units < 0 || sale.tax_total.minor_units < 0
    {
        return Err(PgError::Validation(
            "sale totals must be non-negative".into(),
        ));
    }
    if let Some(tendered) = sale.tendered_minor
        && tendered < 0
    {
        return Err(PgError::Validation(
            "tendered amount must be non-negative".into(),
        ));
    }

    let mut client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    let cur_str = currency_str(&sale.currency)?;
    let status_str = sale.status.as_stored_str();
    tx.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                            discount_percent, discount_label, user_id, created_at, updated_at,
                            subtotal_minor, tax_total_minor, customer_id, version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 1)",
        &[
            &sale.id,
            &sale.total.minor_units,
            &cur_str,
            &sale.line_count,
            &status_str,
            &sale.payment_method,
            &sale.tendered_minor,
            &sale.discount_percent,
            &sale.discount_label,
            &sale.user_id,
            &sale.created_at,
            &sale.updated_at,
            &sale.subtotal.minor_units,
            &sale.tax_total.minor_units,
            &sale.customer_id,
        ],
    )
    .await
    .map_err(|e| PgError::Db(e.to_string()))?;

    for line in &sale.lines {
        let line_cur = currency_str(&line.unit_price.currency)?;
        // Freeze the product cost at write time (ADR #36 reporting).
        let cost_minor: Option<i64> = tx
            .query_opt(
                "SELECT cost_minor FROM products WHERE tenant_id = $1 AND sku = $2",
                &[&tenant_id, &line.sku],
            )
            .await
            .map_err(|e| PgError::Db(e.to_string()))?
            .map(|r| r.get::<_, i64>(0))
            .filter(|&v| v > 0);
        tx.execute(
            "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                                     tax_minor, tax_rate_id, tax_breakdown_json,
                                     serial_number, course, modifiers_json, cost_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            &[
                &line.id,
                &line.sale_id,
                &line.sku,
                &line.qty,
                &line.unit_price.minor_units,
                &line.line_total.minor_units,
                &line_cur,
                &line.line_position,
                &line.tax_amount.minor_units,
                &line.tax_rate_id,
                &line.tax_breakdown_json,
                &line.serial_number,
                &line.course,
                &line.modifiers_json,
                &cost_minor,
            ],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    }

    tx.commit().await.map_err(|e| PgError::Db(e.to_string()))?;
    Ok(())
}

/// Build a [`SaleLine`] from a Postgres row.
fn pg_row_to_sale_line(row: &tokio_postgres::Row) -> Result<SaleLine, PgError> {
    let cur_str: String = row
        .try_get("currency")
        .map_err(|e| PgError::Db(e.to_string()))?;
    let currency = cur_str
        .parse::<Currency>()
        .map_err(|e| PgError::Db(e.to_string()))?;
    Ok(SaleLine {
        id: row.try_get("id").map_err(|e| PgError::Db(e.to_string()))?,
        sale_id: row
            .try_get("sale_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        sku: row.try_get("sku").map_err(|e| PgError::Db(e.to_string()))?,
        qty: row.try_get("qty").map_err(|e| PgError::Db(e.to_string()))?,
        unit_price: Money {
            minor_units: row
                .try_get("unit_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        line_total: Money {
            minor_units: row
                .try_get("line_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        line_position: row
            .try_get("line_position")
            .map_err(|e| PgError::Db(e.to_string()))?,
        tax_amount: Money {
            minor_units: row
                .try_get("tax_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        tax_rate_id: row
            .try_get("tax_rate_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        tax_breakdown_json: row
            .try_get("tax_breakdown_json")
            .map_err(|e| PgError::Db(e.to_string()))?,
        serial_number: row
            .try_get("serial_number")
            .map_err(|e| PgError::Db(e.to_string()))?,
        course: row
            .try_get("course")
            .map_err(|e| PgError::Db(e.to_string()))?,
        modifiers_json: row
            .try_get("modifiers_json")
            .map_err(|e| PgError::Db(e.to_string()))?,
    })
}

/// Get a single sale by id, including line items.
pub async fn get_sale(pool: &Pool, id: &str) -> Result<Option<Sale>, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;

    let sale_row = client
        .query_opt(
            "SELECT id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                    discount_percent, discount_label, user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version
             FROM sales WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    let Some(sale_row) = sale_row else {
        return Ok(None);
    };
    let cur_str: String = sale_row
        .try_get("currency")
        .map_err(|e| PgError::Db(e.to_string()))?;
    let currency = cur_str
        .parse::<Currency>()
        .map_err(|e| PgError::Db(e.to_string()))?;
    let status_str: String = sale_row
        .try_get("status")
        .map_err(|e| PgError::Db(e.to_string()))?;

    let mut sale = Sale {
        id: sale_row
            .try_get("id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        status: SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending),
        total: Money {
            minor_units: sale_row
                .try_get("total_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        line_count: sale_row
            .try_get("line_count")
            .map_err(|e| PgError::Db(e.to_string()))?,
        currency,
        payment_method: sale_row
            .try_get("payment_method")
            .map_err(|e| PgError::Db(e.to_string()))?,
        tendered_minor: sale_row
            .try_get("tendered_minor")
            .map_err(|e| PgError::Db(e.to_string()))?,
        discount_percent: sale_row
            .try_get::<_, i64>("discount_percent")
            .map_err(|e| PgError::Db(e.to_string()))?,
        discount_label: sale_row
            .try_get("discount_label")
            .map_err(|e| PgError::Db(e.to_string()))?,
        user_id: sale_row
            .try_get("user_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        created_at: sale_row
            .try_get("created_at")
            .map_err(|e| PgError::Db(e.to_string()))?,
        updated_at: sale_row
            .try_get("updated_at")
            .map_err(|e| PgError::Db(e.to_string()))?,
        lines: Vec::new(),
        subtotal: Money {
            minor_units: sale_row
                .try_get("subtotal_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        tax_total: Money {
            minor_units: sale_row
                .try_get("tax_total_minor")
                .map_err(|e| PgError::Db(e.to_string()))?,
            currency,
        },
        customer_id: sale_row
            .try_get("customer_id")
            .map_err(|e| PgError::Db(e.to_string()))?,
        version: sale_row
            .try_get("version")
            .map_err(|e| PgError::Db(e.to_string()))?,
    };

    let line_rows = client
        .query(
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                    tax_minor, tax_rate_id, tax_breakdown_json, serial_number, course, modifiers_json
             FROM sale_lines WHERE sale_id = $1 ORDER BY line_position",
            &[&id],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;
    for row in &line_rows {
        sale.lines.push(pg_row_to_sale_line(row)?);
    }

    Ok(Some(sale))
}

/// Transition a sale's status, validating the state machine first.
///
/// The UPDATE is a compare-and-swap (`WHERE id = $1 AND status = $2`) so
/// two concurrent transitions cannot both validate against the same stale
/// status and double-apply (the loser re-reads and reports the current
/// state).
pub async fn update_sale_status(pool: &Pool, id: &str, to: SaleStatus) -> Result<Sale, PgError> {
    let client = pool.get().await.map_err(|e| PgError::Db(e.to_string()))?;

    let current_str: Option<String> = client
        .query_opt("SELECT status FROM sales WHERE id = $1", &[&id])
        .await
        .map_err(|e| PgError::Db(e.to_string()))?
        .map(|r| r.get(0));
    let current_str = match current_str {
        Some(s) => s,
        None => return Err(PgError::NotFound),
    };

    let current = SaleStatus::from_stored_str(&current_str)
        .ok_or_else(|| PgError::Validation(format!("invalid stored status: {current_str}")))?;
    if !SaleStatus::can_transition_to(current, to) {
        return Err(PgError::Validation(format!(
            "cannot transition from {current:?} to {to:?}"
        )));
    }

    let now = now_rfc3339();
    let updated = client
        .execute(
            "UPDATE sales SET status = $1, updated_at = $2, version = version + 1 \
             WHERE id = $3 AND status = $4",
            &[&to.as_stored_str(), &now, &id, &current_str],
        )
        .await
        .map_err(|e| PgError::Db(e.to_string()))?;

    if updated == 0 {
        // Lost the race to a concurrent transition — re-read to report the
        // status we actually saw, rather than silently succeeding.
        let now_str: Option<String> = client
            .query_opt("SELECT status FROM sales WHERE id = $1", &[&id])
            .await
            .map_err(|e| PgError::Db(e.to_string()))?
            .map(|r| r.get(0));
        return match now_str {
            None => Err(PgError::NotFound),
            Some(s) => Err(PgError::Validation(format!(
                "cannot transition from {s:?} to {to:?}"
            ))),
        };
    }

    match get_sale(pool, id).await? {
        Some(sale) => Ok(sale),
        None => Err(PgError::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a deadpool pool from a `postgres://` URL (plaintext — the test
    /// DB runs locally in Docker, mirroring `sync_store`'s integration test).
    async fn test_pool(url: &str) -> Option<deadpool_postgres::Pool> {
        use deadpool_postgres::Manager;
        use std::str::FromStr;
        let config = tokio_postgres::Config::from_str(url).expect("valid postgres URL");
        let manager = Manager::new(config, tokio_postgres::NoTls);
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(5)
            .build()
            .expect("pool build");
        match pool.get().await {
            Ok(client) => {
                if let Err(e) = client.batch_execute(oz_core::migrations::PG_INIT).await {
                    eprintln!("PG REST integration: schema apply failed: {e:?}");
                    return None;
                }
                Some(pool)
            }
            Err(e) => {
                eprintln!("PG REST integration: pool get failed: {e}");
                None
            }
        }
    }

    fn unique_id(prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::now_v7())
    }

    /// Integration test against a live Postgres (the same Docker service
    /// `db.rs` uses, port 15432). Skips when unreachable, so the suite stays
    /// green on machines without a running Postgres.
    #[tokio::test]
    async fn pg_integration_rest_roundtrip() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let Some(pool) = test_pool(&url).await else {
            eprintln!("PG REST integration test skipped (Postgres unreachable at {url})");
            return;
        };

        let tenant = unique_id("pg-rest");
        let sku = unique_id("PG-SKU");
        let currency: Currency = "USD".parse().unwrap();

        // ── Products: create with initial stock, list, get, adjust, oversell ──
        let created = create_product(
            &pool,
            &tenant,
            &sku,
            "PG Espresso",
            Money {
                minor_units: 350,
                currency,
            },
            None,
            None,
            10,
        )
        .await
        .expect("create_product");
        assert_eq!(created.stock_qty, Some(10));
        assert_eq!(created.product.sku.as_str(), sku);
        assert!(created.product.is_active);

        let listed = list_products(&pool, &tenant).await.expect("list_products");
        assert!(
            listed.iter().any(|p| p.product.sku.as_str() == sku),
            "created product must appear in the listing"
        );

        let fetched = get_product(&pool, &tenant, &sku)
            .await
            .expect("get_product")
            .expect("product must exist");
        assert_eq!(fetched.stock_qty, Some(10));
        assert_eq!(fetched.product.name, "PG Espresso");

        let adj = adjust_stock(&pool, &tenant, &sku, -4)
            .await
            .expect("adjust_stock");
        assert_eq!((adj.previous_qty, adj.new_qty), (10, 6));
        assert!(matches!(
            adjust_stock(&pool, &tenant, &sku, -100).await,
            Err(PgError::Validation(_))
        ));
        assert!(matches!(
            adjust_stock(&pool, &tenant, &unique_id("PG-SKU"), 1).await,
            Err(PgError::NotFound)
        ));

        // ── Tax rates ──
        let rate = create_tax_rate(&pool, &tenant, "PG VAT", 1000, true, false)
            .await
            .expect("create_tax_rate");
        assert!(rate.is_default);
        assert_eq!(rate.rate_bps, 1000);
        assert!(matches!(
            create_tax_rate(&pool, &tenant, "", 100, false, false).await,
            Err(PgError::Validation(_))
        ));

        // ── Users (role required) ──
        {
            let client = pool.get().await.unwrap();
            let role_id = format!("role-{tenant}");
            client
                .execute(
                    "INSERT INTO roles (id, name, permissions) VALUES ($1, $2, '[]')",
                    &[&role_id, &role_id],
                )
                .await
                .unwrap();
        }
        let username = format!("pgstaff-{}", uuid::Uuid::now_v7());
        let user = create_user(
            &pool,
            &tenant,
            &username,
            "hash",
            "PG Staff",
            &format!("role-{tenant}"),
        )
        .await
        .expect("create_user");
        assert_eq!(user.username, username);
        assert!(user.is_active);
        assert!(matches!(
            create_user(
                &pool,
                &tenant,
                &username,
                "hash",
                "PG Staff 2",
                &format!("role-{tenant}"),
            )
            .await,
            Err(PgError::Conflict)
        ));
        assert!(matches!(
            create_user(&pool, &tenant, "ghost", "h", "Ghost", "role-missing").await,
            Err(PgError::Validation(_))
        ));

        // ── Plans ──
        set_tenant_plan(&pool, &tenant, TenantPlan::Pro)
            .await
            .expect("set_tenant_plan");
        assert_eq!(
            get_tenant_plan(&pool, &tenant)
                .await
                .expect("get_tenant_plan"),
            Some(TenantPlan::Pro)
        );
        assert_eq!(
            get_tenant_plan(&pool, &unique_id("pg-noplan"))
                .await
                .unwrap(),
            None
        );

        // ── Sales: create (with lines), get, transition ──
        let sale = Sale::from_cart(&oz_core::Cart::new(currency)).expect("from_cart");
        // Hand-build a single-line sale so the ledger row is well-formed.
        let line_id = unique_id("pg-line");
        let mut sale = sale;
        sale.line_count = 1;
        sale.total = Money {
            minor_units: 700,
            currency,
        };
        sale.subtotal = Money {
            minor_units: 700,
            currency,
        };
        sale.lines = vec![SaleLine {
            id: line_id,
            sale_id: sale.id.clone(),
            sku: sku.clone(),
            qty: 2,
            unit_price: Money {
                minor_units: 350,
                currency,
            },
            line_total: Money {
                minor_units: 700,
                currency,
            },
            line_position: 1,
            tax_amount: Money::zero(currency),
            tax_rate_id: None,
            tax_breakdown_json: None,
            serial_number: None,
            course: None,
            modifiers_json: None,
        }];
        create_sale(&pool, &tenant, &sale)
            .await
            .expect("create_sale");

        let fetched_sale = get_sale(&pool, &sale.id)
            .await
            .expect("get_sale")
            .expect("sale must exist");
        assert_eq!(fetched_sale.lines.len(), 1);
        assert_eq!(fetched_sale.lines[0].sku, sku);
        assert_eq!(fetched_sale.total.minor_units, 700);
        assert_eq!(fetched_sale.status, SaleStatus::Pending);
        assert_eq!(
            get_sale(&pool, &unique_id("pg-nosale")).await.unwrap(),
            None
        );

        // Pending → Completed is invalid (the state machine requires Active).
        assert!(matches!(
            update_sale_status(&pool, &sale.id, SaleStatus::Completed).await,
            Err(PgError::Validation(_))
        ));
        // Pending → Active → Completed is the legal path.
        let updated = update_sale_status(&pool, &sale.id, SaleStatus::Active)
            .await
            .expect("update_sale_status");
        assert_eq!(updated.status, SaleStatus::Active);
        let completed = update_sale_status(&pool, &sale.id, SaleStatus::Completed)
            .await
            .expect("update_sale_status");
        assert_eq!(completed.status, SaleStatus::Completed);
        assert!(matches!(
            update_sale_status(&pool, &unique_id("pg-nosale"), SaleStatus::Active).await,
            Err(PgError::NotFound)
        ));

        // ── Terminals: register + client-credentials verify ──
        let term_id = unique_id("pg-term");
        register_terminal(
            &pool,
            &term_id,
            &crate::routes::terminals::hash_secret("secret"),
            "front",
            Some(&tenant),
        )
        .await
        .expect("register_terminal");
        let verified = verify_terminal_credentials(&pool, &term_id, "secret")
            .await
            .expect("verify_terminal_credentials");
        assert_eq!(
            verified.as_ref().and_then(|t| t.tenant_id.as_deref()),
            Some(tenant.as_str())
        );
        assert_eq!(
            verify_terminal_credentials(&pool, &term_id, "wrong")
                .await
                .unwrap()
                .is_none(),
            true
        );

        // ── Categories ──
        assert!(
            list_categories(&pool)
                .await
                .expect("list_categories")
                .is_empty()
        );

        // Clean up the rows this test created so a shared dev DB stays tidy
        // (the sync-store integration test does the same).
        {
            let client = pool.get().await.unwrap();
            let role_id = format!("role-{tenant}");
            client
                .execute("DELETE FROM sale_lines WHERE sale_id = $1", &[&sale.id])
                .await
                .unwrap();
            client
                .execute("DELETE FROM sales WHERE id = $1", &[&sale.id])
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM sync_terminals WHERE tenant_id = $1",
                    &[&tenant],
                )
                .await
                .unwrap();
            client
                .execute("DELETE FROM users WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM roles WHERE id = $1", &[&role_id])
                .await
                .unwrap();
            client
                .execute("DELETE FROM tax_rates WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM stock_movements WHERE item_id IN (SELECT id FROM products WHERE tenant_id = $1)",
                    &[&tenant],
                )
                .await
                .unwrap();
            client
                .execute("DELETE FROM products WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM tenant_plans WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
        }
    }

    /// Concurrent `adjust_stock` calls must not lose updates: the whole
    /// read-modify-write is serialized on the product-row lock, so N
    /// adjustments of -1 land as N distinct movements and the final quantity
    /// is exactly `start - N`. (The pre-fix code read `previous_qty` outside
    /// the transaction, so every concurrent call saw the same starting value
    /// and the last writer won.)
    #[tokio::test]
    async fn pg_integration_concurrent_adjust_stock() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let Some(pool) = test_pool(&url).await else {
            eprintln!("PG concurrent adjust test skipped (Postgres unreachable at {url})");
            return;
        };

        let tenant = unique_id("pg-race");
        let sku = unique_id("PG-RACE");
        let currency: Currency = "USD".parse().unwrap();
        const ADJUSTMENTS: i64 = 20;

        create_product(
            &pool,
            &tenant,
            &sku,
            "Race Stock",
            Money {
                minor_units: 100,
                currency,
            },
            None,
            None,
            100,
        )
        .await
        .expect("create_product");

        // Fire all adjustments concurrently against the same SKU.
        let mut set = tokio::task::JoinSet::new();
        for _ in 0..ADJUSTMENTS {
            let pool = pool.clone();
            let sku = sku.clone();
            let tenant = tenant.clone();
            set.spawn(async move { adjust_stock(&pool, &tenant, &sku, -1).await });
        }
        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            results.push(res.expect("task panicked"));
        }
        assert!(
            results.iter().all(Result::is_ok),
            "all adjustments must succeed, got {:?}",
            results.iter().filter(|r| r.is_err()).count()
        );

        let fetched = get_product(&pool, &tenant, &sku)
            .await
            .expect("get_product")
            .expect("product must exist");
        assert_eq!(
            fetched.stock_qty,
            Some(100 - ADJUSTMENTS),
            "no adjustment may be lost under concurrency"
        );

        // Every adjustment wrote a ledger row (the ledger insert is inside
        // the same serialized transaction).
        let client = pool.get().await.unwrap();
        let product_id: String = client
            .query_one("SELECT id FROM products WHERE sku = $1", &[&sku])
            .await
            .unwrap()
            .get(0);
        let movements: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM stock_movements WHERE item_id = $1",
                &[&product_id],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(
            movements,
            ADJUSTMENTS + 1,
            "initial-stock + each adjustment"
        );

        // Cleanup so the shared dev DB stays tidy.
        let client = pool.get().await.unwrap();
        client
            .execute(
                "DELETE FROM stock_movements WHERE item_id IN (SELECT id FROM products WHERE tenant_id = $1)",
                &[&tenant],
            )
            .await
            .unwrap();
        client
            .execute("DELETE FROM products WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
    }

    /// Two concurrent transitions of the same sale must not both validate
    /// against the same stale status: exactly one wins, the loser re-reads
    /// and reports the current state, and `version` bumps exactly once.
    #[tokio::test]
    async fn pg_integration_concurrent_sale_status_transition() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let Some(pool) = test_pool(&url).await else {
            eprintln!("PG concurrent status test skipped (Postgres unreachable at {url})");
            return;
        };

        let currency: Currency = "USD".parse().unwrap();
        let mut sale = Sale::from_cart(&oz_core::Cart::new(currency)).expect("from_cart");
        sale.line_count = 0;
        sale.total = Money {
            minor_units: 0,
            currency,
        };
        sale.subtotal = Money {
            minor_units: 0,
            currency,
        };
        sale.lines = Vec::new();
        create_sale(&pool, "default", &sale)
            .await
            .expect("create_sale");

        let mut set = tokio::task::JoinSet::new();
        for _ in 0..2 {
            let pool = pool.clone();
            let id = sale.id.clone();
            set.spawn(async move { update_sale_status(&pool, &id, SaleStatus::Active).await });
        }
        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            results.push(res.expect("task panicked"));
        }

        let successes = results.iter().filter(|r| r.is_ok()).count();
        let failures = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(
            successes, 1,
            "exactly one concurrent transition may win (got {successes} ok / {failures} err)"
        );
        assert_eq!(failures, 1);
        assert!(matches!(
            results.iter().find(|r| r.is_err()),
            Some(Err(PgError::Validation(_)))
        ));

        let final_sale = get_sale(&pool, &sale.id)
            .await
            .expect("get_sale")
            .expect("sale must exist");
        assert_eq!(final_sale.status, SaleStatus::Active);
        assert_eq!(final_sale.version, 2, "version must bump exactly once");

        // Cleanup.
        let client = pool.get().await.unwrap();
        client
            .execute("DELETE FROM sales WHERE id = $1", &[&sale.id])
            .await
            .unwrap();
    }

    /// Two tenants can hold the same product SKU and the same username; each
    /// tenant only ever sees and mutates its own rows. This is the contract
    /// the per-tenant `UNIQUE (tenant_id, sku)` / `UNIQUE (tenant_id,
    /// username)` constraints (and the tenant-scoped REST lookups) guarantee.
    #[tokio::test]
    async fn pg_integration_tenant_sku_isolation() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let Some(pool) = test_pool(&url).await else {
            eprintln!("PG tenant-isolation test skipped (Postgres unreachable at {url})");
            return;
        };

        let tenant_a = unique_id("pg-iso-a");
        let tenant_b = unique_id("pg-iso-b");
        let currency: Currency = "USD".parse().unwrap();
        let shared_sku = "SHARED-SKU";

        // Both tenants create the SAME sku — previously a global-UNIQUE
        // conflict, now legal per tenant.
        let a = create_product(
            &pool,
            &tenant_a,
            shared_sku,
            "Tenant A Product",
            Money {
                minor_units: 100,
                currency,
            },
            None,
            None,
            10,
        )
        .await
        .expect("create_product tenant A");
        let b = create_product(
            &pool,
            &tenant_b,
            shared_sku,
            "Tenant B Product",
            Money {
                minor_units: 200,
                currency,
            },
            None,
            None,
            20,
        )
        .await
        .expect("create_product tenant B");
        assert_eq!(a.product.name, "Tenant A Product");
        assert_eq!(b.product.name, "Tenant B Product");

        // Each tenant's by-SKU lookup returns only its own row.
        let a_view = get_product(&pool, &tenant_a, shared_sku)
            .await
            .expect("get_product A")
            .expect("A must see its product");
        let b_view = get_product(&pool, &tenant_b, shared_sku)
            .await
            .expect("get_product B")
            .expect("B must see its product");
        assert_eq!(a_view.product.name, "Tenant A Product");
        assert_eq!(a_view.stock_qty, Some(10));
        assert_eq!(b_view.product.name, "Tenant B Product");
        assert_eq!(b_view.stock_qty, Some(20));

        // Listings are tenant-scoped too.
        let a_list = list_products(&pool, &tenant_a).await.expect("list A");
        let b_list = list_products(&pool, &tenant_b).await.expect("list B");
        assert_eq!(a_list.len(), 1);
        assert_eq!(b_list.len(), 1);
        assert_eq!(a_list[0].product.name, "Tenant A Product");
        assert_eq!(b_list[0].product.name, "Tenant B Product");

        // Stock adjustments are tenant-scoped: adjusting A's stock must not
        // change B's quantity for the same SKU.
        adjust_stock(&pool, &tenant_a, shared_sku, -2)
            .await
            .expect("adjust A");
        let a_after = get_product(&pool, &tenant_a, shared_sku)
            .await
            .unwrap()
            .unwrap();
        let b_after = get_product(&pool, &tenant_b, shared_sku)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(a_after.stock_qty, Some(8));
        assert_eq!(b_after.stock_qty, Some(20), "B's stock must be untouched");

        // A duplicate sku within ONE tenant is still a conflict.
        assert!(matches!(
            create_product(
                &pool,
                &tenant_a,
                shared_sku,
                "Duplicate",
                Money {
                    minor_units: 1,
                    currency,
                },
                None,
                None,
                0,
            )
            .await,
            Err(PgError::Conflict)
        ));

        // Same username in both tenants is legal; duplicate in one is not.
        {
            let client = pool.get().await.unwrap();
            let role_id = unique_id("pg-iso-role");
            client
                .execute(
                    "INSERT INTO roles (id, name, permissions) VALUES ($1, $2, '[]')",
                    &[&role_id, &role_id],
                )
                .await
                .unwrap();
            let username = format!("shared-user-{}", uuid::Uuid::now_v7());
            create_user(&pool, &tenant_a, &username, "h", "A User", &role_id)
                .await
                .expect("create_user A");
            create_user(&pool, &tenant_b, &username, "h", "B User", &role_id)
                .await
                .expect("create_user B");
            assert!(matches!(
                create_user(&pool, &tenant_a, &username, "h", "A Dup", &role_id).await,
                Err(PgError::Conflict)
            ));
            client
                .execute(
                    "DELETE FROM users WHERE tenant_id IN ($1, $2)",
                    &[&tenant_a, &tenant_b],
                )
                .await
                .unwrap();
            client
                .execute("DELETE FROM roles WHERE id = $1", &[&role_id])
                .await
                .unwrap();
        }

        // Cleanup.
        let client = pool.get().await.unwrap();
        client
            .execute(
                "DELETE FROM stock_movements WHERE item_id IN (SELECT id FROM products WHERE tenant_id IN ($1, $2))",
                &[&tenant_a, &tenant_b],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM products WHERE tenant_id IN ($1, $2)",
                &[&tenant_a, &tenant_b],
            )
            .await
            .unwrap();
    }
}
