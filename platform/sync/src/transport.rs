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
    Rejected { reason: String },
}

/// Response from the push endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    pub results: Vec<PushOutcome>,
}

/// Request body for the pull endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// ISO-8601 timestamp of the last successful sync. `None` for initial sync.
    pub since: Option<String>,
}

/// Response from the pull endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResponse {
    /// Items that have changed on the server since the given timestamp.
    pub items: Vec<OfflineQueueItem>,
}

/// The HTTP sync transport.
pub struct SyncTransport {
    client: reqwest::Client,
    base_url: String,
}

impl SyncTransport {
    /// Create a new transport targeting the given server URL.
    pub fn new(server_url: &str, api_key: Option<&str>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = api_key
            && let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
        {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

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
            .map_err(|e| SyncError::Transport(format!("push request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
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
    pub async fn pull_updates(&self, since: Option<&str>) -> Result<PullResponse, SyncError> {
        let url = format!("{}/api/sync/pull", self.base_url);
        let request = PullRequest {
            since: since.map(|s| s.to_owned()),
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Transport(format!("pull request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
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
        // API key is used to set the Authorization header during construction.
        // We verify it doesn't crash and the transport works.
        let transport = SyncTransport::new("http://localhost:3099", Some("sk-test"));
        assert_eq!(transport.base_url, "http://localhost:3099");
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
        let req = PullRequest { since: None };
        let debug = format!("{req:?}");
        assert!(debug.contains("since"));
    }

    #[test]
    fn pull_request_json_some_since() {
        let req = PullRequest {
            since: Some("2026-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["since"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn pull_request_json_none_since() {
        let req = PullRequest { since: None };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json["since"].is_null());
    }

    #[test]
    fn pull_request_serde_roundtrip() {
        let req = PullRequest {
            since: Some("2026-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let rt: PullRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.since, Some("2026-01-01T00:00:00Z".into()));
    }

    // ── PullResponse tests ───────────────────────────────────────────

    #[test]
    fn pull_response_debug() {
        let resp = PullResponse { items: vec![] };
        let debug = format!("{resp:?}");
        assert!(debug.contains("items"));
    }

    #[test]
    fn pull_response_json_field_names() {
        let resp = PullResponse { items: vec![] };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.as_object().unwrap().contains_key("items"));
    }

    #[test]
    fn pull_response_serde_roundtrip() {
        let item = OfflineQueueItem::new("complete_sale", "{}");
        let resp = PullResponse {
            items: vec![item.clone()],
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
        };
        let cloned = req.clone();
        assert_eq!(cloned.since, req.since);
    }
}
