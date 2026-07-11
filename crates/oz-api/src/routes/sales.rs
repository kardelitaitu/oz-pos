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

use oz_core::db::Store;
use oz_core::{Cart, CartLine, CoreError, Money, Sale, SaleStatus, Sku};

use crate::AppState;

// ── Error mapping ─────────────────────────────────────────────────────

fn store_error_response(e: CoreError) -> Response {
    match e {
        CoreError::Validation { message, .. } => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": message})),
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

/// Request body for creating a sale.
#[derive(Deserialize)]
pub struct CreateSaleRequest {
    /// Line items for the sale.
    pub lines: Vec<CreateSaleLine>,
}

/// A single line item in a create-sale request.
#[derive(Deserialize)]
pub struct CreateSaleLine {
    /// Product SKU.
    pub sku: String,
    /// Quantity (must be > 0).
    pub qty: i64,
    /// Unit price for this line.
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
    /// Sale ID.
    pub id: String,
    /// Updated sale status.
    pub status: SaleStatus,
    /// ISO-8601 timestamp of the update.
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
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
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
pub async fn get_sale(State(state): State<AppState>, Path(id): Path<String>) -> Response {
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
        Err(CoreError::Validation { message, .. }) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"error": message})),
        )
            .into_response(),
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
            field: "lines",
            message: "must have at least one line".into(),
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn not_found_error_returns_404() {
        let err = CoreError::NotFound {
            entity: "sale",
            id: "nope-123".into(),
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_returns_500_via_catchall() {
        let err = CoreError::Internal("db connection lost".into());
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn conflict_error_returns_500_via_catchall() {
        // sales.rs store_error_response has no explicit Conflict arm,
        // so it falls through to the catch-all → 500.
        let err = CoreError::Conflict {
            entity: "sale",
            field: "id",
        };
        let resp = store_error_response(err);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── CreateSaleRequest / CreateSaleLine deserialization ──────

    #[test]
    fn create_sale_request_single_line() {
        let json = r#"{"lines":[{"sku":"SKU-1","qty":2,"unit_price":{"minor_units":350,"currency":"USD"}}]}"#;
        let req: CreateSaleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.lines.len(), 1);
        assert_eq!(req.lines[0].sku, "SKU-1");
        assert_eq!(req.lines[0].qty, 2);
        assert_eq!(req.lines[0].unit_price.minor_units, 350);
        assert_eq!(req.lines[0].unit_price.currency, "USD".parse().unwrap());
    }

    #[test]
    fn create_sale_request_multi_line() {
        let json = r#"{"lines":[
            {"sku":"A","qty":2,"unit_price":{"minor_units":100,"currency":"USD"}},
            {"sku":"B","qty":1,"unit_price":{"minor_units":200,"currency":"USD"}}
        ]}"#;
        let req: CreateSaleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.lines.len(), 2);
        assert_eq!(req.lines[0].sku, "A");
        assert_eq!(req.lines[0].qty, 2);
        assert_eq!(req.lines[1].sku, "B");
        assert_eq!(req.lines[1].qty, 1);
        assert_eq!(req.lines[1].unit_price.minor_units, 200);
    }

    #[test]
    fn create_sale_request_empty_lines_deserializes() {
        let json = r#"{"lines":[]}"#;
        let req: CreateSaleRequest = serde_json::from_str(json).unwrap();
        assert!(req.lines.is_empty());
    }

    // ── UpdateSaleStatusRequest deserialization ─────────────────

    #[test]
    fn update_sale_status_request_active() {
        let json = r#"{"status":"active"}"#;
        let req: UpdateSaleStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, SaleStatus::Active);
    }

    #[test]
    fn update_sale_status_request_completed() {
        let json = r#"{"status":"completed"}"#;
        let req: UpdateSaleStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, SaleStatus::Completed);
    }

    #[test]
    fn update_sale_status_request_voided() {
        let json = r#"{"status":"voided"}"#;
        let req: UpdateSaleStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, SaleStatus::Voided);
    }

    #[test]
    fn update_sale_status_request_pending() {
        let json = r#"{"status":"pending"}"#;
        let req: UpdateSaleStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, SaleStatus::Pending);
    }

    // ── SaleStatusResponse serialization ────────────────────────

    #[test]
    fn sale_status_response_serialization() {
        let resp = SaleStatusResponse {
            id: "sale-1".into(),
            status: SaleStatus::Active,
            updated_at: "2025-01-15T10:30:00Z".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"id\":\"sale-1\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"updated_at\":\"2025-01-15T10:30:00Z\""));
    }
}
