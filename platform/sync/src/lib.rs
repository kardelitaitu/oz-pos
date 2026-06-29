//! OZ-POS Sync Engine
//!
//! Offline-first sync with eventual consistency. Provides:
//!
//! - **Queue** — local change log backed by the `offline_queue` SQLite table
//! - **Transport** — async HTTP client for communicating with a remote sync server
//! - **Replication** — push pending changes / pull remote updates orchestration
//! - **Conflict** — last-write-wins (LWW) conflict resolution
//!
//! # Usage
//!
//! ```ignore
//! use platform_sync::{SyncEngine, SyncConfig};
//!
//! let engine = SyncEngine::new(config);
//! let result = engine.run_sync_cycle(&store).await?;
//! ```

#![allow(clippy::items_after_test_module)]

pub mod conflict;
pub mod queue;
pub mod replication;
pub mod transport;

use oz_core::db::Store;
use oz_core::sync_client::SyncConfig;

use crate::queue::SyncQueue;
use crate::replication::ReplicationResult;
use crate::transport::SyncTransport;

/// Convenience result type for sync operations.
pub type SyncResult<T> = Result<T, SyncError>;

/// Common sync error type.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("transport error: {0}")]
    Transport(String),

    #[error("queue error: {0}")]
    Queue(String),

    #[error("replication error: {0}")]
    Replication(String),

    #[error("conflict error: {0}")]
    Conflict(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("database error: {0}")]
    Database(#[from] oz_core::error::CoreError),
}

impl From<reqwest::Error> for SyncError {
    fn from(e: reqwest::Error) -> Self {
        SyncError::Transport(e.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_literal_unwrap)]
mod tests {
    use super::*;
    use oz_core::sync_client::SyncConfig;

    // ── SyncError ────────────────────────────────────────────────

    #[test]
    fn sync_error_transport_display() {
        let err = SyncError::Transport("connection timeout".into());
        assert_eq!(err.to_string(), "transport error: connection timeout");
    }

    #[test]
    fn sync_error_queue_display() {
        let err = SyncError::Queue("item not found".into());
        assert_eq!(err.to_string(), "queue error: item not found");
    }

    #[test]
    fn sync_error_replication_display() {
        let err = SyncError::Replication("push failed".into());
        assert_eq!(err.to_string(), "replication error: push failed");
    }

    #[test]
    fn sync_error_conflict_display() {
        let err = SyncError::Conflict("version mismatch".into());
        assert_eq!(err.to_string(), "conflict error: version mismatch");
    }

    #[test]
    fn sync_error_config_display() {
        let err = SyncError::Config("missing server URL".into());
        assert_eq!(err.to_string(), "configuration error: missing server URL");
    }

    #[test]
    fn sync_error_database_display() {
        let err = SyncError::Database(oz_core::CoreError::NotFound {
            entity: "item",
            id: "x".into(),
        });
        let msg = err.to_string();
        assert!(
            msg.contains("database error"),
            "expected database error, got: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "expected 'not found' in message, got: {msg}"
        );
    }

    #[test]
    fn sync_error_debug() {
        let err = SyncError::Transport("e".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn sync_error_from_requwest_error() {
        // Verify the From<reqwest::Error> impl compiles by checking the
        // conversion function signature at compile time.
        fn assert_convert(_e: reqwest::Error) -> SyncError {
            SyncError::from(_e)
        }
        let _ = assert_convert;
    }

    // ── SyncEngine ───────────────────────────────────────────────

    #[test]
    fn sync_engine_new_creates_transport() {
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let engine = SyncEngine::new(config);
        assert_eq!(engine.config.server_url, "http://localhost:3099");
    }

    #[test]
    fn sync_engine_new_with_api_key() {
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: Some("sk-key".into()),
        };
        let engine = SyncEngine::new(config);
        assert_eq!(engine.config.api_key, Some("sk-key".into()));
    }

    // ── SyncResult ───────────────────────────────────────────────

    #[test]
    fn sync_result_ok() {
        let result: SyncResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn sync_result_err() {
        let result: SyncResult<i32> = Err(SyncError::Config("bad config".into()));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "configuration error: bad config"
        );
    }
}

/// The top-level sync engine that orchestrates queue, transport, replication,
/// and conflict resolution for a single sync cycle.
pub struct SyncEngine {
    pub config: SyncConfig,
    pub transport: SyncTransport,
}

impl SyncEngine {
    /// Create a new sync engine from the given configuration.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            transport: SyncTransport::new(&config.server_url, config.api_key.as_deref()),
            config,
        }
    }

    /// Run a full sync cycle: push pending items, then pull remote updates.
    ///
    /// Returns a [`ReplicationResult`] with counts of pushed/pulled items.
    pub async fn run_sync_cycle(&self, store: &Store<'_>) -> SyncResult<ReplicationResult> {
        let queue = SyncQueue::new();

        // Phase 1: Push pending local changes to the server.
        let pending = queue.list_pending(store)?;
        let push_result = if pending.is_empty() {
            ReplicationResult::default()
        } else {
            let results = self.transport.push_items(&pending).await?;
            for (item, outcome) in pending.iter().zip(results.iter()) {
                match outcome {
                    transport::PushOutcome::Accepted => {
                        queue.mark_synced(store, &item.id)?;
                    }
                    transport::PushOutcome::Conflict(server_item) => {
                        let resolved = conflict::resolve_lww(item, server_item);
                        queue.apply_resolution(store, &resolved)?;
                    }
                    transport::PushOutcome::Rejected { reason } => {
                        queue.mark_failed(store, &item.id, reason)?;
                    }
                }
            }
            ReplicationResult {
                pushed: results.len(),
                ..Default::default()
            }
        };

        // Phase 2: Pull remote updates from the server.
        let last_sync = queue.last_synced_at(store)?;
        let pull_result = self.transport.pull_updates(last_sync.as_deref()).await?;

        for remote_item in &pull_result.items {
            queue.apply_remote(store, remote_item)?;
        }

        tracing::info!(
            pushed = push_result.pushed,
            pulled = pull_result.items.len(),
            "sync cycle complete"
        );

        Ok(ReplicationResult {
            pushed: push_result.pushed,
            pulled: pull_result.items.len(),
        })
    }
}
