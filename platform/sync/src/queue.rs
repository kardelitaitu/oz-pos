//! Sync Queue — local change log for offline-first replication.
/*
last audited 25-07-26 by RSA-Agent (platform-sync slice B: queue deep read)
crate: platform-sync | status: SAFE | lint: CLEAN
findings: exemplary — apply_remote_atomic_full runs quarantine gate, receipt-exists check, domain mutation, and receipt insert in ONE transaction (crash-safe replay protection); failure path drops the tx then records the failure with retry budget 3 for dead-lettering; CRDT delta merge arms for stock payloads; SYNC-10 settings with non-fatal delta write (savepoint-safe inside caller tx); finalize_sale idempotent pending-to-completed only; unsupported actions fail closed; apply_push_conflict is the single SYNC-02 shared resolver entry; apply_remote is the deprecated non-atomic legacy mirror. Minor note: pull-item product payloads take unwrap_or empty-string for sku and name (server-trusted; snapshot path is RUST-04 validated but pull items are not)
next: consider payload validation parity for pull items | perf: prepared upserts
*/
//!
//! Wraps the `oz_core` offline queue Store methods into a clean interface
//! with additional tracking for conflict resolution and last-sync timing.

use oz_core::db::Store;
use oz_core::db::offline::SyncStatusSummary;
use oz_core::error::CoreError;
use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus};
use oz_core::settings::Settings;
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

/// Default originating terminal for remote settings items whose payload
/// omits `terminal_id` (older servers / relay terminals).
fn default_sync_terminal() -> String {
    "sync".into()
}

/// Payload for the `settings.update` / `settings.change` sync action
/// (SYNC-10). Carries the key, the new value, and the terminal that made
/// the change so the local delta ledger records the originator and the
/// daemon can re-emit a `SettingsUpdated` event for UI reactivity.
#[derive(Deserialize)]
struct SettingsUpdatePayload {
    key: String,
    value: String,
    #[serde(default = "default_sync_terminal")]
    terminal_id: String,
}

/// Payload for the `finalize_sale` sync action — the cloud webhook path
/// enqueues `{"sale_id": …}` after payment capture so the pending sale
/// completes on the terminal.
#[derive(Deserialize)]
struct FinalizeSalePayload {
    sale_id: String,
}

