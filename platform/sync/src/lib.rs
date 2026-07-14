#![warn(missing_docs)]

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
pub mod daemon;
pub mod pg_daemon;
pub mod pg_transport;
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
    /// Network or HTTP error communicating with the sync server.
    #[error("transport error: {0}")]
    Transport(String),

    /// Local queue operation failed (read/write/mark).
    #[error("queue error: {0}")]
    Queue(String),

    /// Replication logic error (push/pull cycle).
    #[error("replication error: {0}")]
    Replication(String),

    /// Conflict resolution failed.
    #[error("conflict error: {0}")]
    Conflict(String),

    /// Invalid or missing sync configuration.
    #[error("configuration error: {0}")]
    Config(String),

    /// The client's sync anchor (`since` timestamp) is older than the
    /// oldest retained row on the server. Data in that gap has been
    /// pruned (P-1 retention). The client should log a warning and
    /// retry on the next scheduled cycle.
    #[error("anchor expired: data older than {}", oldest_available.as_deref().unwrap_or("unknown"))]
    AnchorExpired {
        /// ISO-8601 timestamp of the oldest retained row on the server.
        oldest_available: Option<String>,
    },

    /// Database error from the underlying oz-core store.
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
    use oz_core::offline::OfflineQueueItem;
    use oz_core::sync_client::SyncConfig;

    // ── build_batches ────────────────────────────────────────────

    #[test]
    fn build_batches_empty() {
        let batches = build_batches(&[], MAX_BATCH_BYTES);
        assert!(batches.is_empty());
    }

    #[test]
    fn build_batches_single_item() {
        let items = vec![OfflineQueueItem::new("test", "{}")];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
    }

    #[test]
    fn build_batches_multiple_items_one_batch() {
        let items: Vec<_> = (0..5)
            .map(|i| OfflineQueueItem::new("test", &format!("{{\"n\":{i}}}")))
            .collect();
        // 5 tiny items should fit in one 64 KB batch.
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 5);
    }

    #[test]
    fn build_batches_respects_byte_limit() {
        // Create payloads that force splitting: each item serialises to
        // ~33 KB (payload + JSON envelope overhead). Two items exceed the
        // 64 KB budget, forcing a split after the first item.
        let big_payload = "x".repeat(33 * 1024);
        let small = "{}";
        let items = vec![
            OfflineQueueItem::new("a", &big_payload),
            OfflineQueueItem::new("b", &big_payload),
            OfflineQueueItem::new("c", small),
        ];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert!(
            batches.len() >= 2,
            "large items should cause splitting, got {} batches",
            batches.len()
        );
        // Each batch should have at least 1 item.
        for batch in &batches {
            assert!(!batch.is_empty(), "no empty batches allowed");
        }
    }

    #[test]
    fn build_batches_sorts_by_priority() {
        use oz_core::offline::SyncPriority;

        let critical = OfflineQueueItem::with_priority("a", "{}", SyncPriority::Critical);
        let normal = OfflineQueueItem::with_priority("b", "{}", SyncPriority::Normal);
        let low = OfflineQueueItem::with_priority("c", "{}", SyncPriority::Low);
        // Put them in reverse priority order to verify sorting.
        let items = vec![low.clone(), normal.clone(), critical.clone()];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        // All 3 small items should fit in one batch, but Critical must be first.
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch[0].priority, SyncPriority::Critical);
        assert_eq!(batch[1].priority, SyncPriority::Normal);
        assert_eq!(batch[2].priority, SyncPriority::Low);
    }

    #[test]
    fn build_batches_minimum_one_item_per_batch() {
        // An item larger than the byte limit still gets its own batch
        // (minimum 1 item per batch, no empty requests).
        let huge = "x".repeat(128 * 1024); // 128 KB payload
        let items = vec![OfflineQueueItem::new("huge", &huge)];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1, "single huge item still gets a batch");
        assert_eq!(batches[0].len(), 1);
    }

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
    /// Sync configuration (server URL, API key).
    pub config: SyncConfig,
    /// HTTP transport for communicating with the remote sync server.
    pub transport: SyncTransport,
}

/// Maximum bytes per batch (64 KB). P-1 retention spec §Batching.
pub const MAX_BATCH_BYTES: usize = 64 * 1024;

