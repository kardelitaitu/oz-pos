use super::*;
use axum::{Router, body::Body, http::Request, middleware, routing::get};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn dummy_handler() -> &'static str {
    "ok"
}

fn test_app(redirect_url: Option<String>) -> Router {
    Router::new()
        .route("/api/sync/push", get(dummy_handler))
        .route("/api/sync/pull", get(dummy_handler))
        .route("/health", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            redirect_url,
            redirect_middleware,
        ))
}

#[tokio::test]
async fn redirect_when_url_configured() {
    let app = test_app(Some("https://new-server.example.com".into()));
    let req = Request::builder()
        .uri("/api/sync/push")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MISDIRECTED_REQUEST);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "server_migrated");
    assert_eq!(json["new_url"], "https://new-server.example.com");
}

#[tokio::test]
async fn pass_through_when_url_not_set() {
    let app = test_app(None);
    let req = Request::builder()
        .uri("/api/sync/push")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn non_sync_routes_pass_through() {
    let app = test_app(Some("https://new.example.com".into()));
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn redirect_includes_new_url_for_pull() {
    let app = test_app(Some("https://migrated.example.com".into()));
    let req = Request::builder()
        .uri("/api/sync/pull")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MISDIRECTED_REQUEST);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["new_url"], "https://migrated.example.com");
}
