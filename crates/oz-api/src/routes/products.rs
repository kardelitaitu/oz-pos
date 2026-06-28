//! Product endpoints.
//!
//! `GET /api/v1/products` — list all products (with optional category filter).
//! `GET /api/v1/products/:sku` — product detail including stock quantity.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use oz_core::Product;

use crate::AppState;

/// API response for a product with category name and stock from JOINs.
///
/// Flattens the [`Product`] domain struct and adds the extra fields
/// that come from LEFT JOINs on `categories` and `inventory`.
#[derive(Serialize)]
pub struct ProductDetail {
    #[serde(flatten)]
    pub product: Product,
    /// Category display name, from `categories.name`.
    pub category_name: Option<String>,
    /// Current stock quantity, from `inventory.qty`.
    pub stock_qty: Option<i64>,
}

/// Request body for creating a new product.
///
/// The caller supplies `sku`, `name`, and `price` (all required).
/// `category_id`, `barcode`, and `initial_stock` are optional.
/// The server generates `id` and timestamps.
#[derive(Deserialize)]
pub struct CreateProductRequest {
    pub sku: String,
    pub name: String,
    pub price: oz_core::Money,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    /// Initial stock quantity (≥ 0). If omitted or zero, no inventory row is inserted.
    pub initial_stock: Option<i64>,
}

/// Create a new product.
///
/// Validates the SKU and name, generates an id, and inserts into
/// `products`. If `initial_stock` > 0, a matching `inventory` row is
/// inserted as well. Returns 201 with the created `ProductDetail`.
pub async fn create_product(
    State(state): State<AppState>,
    Json(body): Json<CreateProductRequest>,
) -> impl IntoResponse {
    // Validate.
    if body.sku.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "SKU must not be empty"}))).into_response();
    }
    if body.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "name must not be empty"}))).into_response();
    }
    if body.price.minor_units < 0 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "price must be ≥ 0"}))).into_response();
    }
    let initial_stock = body.initial_stock.unwrap_or(0);
    if initial_stock < 0 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "initial_stock must be ≥ 0"}))).into_response();
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let sku_trimmed = body.sku.trim().to_owned();
    let cur_str = std::str::from_utf8(&body.price.currency.0)
        .expect("currency bytes are valid UTF-8")
        .to_owned();

    let db = state.db.lock().await;

    // Insert product.
    let result = db.execute(
        "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            id,
            sku_trimmed,
            body.name.trim(),
            body.price.minor_units,
            cur_str,
            body.category_id,
            body.barcode,
            now,
            now,
        ],
    );

    match result {
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            return (StatusCode::CONFLICT, Json(serde_json::json!({"error": "SKU or barcode already exists"}))).into_response();
        }
        Err(e) => panic!("insert product failed: {e}"),
        Ok(_) => {}
    }

    // Insert inventory row if initial stock > 0.
    if initial_stock > 0 {
        db.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, initial_stock, now],
        )
        .expect("insert inventory row");
    }

    let product = Product {
        id,
        sku: oz_core::Sku::new(sku_trimmed),
        name: body.name.trim().to_owned(),
        price: body.price,
        category_id: body.category_id,
        barcode: body.barcode,
        created_at: now.clone(),
        updated_at: now,
    };

    let detail = ProductDetail {
        product,
        category_name: None,
        stock_qty: if initial_stock > 0 { Some(initial_stock) } else { None },
    };

    (StatusCode::CREATED, Json(detail)).into_response()
}

/// List all products, ordered by name, with category name and current
/// stock from the `inventory` table.
pub async fn list_products(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, p.barcode, p.created_at, p.updated_at,
                    c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             ORDER BY p.name",
        )
        .expect("prepare list_products query");

    let rows = stmt
        .query_map([], |row| {
            let sku_str: String = row.get("sku")?;
            let cur_str: String = row.get("currency")?;
            let product = Product {
                id: row.get("id")?,
                sku: oz_core::Sku::new(sku_str),
                name: row.get("name")?,
                price: oz_core::Money {
                    minor_units: row.get("price_minor")?,
                    currency: cur_str.parse().expect("valid currency in database"),
                },
                category_id: row.get("category_id")?,
                barcode: row.get("barcode")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            };
            Ok(ProductDetail {
                product,
                category_name: row.get("category_name")?,
                stock_qty: row.get("stock_qty")?,
            })
        })
        .expect("execute list_products query");

    let products: Vec<ProductDetail> =
        rows.map(|r| r.expect("deserialize product row")).collect();
    Json(products)
}

/// Get a single product by SKU, including current stock quantity.
/// Returns `null` (JSON `None`) if no product matches the SKU.
pub async fn get_product(
    State(state): State<AppState>,
    Path(sku): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, p.barcode, p.created_at, p.updated_at,
                    c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.sku = ?1",
        )
        .expect("prepare get_product query");

    let result = stmt.query_row([&sku], |row| {
        let sku_str: String = row.get("sku")?;
        let cur_str: String = row.get("currency")?;
        let product = Product {
            id: row.get("id")?,
            sku: oz_core::Sku::new(sku_str),
            name: row.get("name")?,
            price: oz_core::Money {
                minor_units: row.get("price_minor")?,
                currency: cur_str.parse().expect("valid currency in database"),
            },
            category_id: row.get("category_id")?,
            barcode: row.get("barcode")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        };
        Ok(ProductDetail {
            product,
            category_name: row.get("category_name")?,
            stock_qty: row.get("stock_qty")?,
        })
    });

    match result {
        Ok(detail) => Json(Some(detail)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Json(None),
        Err(e) => panic!("get_product query failed: {e}"),
    }
}
