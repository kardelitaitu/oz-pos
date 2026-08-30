//! Token management endpoint.
/*
last audited 25-07-26 by RSA-Agent (oz-api slice A: tokens deep read; API-2 FIXED 25-07-26)
crate: oz-api | status: SAFE | lint: CLEAN
findings: API-2 FIXED — admin_key_authorised now compares via HMAC-SHA256 digests under a fixed context key with subtle-backed verify_slice (constant-time; plain == short-circuited on the first differing byte), 4 new unit tests (exact match, wrong/prefix/suffix probes, dev-open, missing header); dev-open mode remains documented and unreachable behind OZ_PRODUCTION=1 via the API-1 production-secrets gate. Clean structure otherwise — P2 admin-gated label mint, P3 terminal client-credentials path takes tenant from the registration (never the body)
next: none | perf: N/A
*/
//!
//! `POST /api/v1/tokens` — generate a new API token.
//!
//! ADR sync-auth-hardening P2: when the server is started with an
//! `OZ_ADMIN_KEY` environment variable, minting a token requires an
//! `X-Admin-Key` header matching it. When `OZ_ADMIN_KEY` is unset the
//! endpoint stays open (dev mode / local Docker) so auto-provisioning
//! keeps working without extra configuration.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::auth::{TokenResponse, create_token};

/// Request body for creating a new API token.
#[derive(Deserialize)]
pub struct CreateTokenRequest {
    /// Human-readable label for this token (e.g. "kitchen-display-1").
    pub label: String,
    /// Expiry in hours. Defaults to 24 if omitted.
    pub expiry_hours: Option<i64>,
    /// Optional tenant / store ID for multi-tenant cloud isolation.
    pub tenant_id: Option<String>,
    /// Client credentials from a registered terminal (ADR sync-auth-hardening
    /// P3). When both are present the token is minted for that terminal
    /// without requiring the admin key; the tenant is taken from the
    /// terminal's registration, never from the request body.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Device secret paired with `client_id` (ADR sync-auth-hardening P3).
    #[serde(default)]
    pub client_secret: Option<String>,
}

/// Response body containing the newly created token.
#[derive(Serialize)]
pub struct CreateTokenResponse {
    /// The token details (JWT string, expiry, id).
    pub token: TokenResponse,
}

/// Header carrying the admin key that gates token minting (ADR
/// sync-auth-hardening P2).
const ADMIN_KEY_HEADER: &str = "x-admin-key";

/// Check whether the request is authorised to mint a token.
///
/// Returns `true` when the server has no admin key configured (dev mode),
/// or when the `X-Admin-Key` header matches the configured key.
///
/// API-2 fix: the comparison is constant-time. Both values are hashed
/// with HMAC-SHA256 under a fixed context key and the digests are checked
/// with `verify_slice` (subtle-backed), so response timing does not leak
/// how many leading bytes of the header matched — a plain `==` on strings
/// short-circuits on the first differing byte. The fixed HMAC key is fine
/// here: the admin key is not attacker-chosen secret input to the MAC, the
/// hashing exists only to make the equality check data-independent.
pub fn admin_key_authorised(headers: &HeaderMap, configured: Option<&str>) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let Some(key) = configured else {
        return true; // dev mode — no admin key configured
    };
    let Some(supplied) = headers
        .get(header::HeaderName::from_static(ADMIN_KEY_HEADER))
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };

    type HmacSha256 = Hmac<Sha256>;
    let digest = |value: &str| {
        let mut mac = HmacSha256::new_from_slice(b"oz-api-admin-key-compare")
            .expect("HMAC accepts any key length");
        mac.update(value.as_bytes());
        mac.finalize().into_bytes()
    };

    let provided = digest(supplied);
    let mut mac = HmacSha256::new_from_slice(b"oz-api-admin-key-compare")
        .expect("HMAC accepts any key length");
    mac.update(key.as_bytes());
    // `verify_slice` is the constant-time comparison; a digest `==` is not
    // guaranteed data-independent.
    mac.verify_slice(provided.as_slice()).is_ok()
}

/// `POST /api/v1/tokens` — create a new API token.
///
/// Returns 401 when an admin key is configured but missing/mismatched.
/// Returns 500 if JWT encoding fails (should never happen in practice).
pub async fn create_token_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    // ADR sync-auth-hardening P3: terminal client-credentials path. A
    // registered terminal mints its own scoped token — no admin key needed.
    if let (Some(client_id), Some(client_secret)) =
        (body.client_id.as_deref(), body.client_secret.as_deref())
    {
        let verified = if let Some(pool) = &state.pg {
            crate::pg::verify_terminal_credentials(pool, client_id, client_secret)
                .await
                .map_err(|e| e.to_string())
        } else {
            let db = state.db.lock().await;
            let verified = crate::routes::terminals::verify_terminal_credentials(
                &db,
                client_id,
                client_secret,
            );
            drop(db);
            verified.map_err(|e| e.to_string())
        };
        return match verified {
            Ok(Some(terminal)) => {
                match crate::auth::create_token_scoped(
                    &body.label,
                    body.expiry_hours,
                    terminal.tenant_id.as_deref(),
                    Some(&terminal.terminal_id),
                    Some(&state.api_secret),
                ) {
                    Ok(resp) => Json(CreateTokenResponse { token: resp }).into_response(),
                    Err(e) => {
                        tracing::error!(?e, "JWT encoding failed");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": "token generation failed"})),
                        )
                            .into_response()
                    }
                }
            }
            Ok(None) => (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_credentials"})),
            )
                .into_response(),
            Err(e) => {
                tracing::error!(error = %e, "verifying terminal credentials failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "token generation failed"})),
                )
                    .into_response()
            }
        };
    }

    // Admin-gated label mint (P2).
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }
    match create_token(
        &body.label,
        body.expiry_hours,
        body.tenant_id.as_deref(),
        Some(&state.api_secret),
    ) {
        Ok(resp) => Json(CreateTokenResponse { token: resp }).into_response(),
        Err(e) => {
            tracing::error!(?e, "JWT encoding failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "token generation failed"})),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;
