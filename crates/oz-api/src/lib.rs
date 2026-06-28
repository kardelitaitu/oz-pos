//! OZ-POS OpenAPI REST server.
//!
//! Starts an axum HTTP server on `OZ_API_PORT` (default 3099) with JWT
//! authentication on protected routes. The server runs alongside the
//! Tauri front-end so third-party scripts, kitchen displays, and
//! inventory scanners can query the POS data.
//!
//! # Quick start
//!
//! ```ignore
//! // In src-tauri/src/main.rs or a background task:
//! oz_api::serve().await?;
//! ```
//!
//! Then generate a token:
//!
//! ```bash
//! curl -X POST http://localhost:3099/api/v1/tokens \
//!   -H "Content-Type: application/json" \
//!   -d '{"label": "my-script"}'
//! ```
//!
//! Use the token on protected routes:
//!
//! ```bash
//! curl http://localhost:3099/api/v1/products \
//!   -H "Authorization: Bearer <token>"
//! ```

pub mod auth;
pub mod routes;

use axum::{
    Router,
    middleware,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

/// Build the API router with all routes and middleware.
///
/// Public routes (no auth):
/// - `GET /api/v1/health`
///
/// Token management (no auth in this pass; add admin key later):
/// - `POST /api/v1/tokens`
///
/// Protected routes (JWT required):
/// - `GET /api/v1/products`
/// - `GET /api/v1/products/:sku`
/// - `GET /api/v1/categories`
pub fn router() -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let public = Router::new()
        .route("/api/v1/health", get(routes::health::health))
        .route("/api/v1/tokens", post(routes::tokens::create_token_handler));

    let protected = Router::new()
        .route("/api/v1/products", get(routes::products::list_products))
        .route("/api/v1/products/{sku}", get(routes::products::get_product))
        .route("/api/v1/categories", get(routes::categories::list_categories))
        .layer(middleware::from_fn(auth::auth_middleware));

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Start the server, binding to the port from `OZ_API_PORT` (default 3099).
///
/// This function blocks on the server loop. Spawn it in a background
/// `tokio::task` if you need the caller to continue.
pub async fn serve() {
    let port: u16 = std::env::var("OZ_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3099);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind API port");
    info!(port, "OZ-POS API server listening");
    axum::serve(listener, router())
        .await
        .expect("API server exited with error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_route_rejects_without_token() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/products")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_endpoint_creates_valid_token() {
        let app = router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"label":"test","expiry_hours":1}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_route_accepts_valid_token() {
        let token = auth::create_token("test", Some(1));
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/products")
            .header("Authorization", format!("Bearer {}", token.token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
