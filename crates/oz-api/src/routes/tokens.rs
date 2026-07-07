//! Token management endpoint.
//!
//! `POST /api/v1/tokens` — generate a new API token.
//!
//! NOTE: This endpoint is currently UNPROTECTED (any caller can mint
//! tokens). A follow-up will gate it behind an admin key or existing
//! token. For v0.0.1 single-terminal use this is acceptable.

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::auth::{TokenResponse, create_token};

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    /// Human-readable label for this token (e.g. "kitchen-display-1").
    pub label: String,
    /// Expiry in hours. Defaults to 24 if omitted.
    pub expiry_hours: Option<i64>,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    pub token: TokenResponse,
}

pub async fn create_token_handler(Json(body): Json<CreateTokenRequest>) -> impl IntoResponse {
    let resp = create_token(&body.label, body.expiry_hours);
    Json(CreateTokenResponse { token: resp })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn create_token_returns_200_with_jwt() {
        let body = CreateTokenRequest {
            label: "test-client".into(),
            expiry_hours: Some(24),
        };
        let response = create_token_handler(Json(body)).await.into_response();
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
        };
        let response = create_token_handler(Json(body)).await.into_response();
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
