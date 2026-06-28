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

use crate::AppState;

#[derive(Serialize)]
pub struct ProductResponse {
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
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
            "SELECT p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             ORDER BY p.name",
        )
        .expect("prepare list_products query");

    let rows = stmt
        .query_map([], |row| {
            Ok(ProductResponse {
                sku: row.get("sku")?,
                name: row.get("name")?,
                price_minor: row.get("price_minor")?,
                currency: row.get("currency")?,
                category_id: row.get("category_id")?,
                category_name: row.get("category_name")?,
                stock_qty: row.get("stock_qty")?,
            })
        })
        .expect("execute list_products query");

    let products: Vec<ProductResponse> =
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
            "SELECT p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.sku = ?1",
        )
        .expect("prepare get_product query");

    let result = stmt.query_row([&sku], |row| {
        Ok(ProductResponse {
            sku: row.get("sku")?,
            name: row.get("name")?,
            price_minor: row.get("price_minor")?,
            currency: row.get("currency")?,
            category_id: row.get("category_id")?,
            category_name: row.get("category_name")?,
            stock_qty: row.get("stock_qty")?,
        })
    });

    match result {
        Ok(product) => Json(Some(product)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Json(None),
        Err(e) => panic!("get_product query failed: {e}"),
    }
}
