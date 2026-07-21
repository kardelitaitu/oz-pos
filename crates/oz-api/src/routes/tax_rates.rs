//! Tax rate endpoints.
//!
//! `POST /api/v1/tax-rates` — create a new tax rate.

use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use oz_core::db::Store;

use oz_core::CoreError;

use crate::AppState;
use crate::auth::ApiTokenClaims;

/// Request body for creating a tax rate.
#[derive(Deserialize)]
pub struct CreateTaxRateRequest {
    /// Tax rate display name.
    pub name: String,
    /// Rate in basis points (e.g. 1000 = 10%).
    pub rate_bps: i64,
    /// Whether this is the default rate.
    pub is_default: bool,
    /// Whether tax is inclusive of the listed price.
    pub is_inclusive: bool,
}

/// Create a new tax rate.
///
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

/// Create a new tax rate.
///
/// Accepts a `CreateTaxRateRequest` JSON body. Returns 201 with the created
/// tax rate. The `tenant_id` from the JWT claims is stamped on the tax rate
/// row so the cloud server's snapshot endpoint can scope rates per tenant.
pub async fn create_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<ApiTokenClaims>,
    Json(body): Json<CreateTaxRateRequest>,
) -> Response {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.create_tax_rate(
        &body.name,
        body.rate_bps,
        body.is_default,
        body.is_inclusive,
    ) {
        Ok(rate) => {
            // Stamp tenant_id from the JWT so snapshot filtering works.
            if let Err(e) = db.execute(
                "UPDATE tax_rates SET tenant_id = ?1 WHERE id = ?2",
                rusqlite::params![tenant_id, rate.id],
            ) {
                tracing::warn!(
                    tenant_id = tenant_id,
                    tax_rate_id = %rate.id,
                    error = %e,
                    "failed to stamp tenant_id on tax rate — snapshot scoping may be affected"
                );
            }
            (StatusCode::CREATED, Json(rate)).into_response()
        }
        Err(e) => store_error_response(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CreateTaxRateRequest deserialization ────────────────────

    #[test]
    fn create_tax_rate_request_minimal() {
        let json = r#"{"name":"VAT 10%","rate_bps":1000,"is_default":true,"is_inclusive":false}"#;
        let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "VAT 10%");
        assert_eq!(req.rate_bps, 1000);
        assert!(req.is_default);
        assert!(!req.is_inclusive);
    }

    #[test]
    fn create_tax_rate_request_inclusive() {
        let json = r#"{"name":"GST 5%","rate_bps":500,"is_default":false,"is_inclusive":true}"#;
        let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "GST 5%");
        assert_eq!(req.rate_bps, 500);
        assert!(!req.is_default);
        assert!(req.is_inclusive);
    }
}
