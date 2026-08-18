/*
last audited 19-07-26 by RSA-Agent
crate: cloud-server | status: SAFE | lint: CLEAN
findings: 4 unsafe blocks in #[cfg(test)] only — env::set_var/remove_var (Rust 2024 edition). SAFETY comments added 19-07-26.
next: none | perf: N/A
*/

//! Redirect middleware for zero-downtime VPS migration (ADR #11).
//!
//! When the `OZ_SYNC_REDIRECT_URL` environment variable is set, all
//! requests to `/api/sync/*` return a `server_migrated` response so
//! POS clients automatically update their local `sync_server_url` and
//! reconnect to the new server on the next sync cycle.
//!
//! The redirect URL is read once at startup and injected via axum
//! [`State`] — no per-request `std::env::var()` call.

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Middleware that intercepts sync requests when a migration redirect URL
/// is configured.
///
/// Returns HTTP 421 (Misdirected Request) with
/// `{"error":"server_migrated","new_url":"<url>"}` for all `/api/sync/*`
/// paths. The 421 status is intentionally chosen over 301/308 because
/// reqwest (and most HTTP clients) follow redirects automatically — 421
/// ensures the POS client's transport layer sees the response body directly
/// and calls `parse_server_migrated()` to update the local `sync_server_url`.
/// All other requests pass through unchanged.
pub async fn redirect_middleware(
    State(redirect_url): State<Option<String>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(ref new_url) = redirect_url
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

#[cfg(test)] #[path = "redirect_tests.rs"] mod tests;
