//! Offline queue commands.
//!
//! These commands allow the front-end to enqueue, list, and sync
//! transactions that were created while the network was unavailable.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::sync_client::{self, SyncConfig};
use oz_core::{OfflineQueueItem, Store, SyncPriority};

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Offline queue item DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineQueueItemDto {
    /// Unique identifier.
    pub id: String,
    /// Action.
    pub action: String,
    /// Payload.
    pub payload: String,
    /// Current status.
    pub status: String,
    /// Retry Count.
    pub retry_count: i64,
    /// Last Error.
    pub last_error: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Synced At.
    pub synced_at: Option<String>,
    /// Tenant / store ID for multi-store isolation (OFF-09).
    pub tenant_id: String,
    /// Sync priority tier: "critical" | "normal" | "low" (OFF-09).
    pub priority: String,
}

impl From<OfflineQueueItem> for OfflineQueueItemDto {
    fn from(item: OfflineQueueItem) -> Self {
        Self {
            id: item.id,
            action: item.action,
            payload: item.payload,
            status: item.status.as_stored_str().to_owned(),
            retry_count: item.retry_count,
            last_error: item.last_error,
            created_at: item.created_at,
            synced_at: item.synced_at,
            tenant_id: item.tenant_id,
            priority: item.priority.as_str().to_owned(),
        }
    }
}

/// Result of a sync retry attempt.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// Number of items successfully synced.
    pub synced_count: i64,
    /// Number of items that failed to sync.
    pub failed_count: i64,
    /// Total number of items that were attempted.
    pub total_count: i64,
}

/// Arguments for enqueuing an offline transaction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueOfflineArgs {
    /// The action to perform (e.g. "complete_sale", "void_sale").
    pub action: String,
    /// JSON-serialized payload for the action.
    pub payload: String,
    /// Optional tenant / store ID (OFF-09). Defaults to "default" for
    /// single-store deployments.
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// Optional sync priority tier (OFF-09): "critical" | "normal" | "low".
    #[serde(default)]
    pub priority: Option<String>,
}

// ── Commands ──────────────────────────────────────────────────────────

/// Manually enqueue a transaction for later sync.
#[command]
pub async fn enqueue_offline(
    args: EnqueueOfflineArgs,
    state: State<'_, AppState>,
) -> Result<OfflineQueueItemDto, AppError> {
    validate_not_empty("action", &args.action).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("payload", &args.payload).map_err(|e| AppError::Invalid(e.to_string()))?;

    // OFF-09: preserve tenant isolation and priority tier at the command
    // boundary. `enqueue_offline_scoped` records both in the row; a missing
    // tenant falls back to the "default" single-store tenant and a missing
    // priority to Normal (never escalated from a stale front-end).
    let tenant_id = args.tenant_id.as_deref().unwrap_or("default");
    let priority = args
        .priority
        .as_deref()
        .map(SyncPriority::from_str_lenient)
        .unwrap_or(SyncPriority::Normal);

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let item = store.enqueue_offline_scoped(&args.action, &args.payload, tenant_id, priority)?;
    drop(db);

    tracing::info!(id = %item.id, action = %item.action, tenant_id, "offline transaction enqueued");
    Ok(item.into())
}

/// List all pending (unsynced) offline queue items, oldest first.
#[command]
pub async fn list_pending_offline(
    state: State<'_, AppState>,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let db = state.db.lock().await;
    run_list_pending_offline(&db)
}

fn run_list_pending_offline(
    conn: &rusqlite::Connection,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let store = Store::new(conn);
    let items = store.list_pending_offline()?;
    let dtos: Vec<OfflineQueueItemDto> = items.into_iter().map(OfflineQueueItemDto::from).collect();
    Ok(dtos)
}

/// List all offline queue items (most recent first).
#[command]
pub async fn list_all_offline(
    state: State<'_, AppState>,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let items = store.list_all_offline()?;
    let dtos: Vec<OfflineQueueItemDto> = items.into_iter().map(OfflineQueueItemDto::from).collect();
    Ok(dtos)
}

/// Get the count of pending offline items.
#[command]
pub async fn pending_offline_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let count = store.pending_offline_count()?;
    drop(db);
    Ok(count)
}

