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
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    /// Helper: deserialize a response body as JSON.
    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("parse JSON body")
    }

    /// Helper: create an authenticated GET request.
    fn auth_get(uri: &str, token: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap()
    }

    /// Helper: create a POST request with a JSON body.
    fn post_json(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    // ── Health endpoint ──────────────────────────────────────────

    #[tokio::test]
    async fn health_returns_ok() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_returns_json_with_status_and_version() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string(), "version should be a string");
    }

    // ── Token endpoint ───────────────────────────────────────────

    #[tokio::test]
    async fn token_creation_returns_200() {
        let app = router();
        let req = post_json("/api/v1/tokens", r#"{"label":"test","expiry_hours":1}"#);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn token_creation_returns_token_fields() {
        let app = router();
        let req = post_json("/api/v1/tokens", r#"{"label":"my-script","expiry_hours":8}"#);
        let resp = app.oneshot(req).await.unwrap();
        let json = body_json(resp).await;
        let token = &json["token"];
        assert!(token["token"].is_string(), "token field should be a string");
        assert!(token["expires_at"].is_string(), "expires_at should be a string");
        assert!(token["token_id"].is_string(), "token_id should be a string");
    }

    #[tokio::test]
    async fn token_creation_with_default_expiry() {
        let app = router();
        // Omit expiry_hours — should default to 24.
        let req = post_json("/api/v1/tokens", r#"{"label":"no-expiry"}"#);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["token"]["token"].is_string());
    }

    #[tokio::test]
    async fn token_creation_missing_label_returns_error() {
        let app = router();
        let req = post_json("/api/v1/tokens", r#"{}"#);
        let resp = app.oneshot(req).await.unwrap();
        // axum 0.8: valid JSON but missing required field → 422 Unprocessable Entity.
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn token_creation_invalid_json() {
        let app = router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn token_creation_wrong_method_get() {
        let app = router();
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/tokens")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn token_creation_empty_body() {
        let app = router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn token_creation_wrong_content_type() {
        let app = router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "text/plain")
            .body(Body::from(r#"{"label":"test"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // axum's Json extractor requires Content-Type: application/json.
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn two_tokens_have_different_values() {
        let app = router();
        let req1 = post_json("/api/v1/tokens", r#"{"label":"a"}"#);
        let req2 = post_json("/api/v1/tokens", r#"{"label":"b"}"#);
        let json1 = body_json(app.clone().oneshot(req1).await.unwrap()).await;
        let json2 = body_json(app.oneshot(req2).await.unwrap()).await;
        assert_ne!(
            json1["token"]["token"].as_str(),
            json2["token"]["token"].as_str(),
            "two tokens should have different JWT strings"
        );
        assert_ne!(
            json1["token"]["token_id"].as_str(),
            json2["token"]["token_id"].as_str(),
            "two tokens should have different IDs"
        );
    }

    // ── Auth middleware ───────────────────────────────────────────

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
    async fn protected_route_accepts_valid_token() {
        let token = auth::create_token("test", Some(1));
        let app = router();
        let req = auth_get("/api/v1/products", &token.token);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_route_rejects_expired_token() {
        let token = auth::create_token("expired", Some(-1));
        let app = router();
        let req = auth_get("/api/v1/products", &token.token);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_malformed_header() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/products")
            .header("Authorization", "NotBearer xyz")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_empty_auth_header() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/products")
            .header("Authorization", "")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_garbage_token() {
        let app = router();
        let req = auth_get("/api/v1/products", "not.a.real.jwt");
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_tampered_token() {
        let token = auth::create_token("tamper", Some(24));
        let app = router();
        let req = auth_get("/api/v1/products", &format!("{}x", token.token));
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Product endpoints ────────────────────────────────────────

    #[tokio::test]
    async fn products_list_returns_empty_array() {
        let token = auth::create_token("test", Some(1));
        let app = router();
        let req = auth_get("/api/v1/products", &token.token);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_array(), "should return a JSON array");
        assert_eq!(json.as_array().unwrap().len(), 0, "should be empty");
    }

    #[tokio::test]
    async fn product_get_by_sku_requires_auth() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/products/ABC123")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn product_get_by_sku_returns_null() {
        let token = auth::create_token("test", Some(1));
        let app = router();
        let req = auth_get("/api/v1/products/ABC123", &token.token);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_null(), "should return null for unknown SKU");
    }

    // ── Category endpoints ───────────────────────────────────────

    #[tokio::test]
    async fn categories_list_requires_auth() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/categories")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn categories_list_returns_empty_array() {
        let token = auth::create_token("test", Some(1));
        let app = router();
        let req = auth_get("/api/v1/categories", &token.token);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_array(), "should return a JSON array");
        assert_eq!(json.as_array().unwrap().len(), 0, "should be empty");
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_route_returns_401() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // The auth middleware on the protected router fires before axum
        // can determine no route matches, so unauthorised is returned.
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn root_returns_401() {
        let app = router();
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cors_headers_present_on_health() {
        let app = router();
        let req = Request::builder()
            .uri("/api/v1/health")
            .header("Origin", "http://example.com")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // CORS should allow all origins.
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap());
        assert_eq!(allow_origin, Some("*"));
    }

    #[tokio::test]
    async fn cors_preflight_returns_ok() {
        let app = router();
        let req = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/products")
            .header("Origin", "http://example.com")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
