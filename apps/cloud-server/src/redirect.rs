//! Redirect middleware for zero-downtime VPS migration (ADR #11).
//!
//! When the `OZ_SYNC_REDIRECT_URL` environment variable is set, all
//! requests to `/api/sync/*` return a `server_migrated` response so
//! POS clients automatically update their local `sync_server_url` and
//! reconnect to the new server on the next sync cycle.
//!
//! When the env var is absent, the middleware is a no-op (pass-through).

use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Middleware that intercepts sync requests when a migration redirect is
/// configured via the `OZ_SYNC_REDIRECT_URL` environment variable.
///
/// Returns HTTP 421 (Misdirected Request) with
/// `{"error":"server_migrated","new_url":"<url>"}` for all `/api/sync/*`
/// paths. The 421 status is intentionally chosen over 301/308 because
/// reqwest (and most HTTP clients) follow redirects automatically — 421
/// ensures the POS client's transport layer sees the response body directly
/// and calls `parse_server_migrated()` to update the local `sync_server_url`.
/// All other requests pass through unchanged.
pub async fn redirect_middleware(req: Request, next: Next) -> Response {
    if let Ok(new_url) = std::env::var("OZ_SYNC_REDIRECT_URL")
        && req.uri().path().starts_with("/api/sync/")
    {
        let body = serde_json::json!({
            "error": "server_migrated",
            "new_url": new_url,
        })
        .to_string();

        return Response::builder()
            .status(StatusCode::MISDIRECTED_REQUEST)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            });
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, middleware, routing::get};
    use http_body_util::BodyExt;
    use std::env;
    use tower::ServiceExt;

    /// Serialise env var tests — `std::env::set_var` / `remove_var` are
    /// process-global and tokio tests run concurrently. Uses tokio::sync::Mutex
    /// so the guard is `Send` and can be held safely across `.await` points.
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn dummy_handler() -> &'static str {
        "ok"
    }

    fn test_app() -> Router {
        Router::new()
            .route("/api/sync/push", get(dummy_handler))
            .route("/api/sync/pull", get(dummy_handler))
            .route("/health", get(dummy_handler))
            .layer(middleware::from_fn(redirect_middleware))
    }

    #[tokio::test]
    async fn redirect_when_env_var_set() {
        let _guard = ENV_LOCK.lock().await;
        unsafe { env::set_var("OZ_SYNC_REDIRECT_URL", "https://new-server.example.com") };

        let app = test_app();
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

        unsafe { env::remove_var("OZ_SYNC_REDIRECT_URL") };
    }

    #[tokio::test]
    async fn pass_through_when_env_var_not_set() {
        let _guard = ENV_LOCK.lock().await;
        unsafe { env::remove_var("OZ_SYNC_REDIRECT_URL") };

        let app = test_app();
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
        let _guard = ENV_LOCK.lock().await;
        unsafe { env::set_var("OZ_SYNC_REDIRECT_URL", "https://new.example.com") };

        let app = test_app();
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        unsafe { env::remove_var("OZ_SYNC_REDIRECT_URL") };
    }

    #[tokio::test]
    async fn redirect_includes_new_url_for_pull() {
        let _guard = ENV_LOCK.lock().await;
        unsafe { env::set_var("OZ_SYNC_REDIRECT_URL", "https://migrated.example.com") };

        let app = test_app();
        let req = Request::builder()
            .uri("/api/sync/pull")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::MISDIRECTED_REQUEST);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["new_url"], "https://migrated.example.com");

        unsafe { env::remove_var("OZ_SYNC_REDIRECT_URL") };
    }
}
