//! Product endpoints.
//!
//! `GET /api/v1/products` — list all products.
//! `GET /api/v1/products/:sku` — product detail including stock quantity.
//! `POST /api/v1/products` — create a new product.
//! `PATCH /api/v1/products/:sku/stock` — adjust stock quantity.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use oz_core::db::Store;
use oz_core::{CoreError, ProductWithDetails};

use crate::AppState;
use crate::auth::ApiTokenClaims;

// ── Error mapping ─────────────────────────────────────────────────────

/// Convert a [`CoreError`] from the Store into an HTTP response.
fn store_error_response(e: CoreError) -> Response {
    match e {
        CoreError::Validation { message, .. } => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": message})),
        )
            .into_response(),
        CoreError::Conflict { .. } => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "resource already exists"})),
        )
            .into_response(),
        CoreError::NotFound { .. } => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        e => {
            tracing::error!("unexpected store error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal error"})),
            )
                .into_response()
        }
    }
}

// ── Request / Response types ──────────────────────────────────────────

/// Request body for creating a new product.
#[derive(Deserialize)]
pub struct CreateProductRequest {
    /// Unique product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Base unit price.
    pub price: oz_core::Money,
    /// Optional category ID.
    pub category_id: Option<String>,
    /// Optional barcode string.
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
    /// The product SKU that was adjusted.
    pub sku: String,
    /// Stock quantity before the adjustment.
    pub previous_qty: i64,
    /// Stock quantity after the adjustment.
    pub new_qty: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────

/// List all products, ordered by name, with category name and stock.
pub async fn list_products(State(state): State<AppState>) -> Response {
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
pub async fn get_product(State(state): State<AppState>, Path(sku): Path<String>) -> Response {
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
///
/// The `tenant_id` from the JWT claims is stamped on the product row
/// so the cloud server's snapshot endpoint can scope products per tenant.
pub async fn create_product(
    State(state): State<AppState>,
    Extension(claims): Extension<ApiTokenClaims>,
    Json(body): Json<CreateProductRequest>,
) -> Response {
    let initial_stock = body.initial_stock.unwrap_or(0);
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.create_product(
        &body.sku,
        &body.name,
        body.price,
        body.category_id.as_deref(),
        body.barcode.as_deref(),
        initial_stock,
        None,
    ) {
        Ok(product) => {
            // Stamp tenant_id from the JWT so snapshot filtering works.
            if let Err(e) = db.execute(
                "UPDATE products SET tenant_id = ?1 WHERE sku = ?2",
                rusqlite::params![tenant_id, body.sku],
            ) {
                tracing::warn!(
                    tenant_id = tenant_id,
                    sku = %body.sku,
                    error = %e,
                    "failed to stamp tenant_id on product — snapshot scoping may be affected"
                );
            }
            let detail = ProductWithDetails {
                product,
                category_name: None,
                stock_qty: if initial_stock > 0 {
                    Some(initial_stock)
                } else {
                    None
                },
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
#[allow(deprecated)]
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
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "product not found"})),
            )
                .into_response();
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // ── store_error_response ────────────────────────────────────

    #[test]
    fn validation_error_returns_400() {
        let err = CoreError::Validation {
            field: "sku",
            message: "must not be empty".into(),
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn conflict_error_returns_409() {
        let err = CoreError::Conflict {
            entity: "product",
            field: "sku",
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn not_found_error_returns_404() {
        let err = CoreError::NotFound {
            entity: "product",
            id: "NOPE-001".into(),
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_returns_500() {
        let err = CoreError::Internal("serialization failed".into());
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn db_error_returns_500_via_catchall() {
        let err = CoreError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn money_overflow_error_returns_500_via_catchall() {
        let err = CoreError::MoneyOverflow {
            left: 1_000_000,
            right: 500_000,
            currency: "USD".into(),
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── CreateProductRequest deserialization ────────────────────

    #[test]
    fn create_product_request_minimal_fields() {
        let json =
            r#"{"sku":"SKU-1","name":"Widget","price":{"minor_units":199,"currency":"USD"}}"#;
        let req: CreateProductRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.sku, "SKU-1");
        assert_eq!(req.name, "Widget");
        assert_eq!(req.price.minor_units, 199);
        assert_eq!(req.price.currency, "USD".parse().unwrap());
        assert!(req.category_id.is_none());
        assert!(req.barcode.is_none());
        assert!(req.initial_stock.is_none());
    }

    #[test]
    fn create_product_request_all_fields() {
        let json = r#"{"sku":"SKU-2","name":"Gadget","price":{"minor_units":499,"currency":"IDR"},"category_id":"cat-1","barcode":"5901234123457","initial_stock":10}"#;
        let req: CreateProductRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.sku, "SKU-2");
        assert_eq!(req.name, "Gadget");
        assert_eq!(req.price.minor_units, 499);
        assert_eq!(req.price.currency, "IDR".parse().unwrap());
        assert_eq!(req.category_id, Some("cat-1".into()));
        assert_eq!(req.barcode, Some("5901234123457".into()));
        assert_eq!(req.initial_stock, Some(10));
    }

    // ── PatchStockRequest deserialization ───────────────────────

    #[test]
    fn patch_stock_request_positive_delta() {
        let json = r#"{"delta":25}"#;
        let req: PatchStockRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.delta, 25);
    }

    #[test]
    fn patch_stock_request_negative_delta() {
        let json = r#"{"delta":-10}"#;
        let req: PatchStockRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.delta, -10);
    }

    // ── PatchStockResponse serialization ────────────────────────

    #[test]
    fn patch_stock_response_serialization() {
        let resp = PatchStockResponse {
            sku: "DRINK-001".into(),
            previous_qty: 50,
            new_qty: 40,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"sku\":\"DRINK-001\""));
        assert!(json.contains("\"previous_qty\":50"));
        assert!(json.contains("\"new_qty\":40"));
    }
}