/// Outcome of applying a remote item atomically (SYNC-10).
///
/// [`SyncQueue::apply_remote_atomic`] returns only `applied` for legacy
/// callers; the reporting variant [`SyncQueue::apply_remote_atomic_full`]
/// additionally surfaces a settings change (changed key + originating
/// terminal) so the sync daemon can publish `SettingsUpdated` after the
/// transaction commits.
#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    /// Whether the mutation was applied (false on idempotent replay skip).
    pub applied: bool,
    /// `Some((key, terminal_id))` when this item applied a settings change.
    pub settings_change: Option<(String, String)>,
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

    /// Apply a remote item and its idempotency receipt atomically.
    ///
    /// The existence check, domain mutation, and receipt insert share one
    /// SQLite transaction. A crash before commit therefore rolls back both
    /// the mutation and the receipt, while a replay after commit is skipped.
    ///
    /// Returns only whether the mutation applied — see
    /// [`apply_remote_atomic_full`](Self::apply_remote_atomic_full) for the
    /// variant that also reports settings changes for `SettingsUpdated`.
    pub fn apply_remote_atomic(
        &self,
        store: &Store<'_>,
        item: &OfflineQueueItem,
    ) -> Result<bool, CoreError> {
        Ok(self.apply_remote_atomic_full(store, item)?.applied)
    }

    /// Apply a remote item and its idempotency receipt atomically, reporting
    /// settings changes (SYNC-10).
    ///
    /// Identical transaction semantics to
    /// [`apply_remote_atomic`](Self::apply_remote_atomic), but the outcome
    /// also carries the changed settings key and its originating terminal so
    /// the sync daemon can publish `SettingsUpdated` after the commit —
    /// making a change made on another terminal reactive in this one's UI.
    pub fn apply_remote_atomic_full(
        &self,
        store: &Store<'_>,
        item: &OfflineQueueItem,
    ) -> Result<ApplyOutcome, CoreError> {
        // A quarantined item must not be retried by every subsequent page
        // pull. Operators can inspect the retained payload and explicitly
        // requeue it after correcting the source or client version.
        if store.is_remote_failure_dead_lettered(&item.id)? {
            return Ok(ApplyOutcome::default());
        }

        let tx = store.conn().unchecked_transaction()?;
        let already: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_applied_items WHERE item_id = ?1)",
            rusqlite::params![item.id],
            |row| row.get(0),
        )?;
        if already {
            tx.commit()?;
            return Ok(ApplyOutcome::default());
        }

        match self.apply_remote_in_tx(&tx, item) {
            Ok(()) => {
                store.mark_remote_item_applied_in_tx(&tx, &item.id, &item.action)?;
                store.clear_remote_failure_in_tx(&tx, &item.id)?;
                tx.commit()?;
                Ok(ApplyOutcome {
                    applied: true,
                    settings_change: settings_change_of(item),
                })
            }
            Err(error) => {
                drop(tx);
                store.record_remote_failure(
                    &item.id,
                    &item.action,
                    &item.payload,
                    &error.to_string(),
                    3,
                )?;
                Err(error)
            }
        }
    }

    /// Apply a remote mutation using a caller-owned transaction.
    #[allow(deprecated)]
    fn apply_remote_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        item: &OfflineQueueItem,
    ) -> Result<(), CoreError> {
        match item.action.as_str() {
            "complete_sale" => {
                let payload: SalePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid sale payload: {e}")))?;
                for line in &payload.line_items {
                    Store::new(tx).adjust_stock_in_tx(tx, &line.sku, -line.qty)?;
                }
            }
            "stock.adjusted" => {
                let payload: Value = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid stock payload: {e}")))?;
                let apply_one = |value: Value| -> Result<(), CoreError> {
                    let sub: StockAdjustmentPayload = serde_json::from_value(value)
                        .map_err(|e| CoreError::Internal(format!("invalid stock payload: {e}")))?;
                    Store::new(tx).adjust_stock_in_tx(tx, &sub.sku, sub.delta)?;
                    Ok(())
                };
                if payload.get("merge_type").and_then(|m| m.as_str()) == Some("crdt_delta") {
                    apply_one(payload.get("local").cloned().unwrap_or(Value::Null))?;
                    apply_one(payload.get("remote").cloned().unwrap_or(Value::Null))?;
                } else {
                    apply_one(payload)?;
                }
            }
            "product.created" => {
                let payload: Value = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid product payload: {e}")))?;
                let sku = payload["sku"].as_str().unwrap_or("");
                let name = payload["name"].as_str().unwrap_or("");
                let price_minor = payload["price_minor"].as_i64().unwrap_or(-1);
                let currency = payload["currency"].as_str().unwrap_or("");
                let currency_parsed: oz_core::Currency =
                    currency
                        .parse()
                        .map_err(|e: oz_core::money::InvalidCurrencyCode| {
                            CoreError::Internal(format!("invalid currency in sync payload: {e}"))
                        })?;
                let initial_stock = payload["initial_stock"].as_i64().unwrap_or(0);
                let product_type = payload["product_type"].as_str().unwrap_or("retail");
                Store::new(tx).create_product_if_absent_in_tx(
                    tx,
                    sku,
                    name,
                    oz_core::Money {
                        minor_units: price_minor,
                        currency: currency_parsed,
                    },
                    payload["category_id"].as_str(),
                    payload["barcode"].as_str(),
                    initial_stock,
                    product_type,
                )?;
            }
            "stock.movement" => {
                let payload: Value = serde_json::from_str(&item.payload).map_err(|e| {
                    CoreError::Internal(format!("invalid stock.movement payload: {e}"))
                })?;
                let apply_one = |value: &Value| -> Result<(), CoreError> {
                    let m: StockMovementPayload =
                        serde_json::from_value(value.clone()).map_err(|e| {
                            CoreError::Internal(format!("invalid stock.movement payload: {e}"))
                        })?;
                    Store::new(tx).insert_stock_movement_in_tx(
                        tx,
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
            }
            // SYNC-10: a settings change made on another terminal — write the
            // value row and a versioned delta row inside this transaction.
            // `write_delta` uses a nested SAVEPOINT, which is safe inside the
            // caller's transaction; a delta failure is non-fatal (the value
            // row still landed) and the change is still reported so the UI
            // refetches (matches `set_tracked`'s delta philosophy).
            "settings.update" | "settings.change" => {
                let payload: SettingsUpdatePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid settings payload: {e}")))?;
                Settings::set(tx, &payload.key, &payload.value)?;
                if let Err(e) =
                    Settings::write_delta(tx, &payload.key, &payload.value, &payload.terminal_id)
                {
                    tracing::warn!(
                        key = %payload.key,
                        terminal_id = %payload.terminal_id,
                        error = %e,
                        "sync settings delta write failed (non-fatal)"
                    );
                }
            }
            // A sale completed on the CLOUD (payment captured via the
            // Stripe/Square webhook) — finalize the pending sale locally.
            // The webhook enqueues `{"sale_id": …}`; without this arm the
            // item dead-lettered as "unsupported" and the sale stayed
            // pending forever. `finalize_sale` is idempotent (only
            // transitions status='pending' → 'completed').
            "finalize_sale" => {
                let payload: FinalizeSalePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid finalize payload: {e}")))?;
                Store::finalize_sale_in_tx(tx, &payload.sale_id)?;
            }
            _ => {
                return Err(CoreError::Internal(format!(
                    "unsupported remote sync action: {}",
                    item.action
                )));
            }
        }
        Ok(())
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
            // SYNC-10 parity: the legacy (non-atomic) dispatcher applies
            // remote settings changes with the same row + delta semantics.
            "settings.update" | "settings.change" => {
                let payload: SettingsUpdatePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid settings payload: {e}")))?;
                Settings::set(store.conn(), &payload.key, &payload.value)?;
                if let Err(e) = Settings::write_delta(
                    store.conn(),
                    &payload.key,
                    &payload.value,
                    &payload.terminal_id,
                ) {
                    tracing::warn!(
                        key = %payload.key,
                        terminal_id = %payload.terminal_id,
                        error = %e,
                        "sync settings delta write failed (non-fatal)"
                    );
                }
                Ok(())
            }
            // A sale completed on the CLOUD (payment captured via the
            // Stripe/Square webhook) — finalize the pending sale locally.
            // Idempotent: only transitions status='pending' → 'completed'.
            "finalize_sale" => {
                let payload: FinalizeSalePayload = serde_json::from_str(&item.payload)
                    .map_err(|e| CoreError::Internal(format!("invalid finalize payload: {e}")))?;
                store.finalize_sale(&payload.sale_id)?;
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

/// Extract the settings change an item carries, if any (SYNC-10).
///
/// Called only on the successful-apply path of
/// [`SyncQueue::apply_remote_atomic_full`]. The apply arm parses the same
/// `SettingsUpdatePayload` before it can succeed, so this re-parse can
/// never diverge from what was applied — it only exists to surface the
/// changed key and originating terminal for `SettingsUpdated`.
fn settings_change_of(item: &OfflineQueueItem) -> Option<(String, String)> {
    if item.action != "settings.update" && item.action != "settings.change" {
        return None;
    }
    let payload: SettingsUpdatePayload = serde_json::from_str(&item.payload).ok()?;
    Some((payload.key, payload.terminal_id))
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
    fn apply_remote_atomic_replay_changes_stock_once() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem {
            id: "remote-sale-once".into(),
            action: "complete_sale".into(),
            payload: r#"{"line_items":[{"sku":"COFFEE","qty":2}]}"#.into(),
            ..OfflineQueueItem::new("complete_sale", "{}")
        };

        assert!(queue.apply_remote_atomic(&store, &remote).unwrap());
        assert!(!queue.apply_remote_atomic(&store, &remote).unwrap());
        assert_eq!(inventory_qty(&store, "COFFEE"), 48);
        assert!(store.is_remote_item_applied(&remote.id).unwrap());
    }

    #[test]
    fn apply_remote_atomic_failure_rolls_back_mutation_and_receipt() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem {
            id: "remote-sale-invalid".into(),
            action: "complete_sale".into(),
            payload: r#"{"line_items":[{"sku":"COFFEE","qty":2},{"sku":"MISSING","qty":1}]}"#
                .into(),
            ..OfflineQueueItem::new("complete_sale", "{}")
        };

        assert!(queue.apply_remote_atomic(&store, &remote).is_err());
        assert_eq!(inventory_qty(&store, "COFFEE"), 50);
        assert!(!store.is_remote_item_applied(&remote.id).unwrap());
        assert!(!store.is_remote_failure_dead_lettered(&remote.id).unwrap());

        // The third failed attempt quarantines the poison item. A later
        // replay is skipped without mutating state or advancing a receipt.
        assert!(queue.apply_remote_atomic(&store, &remote).is_err());
        assert!(queue.apply_remote_atomic(&store, &remote).is_err());
        assert!(store.is_remote_failure_dead_lettered(&remote.id).unwrap());
        assert!(!queue.apply_remote_atomic(&store, &remote).unwrap());
    }

    #[test]
    fn apply_remote_atomic_clears_stale_failure_after_success() {
        let store = setup_store();
        seed_product_and_inventory(&store);
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem {
            id: "remote-sale-recovered".into(),
            action: "complete_sale".into(),
            payload: r#"{"line_items":[{"sku":"COFFEE","qty":1}]}"#.into(),
            ..OfflineQueueItem::new("complete_sale", "{}")
        };

        store
            .record_remote_failure(
                &remote.id,
                &remote.action,
                &remote.payload,
                "temporary failure",
                3,
            )
            .unwrap();
        assert_eq!(store.list_remote_failures().unwrap().len(), 1);

        assert!(queue.apply_remote_atomic(&store, &remote).unwrap());
        assert_eq!(inventory_qty(&store, "COFFEE"), 49);
        assert!(store.list_remote_failures().unwrap().is_empty());
    }

    #[test]
    fn apply_remote_atomic_rejects_conflicting_existing_product() {
        let store = setup_store();
        let queue = SyncQueue::new();
        store
            .conn()
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at,
                                      product_type, version)
                 VALUES ('prod-existing', 'COFFEE', 'Existing Coffee', 350, 'USD',
                         '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 'retail', 1)",
                [],
            )
            .unwrap();
        let payload = serde_json::json!({
            "sku": "COFFEE",
            "name": "Different Coffee",
            "price_minor": 450,
            "currency": "USD",
            "initial_stock": 0,
            "product_type": "retail"
        })
        .to_string();
        let remote = OfflineQueueItem::new("product.created", &payload);

        assert!(queue.apply_remote_atomic(&store, &remote).is_err());
        assert!(!store.is_remote_item_applied(&remote.id).unwrap());
        assert_eq!(
            store.get_product("COFFEE").unwrap().unwrap().product.name,
            "Existing Coffee"
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

    #[test]
    fn apply_remote_atomic_rejects_unknown_action_without_receipt() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem::new("unknown.action", r#"{\"data\":\"test\"}"#);
        assert!(queue.apply_remote_atomic(&store, &remote).is_err());
        assert!(!store.is_remote_item_applied(&remote.id).unwrap());
    }

    // ── finalize_sale (cloud webhook → terminal) ─────────────────

    /// The cloud server's webhook path enqueues a `finalize_sale` item
    /// (`{"sale_id": …}`) into offline_queue so the pending sale completes
    /// on the terminal after payment capture. The dispatcher MUST apply
    /// it: transition the sale to completed.
    #[test]
    fn apply_remote_atomic_finalizes_pending_sale() {
        let store = setup_store();
        let queue = SyncQueue::new();

        // Seed a pending sale the way the terminal's complete flow leaves it.
        let sale_id = "sale-finalize-1";
        store
            .conn()
            .execute(
                "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                    tendered_minor, discount_percent, discount_label, user_id,
                                    created_at, updated_at, subtotal_minor, tax_total_minor,
                                    deduction_locations, version)
                 VALUES (?1, 1000, 'USD', 1, 'pending', 'CARD', 1000, 0, NULL, 'user-1',
                         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1000, 0, '[]', 1)",
                [sale_id],
            )
            .unwrap();

        // The webhook's exact payload shape.
        let remote =
            OfflineQueueItem::new("finalize_sale", format!(r#"{{"sale_id":"{sale_id}"}}"#));
        let outcome = queue
            .apply_remote_atomic_full(&store, &remote)
            .expect("finalize_sale must apply, not dead-letter as unsupported");
        assert!(outcome.applied, "finalize_sale must be marked applied");

        // The pending sale must now be completed.
        let status: String = store
            .conn()
            .query_row("SELECT status FROM sales WHERE id = ?1", [sale_id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            status, "completed",
            "sale must be finalized by the remote item"
        );
    }

    #[test]
    fn apply_remote_legacy_finalizes_pending_sale() {
        let store = setup_store();
        let queue = SyncQueue::new();

        let sale_id = "sale-finalize-legacy";
        store
            .conn()
            .execute(
                "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                    tendered_minor, discount_percent, discount_label, user_id,
                                    created_at, updated_at, subtotal_minor, tax_total_minor,
                                    deduction_locations, version)
                 VALUES (?1, 1000, 'USD', 1, 'pending', 'CARD', 1000, 0, NULL, 'user-1',
                         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1000, 0, '[]', 1)",
                [sale_id],
            )
            .unwrap();

        let remote =
            OfflineQueueItem::new("finalize_sale", format!(r#"{{"sale_id":"{sale_id}"}}"#));
        queue
            .apply_remote(&store, &remote)
            .expect("legacy apply_remote must accept finalize_sale");
        let status: String = store
            .conn()
            .query_row("SELECT status FROM sales WHERE id = ?1", [sale_id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "completed");
    }

    // ── SYNC-10: remote settings application + reactivity ────────

    /// Helper: a remote `settings.update` item as the cloud server would
    /// deliver it (fixed id so the idempotency ledger absorbs replays).
    fn remote_settings_update(id: &str) -> OfflineQueueItem {
        let mut item = OfflineQueueItem::new(
            "settings.update",
            r#"{"key":"store.name","value":"Remote Acme","terminal_id":"term-remote","version":3}"#,
        );
        item.id = id.into();
        item.created_at = "2026-01-02T00:00:00.000Z".into();
        item
    }

    /// SYNC-10 Red: a remote `settings.update` must apply the value row AND
    /// a versioned delta ledger row atomically with the idempotency receipt
    /// — today it errors as an unsupported action and gets quarantined.
    #[test]
    fn apply_remote_atomic_settings_update_writes_row_and_delta() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = remote_settings_update("remote-setting-1");

        assert!(
            queue.apply_remote_atomic(&store, &remote).unwrap(),
            "settings.update must apply instead of erroring as unsupported"
        );
        assert_eq!(
            oz_core::settings::Settings::get(store.conn(), "store.name")
                .unwrap()
                .as_deref(),
            Some("Remote Acme"),
            "the settings row must be updated"
        );
        assert_eq!(
            oz_core::settings::Settings::get_version(store.conn(), "store.name", "term-remote")
                .unwrap(),
            Some(1),
            "a versioned delta row must be written for the (key, terminal) pair"
        );
        assert!(
            store.is_remote_item_applied("remote-setting-1").unwrap(),
            "the idempotency receipt must be recorded with the mutation"
        );
    }

    /// SYNC-10 Red: the atomic apply must surface the settings change
    /// (changed key + originating terminal) so the daemon can publish
    /// `SettingsUpdated` for UI reactivity. `apply_remote_atomic_full` is
    /// the reporting variant; the legacy bool wrapper keeps old callers.
    #[test]
    fn apply_remote_atomic_full_surfaces_settings_change() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = remote_settings_update("remote-setting-2");

        let outcome = queue.apply_remote_atomic_full(&store, &remote).unwrap();
        assert!(outcome.applied);
        assert_eq!(
            outcome.settings_change,
            Some(("store.name".to_string(), "term-remote".to_string())),
            "the changed key and originating terminal must be reported"
        );
    }

    /// SYNC-10: replay of the same remote settings item must NOT publish a
    /// second change (the ledger skips it, so the outcome carries no change).
    #[test]
    fn apply_remote_atomic_full_replay_reports_no_settings_change() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = remote_settings_update("remote-setting-3");

        let first = queue.apply_remote_atomic_full(&store, &remote).unwrap();
        let replay = queue.apply_remote_atomic_full(&store, &remote).unwrap();
        assert!(first.applied);
        assert!(first.settings_change.is_some());
        assert!(
            !replay.applied && replay.settings_change.is_none(),
            "a replayed item must not re-apply or re-report the change (SYNC-01)"
        );
    }

    /// SYNC-10: the non-atomic dispatcher (legacy SyncEngine path) applies
    /// settings updates with the same row + delta semantics.
    #[test]
    fn apply_remote_settings_update_non_atomic() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = remote_settings_update("remote-setting-4");

        queue.apply_remote(&store, &remote).unwrap();
        assert_eq!(
            oz_core::settings::Settings::get(store.conn(), "store.name")
                .unwrap()
                .as_deref(),
            Some("Remote Acme")
        );
        assert_eq!(
            oz_core::settings::Settings::get_version(store.conn(), "store.name", "term-remote")
                .unwrap(),
            Some(1)
        );
    }

    /// SYNC-10: the `settings.change` action alias (the audit catalog's
    /// spelling) applies identically to `settings.update`.
    #[test]
    fn apply_remote_settings_change_alias() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let mut remote = remote_settings_update("remote-setting-5");
        remote.action = "settings.change".into();

        let outcome = queue.apply_remote_atomic_full(&store, &remote).unwrap();
        assert!(outcome.applied);
        assert_eq!(
            outcome.settings_change,
            Some(("store.name".to_string(), "term-remote".to_string()))
        );
        assert_eq!(
            oz_core::settings::Settings::get(store.conn(), "store.name")
                .unwrap()
                .as_deref(),
            Some("Remote Acme")
        );
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
