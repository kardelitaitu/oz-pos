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
//! // In apps/desktop-client/src/main.rs or a background task:
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

use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use rusqlite::Connection;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

/// Shared application state passed to all axum handlers.
///
/// Wraps the SQLite connection in `Arc<Mutex<>>` so axum can cheaply
/// clone it for the [`State`](axum::extract::State) extractor while
/// ensuring only one handler writes to the database at a time.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

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
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let public = Router::new()
        .route("/api/v1/health", get(routes::health::health))
        .route("/api/v1/tokens", post(routes::tokens::create_token_handler));

    let protected = Router::new()
        .route(
            "/api/v1/products",
            get(routes::products::list_products).post(routes::products::create_product),
        )
        .route("/api/v1/products/{sku}", get(routes::products::get_product))
        .route(
            "/api/v1/products/{sku}/stock",
            patch(routes::products::patch_stock),
        )
        .route(
            "/api/v1/categories",
            get(routes::categories::list_categories),
        )
        .route("/api/v1/sales", post(routes::sales::create_sale))
        .route("/api/v1/sales/{id}", get(routes::sales::get_sale))
        .route(
            "/api/v1/sales/{id}/status",
            patch(routes::sales::update_sale_status),
        )
        .layer(middleware::from_fn(auth::auth_middleware));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Start the server, binding to the port from `OZ_API_PORT` (default 3099).
