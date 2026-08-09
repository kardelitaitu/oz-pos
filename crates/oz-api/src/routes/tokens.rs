//! Token management endpoint.
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
fn admin_key_authorised(headers: &HeaderMap, configured: Option<&str>) -> bool {
    let Some(key) = configured else {
        return true; // dev mode — no admin key configured
    };
    headers
        .get(header::HeaderName::from_static(ADMIN_KEY_HEADER))
        .and_then(|v| v.to_str().ok())
        .map(|supplied| supplied == key)
        .unwrap_or(false)
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
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }
    match create_token(&body.label, body.expiry_hours, body.tenant_id.as_deref()) {
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
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn state_with_admin_key(key: Option<&str>) -> AppState {
        AppState {
            db: Arc::new(Mutex::new(oz_core::migrations::fresh_db())),
            admin_key: key.map(|s| s.to_owned()),
        }
    }

    fn request_body() -> CreateTokenRequest {
        CreateTokenRequest {
            label: "test-client".into(),
            expiry_hours: Some(24),
            tenant_id: None,
        }
    }

    fn request_with_header(key: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(k) = key {
            headers.insert(
                header::HeaderName::from_static(ADMIN_KEY_HEADER),
                k.parse().unwrap(),
            );
        }
        headers
    }

    #[tokio::test]
    async fn token_minting_is_open_when_no_admin_key_configured() {
        let response = create_token_handler(
            State(state_with_admin_key(None)),
            HeaderMap::new(),
            Json(request_body()),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["token"]["token"].as_str().unwrap().len() > 20);
    }

    #[tokio::test]
    async fn token_minting_rejects_missing_admin_key_when_configured() {
        let response = create_token_handler(
            State(state_with_admin_key(Some("sekret"))),
            HeaderMap::new(),
            Json(request_body()),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_minting_rejects_wrong_admin_key() {
        let response = create_token_handler(
            State(state_with_admin_key(Some("sekret"))),
            request_with_header(Some("wrong-key")),
            Json(request_body()),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_minting_allows_matching_admin_key() {
        let response = create_token_handler(
            State(state_with_admin_key(Some("sekret"))),
            request_with_header(Some("sekret")),
            Json(request_body()),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_token_returns_200_with_jwt() {
        let response = create_token_handler(
            State(state_with_admin_key(None)),
            HeaderMap::new(),
            Json(request_body()),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["token"]["token"].as_str().unwrap().len() > 20);
        assert_eq!(json["token"]["token_id"].as_str().unwrap().len(), 36); // UUID
    }

    #[tokio::test]
    async fn create_token_defaults_expiry() {
        let body = CreateTokenRequest {
            label: "default-expiry".into(),
            expiry_hours: None,
            tenant_id: None,
        };
        let response = create_token_handler(
            State(state_with_admin_key(None)),
            HeaderMap::new(),
            Json(body),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // expires_at should be present and non-empty
        assert!(!json["token"]["expires_at"].as_str().unwrap().is_empty());
    }

    #[test]
    fn create_token_request_deserialization() {
        let json = r#"{"label":"my-token","expiry_hours":12}"#;
        let req: CreateTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.label, "my-token");
        assert_eq!(req.expiry_hours, Some(12));
        assert_eq!(req.tenant_id, None);
    }

    #[test]
    fn create_token_response_is_serializable() {
        let resp = CreateTokenResponse {
            token: TokenResponse {
                token: "fake.jwt.token".into(),
                expires_at: "2026-07-07T00:00:00Z".into(),
                token_id: "abc-123".into(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("fake.jwt.token"));
    }
}
