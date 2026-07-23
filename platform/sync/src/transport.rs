//! Sync Transport — async HTTP client for communicating with the remote
//! sync server.
//!
//! The transport layer handles:
//!
//! - **Push** — sending pending offline queue items to the server
//! - **Pull** — fetching updates from the server since the last sync
//!
//! # Wire format
//!
//! All requests/responses use JSON. The server exposes two endpoints:
//!
//! - `POST /api/sync/push` — receives an array of queue items
//! - `POST /api/sync/pull` — receives a `since` timestamp, returns updates

use oz_core::offline::OfflineQueueItem;
use serde::{Deserialize, Serialize};

use crate::SyncError;

/// Outcome of pushing a single item to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PushOutcome {
    /// Item was accepted and applied by the server.
    Accepted,
    /// Item conflicted with the server version. The server's version is
    /// returned for local conflict resolution.
    Conflict(OfflineQueueItem),
    /// Item was rejected with a reason.
    Rejected {
        /// Human-readable reason for the rejection (e.g. "duplicate id").
        reason: String,
    },
}

/// Response from the push endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    /// Per-item outcomes in the same order as the push request.
    pub results: Vec<PushOutcome>,
}

/// Request body for the pull endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// ISO-8601 timestamp of the last successful sync. `None` for initial sync.
    pub since: Option<String>,
    /// Opaque cursor for paginated pulls (P-3). `None` for first page.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// Response from the pull endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResponse {
    /// Items that have changed on the server since the given timestamp.
    pub items: Vec<OfflineQueueItem>,
    /// Opaque cursor for the next page (P-3). `None` when no more pages.
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Response from the snapshot endpoint (P-3 Steps 3-5).
///
/// Contains the server's authoritative reference data for a tenant.
/// The client imports this wholesale when its sync anchor has expired
/// (data pruned server-side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshotResponse {
    /// Product rows keyed by SKU.
    pub products: Vec<serde_json::Value>,
    /// Tax-rate rows keyed by ID.
    pub tax_rates: Vec<serde_json::Value>,
    /// User rows keyed by username.
    pub users: Vec<serde_json::Value>,
}

/// Classifies a `reqwest::Error` into a human-readable transport error message
/// that distinguishes between connection failures, timeouts, DNS errors, etc.
///
/// This produces actionable diagnostics instead of the raw `reqwest` error string,
/// helping operators understand *why* a sync failed (server down vs network issue).
fn classify_transport_error(e: &reqwest::Error, url: &str) -> String {
    if e.is_timeout() {
        format!("request timed out after 30s to {url}")
    } else if e.is_connect() {
        let msg = e.to_string().to_lowercase();
        if msg.contains("connection refused") {
            format!("cloud server not running at {url} (connection refused)")
        } else {
            format!("cannot connect to {url}: {e}")
        }
    } else if e.is_request() {
        format!("request failed: {e}")
    } else {
        format!("transport error: {e}")
    }
}

/// The HTTP sync transport.
pub struct SyncTransport {
    client: reqwest::Client,
    base_url: String,
}

impl SyncTransport {
    /// Create a new transport targeting the given server URL.
    ///
    ///
    /// If the HTTP client cannot be built (e.g. TLS backend unavailable),
    /// falls back to a default client and logs an error. Previously this
    /// fallback was silent — now the error is logged so operators can
    /// detect the degraded state.
    pub fn new(server_url: &str, api_key: Option<&str>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = api_key
            && let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
        {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .gzip(true)
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(
                    error = %e,
                    "failed to build HTTP client for sync transport — falling back to default (no auth, no timeout)"
                );
                tracing::warn!(
                    "sync transport operating without timeout — requests may hang indefinitely"
                );
                reqwest::Client::new()
            });

