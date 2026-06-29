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
}
