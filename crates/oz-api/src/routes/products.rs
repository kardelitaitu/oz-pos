//! Product endpoints.
//!
//! `GET /api/v1/products` — list all products (with optional category filter).
//! `GET /api/v1/products/:sku` — product detail including stock quantity.

use axum::{
    Json,
    extract::Path,
    response::IntoResponse,
};
use serde::Serialize;

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

/// List products. In this scaffold pass, returns an empty list — the
/// SQL layer will be wired up when `db.rs` lands.
pub async fn list_products() -> impl IntoResponse {
    // Placeholder: query from SQLite when the DB layer is ready.
    Json(Vec::<ProductResponse>::new())
}

/// Get a single product by SKU, including current stock.
pub async fn get_product(Path(sku): Path<String>) -> impl IntoResponse {
    // Placeholder: query from SQLite when the DB layer is ready.
    let _ = sku;
    Json(None::<ProductResponse>)
}