        Self {
            client,
            base_url: server_url.trim_end_matches('/').to_owned(),
        }
    }

    /// Push pending items to the server.
    ///
    /// Returns a vector of outcomes, one per item in the same order.
    pub async fn push_items(
        &self,
        items: &[OfflineQueueItem],
    ) -> Result<Vec<PushOutcome>, SyncError> {
        let url = format!("{}/api/sync/push", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(items)
            .send()
            .await
            .map_err(|e| SyncError::Transport(classify_transport_error(&e, &url)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            // ADR #11: Detect server migration redirect.
            if let Some(new_url) = parse_server_migrated(&body) {
                return Err(SyncError::ServerMigrated { new_url });
            }

            return Err(SyncError::Transport(format!(
                "push returned {status}: {body}"
            )));
        }

        let push_resp: PushResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Transport(format!("push response parse failed: {e}")))?;

        Ok(push_resp.results)
    }

    /// Pull updates from the server since the given timestamp.
    ///
    /// Pass `None` to pull all available data (initial sync).
    /// Pull updates from the server since the given timestamp.
    ///
    /// Pass `None` for `since` to pull all available data (initial sync).
    /// Pass `cursor` for paginated subsequent pages (P-3).
    pub async fn pull_updates(
        &self,
        since: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<PullResponse, SyncError> {
        let url = format!("{}/api/sync/pull", self.base_url);
        let request = PullRequest {
            since: since.map(|s| s.to_owned()),
            cursor: cursor.map(|c| c.to_owned()),
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Transport(classify_transport_error(&e, &url)))?;

        // P-1 retention: 410 Gone means the client's anchor has expired
        // (data older than the `since` timestamp has been pruned).
        if resp.status() == reqwest::StatusCode::GONE {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let oldest_available = body
                .get("oldest_available")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            return Err(SyncError::AnchorExpired { oldest_available });
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            // ADR #11: Detect server migration redirect.
            if let Some(new_url) = parse_server_migrated(&body) {
                return Err(SyncError::ServerMigrated { new_url });
            }

            return Err(SyncError::Transport(format!(
                "pull returned {status}: {body}"
            )));
        }

        let pull_resp: PullResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Transport(format!("pull response parse failed: {e}")))?;

        Ok(pull_resp)
    }

    /// Check whether the cloud server is reachable by calling `GET /api/health`.
    ///
    /// Returns `Ok(())` when the server responds with a 2xx status.
    /// Returns `Err` with a classified transport error otherwise.
    ///
    /// Uses a short 5-second timeout (separate from the 30-second sync timeout)
    /// so that health checks don't block the daemon when the server is down.
    pub async fn health_check(&self) -> Result<(), SyncError> {
        let url = format!("{}/api/health", self.base_url);
        // Use a short-lived client with a 5-second timeout for health checks.
        // This prevents the daemon from stalling for 30 seconds on every cycle
        // when the server is unreachable.
        let health_client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        let resp = health_client
            .get(&url)
            .send()
            .await
            .map_err(|e| SyncError::Transport(classify_transport_error(&e, &url)))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(SyncError::Transport(format!(
                "health check returned {status}: {body}"
            )))
        }
    }

    /// Fetch the server's authoritative snapshot of reference data (P-3).
    ///
    /// Called when the client's sync anchor has expired — the server's
    /// delta log has been pruned beyond the client's last sync point.
    /// The snapshot provides a fresh baseline from which delta pulls resume.
    pub async fn fetch_snapshot(&self) -> Result<SyncSnapshotResponse, SyncError> {
        let url = format!("{}/api/sync/snapshot", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SyncError::Transport(classify_transport_error(&e, &url)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            // ADR #11: Detect server migration redirect.
            if let Some(new_url) = parse_server_migrated(&body) {
                return Err(SyncError::ServerMigrated { new_url });
            }

            return Err(SyncError::Transport(format!(
                "snapshot returned {status}: {body}"
            )));
        }

        let snapshot: SyncSnapshotResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Transport(format!("snapshot parse failed: {e}")))?;

        Ok(snapshot)
    }
}

