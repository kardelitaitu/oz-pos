//! User endpoints.
//!
//! `POST /api/v1/users` — create a new user.

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

/// Request body for creating a user.
#[derive(Deserialize)]
pub struct CreateUserRequest {
    /// Unique username for login.
    pub username: String,
    /// SHA-256 hash of the user's PIN.
    pub pin_hash: String,
    /// Display name shown in the UI.
    pub display_name: String,
    /// Role ID (e.g. "role-cashier", "role-owner").
    pub role_id: String,
}

/// Create a new user.
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

/// Create a new user.
///
/// Accepts a `CreateUserRequest` JSON body. Returns 201 with the created
/// user. The `tenant_id` from the JWT claims is stamped on the user row
/// so the cloud server's snapshot endpoint can scope users per tenant.
pub async fn create_user(
    State(state): State<AppState>,
    Extension(claims): Extension<ApiTokenClaims>,
    Json(body): Json<CreateUserRequest>,
) -> Response {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.create_user(
        &body.username,
        &body.pin_hash,
        &body.display_name,
        &body.role_id,
    ) {
        Ok(user) => {
            // Stamp tenant_id from the JWT so snapshot filtering works.
            if let Err(e) = db.execute(
                "UPDATE users SET tenant_id = ?1 WHERE id = ?2",
                rusqlite::params![tenant_id, user.id],
            ) {
                tracing::warn!(
                    tenant_id = tenant_id,
                    user_id = %user.id,
                    error = %e,
                    "failed to stamp tenant_id on user — snapshot scoping may be affected"
                );
            }
            (StatusCode::CREATED, Json(user)).into_response()
        }
        Err(e) => store_error_response(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // ── CreateUserRequest deserialization ───────────────────────

    #[test]
    fn create_user_request_minimal() {
        let json = r#"{"username":"alice","pin_hash":"hash123","display_name":"Alice","role_id":"role-cashier"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.pin_hash, "hash123");
        assert_eq!(req.display_name, "Alice");
        assert_eq!(req.role_id, "role-cashier");
    }

    #[test]
    fn create_user_request_owner_role() {
        let json = r#"{"username":"owner","pin_hash":"abc","display_name":"Owner","role_id":"role-owner"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "owner");
        assert_eq!(req.role_id, "role-owner");
    }
}