///
/// Opens the SQLite database at `OZ_DB_PATH` (default `oz-pos.db`), runs
/// migrations, and blocks on the server loop. Spawn in a background
/// `tokio::task` if the caller needs to continue.
pub async fn serve() {
    let db_path = std::env::var("OZ_DB_PATH").unwrap_or_else(|_| "oz-pos.db".into());
    let mut conn = Connection::open(&db_path).expect("failed to open API database");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enabling foreign_keys");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enabling WAL");
    oz_core::migrations::run(&mut conn).expect("running migrations");

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
    };

    let port: u16 = std::env::var("OZ_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3099);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind API port");
    info!(port, "OZ-POS API server listening");
    axum::serve(listener, router(state))
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

    /// Helper: open an in-memory connection with all migrations pre-applied.
    fn fresh_conn() -> Connection {
        oz_core::migrations::fresh_db()
    }

    /// Helper: build a router backed by an empty in-memory database.
    fn test_app() -> Router {
        let state = AppState {
            db: Arc::new(Mutex::new(fresh_conn())),
        };
        router(state)
    }

    /// Helper: build a router with seeded products, categories, and inventory.
    fn test_app_seeded() -> Router {
        let conn = fresh_conn();
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour) VALUES
                ('cat-drinks', 'Drinks',  '#06b6d4'),
                ('cat-food',   'Food',    '#f97316');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id) VALUES
                ('prod-1', 'DRINK-001', 'Espresso',        350, 'USD', 'cat-drinks'),
                ('prod-2', 'FOOD-001',  'Bagel',           450, 'USD', 'cat-food'),
                ('prod-3', 'DRINK-002', 'Green Tea',       275, 'USD', 'cat-drinks');
             INSERT INTO inventory (product_id, qty) VALUES
                ('prod-1', 50),
                ('prod-2', 12);",
        )
        .unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
        };
        router(state)
    }

    // ── Helpers ───────────────────────────────────────────────────

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("parse JSON body")
    }

    fn auth_get(uri: &str, token: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    }

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
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_returns_json_with_status_and_version() {
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string(), "version should be a string");
    }

    // ── Token endpoint ───────────────────────────────────────────

    #[tokio::test]
    async fn token_creation_returns_200() {
        let req = post_json("/api/v1/tokens", r#"{"label":"test","expiry_hours":1}"#);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn token_creation_returns_token_fields() {
        let req = post_json(
            "/api/v1/tokens",
            r#"{"label":"my-script","expiry_hours":8}"#,
        );
        let resp = test_app().oneshot(req).await.unwrap();
        let json = body_json(resp).await;
        let token = &json["token"];
        assert!(token["token"].is_string(), "token field should be a string");
        assert!(
            token["expires_at"].is_string(),
            "expires_at should be a string"
        );
        assert!(token["token_id"].is_string(), "token_id should be a string");
    }

    #[tokio::test]
    async fn token_creation_with_default_expiry() {
        let req = post_json("/api/v1/tokens", r#"{"label":"no-expiry"}"#);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["token"]["token"].is_string());
    }

    #[tokio::test]
    async fn token_creation_missing_label_returns_error() {
        let req = post_json("/api/v1/tokens", r#"{}"#);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn token_creation_invalid_json() {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn token_creation_wrong_method_get() {
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/tokens")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn token_creation_empty_body() {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn token_creation_wrong_content_type() {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tokens")
            .header("Content-Type", "text/plain")
            .body(Body::from(r#"{"label":"test"}"#))
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn two_tokens_have_different_values() {
        let app = test_app();
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
        let req = Request::builder()
            .uri("/api/v1/products")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_accepts_valid_token() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/products", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_route_rejects_expired_token() {
        let token = auth::create_token("expired", Some(-1));
        let req = auth_get("/api/v1/products", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_malformed_header() {
        let req = Request::builder()
            .uri("/api/v1/products")
            .header("Authorization", "NotBearer xyz")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_empty_auth_header() {
        let req = Request::builder()
            .uri("/api/v1/products")
            .header("Authorization", "")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_garbage_token() {
        let req = auth_get("/api/v1/products", "not.a.real.jwt");
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_tampered_token() {
        let token = auth::create_token("tamper", Some(24));
        let req = auth_get("/api/v1/products", &format!("{}x", token.token));
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Product endpoints (empty DB) ─────────────────────────────

    #[tokio::test]
    async fn products_list_returns_empty_array() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/products", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_array(), "should return a JSON array");
        assert_eq!(json.as_array().unwrap().len(), 0, "should be empty");
    }

    #[tokio::test]
    async fn product_get_by_sku_requires_auth() {
        let req = Request::builder()
            .uri("/api/v1/products/ABC123")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn product_get_by_sku_returns_null_for_unknown() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/products/ABC123", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_null(), "should return null for unknown SKU");
    }

    // ── Product endpoints (seeded DB) ────────────────────────────

    #[tokio::test]
    async fn products_list_returns_seeded_products() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/products", &token.token);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3, "should return 3 seeded products");
    }

    #[tokio::test]
    async fn product_get_by_sku_returns_detail_with_stock() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/products/DRINK-001", &token.token);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["sku"], "DRINK-001");
        assert_eq!(json["name"], "Espresso");
        assert_eq!(json["price"]["minor_units"], 350);
        assert_eq!(json["price"]["currency"], "USD");
        assert_eq!(json["category_id"], "cat-drinks");
        assert_eq!(json["category_name"], "Drinks");
        assert_eq!(json["stock_qty"], 50);
        // New fields from the Product domain type.
        assert_eq!(json["id"], "prod-1");
        assert!(json["barcode"].is_null());
        assert!(json["created_at"].is_string());
        assert!(json["updated_at"].is_string());
    }

    #[tokio::test]
    async fn product_get_by_sku_returns_null_for_existing_but_unstocked() {
        let token = auth::create_token("test", Some(1));
        // DRINK-002 exists but has no inventory row.
        let req = auth_get("/api/v1/products/DRINK-002", &token.token);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["sku"], "DRINK-002");
        assert_eq!(json["name"], "Green Tea");
        assert_eq!(json["price"]["minor_units"], 275);
        assert!(json["stock_qty"].is_null(), "no inventory row → null stock");
    }

    // ── Product creation endpoint ───────────────────────────────

    fn auth_post_json(uri: &str, token: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    #[tokio::test]
    async fn create_product_returns_201() {
        let token = auth::create_token("test", Some(1));
        let body =
            r#"{"sku":"NEW-001","name":"New Item","price":{"minor_units":199,"currency":"USD"}}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_product_returns_fields() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"NEW-002","name":"Widget","price":{"minor_units":499,"currency":"USD"},"category_id":"cat-drinks","barcode":"5901234123457"}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["sku"], "NEW-002");
        assert_eq!(json["name"], "Widget");
        assert_eq!(json["price"]["minor_units"], 499);
        assert_eq!(json["price"]["currency"], "USD");
        assert_eq!(json["category_id"], "cat-drinks");
        assert_eq!(json["barcode"], "5901234123457");
        assert!(json["id"].is_string());
        assert!(json["created_at"].is_string());
        assert!(json["updated_at"].is_string());
    }

    #[tokio::test]
    async fn create_product_with_initial_stock() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"STOCKED-1","name":"Stocked","price":{"minor_units":100,"currency":"USD"},"initial_stock":25}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["stock_qty"], 25);
    }

    #[tokio::test]
    async fn create_product_with_zero_stock_no_inventory_row() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"NOSTOCK-1","name":"NoStock","price":{"minor_units":100,"currency":"USD"},"initial_stock":0}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert!(json["stock_qty"].is_null(), "zero stock → no inventory row");
    }

    #[tokio::test]
    async fn create_product_duplicate_sku_returns_409() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"DRINK-001","name":"Duplicate","price":{"minor_units":100,"currency":"USD"}}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_product_requires_auth() {
        let body = r#"{"sku":"NEW-001","name":"New","price":{"minor_units":100,"currency":"USD"}}"#;
        let req = post_json("/api/v1/products", body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn create_product_empty_sku_returns_400() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"   ","name":"Bad","price":{"minor_units":100,"currency":"USD"}}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_product_empty_name_returns_400() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"SKU-OK","name":"","price":{"minor_units":100,"currency":"USD"}}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_product_negative_price_returns_400() {
        let token = auth::create_token("test", Some(1));
        let body =
            r#"{"sku":"SKU-OK","name":"Bad Price","price":{"minor_units":-1,"currency":"USD"}}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_product_negative_initial_stock_returns_400() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"sku":"SKU-OK","name":"Bad Stock","price":{"minor_units":100,"currency":"USD"},"initial_stock":-5}"#;
        let req = auth_post_json("/api/v1/products", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Stock adjustment endpoint ───────────────────────────────

    fn auth_patch_json(uri: &str, token: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    #[tokio::test]
    async fn patch_stock_sell_reduces_qty() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"delta":-10}"#;
        let req = auth_patch_json("/api/v1/products/DRINK-001/stock", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["sku"], "DRINK-001");
        assert_eq!(json["previous_qty"], 50);
        assert_eq!(json["new_qty"], 40);
    }

    #[tokio::test]
    async fn patch_stock_restock_increases_qty() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"delta":25}"#;
        let req = auth_patch_json("/api/v1/products/DRINK-001/stock", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["previous_qty"], 50);
        assert_eq!(json["new_qty"], 75);
    }

    #[tokio::test]
    async fn patch_stock_oversell_returns_422() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"delta":-100}"#;
        let req = auth_patch_json("/api/v1/products/DRINK-001/stock", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn patch_stock_unknown_product_returns_404() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"delta":10}"#;
        let req = auth_patch_json("/api/v1/products/NOPE-999/stock", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn patch_stock_no_inventory_row_treats_as_zero() {
        let token = auth::create_token("test", Some(1));
        // DRINK-002 exists but has no inventory row.
        let body = r#"{"delta":30}"#;
        let req = auth_patch_json("/api/v1/products/DRINK-002/stock", &token.token, body);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["previous_qty"], 0);
        assert_eq!(json["new_qty"], 30);
    }

    #[tokio::test]
    async fn patch_stock_requires_auth() {
        let body = r#"{"delta":10}"#;
        let req = Request::builder()
            .method("PATCH")
            .uri("/api/v1/products/DRINK-001/stock")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Category endpoints ───────────────────────────────────────

    #[tokio::test]
    async fn categories_list_requires_auth() {
        let req = Request::builder()
            .uri("/api/v1/categories")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn categories_list_returns_empty_array() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/categories", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_array(), "should return a JSON array");
        assert_eq!(json.as_array().unwrap().len(), 0, "should be empty");
    }

    #[tokio::test]
    async fn categories_list_returns_seeded_categories() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/categories", &token.token);
        let resp = test_app_seeded().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2, "should return 2 seeded categories");
        assert_eq!(arr[0]["name"], "Drinks");
        assert_eq!(arr[0]["colour"], "#06b6d4");
        assert_eq!(arr[1]["name"], "Food");
        assert_eq!(arr[1]["colour"], "#f97316");
    }

    // ── Sale endpoints ───────────────────────────────────────────

    #[tokio::test]
    async fn create_sale_returns_201() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{
            "lines": [
                {"sku": "COFFEE", "qty": 2, "unit_price": {"minor_units": 350, "currency": "USD"}}
            ]
        }"#;
        let req = auth_post_json("/api/v1/sales", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "pending");
        assert_eq!(json["line_count"], 1);
        assert_eq!(json["total"]["minor_units"], 700);
        assert!(json["id"].is_string());
        assert!(json["lines"].is_array());
        assert_eq!(json["lines"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_sale_multi_line() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{
            "lines": [
                {"sku": "COFFEE", "qty": 2, "unit_price": {"minor_units": 350, "currency": "USD"}},
                {"sku": "BAGEL",  "qty": 1, "unit_price": {"minor_units": 450, "currency": "USD"}}
            ]
        }"#;
        let req = auth_post_json("/api/v1/sales", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["line_count"], 2);
        assert_eq!(json["total"]["minor_units"], 1150);
        let lines = json["lines"].as_array().unwrap();
        assert_eq!(lines[0]["line_position"], 1);
        assert_eq!(lines[1]["line_position"], 2);
        assert_eq!(lines[0]["sku"], "COFFEE");
        assert_eq!(lines[1]["sku"], "BAGEL");
    }

    #[tokio::test]
    async fn create_sale_empty_lines_rejected() {
        let token = auth::create_token("test", Some(1));
        let body = r#"{"lines": []}"#;
        let req = auth_post_json("/api/v1/sales", &token.token, body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_sale_requires_auth() {
        let body =
            r#"{"lines": [{"sku":"X","qty":1,"unit_price":{"minor_units":100,"currency":"USD"}}]}"#;
        let req = post_json("/api/v1/sales", body);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn get_sale_returns_detail() {
        let token = auth::create_token("test", Some(1));
        // Create a sale first.
        let create_body = r#"{
            "lines": [
                {"sku": "COFFEE", "qty": 2, "unit_price": {"minor_units": 350, "currency": "USD"}}
            ]
        }"#;
        let app = test_app();
        let create_req = auth_post_json("/api/v1/sales", &token.token, create_body);
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let create_json = body_json(create_resp).await;
        let sale_id = create_json["id"].as_str().unwrap().to_string();

        // Fetch the sale.
        let get_req = auth_get(&format!("/api/v1/sales/{sale_id}"), &token.token);
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
        let json = body_json(get_resp).await;
        assert_eq!(json["id"], sale_id);
        assert_eq!(json["status"], "pending");
        assert_eq!(json["line_count"], 1);
    }

    #[tokio::test]
    async fn get_sale_not_found_returns_null() {
        let token = auth::create_token("test", Some(1));
        let req = auth_get("/api/v1/sales/nonexistent-id", &token.token);
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_null());
    }

    #[tokio::test]
    async fn get_sale_requires_auth() {
        let req = Request::builder()
            .uri("/api/v1/sales/some-id")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn update_sale_status_pending_to_active() {
        let token = auth::create_token("test", Some(1));
        let app = test_app();

        // Create a sale.
        let create_body = r#"{
            "lines": [{"sku": "TEA", "qty": 1, "unit_price": {"minor_units": 200, "currency": "USD"}}]
        }"#;
        let create_req = auth_post_json("/api/v1/sales", &token.token, create_body);
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let sale_id = body_json(create_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Transition to active.
        let patch_body = r#"{"status": "active"}"#;
        let patch_req = auth_patch_json(
            &format!("/api/v1/sales/{sale_id}/status"),
            &token.token,
            patch_body,
        );
        let patch_resp = app.clone().oneshot(patch_req).await.unwrap();
        assert_eq!(patch_resp.status(), StatusCode::OK);
        let json = body_json(patch_resp).await;
        assert_eq!(json["status"], "active");
        assert!(json["updated_at"].is_string());
    }

    #[tokio::test]
    async fn update_sale_status_full_flow() {
        let token = auth::create_token("test", Some(1));
        let app = test_app();

        let create_body = r#"{
            "lines": [{"sku": "A", "qty": 1, "unit_price": {"minor_units": 100, "currency": "USD"}}]
        }"#;
        let create_req = auth_post_json("/api/v1/sales", &token.token, create_body);
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let sale_id = body_json(create_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Active -> Completed.
        let r1 = auth_patch_json(
            &format!("/api/v1/sales/{sale_id}/status"),
            &token.token,
            r#"{"status": "active"}"#,
        );
        let resp1 = app.clone().oneshot(r1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let r2 = auth_patch_json(
            &format!("/api/v1/sales/{sale_id}/status"),
            &token.token,
            r#"{"status": "completed"}"#,
        );
        let resp2 = app.clone().oneshot(r2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        assert_eq!(body_json(resp2).await["status"], "completed");
    }

    #[tokio::test]
    async fn update_sale_status_invalid_transition_returns_422() {
        let token = auth::create_token("test", Some(1));
        let app = test_app();

        let create_body = r#"{
            "lines": [{"sku": "B", "qty": 1, "unit_price": {"minor_units": 100, "currency": "USD"}}]
        }"#;
        let create_req = auth_post_json("/api/v1/sales", &token.token, create_body);
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let sale_id = body_json(create_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Pending -> Completed is invalid.
        let req = auth_patch_json(
            &format!("/api/v1/sales/{sale_id}/status"),
            &token.token,
            r#"{"status": "completed"}"#,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_sale_status_not_found_returns_404() {
        let token = auth::create_token("test", Some(1));
        let req = auth_patch_json(
            "/api/v1/sales/nope-999/status",
            &token.token,
            r#"{"status": "active"}"#,
        );
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_sale_status_requires_auth() {
        let req = Request::builder()
            .method("PATCH")
            .uri("/api/v1/sales/some-id/status")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"status": "active"}"#))
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_route_returns_401() {
        let req = Request::builder()
            .uri("/api/v1/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn root_returns_401() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cors_headers_present_on_health() {
        let req = Request::builder()
            .uri("/api/v1/health")
            .header("Origin", "http://example.com")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap());
        assert_eq!(allow_origin, Some("*"));
    }

    #[tokio::test]
    async fn cors_preflight_returns_ok() {
        let req = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/products")
            .header("Origin", "http://example.com")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        let resp = test_app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
