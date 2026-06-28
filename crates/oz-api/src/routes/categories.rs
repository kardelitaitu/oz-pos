//! Category endpoints.
//!
//! `GET /api/v1/categories` — list all product categories.

use axum::{Json, extract::State, http::StatusCode, response::{IntoResponse, Response}};

use oz_core::db::Store;
use oz_core::CoreError;

use crate::AppState;

/// Convert a Store error into an HTTP response.
fn store_error_response(e: CoreError) -> Response {
    tracing::error!("store error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"}))).into_response()
}

/// List all categories, ordered by name.
pub async fn list_categories(
    State(state): State<AppState>,
) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.list_categories() {
        Ok(categories) => Json(categories).into_response(),
        Err(e) => store_error_response(e),
    }
}
