//! User endpoints.
/*
last audited 25-07-26 by RSA-Agent (oz-api slice C: users deep read)
crate: oz-api | status: SAFE | lint: CLEAN
findings: API-4 LOW-MED: POST /api/v1/users is JWT-protected but performs NO privilege check — any valid token (label/terminal-scoped) can create a user with any role_id including role-owner, then obtain owner sessions; propose requiring admin-key minted tokens or an owner-scope claim; accepts caller-computed pin_hash (documented contract); SQLite path stamps tenant_id in a follow-up UPDATE (documented non-atomic degrade, same as products)
next: privilege-gate user creation (API-4) | perf: N/A
*/
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
    /// Role ID (e.g. "role-staff", "role-owner").
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

    if let Some(pool) = &state.pg {
        return match crate::pg::create_user(
            pool,
            tenant_id,
            &body.username,
            &body.pin_hash,
            &body.display_name,
            &body.role_id,
        )
        .await
        {
            Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
            Err(e) => e.into_response(),
        };
    }

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
#[path = "users_tests.rs"]
mod tests;
