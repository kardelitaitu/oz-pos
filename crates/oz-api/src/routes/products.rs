//! Product endpoints.
//!
//! `GET /api/v1/products` — list all products (with optional category filter).
//! `GET /api/v1/products/:sku` — product detail including stock quantity.

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;

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
