//! Sync Queue — local change log for offline-first replication.
//!
//! Wraps the `oz_core` offline queue Store methods into a clean interface
//! with additional tracking for conflict resolution and last-sync timing.

use oz_core::db::Store;
use oz_core::db::offline::SyncStatusSummary;
use oz_core::error::CoreError;
use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct SalePayload {
    #[serde(default)]
    line_items: Vec<SaleLinePayload>,
}

#[derive(Deserialize)]
struct SaleLinePayload {
    sku: String,
    #[serde(default)]
    qty: i64,
}

#[derive(Deserialize)]
struct StockAdjustmentPayload {
    sku: String,
    delta: i64,
}

/// Payload for the `stock.movement` sync action (ADR #6 cross-store routing).
/// Carries a full `StockMovement` row for insertion into the local ledger.
#[derive(Deserialize)]
struct StockMovementPayload {
    id: String,
    item_id: String,
    delta: i64,
    reason: Option<String>,
    source_terminal_id: Option<String>,
    source_user_id: Option<String>,
    store_id: String,
    created_at: String,
}

/// A resolved item after conflict resolution — may be accepted from either
/// the local or remote side, or a merged version.
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    /// The original local item, if applicable.
    pub local: Option<OfflineQueueItem>,
    /// The original remote item, if applicable.
    pub remote: Option<OfflineQueueItem>,
    /// The winning item to persist.
    pub winner: OfflineQueueItem,
}

/// Wraps the offline queue database operations with sync-specific helpers.
pub struct SyncQueue;

impl SyncQueue {
    /// Create a new sync queue interface.
    pub fn new() -> Self {
        Self
    }

    /// List all pending (unsynced) items, oldest first.
    pub fn list_pending(&self, store: &Store<'_>) -> Result<Vec<OfflineQueueItem>, CoreError> {
        store.list_pending_offline()
    }

    /// List all items (most recent first).
    pub fn list_all(&self, store: &Store<'_>) -> Result<Vec<OfflineQueueItem>, CoreError> {
        store.list_all_offline()
    }

    /// Enqueue a new offline transaction.
    pub fn enqueue(
        &self,
        store: &Store<'_>,
        action: &str,
        payload: &str,
    ) -> Result<OfflineQueueItem, CoreError> {
        store.enqueue_offline(action, payload)
    }

    /// Enqueue a transaction with dedup by action + payload.
    ///
    /// If a pending item with the same `action` and `payload` already
    /// exists, returns `Ok(None)` — no duplicate is created.
    /// This prevents duplicate entries when the same event is enqueued
    /// multiple times across different terminals or due to retry logic.
    pub fn enqueue_dedup(
        &self,
        store: &Store<'_>,
        action: &str,
        payload: &str,
    ) -> Result<Option<OfflineQueueItem>, CoreError> {
        store.enqueue_offline_dedup(action, payload)
    }

    /// Mark an item as successfully synced.
    pub fn mark_synced(&self, store: &Store<'_>, id: &str) -> Result<(), CoreError> {
        store.mark_offline_synced(id)
    }

    /// Mark an item as failed with an error message.
    pub fn mark_failed(&self, store: &Store<'_>, id: &str, error: &str) -> Result<(), CoreError> {
        store.mark_offline_failed(id, error)
    }

    /// Get the count of pending items.
    pub fn pending_count(&self, store: &Store<'_>) -> Result<i64, CoreError> {
        store.pending_offline_count()
    }

    /// Delete an item from the queue.
    pub fn delete(&self, store: &Store<'_>, id: &str) -> Result<(), CoreError> {
        store.delete_offline_item(id)
    }

    /// Get a summary of the offline queue status.
    ///
    /// Returns counts by status, total retries, last sync timestamp,
    /// and oldest pending timestamp — for dashboard observability.
    pub fn status_summary(&self, store: &Store<'_>) -> Result<SyncStatusSummary, CoreError> {
        store.offline_queue_status_summary()
    }

