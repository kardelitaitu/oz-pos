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