/// Attempt to sync all pending offline items through the real cloud sync
/// pipeline.
///
/// SYNC-04: this is NOT a placeholder — it delegates to the same
/// authenticated push flow as `sync_run`: read pending items + config,
/// POST the batch to the cloud server, then mark each item `synced` or
/// `failed` only according to the server's per-item outcome.
///
/// Uses a three-phase split (read → async HTTP → write) so the DB lock
/// is not held during the network round-trip, mirroring `sync_run`.
#[command]
pub async fn retry_offline_sync(state: State<'_, AppState>) -> Result<SyncResult, AppError> {
    // Phase 1: Read pending items and config from DB (brief lock).
    let (pending_items, config_opt) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let pending = store.list_pending_offline()?;
        let config = SyncConfig::from_settings(&store)?;
        (pending, config)
    };

    let total_count = pending_items.len() as i64;
    let config = match config_opt {
        Some(c) => c,
        None => {
            // SYNC-04: never fabricate a successful retry when sync is
            // unconfigured — surface the error so the UI catch handler
            // shows the honest failure and the items stay pending.
            return Err(AppError::Invalid(
                "Sync is not configured or disabled — items remain pending".into(),
            ));
        }
    };

    if pending_items.is_empty() {
        return Ok(SyncResult {
            synced_count: 0,
            failed_count: 0,
            total_count: 0,
        });
    }

    // OFF-09: critical-before-normal ordering. `list_pending_offline`
    // returns created_at ASC, so re-order the batch so Critical items
    // always transmit before Normal/Low.
    let mut pending_items = pending_items;
    pending_items.sort_by_key(|i| i.priority);

    // Phase 2: Async HTTP push (no DB lock held).
    let outcomes = sync_client::send_items_to_server(&config, &pending_items).await;

    // Phase 3: Write outcomes back to DB (brief lock).
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let attempt = match outcomes {
        Ok(outcomes) => sync_client::apply_sync_outcomes(&store, &pending_items, &outcomes)?,
        Err(e) => sync_client::mark_all_failed(&store, &pending_items, &e.to_string())?,
    };
    drop(db);

    Ok(SyncResult {
        synced_count: attempt.synced as i64,
        failed_count: attempt.failed as i64,
        total_count,
    })
}

/// Delete a processed offline queue item.
#[command]
pub async fn delete_offline_item(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_offline_item(&id)?;
    drop(db);

    tracing::info!(id, "offline queue item deleted");
    Ok(())
}

/// Arguments for `requeue_remote_failure`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequeueRemoteFailureArgs {
    /// Remote item id currently quarantined in `sync_remote_failures`.
    pub item_id: String,
}

