//! Product endpoints.
//!
//! `GET /api/v1/products` — list all products.
//! `GET /api/v1/products/:sku` — product detail including stock quantity.
//! `POST /api/v1/products` — create a new product.
//! `PATCH /api/v1/products/:sku/stock` — adjust stock quantity.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use oz_core::{CoreError, ProductWithDetails};
use oz_core::db::Store;

use crate::AppState;

// ── Error mapping ─────────────────────────────────────────────────────

/// Convert a [`CoreError`] from the Store into an HTTP response.
fn store_error_response(e: CoreError) -> Response {
    match e {
        CoreError::Validation { message, .. } => {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": message}))).into_response()
        }
        CoreError::Conflict { .. } => {
            (StatusCode::CONFLICT, Json(serde_json::json!({"error": "resource already exists"}))).into_response()
        }
        CoreError::NotFound { .. } => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response()
        }
        e => {
            tracing::error!("unexpected store error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response()
        }
    }
}

// ── Request / Response types ──────────────────────────────────────────

/// Request body for creating a new product.
#[derive(Deserialize)]
pub struct CreateProductRequest {
    pub sku: String,
    pub name: String,
    pub price: oz_core::Money,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    /// Initial stock quantity (>= 0). If omitted or zero, no inventory row is inserted.
    pub initial_stock: Option<i64>,
}

/// Request body for adjusting stock.
#[derive(Deserialize)]
pub struct PatchStockRequest {
    /// Positive to restock, negative to sell.
    pub delta: i64,
}

/// Response after a successful stock adjustment.
#[derive(Serialize)]
pub struct PatchStockResponse {
    pub sku: String,
    pub previous_qty: i64,
    pub new_qty: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────

/// List all products, ordered by name, with category name and stock.
pub async fn list_products(
    State(state): State<AppState>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.list_products() {
        Ok(products) => Json(products).into_response(),
        Err(e) => store_error_response(e),
    }
}

/// Get a single product by SKU, including category name and stock.
///
/// Returns JSON `null` when the product is not found.
pub async fn get_product(
    State(state): State<AppState>,
    Path(sku): Path<String>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.get_product(&sku) {
        Ok(Some(p)) => Json(Some(p)).into_response(),
        Ok(None) => Json(None as Option<ProductWithDetails>).into_response(),
        Err(e) => store_error_response(e),
    }
}

/// Create a new product.
///
/// Accepts a `CreateProductRequest` JSON body. Validation (empty SKU,
/// empty name, negative price, negative stock) is handled by the Store,
/// returning 400 on failure. Returns 201 with the created product.
pub async fn create_product(
    State(state): State<AppState>,
    Json(body): Json<CreateProductRequest>,
) -> Response {
    let initial_stock = body.initial_stock.unwrap_or(0);

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.create_product(
        &body.sku,
        &body.name,
        body.price,
        body.category_id.as_deref(),
        body.barcode.as_deref(),
        initial_stock,
    ) {
        Ok(product) => {
            let detail = ProductWithDetails {
                product,
                category_name: None,
                stock_qty: if initial_stock > 0 { Some(initial_stock) } else { None },
            };
            (StatusCode::CREATED, Json(detail)).into_response()
        }
        Err(e) => store_error_response(e),
    }
}

/// Adjust stock for a product by SKU.
///
/// Accepts `{ "delta": +/-N }`. The Store handles validation, the
/// `checked_add` / `>=0` guard, and the atomic upsert. Returns 200
/// with previous and new quantities.
///
/// Status codes:
/// - 200 — stock adjusted successfully
/// - 404 — product not found
/// - 422 — adjustment would cause negative stock
pub async fn patch_stock(
    State(state): State<AppState>,
    Path(sku): Path<String>,
    Json(body): Json<PatchStockRequest>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Read previous stock while we still have it.
    let product = match store.get_product(&sku) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "product not found"}))).into_response();
        }
        Err(e) => return store_error_response(e),
    };
    let previous_qty = product.stock_qty.unwrap_or(0);

    match store.adjust_stock(&sku, body.delta) {
        Ok(new_qty) => {
            let response = PatchStockResponse {
                sku,
                previous_qty,
                new_qty,
            };
            Json(response).into_response()
        }
        Err(CoreError::Validation { .. }) => {
            // Oversell or overflow -> 422 with details.
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "adjustment would cause negative stock",
                    "previous_qty": previous_qty,
                    "delta": body.delta,
                })),
            )
                .into_response()
        }
        Err(e) => store_error_response(e),
    }
}
