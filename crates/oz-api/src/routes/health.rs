//! Health check endpoint.
//!
//! `GET /api/v1/health` — returns the server status and version.
//! Not protected by auth; used for monitoring and readiness probes.

use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
