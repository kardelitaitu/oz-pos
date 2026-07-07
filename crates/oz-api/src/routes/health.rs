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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn health_returns_200_with_ok_status() {
        let response = health().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn health_returns_cargo_pkg_version() {
        let response = health().await.into_response();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn health_response_is_serializable() {
        let resp = HealthResponse {
            status: "ok",
            version: "0.0.3",
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"0.0.3\""));
    }
}