/// Split pending items into batches that each serialise to ≤ `max_bytes`
/// bytes of JSON. Ensures at least one item per batch (no empty requests).
///
/// Items are sorted by priority (P-2) before chunking: all Critical items
/// transmit before any Normal item, which transmit before Low items.
/// Within each priority tier, original arrival order is preserved.
pub fn build_batches(items: &[oz_core::offline::OfflineQueueItem], max_bytes: usize) -> Vec<Vec<oz_core::offline::OfflineQueueItem>> {
    // Sort by priority (Critical=0, Normal=1, Low=2) — stable sort
    // preserves arrival order within each tier.
    let mut sorted: Vec<oz_core::offline::OfflineQueueItem> = items.to_vec();
    sorted.sort_by_key(|item| item.priority);

    let mut batches: Vec<Vec<oz_core::offline::OfflineQueueItem>> = Vec::new();
    let mut current: Vec<oz_core::offline::OfflineQueueItem> = Vec::new();
    let mut current_bytes = 0usize;

    for item in &sorted {
        // Estimate the JSON size of this item alone.
        let item_bytes = serde_json::to_vec(item)
            .map(|v| v.len())
            .unwrap_or(0);

        // If adding this item would exceed the budget and we already have
        // items in the current batch, finalise and start a new batch.
        if !current.is_empty() && current_bytes + item_bytes > max_bytes {
            batches.push(std::mem::take(&mut current));
            current_bytes = 0;
        }

        current_bytes += item_bytes;
        current.push(item.clone());
    }

    // Don't drop the last partial batch.
    if !current.is_empty() {
        batches.push(current);
    }

    batches
}

impl SyncEngine {
    /// Create a new sync engine from the given configuration.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            transport: SyncTransport::new(&config.server_url, config.api_key.as_deref()),
            config,
        }
    }

    /// Run a full sync cycle: push pending items in batches, then pull remote updates.
    ///
    /// Items are split into ≤ 64 KB batches (P-1 batching) and sent sequentially.
    /// Each batch commits independently — a failure in batch N does not roll back
    /// the results of batches 1..N-1.
    ///
    /// Returns a [`ReplicationResult`] with counts of pushed/pulled items.
    pub async fn run_sync_cycle(&self, store: &Store<'_>) -> SyncResult<ReplicationResult> {
        let cycle_start = std::time::Instant::now();
        let queue = SyncQueue::new();

        // Phase 1: Push pending local changes in batches.
        let pending = queue.list_pending(store)?;
        let pending_count = pending.len();
        let mut total_pushed = 0usize;
        let mut total_bytes_sent = 0usize;
        let batch_count;

        if !pending.is_empty() {
            let batches = build_batches(&pending, MAX_BATCH_BYTES);
            batch_count = batches.len();
            for (batch_idx, batch) in batches.iter().enumerate() {
                let batch_items = batch.len();
                let batch_bytes = serde_json::to_vec(batch)
                    .map(|v| v.len())
                    .unwrap_or(0);
                total_bytes_sent += batch_bytes;

                tracing::debug!(
                    batch = batch_idx + 1,
                    total_batches = batch_count,
                    items = batch_items,
                    bytes = batch_bytes,
                    "pushing batch"
                );

                let results = self.transport.push_items(batch).await?;
                for (item, outcome) in batch.iter().zip(results.iter()) {
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
                total_pushed += results.len();
            }
        } else {
            batch_count = 0;
        }

        // Phase 2: Pull remote updates from the server.
        // P-3: Paginated pull — loop until next_cursor is null.
        let last_sync = queue.last_synced_at(store)?;
        let mut total_pulled = 0usize;
        let mut cursor: Option<String> = None;
        let mut pages = 0u32;

        loop {
            pages += 1;
            let pull_result = match self
                .transport
                .pull_updates(last_sync.as_deref(), cursor.as_deref())
                .await
            {
                Ok(result) => result,
                Err(SyncError::AnchorExpired { oldest_available }) => {
                    tracing::warn!(
                        oldest_available = oldest_available,
                        "sync anchor expired — data has been pruned on server; retrying next cycle"
                    );
                    return Ok(ReplicationResult {
                        pushed: total_pushed,
                        pulled: total_pulled,
                    });
                }
                Err(e) => return Err(e),
            };

            let page_count = pull_result.items.len();
            total_pulled += page_count;
            let has_more = pull_result.next_cursor.is_some();

            tracing::debug!(
                page = pages,
                items = page_count,
                has_more = has_more,
                "pulled page"
            );

            for remote_item in &pull_result.items {
                queue.apply_remote(store, remote_item)?;
            }

            cursor = pull_result.next_cursor;
            if !has_more {
                break;
            }
        }

        let elapsed_ms = cycle_start.elapsed().as_millis() as u64;

        tracing::info!(
            pending = pending_count,
            pushed = total_pushed,
            pulled = total_pulled,
            batches = batch_count,
            pages = pages,
            bytes_sent = total_bytes_sent,
            elapsed_ms = elapsed_ms,
            "sync cycle complete"
        );

        Ok(ReplicationResult {
            pushed: total_pushed,
            pulled: total_pulled,
        })
    }
}