    /// Get the timestamp of the most recently synced item.
    ///
    /// Returns `None` if nothing has been synced yet.
    pub fn last_synced_at(&self, store: &Store<'_>) -> Result<Option<String>, CoreError> {
        let all = store.list_all_offline()?;
        Ok(all
            .iter()
            .filter(|i| matches!(i.status, OfflineQueueStatus::Synced))
            .filter_map(|i| i.synced_at.as_deref())
            .max_by(|a, b| a.cmp(b))
            .map(|s| s.to_owned()))
    }

    /// Apply a conflict-resolution outcome to the queue.
    ///
    /// Marks the local item with a conflict-resolution marker in its
    /// `last_error` field so the status summary can count it. If the
    /// winner is a merged (CRDT) item, a new queue entry is created.
    pub fn apply_resolution(
        &self,
        store: &Store<'_>,
        resolved: &ResolvedItem,
    ) -> Result<(), CoreError> {
        // Determine the resolution type from the winner identity.
        let resolution_tag = match (&resolved.local, &resolved.remote) {
            (Some(local), _) if resolved.winner.id == local.id => "local won",
            (_, Some(remote)) if resolved.winner.id == remote.id => "remote won",
            _ => "crdt merge",
        };
        // Mark the local item with a conflict marker and sync it.
        if let Some(ref local) = resolved.local {
            store.mark_offline_resolved(&local.id, resolution_tag)?;
        }
        // If the winner is a merged item (neither purely local nor remote),
        // enqueue it for the next sync cycle.
        let is_new_winner = match (&resolved.local, &resolved.remote) {
            (Some(local), _) if resolved.winner.id == local.id => false,
            (_, Some(remote)) if resolved.winner.id == remote.id => false,
            _ => true,
        };
        if is_new_winner {
            store.enqueue_offline(&resolved.winner.action, &resolved.winner.payload)?;
        }
        Ok(())
    }

    /// Apply a push-conflict outcome using the shared ADR #21 resolver.
    ///
    /// **This is the single conflict-application service** used by both the
    /// immediate [`SyncEngine`](crate::SyncEngine) and the background
    /// [`SyncDaemon`](crate::daemon::SyncDaemon), so the same ADR #21
    /// strategy (version LWW / sale status DAG / stock CRDT merge) applies
    /// regardless of which trigger processes the conflict (SYNC-02).
    ///
    /// Resolves the conflict, persists the resolution (marking the local
    /// item resolved with an auditable tag), and re-enqueues the merged
    /// winner when the resolver produced a CRDT merge — whose payload is
    /// now consumable by [`SyncQueue::apply_remote`] (SYNC-05).
    pub fn apply_push_conflict(
        &self,
        store: &Store<'_>,
        local: &OfflineQueueItem,
        server_item: &OfflineQueueItem,
    ) -> Result<(), CoreError> {
        let resolved = crate::conflict::resolve_conflict(local, server_item);
        self.apply_resolution(store, &resolved)
    }

