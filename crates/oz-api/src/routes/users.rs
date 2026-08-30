//! User endpoints.
/*
last audited 31-08-26 by TDD-Agent (round K; API-4 gated for device tokens, pin_hash doc corrected)
crate: oz-api | status: SAFE | lint: CLEAN
findings: API-4 PARTIALLY FIXED — POST /api/v1/users now returns 403 for any token whose terminal_id claim is set, i.e. tokens minted through the terminal client-credentials path (ADR sync-auth-hardening P3). That closes the escalation from a tampered or secret-extracted POS terminal to a role-owner session over the whole tenant. Residual, deliberate: legacy tokens also carry terminal_id None and cannot be distinguished from admin-minted ones, so they still pass. The complete fix is to move this route to the admin-key tier — /api/v1/settings and /api/v1/tenants/{tenant_id}/plan are both already admin-key-only, which is why user creation being plain-JWT was the outlier, not the design. pin_hash remains caller-computed and is NOT shape-validated here (the CLI does validate PHC at commands/user.rs:84), so a client can still store a non-hash value; it then can never verify, which fails closed. B54: this file's pin_hash doc said "SHA-256 hash of the user's PIN"; hash_pin actually produces Argon2id PHC and verify_pin rejects unparseable input cleanly, so following the documented contract yielded a 201 and a permanently locked-out user.
next: move user creation to the admin-key tier to finish API-4; consider validating pin_hash shape here as the CLI does | perf: N/A
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
    /// PHC-formatted Argon2id hash of the user's PIN, as produced by
    /// `oz_core::auth::hash_pin` — e.g.
    /// `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`.
    ///
    /// This is NOT a SHA-256 digest, which an earlier version of this
    /// comment claimed. `verify_pin` treats an unparseable hash as a clean
    /// rejection, so a client that followed the old wording would get a
    /// 201 and a user who can never log in. The value is stored verbatim:
    /// hash it before sending.
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

/// May this token create users?
///
/// Pure so the rule is testable without a store or an HTTP stack.
///
/// `terminal_id` is `Some` only for tokens minted through the terminal
/// client-credentials path (ADR sync-auth-hardening P3), so this rejects
/// device credentials while leaving admin-minted tokens — and legacy
/// tokens, which cannot be told apart from admin ones — alone.
fn may_manage_users(claims: &ApiTokenClaims) -> bool {
    claims.terminal_id.is_none()
}

/// Create a new user.
///
/// Accepts a `CreateUserRequest` JSON body. Returns 201 with the created
/// user. The `tenant_id` from the JWT claims is stamped on the user row
/// so the cloud server's snapshot endpoint can scope users per tenant.
///
/// Returns 403 for a terminal-scoped token: user management is an
/// admin-tier operation (API-4).
pub async fn create_user(
    State(state): State<AppState>,
    Extension(claims): Extension<ApiTokenClaims>,
    Json(body): Json<CreateUserRequest>,
) -> Response {
    // API-4: settings and tenant plans already require the admin key; this
    // route was the one admin-tier write still reachable with a plain JWT.
    // A terminal token is a device credential, and creating a role-owner
    // user with it turns a tampered till into an owner session over the
    // whole tenant. Checked before anything touches the store.
    if !may_manage_users(&claims) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "insufficient_scope",
                "message": "user management requires an admin-minted token; \
                            this token is scoped to a registered terminal",
            })),
        )
            .into_response();
    }

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