/// Parse a `server_migrated` redirect from a JSON response body (ADR #11).
///
/// Returns `Some(new_url)` if the body contains `{"error":"server_migrated","new_url":"..."}`,
/// or `None` otherwise.
fn parse_server_migrated(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    if v.get("error")?.as_str()? == "server_migrated" {
        v.get("new_url")?.as_str().map(|s| s.to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_construction() {
        let transport = SyncTransport::new("http://localhost:3099", None);
        assert_eq!(transport.base_url, "http://localhost:3099");
    }

    #[test]
    fn transport_strips_trailing_slash() {
        let transport = SyncTransport::new("http://localhost:3099/", None);
        assert_eq!(transport.base_url, "http://localhost:3099");
    }

    #[test]
    fn transport_with_api_key() {
        let transport = SyncTransport::new("http://localhost:3099", Some("sk-test"));
        assert_eq!(transport.base_url, "http://localhost:3099");
    }

    // ── parse_server_migrated (ADR #11) ─────────────────────────────

    #[test]
    fn parse_server_migrated_detects_redirect() {
        let body = r#"{"error":"server_migrated","new_url":"https://new.example.com"}"#;
        assert_eq!(
            super::parse_server_migrated(body),
            Some("https://new.example.com".into())
        );
    }

    #[test]
    fn parse_server_migrated_ignores_other_errors() {
        assert_eq!(super::parse_server_migrated(r#"{"error":"timeout"}"#), None);
        assert_eq!(super::parse_server_migrated(r#"{"status":"ok"}"#), None);
        assert_eq!(super::parse_server_migrated("not json"), None);
    }

    #[test]
    fn parse_server_migrated_requires_new_url() {
        // Missing new_url field — should return None.
        assert_eq!(
            super::parse_server_migrated(r#"{"error":"server_migrated"}"#),
            None
        );
    }

    #[test]
    fn parse_server_migrated_empty_string() {
        assert_eq!(super::parse_server_migrated(""), None);
    }

    #[test]
    fn parse_server_migrated_null_new_url() {
        // new_url is present but null — should return None.
        assert_eq!(
            super::parse_server_migrated(r#"{"error":"server_migrated","new_url":null}"#),
            None
        );
    }

    #[test]
    fn parse_server_migrated_extra_fields_ok() {
        // Extra fields should not interfere with detection.
        let body = r#"{"error":"server_migrated","new_url":"https://x.com","extra":true}"#;
        assert_eq!(
            super::parse_server_migrated(body),
            Some("https://x.com".into())
        );
    }

    // ── PushOutcome serde + Debug ────────────────────────────────────

    #[test]
    fn push_outcome_accepted_debug() {
        let outcome = PushOutcome::Accepted;
        let debug = format!("{outcome:?}");
        assert!(debug.contains("Accepted"));
    }

    #[test]
    fn push_outcome_accepted_json() {
        let json = serde_json::to_value(PushOutcome::Accepted).unwrap();
        assert_eq!(json["outcome"], "accepted");
    }

    #[test]
    fn push_outcome_rejected_debug_and_json() {
        let outcome = PushOutcome::Rejected {
            reason: "duplicate id".into(),
        };
        let debug = format!("{outcome:?}");
        assert!(debug.contains("Rejected"));
        assert!(debug.contains("duplicate id"));

        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["outcome"], "rejected");
        assert_eq!(json["reason"], "duplicate id");
    }

    #[test]
    fn push_outcome_conflict_roundtrip() {
        let item = OfflineQueueItem::new("void_sale", "{}");
        let outcome = PushOutcome::Conflict(item.clone());
        let json = serde_json::to_string(&outcome).unwrap();
        let rt: PushOutcome = serde_json::from_str(&json).unwrap();
        match rt {
            PushOutcome::Conflict(rt_item) => {
                assert_eq!(rt_item.id, item.id);
                assert_eq!(rt_item.action, item.action);
            }
            _ => panic!("expected Conflict variant"),
        }
    }

    #[test]
    fn push_outcome_all_variants_serde_roundtrip() {
        let outcomes = vec![
            PushOutcome::Accepted,
            PushOutcome::Rejected {
                reason: "test".into(),
            },
            PushOutcome::Conflict(OfflineQueueItem::new("void", "{}")),
        ];
        for outcome in &outcomes {
            let json = serde_json::to_string(outcome).unwrap();
            let rt: PushOutcome = serde_json::from_str(&json).unwrap();
            let rt_json = serde_json::to_string(&rt).unwrap();
            assert_eq!(json, rt_json);
        }
    }

    // ── PushResponse tests ───────────────────────────────────────────

    #[test]
    fn push_response_debug() {
        let resp = PushResponse { results: vec![] };
        let debug = format!("{resp:?}");
        assert!(debug.contains("results"));
    }

    #[test]
    fn push_response_json_field_names() {
        let resp = PushResponse { results: vec![] };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.as_object().unwrap().contains_key("results"));
    }

    #[test]
    fn push_response_serde_roundtrip() {
        let resp = PushResponse {
            results: vec![
                PushOutcome::Accepted,
                PushOutcome::Rejected {
                    reason: "dup".into(),
                },
            ],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let rt: PushResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.results.len(), 2);
    }

    // ── PullRequest tests ────────────────────────────────────────────

    #[test]
    fn pull_request_debug() {
        let req = PullRequest {
            since: None,
            cursor: None,
        };
        let debug = format!("{req:?}");
        assert!(debug.contains("since"));
    }

    #[test]
    fn pull_request_json_some_since() {
        let req = PullRequest {
            since: Some("2026-01-01T00:00:00Z".into()),
            cursor: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["since"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn pull_request_json_none_since() {
        let req = PullRequest {
            since: None,
            cursor: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json["since"].is_null());
    }

    #[test]
    fn pull_request_serde_roundtrip() {
        let req = PullRequest {
            since: Some("2026-01-01T00:00:00Z".into()),
            cursor: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let rt: PullRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.since, Some("2026-01-01T00:00:00Z".into()));
    }

    // ── PullResponse tests ───────────────────────────────────────────

    #[test]
    fn pull_response_debug() {
        let resp = PullResponse {
            items: vec![],
            next_cursor: None,
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("items"));
    }

    #[test]
    fn pull_response_json_field_names() {
        let resp = PullResponse {
            items: vec![],
            next_cursor: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.as_object().unwrap().contains_key("items"));
    }

    #[test]
    fn pull_response_serde_roundtrip() {
        let item = OfflineQueueItem::new("complete_sale", "{}");
        let resp = PullResponse {
            items: vec![item.clone()],
            next_cursor: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let rt: PullResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.items.len(), 1);
        assert_eq!(rt.items[0].id, item.id);
    }

    // ── Clone tests ──────────────────────────────────────────────────

    #[test]
    fn push_outcome_clone() {
        let outcome = PushOutcome::Rejected {
            reason: "test".into(),
        };
        let cloned = outcome.clone();
        let json1 = serde_json::to_string(&outcome).unwrap();
        let json2 = serde_json::to_string(&cloned).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn pull_request_clone() {
        let req = PullRequest {
            since: Some("2026-01-01".into()),
            cursor: None,
        };
        let cloned = req.clone();
        assert_eq!(cloned.since, req.since);
    }

    // ── ADR #11: Transport integration tests ──────────────────

    use crate::test_helpers::spawn_redirect_server;

    #[tokio::test]
    async fn push_items_returns_server_migrated_on_redirect() {
        let new_url = "https://migrated.example.com";
        let server_url = spawn_redirect_server(new_url).await;
        let transport = SyncTransport::new(&server_url, None);

        let item = OfflineQueueItem::new("test_action", r#"{"key":"val"}"#);
        let result = transport.push_items(&[item]).await;

        match result {
            Err(SyncError::ServerMigrated { new_url: url }) => {
                assert_eq!(url, new_url, "ServerMigrated should carry the new_url");
            }
            other => panic!("expected SyncError::ServerMigrated, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn pull_updates_returns_server_migrated_on_redirect() {
        let new_url = "https://pull-migrated.example.com";
        let server_url = spawn_redirect_server(new_url).await;
        let transport = SyncTransport::new(&server_url, None);

        let result = transport.pull_updates(None, None).await;

        match result {
            Err(SyncError::ServerMigrated { new_url: url }) => {
                assert_eq!(url, new_url, "ServerMigrated should carry the new_url");
            }
            other => panic!("expected SyncError::ServerMigrated, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn fetch_snapshot_returns_server_migrated_on_redirect() {
        let new_url = "https://snapshot-migrated.example.com";
        let server_url = spawn_redirect_server(new_url).await;
        let transport = SyncTransport::new(&server_url, None);

        let result = transport.fetch_snapshot().await;

        match result {
            Err(SyncError::ServerMigrated { new_url: url }) => {
                assert_eq!(url, new_url, "ServerMigrated should carry the new_url");
            }
            other => panic!("expected SyncError::ServerMigrated, got {:?}", other),
        }
    }

    // ── SyncSnapshotResponse tests ──────────────────────────────

    #[test]
    fn sync_snapshot_response_debug() {
        let resp = SyncSnapshotResponse {
            products: vec![],
            tax_rates: vec![],
            users: vec![],
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("products"));
        assert!(debug.contains("tax_rates"));
        assert!(debug.contains("users"));
    }

    #[test]
    fn sync_snapshot_response_serde_roundtrip() {
        let resp = SyncSnapshotResponse {
            products: vec![serde_json::json!({"sku": "ITEM-1"})],
            tax_rates: vec![serde_json::json!({"id": 1, "rate": 10})],
            users: vec![serde_json::json!({"username": "admin"})],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let rt: SyncSnapshotResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.products.len(), 1);
        assert_eq!(rt.tax_rates.len(), 1);
        assert_eq!(rt.users.len(), 1);
    }

    #[test]
    fn sync_snapshot_response_clone() {
        let resp = SyncSnapshotResponse {
            products: vec![serde_json::json!({"sku": "ITEM-1"})],
            tax_rates: vec![],
            users: vec![],
        };
        let cloned = resp.clone();
        let json1 = serde_json::to_string(&resp).unwrap();
        let json2 = serde_json::to_string(&cloned).unwrap();
        assert_eq!(json1, json2);
    }

    // ── classify_transport_error tests ──────────────────────────────

    #[test]
    fn classify_transport_error_timeout() {
        // Simulate a timeout by creating a request that times out.
        // We test the classification logic by checking the message pattern.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(1))
                .build()
                .unwrap();
            client
                .get("http://127.0.0.1:1/timeout")
                .send()
                .await
                .unwrap_err()
        });
        let msg = super::classify_transport_error(&err, "http://example.com");
        assert!(
            msg.contains("timed out") || msg.contains("timeout"),
            "expected timeout message, got: {msg}"
        );
    }

    #[test]
    fn classify_transport_error_connection_refused() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            let client = reqwest::Client::new();
            client
                .get("http://127.0.0.1:1/refused")
                .send()
                .await
                .unwrap_err()
        });
        let msg = super::classify_transport_error(&err, "http://127.0.0.1:1");
        assert!(
            msg.contains("cloud server not running") || msg.contains("cannot connect"),
            "expected connection error message, got: {msg}"
        );
    }

    #[test]
    fn classify_transport_error_includes_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(1))
                .build()
                .unwrap();
            client
                .get("http://192.0.2.1:9999/test")
                .send()
                .await
                .unwrap_err()
        });
        let url = "http://192.0.2.1:9999";
        let msg = super::classify_transport_error(&err, url);
        // The error message should either contain the URL or describe the issue.
        assert!(!msg.is_empty(), "error message should not be empty");
        assert!(
            msg.contains(url)
                || msg.contains("timed out")
                || msg.contains("cannot connect")
                || msg.contains("cloud server not running"),
            "expected descriptive error message, got: {msg}"
        );
    }

    #[test]
    fn classify_transport_error_non_empty() {
        // All classification branches should produce non-empty messages.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(1))
                .build()
                .unwrap();
            client
                .get("http://127.0.0.1:1/test")
                .send()
                .await
                .unwrap_err()
        });
        let msg = super::classify_transport_error(&err, "http://test.example.com");
        assert!(!msg.is_empty(), "classification should produce a message");
    }

    // ── health_check integration test ───────────────────────────────

    #[tokio::test]
    async fn health_check_succeeds_with_healthy_server() {
        let server_url =
            crate::test_helpers::spawn_redirect_server("https://migrated.example.com").await;
        let transport = SyncTransport::new(&server_url, None);

        let result = transport.health_check().await;
        assert!(
            result.is_ok(),
            "health check should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn health_check_fails_when_server_returns_error() {
        use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};

        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn sick_health() -> impl IntoResponse {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status": "error"})),
            )
        }

        let app = Router::new().route("/api/health", get(sick_health));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let server_url = format!("http://localhost:{port}");
        let transport = SyncTransport::new(&server_url, None);

        let result = transport.health_check().await;
        assert!(result.is_err(), "health check should fail on 500");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("500") || err.contains("Internal Server Error"),
            "error should mention status code, got: {err}"
        );
    }
}