    /// Apply a remote item to the local store.
    ///
    /// Parses the `action` field and dispatches to the appropriate local
    /// mutation (stock deduction for sales, stock adjustment, etc.).
    #[allow(deprecated)]
    pub fn apply_remote(
        &self,
        store: &Store<'_>,
        item: &OfflineQueueItem,
    ) -> Result<(), CoreError> {
        match item.action.as_str() {
            // A sale completed on another terminal — deduct stock.
            "complete_sale" => {
                let payload: SalePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid sale payload: {e}")))?;
                for line in &payload.line_items {
                    store.adjust_stock(&line.sku, -line.qty)?;
                }
                Ok(())
            }
            // Stock adjustment from another terminal. Supports BOTH a flat
            // `{sku, delta}` payload AND the SYNC-05 CRDT merge envelope
            // (`{local, remote, merge_type: "crdt_delta"}`) produced by
            // `resolve_stock_crdt` — both deltas are valid CRDT facts and
            // must be applied. NOTE: `adjust_stock` is NOT idempotent (it
            // appends a new stock_movements row), so re-applying a merged
            // winner must be prevented by the caller's replay ledger / queue
            // dedup (the daemon's sync_applied_items + mark-synced guards).
            "stock.adjusted" => {
                let payload: Value = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid stock payload: {e}")))?;
                if payload.get("merge_type").and_then(|m| m.as_str()) == Some("crdt_delta") {
                    for side in ["local", "remote"] {
                        let sub: StockAdjustmentPayload = serde_json::from_value(
                            payload.get(side).cloned().unwrap_or(Value::Null),
                        )
                        .map_err(|e| {
                            CoreError::Internal(format!("invalid crdt stock delta: {e}"))
                        })?;
                        store.adjust_stock(&sub.sku, sub.delta)?;
                    }
                } else {
                    let sub: StockAdjustmentPayload = serde_json::from_value(payload)
                        .map_err(|e| CoreError::Internal(format!("invalid stock payload: {e}")))?;
                    store.adjust_stock(&sub.sku, sub.delta)?;
                }
                Ok(())
            }
            // A new product created on another terminal — create locally.
            "product.created" => {
                let payload: serde_json::Value = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid product payload: {e}")))?;
                let sku = payload["sku"].as_str().unwrap_or("");
                let name = payload["name"].as_str().unwrap_or("Unknown");
                let price_minor = payload["price_minor"].as_i64().unwrap_or(0);
                let currency = payload["currency"].as_str().unwrap_or("USD");
                let currency_parsed: oz_core::Currency =
                    currency
                        .parse()
                        .map_err(|e: oz_core::money::InvalidCurrencyCode| {
                            CoreError::Internal(format!("invalid currency in sync payload: {e}"))
                        })?;
                if !sku.is_empty() && store.get_product(sku).ok().flatten().is_none() {
                    let price = oz_core::Money {
                        minor_units: price_minor,
                        currency: currency_parsed,
                    };
                    let category_id = payload["category_id"].as_str();
                    let barcode = payload["barcode"].as_str();
                    let initial_stock = payload["initial_stock"].as_i64().unwrap_or(0);
                    let product_type = payload["product_type"].as_str().unwrap_or("retail");
                    store.create_product(
                        sku,
                        name,
                        price,
                        category_id,
                        barcode,
                        initial_stock,
                        Some(product_type),
                    )?;
                }
                Ok(())
            }
            // ADR #6: Remote stock movement from another store or register.
            // Insert directly into the ledger; the daemon rebuilds the
            // stock_summary cache after applying all remote items. Also
            // accepts the SYNC-05 CRDT merge envelope (both rows inserted).
            "stock.movement" => {
                let payload: Value = serde_json::from_str(&item.payload).map_err(|e| {
                    CoreError::Internal(format!("invalid stock.movement payload: {e}"))
                })?;
                let apply_one = |value: &Value| -> Result<(), CoreError> {
                    let m: StockMovementPayload =
                        serde_json::from_value(value.clone()).map_err(|e| {
                            CoreError::Internal(format!("invalid stock.movement payload: {e}"))
                        })?;
                    store.insert_stock_movement(
                        &m.id,
                        &m.item_id,
                        m.delta,
                        m.reason.as_deref(),
                        m.source_terminal_id.as_deref(),
                        m.source_user_id.as_deref(),
                        &m.store_id,
                        &m.created_at,
                    )
                };
                if payload.get("merge_type").and_then(|m| m.as_str()) == Some("crdt_delta") {
                    apply_one(payload.get("local").unwrap_or(&Value::Null))?;
                    apply_one(payload.get("remote").unwrap_or(&Value::Null))?;
                } else {
                    apply_one(&payload)?;
                }
                Ok(())
            }
            // Unsupported action — log and skip.
            _ => {
                tracing::warn!(action = %item.action, "unsupported remote sync action");
                Ok(())
            }
        }
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn setup_store() -> Store<'static> {
        let conn: &'static Connection = Box::leak(Box::new(migrations::fresh_db()));
        Store::new(conn)
    }