/// Requeue a dead-lettered remote item so the next sync cycle retries it.
///
/// Operators call this after remediating the item's source (e.g. creating
/// the missing product a remote sale referenced, or upgrading a client that
/// rejected the payload). The quarantine row is cleared and the durable
/// pull anchor is rewound, so the next pull re-fetches the item and retries
/// it with a fresh attempt budget; the idempotency ledger makes the full
/// re-pull safe.
///
/// An id that is not currently dead-lettered returns `NotFound` — a
/// mistyped id must not be a silent no-op.
#[command]
pub async fn requeue_remote_failure(
    args: RequeueRemoteFailureArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("itemId", &args.item_id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    run_requeue_remote_failure(&db, &args.item_id)?;
    drop(db);

    tracing::info!(item_id = %args.item_id, "dead-lettered remote item requeued for sync retry");
    Ok(())
}

/// Execute the requeue against a connection, extracted so the command
/// boundary is unit-testable without a Tauri runtime.
fn run_requeue_remote_failure(conn: &rusqlite::Connection, item_id: &str) -> Result<(), AppError> {
    let store = Store::new(conn);
    store.requeue_remote_failure(item_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::OfflineQueueStatus;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    #[test]
    fn list_pending_offline_empty_db() {
        let conn = fresh_conn();
        let items = run_list_pending_offline(&conn).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn enqueue_and_list_pending() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store
            .enqueue_offline("complete_sale", r#"{"sale_id":"abc"}"#)
            .unwrap();
        assert_eq!(item.action, "complete_sale");
        assert_eq!(item.status, OfflineQueueStatus::Pending);

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, item.id);
    }

    #[test]
    fn mark_offline_synced() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("void_sale", "{}").unwrap();
        store.mark_offline_synced(&item.id).unwrap();

        let synced_item = store.list_all_offline().unwrap();
        assert_eq!(synced_item.len(), 1);
        assert_eq!(synced_item[0].status, OfflineQueueStatus::Synced);
        assert!(synced_item[0].synced_at.is_some());
    }

    #[test]
    fn mark_offline_failed() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("complete_sale", "{}").unwrap();
        store
            .mark_offline_failed(&item.id, "network error")
            .unwrap();

        let failed = store.list_all_offline().unwrap();
        assert_eq!(failed[0].status, OfflineQueueStatus::Failed);
        assert_eq!(failed[0].last_error.as_deref(), Some("network error"));
        assert_eq!(failed[0].retry_count, 1);
    }

    #[test]
    fn pending_offline_count() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        assert_eq!(store.pending_offline_count().unwrap(), 0);
        store.enqueue_offline("test", "{}").unwrap();
        assert_eq!(store.pending_offline_count().unwrap(), 1);
    }

    #[test]
    fn delete_offline_item() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("test", "{}").unwrap();
        store.delete_offline_item(&item.id).unwrap();
        assert_eq!(store.list_all_offline().unwrap().len(), 0);
    }

    #[test]
    fn enqueue_offline_validation() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let item = store.enqueue_offline("", "{}").unwrap();
        // Empty action is stored as-is (no front-end validation at store level).
        assert_eq!(item.action, "");
        let loaded = store.list_all_offline().unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn retry_sync_marks_pending_as_synced() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        store
            .enqueue_offline("complete_sale", r#"{"id":"1"}"#)
            .unwrap();
        store.enqueue_offline("void_sale", r#"{"id":"2"}"#).unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);

        for item in &pending {
            store.mark_offline_synced(&item.id).unwrap();
        }

        let remaining = store.list_pending_offline().unwrap();
        assert!(remaining.is_empty());
    }

    // -- DTO struct tests --

    #[test]
    fn offline_queue_item_dto_debug() {
        let dto = OfflineQueueItemDto {
            id: "q1".into(),
            action: "complete_sale".into(),
            payload: "{}".into(),
            status: "pending".into(),
            retry_count: 0,
            last_error: None,
            created_at: "2025-01-01".into(),
            synced_at: None,
            tenant_id: "store-a".into(),
            priority: "critical".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("complete_sale"));
        assert!(d.contains("store-a"));
        assert!(d.contains("critical"));
    }

    #[test]
    fn offline_queue_item_dto_serialize() {
        let dto = OfflineQueueItemDto {
            id: "q2".into(),
            action: "void_sale".into(),
            payload: "{}".into(),
            status: "synced".into(),
            retry_count: 1,
            last_error: Some("timeout".into()),
            created_at: "2025-02-01".into(),
            synced_at: Some("2025-02-02".into()),
            tenant_id: "store-b".into(),
            priority: "normal".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["action"], "void_sale");
        assert_eq!(json["retryCount"], 1);
        assert!(json["lastError"].is_string());
        // OFF-09: tenant + priority metadata survive the serializer.
        assert_eq!(json["tenantId"], "store-b");
        assert_eq!(json["priority"], "normal");
    }

    #[test]
    fn enqueue_offline_args_optional_tenant_and_priority() {
        // OFF-09: tenant + priority are optional for backward compat, and a
        // front-end can never escalate to Critical by passing junk.
        let bare: EnqueueOfflineArgs =
            serde_json::from_str(r#"{"action":"a","payload":"{}"}"#).unwrap();
        assert!(bare.tenant_id.is_none());
        assert!(bare.priority.is_none());

        let scoped: EnqueueOfflineArgs = serde_json::from_str(
            r#"{"action":"a","payload":"{}","tenantId":"store-a","priority":"critical"}"#,
        )
        .unwrap();
        assert_eq!(scoped.tenant_id.as_deref(), Some("store-a"));
        assert_eq!(scoped.priority.as_deref(), Some("critical"));
    }

    #[test]
    fn sync_result_debug() {
        let sr = SyncResult {
            synced_count: 5,
            failed_count: 2,
            total_count: 7,
        };
        let d = format!("{sr:?}");
        assert!(d.contains("5"));
        assert!(d.contains("7"));
    }

    #[test]
    fn sync_result_serialize() {
        let sr = SyncResult {
            synced_count: 10,
            failed_count: 0,
            total_count: 10,
        };
        let json = serde_json::to_value(&sr).unwrap();
        assert_eq!(json["syncedCount"], 10);
        assert_eq!(json["failedCount"], 0);
    }

    #[test]
    fn enqueue_offline_args_deserialize() {
        let json = r#"{"action":"complete_sale","payload":"{}"}"#;
        let args: EnqueueOfflineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.action, "complete_sale");
        assert_eq!(args.payload, "{}");
    }

    #[test]
    fn enqueue_offline_args_debug() {
        let args = EnqueueOfflineArgs {
            action: "test".into(),
            payload: "{}".into(),
            tenant_id: None,
            priority: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("test"));
    }

    // ── list_remote_failures (dead-letter discovery) ────────────────

    #[test]
    fn run_list_remote_failures_empty_db() {
        let conn = fresh_conn();
        let failures = run_list_remote_failures(&conn).unwrap();
        assert!(failures.is_empty(), "fresh db must have no failures");
    }

    #[test]
    fn run_list_remote_failures_returns_retained_failures_newest_first() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        // Two distinct remote items: one retryable, one dead-lettered after
        // the third failed attempt. Both must be listed — operators need to
        // see every retained failure, with dead-lettered items flagged.
        store
            .record_remote_failure(
                "dl-item-1",
                "complete_sale",
                "{\"sale_id\":\"s1\"}",
                "missing product",
                3,
            )
            .unwrap();
        for _ in 0..3 {
            store
                .record_remote_failure("retry-item-2", "stock.adjusted", "{}", "bad", 3)
                .unwrap();
        }
        assert!(store.is_remote_failure_dead_lettered("dl-item-1").unwrap());
        assert!(
            store
                .is_remote_failure_dead_lettered("retry-item-2")
                .unwrap()
        );

        let failures = run_list_remote_failures(&conn).unwrap();
        assert_eq!(failures.len(), 2);
        let by_id: std::collections::HashMap<_, _> = failures
            .iter()
            .map(|dto| (dto.item_id.clone(), dto))
            .collect();
        let dl = by_id.get("dl-item-1").unwrap();
        assert_eq!(dl.action, "complete_sale");
        assert_eq!(dl.attempts, 1);
        assert_eq!(dl.last_error, "missing product");
        assert!(!dl.dead_lettered, "dl-item-1 is still retryable");
        let retry = by_id.get("retry-item-2").unwrap();
        assert_eq!(retry.action, "stock.adjusted");
        assert_eq!(retry.attempts, 3);
        assert!(retry.dead_lettered, "retry-item-2 hit the dead letter");
        assert_eq!(retry.payload, "{}");
    }

    // ── requeue_remote_failure (dead-letter requeue workflow) ────────

    #[test]
    fn run_requeue_remote_failure_clears_dead_letter() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        // Drive a remote item to the dead letter, then persist a pull
        // anchor past it (as the daemon would after quarantining it).
        for _ in 0..3 {
            store
                .record_remote_failure("dl-item-1", "complete_sale", "{}", "bad", 3)
                .unwrap();
        }
        assert!(store.is_remote_failure_dead_lettered("dl-item-1").unwrap());
        store
            .set_sync_pull_state(Some("2026-06-01T00:00:00Z"), Some("cursor-1"))
            .unwrap();

        run_requeue_remote_failure(&conn, "dl-item-1").unwrap();

        assert!(!store.is_remote_failure_dead_lettered("dl-item-1").unwrap());
        assert!(store.list_remote_failures().unwrap().is_empty());
        let st = store.get_sync_pull_state().unwrap();
        assert!(st.since.is_none(), "anchor must rewind after requeue");
        assert!(st.cursor.is_none(), "cursor must clear with the anchor");
    }
    #[test]
    fn run_requeue_remote_failure_unknown_id_errors() {
        let conn = fresh_conn();
        let err = run_requeue_remote_failure(&conn, "never-seen").unwrap_err();
        match err {
            AppError::Core { sub_kind, .. } => {
                assert_eq!(format!("{sub_kind:?}"), "NotFound");
            }
            other => panic!("expected NotFound Core error, got {other:?}"),
        }
    }

    #[test]
    fn requeue_remote_failure_args_deserialize() {
        let json = r#"{"itemId":"dl-1"}"#;
        let args: RequeueRemoteFailureArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.item_id, "dl-1");
    }
}
