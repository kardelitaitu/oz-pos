//! Category endpoints.
//!
//! `GET /api/v1/categories` — list all product categories.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use oz_core::CoreError;
use oz_core::db::Store;

use crate::AppState;

/// Convert a Store error into an HTTP response.
fn store_error_response(e: CoreError) -> Response {
    tracing::error!("store error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "internal error"})),
    )
        .into_response()
}

/// List all categories, ordered by name.
pub async fn list_categories(State(state): State<AppState>) -> Response {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.list_categories() {
        Ok(categories) => Json(categories).into_response(),
        Err(e) => store_error_response(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // ── store_error_response ────────────────────────────────────

    /// categories.rs uses a simplified store_error_response that
    /// maps ALL CoreError variants to 500 (unlike the more granular
    /// versions in products.rs or sales.rs).
    #[test]
    fn all_errors_return_500() {
        let errors = vec![
            CoreError::Internal("db fail".into()),
            CoreError::NotFound {
                entity: "category",
                id: "nope".into(),
            },
            CoreError::Validation {
                field: "name",
                message: "required".into(),
            },
            CoreError::Conflict {
                entity: "category",
                field: "name",
            },
            CoreError::Db(rusqlite::Error::InvalidParameterName("x".into())),
            CoreError::MoneyOverflow {
                left: 1,
                right: 1,
                currency: "USD".into(),
            },
            CoreError::CurrencyMismatch("USD".into(), "EUR".into()),
        ];
        for err in errors {
            let resp = store_error_response(err);
            assert_eq!(
                resp.status(),
                StatusCode::INTERNAL_SERVER_ERROR,
                "all CoreError variants should return 500"
            );
        }
    }
}
