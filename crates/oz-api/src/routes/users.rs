//! User endpoints.
/*
last audited 31-08-26 by RSA-Agent (user-role campaign, Section G; consolidates TDD round K / API-4)
crate: oz-api | status: SAFE | lint: CLEAN
findings: API-4 PARTIALLY FIXED — POST /api/v1/users returns 403 for terminal-scoped tokens (may_manage_users: terminal_id claim present), closing the tampered-till -> role-owner escalation; residual deliberate: legacy tokens are indistinguishable from admin-minted and still pass — the complete fix is the admin-key tier (settings/tenants routes already are). pin_hash stored verbatim without PHC shape validation here (the CLI validates at commands/user.rs:84); an unparseable value fails closed at login (verify_pin clean rejection). Section G additions: G-1 the cloud path does NOT call enforce_staff_quota (desktop/tablet create_staff_scoped do at staff.rs:571/636) — a Plus-tier tenant can bypass the C1.1 staff limit via the API; G-2 role_id is accepted verbatim with no registry/builtin-role validation — a typo'd role fails closed at the gate but yields a zombie user; G-3 INFO — staff:delete has NO enforcement consumer across desktop/tablet/cloud/CLI (reserved sensitive key; deactivation rides STAFF_UPDATE)
next: prioritize G-1 (quota evasion) alongside the admin-key-tier move; validate role_id against the registry; shape-validate pin_hash | perf: N/A
*/
//!
//! `POST /api/v1/users` — create a new user.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use oz_core::db::Store;

use oz_core::CoreError;

use crate::auth::ApiTokenClaims;
use crate::routes::tokens::admin_key_authorised;
use crate::AppState;

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
/// Returns 401 without the operator admin key (G-1: user creation is an
/// operator-tier action — the JWT-authenticated self-service path let a
/// Plus-tier tenant bypass the C1.1 staff cap that the desktop enforces;
/// the staff quota is a licensing-side concern and the admin key is the
/// same operator authority the CLI uses, where no quota applies by
/// design). Returns 403 for a terminal-scoped token as defense in depth.
pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(claims): Extension<ApiTokenClaims>,
    Json(body): Json<CreateUserRequest>,
) -> Response {
    // G-1 / API-4: settings and tenant plans already require the admin key;
    // this route was the one admin-tier write still reachable with a plain
    // tenant JWT. A terminal token is a device credential, and creating a
    // role-owner user with it turns a tampered till into an owner session
    // over the whole tenant. Both gates run before anything touches the
    // store.
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }
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
