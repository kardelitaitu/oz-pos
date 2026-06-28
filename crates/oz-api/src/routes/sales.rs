//! Sale endpoints.
//!
//! `POST /api/v1/sales` — create a sale from cart lines.
//! `PATCH /api/v1/sales/{id}/status` — transition sale status.
//! `GET /api/v1/sales/{id}` — get sale detail with line items.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use oz_core::{Cart, CartLine, CoreError, Money, Sale, SaleStatus, Sku};
use oz_core::db::Store;

use crate::AppState;

// ── Error mapping ─────────────────────────────────────────────────────

fn store_error_response(e: CoreError) -> Response {
    match e {
        CoreError::Validation { message, .. } => {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": message}))).into_response()
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

/// Request body for creating a sale.
#[derive(Deserialize)]
pub struct CreateSaleRequest {
    /// Line items for the sale.
    pub lines: Vec<CreateSaleLine>,
}

/// A single line item in a create-sale request.
#[derive(Deserialize)]
pub struct CreateSaleLine {
    pub sku: String,
    pub qty: i64,
    pub unit_price: Money,
}

/// Request body for updating sale status.
#[derive(Deserialize)]
pub struct UpdateSaleStatusRequest {
    /// Target status (kebab-case, e.g. `"active"`, `"completed"`, `"voided"`).
    pub status: SaleStatus,
}

/// Response after a status update.
#[derive(Serialize)]
pub struct SaleStatusResponse {
    pub id: String,
    pub status: SaleStatus,
    pub updated_at: String,
}

// ── Handlers ──────────────────────────────────────────────────────────

/// Create a sale from cart lines.
///
/// Accepts `{ "lines": [{ "sku": "...", "qty": N, "unit_price": {...} }, ...] }`.
/// Builds a `Cart`, converts to `Sale` via the domain type, and persists
/// header + lines in a single transaction. Returns 201 with the full sale.
pub async fn create_sale(
    State(state): State<AppState>,
    Json(body): Json<CreateSaleRequest>,
) -> Response {
    if body.lines.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"error": "sale must have at least one line"})),
        )
            .into_response();
    }

    // Infer the currency from the first line; all subsequent lines must match.
    let first = &body.lines[0];
    let mut cart = Cart::new(first.unit_price.currency);

    for line in &body.lines {
        let cl = CartLine::new(Sku::new(&line.sku), line.qty, line.unit_price);
        if let Err(e) = cart.add_line(cl) {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.to_string()}))).into_response();
        }
    }

    let sale = match Sale::from_cart(&cart) {
        Some(s) => s,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": "cart total overflow"})),
            )
                .into_response();
        }
    };

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.create_sale(&sale) {
        Ok(()) => (StatusCode::CREATED, Json(sale)).into_response(),
        Err(e) => store_error_response(e),
    }
}

/// Get a single sale by id, including all line items.
///
/// Returns JSON `null` when the sale is not found.
pub async fn get_sale(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.get_sale(&id) {
        Ok(Some(sale)) => Json(Some(sale)).into_response(),
        Ok(None) => Json(None as Option<Sale>).into_response(),
        Err(e) => store_error_response(e),
    }
}

/// Transition a sale's status.
///
/// Accepts `{ "status": "active|completed|voided" }`. Validates the
/// state machine transition. Returns 200 with the updated sale on
/// success, 404 if the sale doesn't exist, 422 for invalid transitions.
pub async fn update_sale_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSaleStatusRequest>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.update_sale_status(&id, body.status) {
        Ok(sale) => {
            let resp = SaleStatusResponse {
                id: sale.id,
                status: sale.status,
                updated_at: sale.updated_at,
            };
            Json(resp).into_response()
        }
        Err(CoreError::Validation { message, .. }) => {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": message}))).into_response()
        }
        Err(e) => store_error_response(e),
    }
}