    #[test]
    fn queue_empty_pending() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let pending = queue.list_pending(&store).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_enqueue_and_list() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue
            .enqueue(&store, "complete_sale", r#"{"sale_id":"s1"}"#)
            .unwrap();
        assert_eq!(item.action, "complete_sale");

        let pending = queue.list_pending(&store).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, item.id);
    }

    #[test]
    fn queue_mark_synced() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_synced(&store, &item.id).unwrap();

        let pending = queue.list_pending(&store).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_mark_failed() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue
            .mark_failed(&store, &item.id, "network error")
            .unwrap();

        let all = queue.list_all(&store).unwrap();
        assert_eq!(all[0].status, OfflineQueueStatus::Failed);
    }

    #[test]
    fn queue_last_synced_at_none() {
        let store = setup_store();
        let queue = SyncQueue::new();
        assert!(queue.last_synced_at(&store).unwrap().is_none());
    }

    #[test]
    fn queue_last_synced_at_after_sync() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_synced(&store, &item.id).unwrap();
        assert!(queue.last_synced_at(&store).unwrap().is_some());
    }

    #[test]
    fn queue_delete_removes_item() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.delete(&store, &item.id).unwrap();
        let all = queue.list_all(&store).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn queue_delete_nonexistent_does_not_error() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let result = queue.delete(&store, "nonexistent-id");
        assert!(result.is_ok());
    }

    #[test]
    fn queue_pending_count() {
        let store = setup_store();
        let queue = SyncQueue::new();
        assert_eq!(queue.pending_count(&store).unwrap(), 0);

        queue.enqueue(&store, "a", "{}").unwrap();
        queue.enqueue(&store, "b", "{}").unwrap();
        assert_eq!(queue.pending_count(&store).unwrap(), 2);

        // After marking one synced, count decreases.
        let pending = queue.list_pending(&store).unwrap();
        queue.mark_synced(&store, &pending[0].id).unwrap();
        assert_eq!(queue.pending_count(&store).unwrap(), 1);
    }

    #[test]
    fn queue_list_all_returns_all_statuses() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "a", "{}").unwrap();
        let _item2 = queue.enqueue(&store, "b", "{}").unwrap();

        queue.mark_synced(&store, &item1.id).unwrap();

        let all = queue.list_all(&store).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn queue_list_pending_returns_oldest_first() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "first", "{}").unwrap();
        let item2 = queue.enqueue(&store, "second", "{}").unwrap();

        let pending = queue.list_pending(&store).unwrap();
        assert_eq!(pending[0].id, item1.id, "oldest item should be first");
        assert_eq!(pending[1].id, item2.id);
    }

    // ── Dedup tests (P1-5) ────────────────────────────────────────────

    #[test]
    fn queue_enqueue_dedup_skips_duplicate() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let payload = r#"{"sale_id":"s-1"}"#;
        let first = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(first.is_some(), "first call should enqueue");

        let second = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(second.is_none(), "duplicate should be skipped");

        let count = queue.pending_count(&store).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn queue_enqueue_dedup_allows_different_payload() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let first = queue
            .enqueue_dedup(&store, "complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(first.is_some());

        let second = queue
            .enqueue_dedup(&store, "complete_sale", r#"{"sale_id":"s-2"}"#)
            .unwrap();
        assert!(second.is_some(), "different sale_id should not be deduped");

        let count = queue.pending_count(&store).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn queue_enqueue_dedup_allows_different_action() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let payload = r#"{"id":"x"}"#;
        let first = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(first.is_some());

        let second = queue.enqueue_dedup(&store, "void_sale", payload).unwrap();
        assert!(second.is_some(), "different action should not be deduped");

        let count = queue.pending_count(&store).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn queue_enqueue_dedup_cross_terminal_scenario() {
        // Simulate: Terminal A completes a sale and enqueues it.
        // That sale syncs to Terminal B, which also tries to enqueue
        // the exact same payload — the dedup should prevent duplicates.
        let store = setup_store();
        let queue = SyncQueue::new();

        let payload = r#"{"sale_id":"s-cross-1","items":[{"sku":"COFFEE","qty":2}]}"#;

        // Terminal A enqueues
        let a = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(a.is_some(), "Terminal A should enqueue");

        // Terminal B receives the same payload via sync and tries to enqueue
        let b = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(b.is_none(), "Terminal B duplicate should be deduped");

        // Verify only one pending item exists
        let count = queue.pending_count(&store).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn queue_enqueue_dedup_allows_after_mark_synced() {
        // After an item is synced, a new enqueue with the same payload
        // should not be deduped (only checks Pending items).
        let store = setup_store();
        let queue = SyncQueue::new();

        let payload = r#"{"sale_id":"s-1"}"#;
        let first = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(first.is_some());
        let id = first.unwrap().id.clone();

        queue.mark_synced(&store, &id).unwrap();

        let second = queue
            .enqueue_dedup(&store, "complete_sale", payload)
            .unwrap();
        assert!(
            second.is_some(),
            "should re-enqueue after original is synced"
        );
    }

    // ── P1-6: SyncStatusSummary tests ────────────────────────────

    #[test]
    fn queue_status_summary_empty() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let summary = queue.status_summary(&store).unwrap();
        assert_eq!(summary.pending_count, 0);
        assert_eq!(summary.synced_count, 0);
        assert_eq!(summary.failed_count, 0);
        assert!(summary.last_synced_at.is_none());
        assert!(summary.oldest_pending_at.is_none());
    }

    #[test]
    fn queue_status_summary_with_data() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let item1 = queue.enqueue(&store, "a", "{}").unwrap();
        queue.enqueue(&store, "b", "{}").unwrap();
        queue.mark_synced(&store, &item1.id).unwrap();

        let summary = queue.status_summary(&store).unwrap();
        assert_eq!(summary.pending_count, 1);
        assert_eq!(summary.synced_count, 1);
        assert_eq!(summary.failed_count, 0);
        assert!(summary.last_synced_at.is_some());
        assert!(summary.oldest_pending_at.is_some());
    }

    #[test]
    fn queue_status_summary_after_mark_failed() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_failed(&store, &item.id, "timeout").unwrap();

        let summary = queue.status_summary(&store).unwrap();
        assert_eq!(summary.pending_count, 0);
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.total_retry_count, 1);
    }

    #[test]
    fn queue_last_synced_at_multiple_items() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "a", "{}").unwrap();
        let item2 = queue.enqueue(&store, "b", "{}").unwrap();

        queue.mark_synced(&store, &item1.id).unwrap();
        let ts1 = queue.last_synced_at(&store).unwrap().unwrap();

        queue.mark_synced(&store, &item2.id).unwrap();
        let ts2 = queue.last_synced_at(&store).unwrap().unwrap();

        // The timestamp of the most recently synced item should be >= the earlier one.
        assert!(ts2 >= ts1, "last synced at should increase");
    }

    #[test]
    fn queue_apply_resolution_local_wins() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue.enqueue(&store, "test", "{}").unwrap();

        let remote = OfflineQueueItem {
            id: uuid::Uuid::now_v7().to_string(),
            action: "test".into(),
            payload: "{}".into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            synced_at: None,
            tenant_id: "default".into(),
            priority: oz_core::offline::SyncPriority::Normal,
        };

        let resolved = ResolvedItem {
            local: Some(local.clone()),
            remote: Some(remote),
            winner: local.clone(),
        };

        queue.apply_resolution(&store, &resolved).unwrap();

        // Local item should be marked synced.
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
    }

    #[test]
    fn queue_apply_resolution_remote_wins() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue.enqueue(&store, "test", "{}").unwrap();

        let remote = OfflineQueueItem {
            id: uuid::Uuid::now_v7().to_string(),
            action: "test".into(),
            payload: r#"{"from":"server"}"#.into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: "2025-06-01T12:00:00.000Z".into(),
            synced_at: None,
            tenant_id: "default".into(),
            priority: oz_core::offline::SyncPriority::Normal,
        };

        let resolved = ResolvedItem {
            local: Some(local.clone()),
            remote: Some(remote.clone()),
            winner: remote,
        };

        queue.apply_resolution(&store, &resolved).unwrap();

        // Local item should be marked synced. No new item enqueued because
        // the winner is the remote item (not a merge).
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
    }

    fn seed_product_and_inventory(store: &Store<'_>) {
        store.conn().execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('prod-coffee', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('prod-bagel', 'BAGEL', 'Bagel', 450, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty, updated_at) VALUES
                ('prod-coffee', 50, '2025-01-01T00:00:00.000Z'),
                ('prod-bagel', 30, '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    fn inventory_qty(store: &Store<'_>, sku: &str) -> i64 {
        let pid = store.product_id_by_sku(sku).unwrap().unwrap();
        store.get_stock(&pid).unwrap()
    }

    #[test]
    fn apply_remote_complete_sale_deducts_stock() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        let payload = r#"{"line_items":[{"sku":"COFFEE","qty":2},{"sku":"BAGEL","qty":1}]}"#;
        let remote = OfflineQueueItem::new("complete_sale", payload);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok(), "apply_remote should succeed");

        assert_eq!(
            inventory_qty(&store, "COFFEE"),
            48,
            "COFFEE should drop from 50 to 48"
        );
        assert_eq!(
            inventory_qty(&store, "BAGEL"),
            29,
            "BAGEL should drop from 30 to 29"
        );
    }

    #[test]
    fn apply_remote_stock_adjustment() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        // Add 10 units.
        let payload = r#"{"sku":"COFFEE","delta":10}"#;
        let remote = OfflineQueueItem::new("stock.adjusted", payload);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok());
        assert_eq!(
            inventory_qty(&store, "COFFEE"),
            60,
            "COFFEE should increase from 50 to 60"
        );

        // Remove 5 units.
        let payload = r#"{"sku":"BAGEL","delta":-5}"#;
        let remote = OfflineQueueItem::new("stock.adjusted", payload);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok());
        assert_eq!(
            inventory_qty(&store, "BAGEL"),
            25,
            "BAGEL should drop from 30 to 25"
        );
    }

    #[test]
    fn apply_remote_unknown_action_is_noop() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem::new("unknown.action", r#"{"data":"test"}"#);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok(), "unknown action should not error");
        let all = store.list_all_offline().unwrap();
        assert!(all.is_empty(), "no queue items should be created");
    }

    // ── stock.movement cross-store delta routing (ADR #6) ────────

    #[test]
    fn apply_remote_stock_movement_inserts_into_ledger() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        let payload = serde_json::json!({
            "id": "sm-remote-1",
            "item_id": "prod-coffee",
            "delta": 10,
            "reason": "cross-store-transfer",
            "source_terminal_id": "term-store-b",
            "source_user_id": "user-store-b",
            "store_id": "store-b",
            "created_at": "2026-01-15T00:00:00Z"
        })
        .to_string();

        let remote = OfflineQueueItem::new("stock.movement", &payload);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok(), "stock.movement should succeed");

        // Verify the movement was inserted into the ledger.
        let movements = store.list_stock_movements("prod-coffee", 10, 0).unwrap();
        let sm = movements.iter().find(|m| m.id == "sm-remote-1");
        assert!(sm.is_some(), "remote stock movement should be in ledger");
        let sm = sm.unwrap();
        assert_eq!(sm.delta, 10);
        assert_eq!(sm.store_id, "store-b");
        assert_eq!(sm.reason.as_deref(), Some("cross-store-transfer"));
        assert_eq!(sm.source_terminal_id.as_deref(), Some("term-store-b"));
    }

    #[test]
    fn apply_remote_stock_movement_negative_delta() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        let payload = serde_json::json!({
            "id": "sm-remote-2",
            "item_id": "prod-bagel",
            "delta": -5,
            "reason": null,
            "source_terminal_id": null,
            "source_user_id": null,
            "store_id": "store-a",
            "created_at": "2026-01-15T00:00:00Z"
        })
        .to_string();

        let remote = OfflineQueueItem::new("stock.movement", &payload);
        queue.apply_remote(&store, &remote).unwrap();

        let movements = store.list_stock_movements("prod-bagel", 10, 0).unwrap();
        let sm = movements.iter().find(|m| m.id == "sm-remote-2").unwrap();
        assert_eq!(sm.delta, -5);
        assert_eq!(sm.store_id, "store-a");
    }

    #[test]
    fn apply_remote_stock_movement_rebuilds_summary() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        // Insert movements from another store directly into the ledger.
        let payload = serde_json::json!({
            "id": "sm-cross-1",
            "item_id": "prod-coffee",
            "delta": 30,
            "reason": "transfer-in",
            "source_terminal_id": null,
            "source_user_id": null,
            "store_id": "store-b",
            "created_at": "2026-01-15T00:00:00Z"
        })
        .to_string();
        let remote = OfflineQueueItem::new("stock.movement", &payload);
        queue.apply_remote(&store, &remote).unwrap(); // Rebuild to verify the ledger-based computation.
        store.rebuild_stock_summary().unwrap();

        // Ledger SUM = just the cross-store delta (30) since the migration
        // backfill ran against empty inventory (pre-seed).
        let from_ledger = store.get_stock_from_ledger("prod-coffee").unwrap();
        assert_eq!(
            from_ledger, 30,
            "SUM of deltas for prod-coffee should be 30"
        );

        // The materialized inventory should now also reflect 30.
        let inv_qty = store.get_stock("prod-coffee").unwrap();
        assert_eq!(inv_qty, 30, "inventory should be rebuilt to 30");
    }

    // ── SYNC-02: shared conflict-application service ────────────────

    #[test]
    fn apply_push_conflict_routes_version_lww() {
        // A higher local product version must win, exactly as the
        // SyncEngine would resolve it (SYNC-02 shared service).
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue
            .enqueue(
                &store,
                "product.update",
                r#"{"version":5,"name":"Local New"}"#,
            )
            .unwrap();
        let server_item =
            OfflineQueueItem::new("product.update", r#"{"version":3,"name":"Server Stale"}"#);

        queue
            .apply_push_conflict(&store, &local, &server_item)
            .unwrap();

        // Local item marked resolved (synced) with the local-won tag; no
        // re-enqueued remote winner.
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1, "local winner must not enqueue a new item");
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
        assert!(
            all[0]
                .last_error
                .as_deref()
                .unwrap_or("")
                .contains("resolved: conflict (local won)"),
            "local item must carry the resolution tag, got: {:?}",
            all[0].last_error
        );
    }

    #[test]
    fn apply_push_conflict_routes_sale_status_dag() {
        // Completed must win over pending even when the local item is
        // NEWER — proves the daemon path can no longer discard an advanced
        // sale state via blanket "remote wins" (SYNC-02).
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue
            .enqueue(
                &store,
                "complete_sale",
                r#"{"status":"pending","version":2}"#,
            )
            .unwrap();
        let server_item =
            OfflineQueueItem::new("complete_sale", r#"{"status":"completed","version":1}"#);

        queue
            .apply_push_conflict(&store, &local, &server_item)
            .unwrap();

        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
        assert!(
            all[0]
                .last_error
                .as_deref()
                .unwrap_or("")
                .contains("resolved: conflict (remote won)"),
            "completed server sale must win the DAG, got: {:?}",
            all[0].last_error
        );
    }

    // ── SYNC-05: CRDT merge payloads are consumable end-to-end ─────

    #[test]
    fn apply_remote_consumes_crdt_merge_stock_adjusted() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        // Build the exact merged payload resolve_stock_crdt produces.
        let merged = serde_json::json!({
            "local": {"sku": "COFFEE", "delta": 10},
            "remote": {"sku": "COFFEE", "delta": -3},
            "merge_type": "crdt_delta"
        })
        .to_string();
        let remote = OfflineQueueItem::new("stock.adjusted", &merged);

        queue.apply_remote(&store, &remote).unwrap();

        // Both deltas applied: 50 + 10 - 3 = 57.
        assert_eq!(inventory_qty(&store, "COFFEE"), 57);
    }

    #[test]
    fn apply_remote_consumes_crdt_merge_stock_movement() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        let merged = serde_json::json!({
            "local": {
                "id": "sm-merge-1", "item_id": "prod-coffee", "delta": 10,
                "reason": "merge-a", "source_terminal_id": null,
                "source_user_id": null, "store_id": "store-b",
                "created_at": "2026-01-15T00:00:00Z"
            },
            "remote": {
                "id": "sm-merge-2", "item_id": "prod-bagel", "delta": -2,
                "reason": "merge-b", "source_terminal_id": null,
                "source_user_id": null, "store_id": "store-c",
                "created_at": "2026-01-15T00:00:01Z"
            },
            "merge_type": "crdt_delta"
        })
        .to_string();
        let remote = OfflineQueueItem::new("stock.movement", &merged);

        queue.apply_remote(&store, &remote).unwrap();

        // Both movement rows inserted into the ledger.
        let movements = store.list_stock_movements("prod-coffee", 10, 0).unwrap();
        assert!(
            movements.iter().any(|m| m.id == "sm-merge-1"),
            "local-side movement must be inserted"
        );
        let movements = store.list_stock_movements("prod-bagel", 10, 0).unwrap();
        assert!(
            movements.iter().any(|m| m.id == "sm-merge-2"),
            "remote-side movement must be inserted"
        );
    }

    #[test]
    fn crdt_merge_end_to_end_resolve_to_apply() {
        // Full SYNC-05 path: resolve_conflict → apply_resolution (enqueue
        // merged winner) → apply_remote (consume merged payload). Both
        // deltas must survive the entire pipeline.
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();

        // The LOCAL item must already be in the queue — apply_resolution
        // marks it resolved (mark_offline_resolved) and fails with NotFound
        // when no row exists (mirrors a real push-conflict where the local
        // item is a pending queue row).
        let local = queue
            .enqueue(&store, "stock.adjusted", r#"{"sku":"COFFEE","delta":10}"#)
            .unwrap();
        let remote = OfflineQueueItem::new("stock.adjusted", r#"{"sku":"COFFEE","delta":-3}"#);

        // Step 1: resolve — merged winner carries both deltas.
        let resolved = crate::conflict::resolve_conflict(&local, &remote);
        let payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(payload["merge_type"], "crdt_delta");

        // Step 2: persist — merged winner enqueued as a new pending item.
        queue.apply_resolution(&store, &resolved).unwrap();
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1, "merged winner must be re-enqueued");
        let winner = &pending[0];
        assert_eq!(winner.action, "stock.adjusted");
        assert!(
            winner.payload.contains("crdt_delta"),
            "winner payload must keep the merge envelope"
        );

        // Step 3: consume — the normal remote dispatcher applies BOTH deltas.
        queue.apply_remote(&store, winner).unwrap();
        assert_eq!(inventory_qty(&store, "COFFEE"), 57, "50 + 10 - 3");
    }
}
