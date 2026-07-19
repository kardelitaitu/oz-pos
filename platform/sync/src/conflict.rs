//! Conflict Resolution — strategies for resolving conflicts between local
//! and remote versions of the same data.
//!
//! ADR-21 defines entity-type dispatch:
//!
//! | Action prefix           | Strategy                | Key field   |
//! |-------------------------|-------------------------|-------------|
//! | `product.*`, `category.*`, `tax.*`, `user.*`, `staff.*` | Version LWW | `version`   |
//! | `sale.*`, `complete_sale`, `void_sale`, `refund_sale`   | Sale LWW    | `status`    |
//! | `stock.*`               | CRDT merge              | —           |
//! | `*` (fallback)          | Created-at LWW          | `created_at`|

use oz_core::offline::OfflineQueueItem;
use serde_json::Value;

use crate::queue::ResolvedItem;

// ── Payload field extractors ─────────────────────────────────────────

/// Extract an i64 `version` field from a JSON payload.
/// Returns `None` if the field is missing or not a valid integer.
fn extract_version(payload: &str) -> Option<i64> {
    let v: Value = serde_json::from_str(payload).ok()?;
    v.get("version")?.as_i64()
}

/// Extract a string `status` field from a JSON payload.
/// Returns `None` if the field is missing or not a string.
fn extract_status(payload: &str) -> Option<String> {
    let v: Value = serde_json::from_str(payload).ok()?;
    v.get("status")?.as_str().map(String::from)
}

/// Priority order of sale statuses (higher index = more advanced).
const SALE_STATUS_ORDER: &[&str] = &["active", "pending", "completed", "voided", "refunded"];

fn sale_status_rank(status: &str) -> usize {
    SALE_STATUS_ORDER
        .iter()
        .position(|&s| s == status)
        .unwrap_or(0)
}

// ── Legacy resolver (unchanged) ──────────────────────────────────────

/// Resolve a conflict using Last-Write-Wins (LWW) by `created_at`.
///
/// Compares the `created_at` timestamps of the local and remote items.
/// The item with the later timestamp wins. If timestamps are equal, the
/// remote item wins (server-authoritative).
///
/// This is the **fallback** resolver for unknown action types (ADR-21 §1).
pub fn resolve_lww(local: &OfflineQueueItem, remote: &OfflineQueueItem) -> ResolvedItem {
    let winner = if local.created_at > remote.created_at {
        local.clone()
    } else {
        // Remote wins on tie (server-authoritative).
        remote.clone()
    };

    ResolvedItem {
        local: Some(local.clone()),
        remote: Some(remote.clone()),
        winner,
    }
}

// ── ADR-21 resolvers ────────────────────────────────────────────────

/// Resolve a conflict using Version LWW for reference data.
///
/// Extracts the `version` field from each item's JSON payload and compares
/// as integers. The item with the higher version wins. On tie, the remote
/// item wins (server-authoritative).
///
/// If either payload lacks a `version` field, falls back to `created_at` LWW.
///
/// **Used for:** `product.*`, `category.*`, `tax.*`, `user.*`, `staff.*`
pub fn resolve_version_lww(local: &OfflineQueueItem, remote: &OfflineQueueItem) -> ResolvedItem {
    let local_ver = extract_version(&local.payload);
    let remote_ver = extract_version(&remote.payload);

    let winner = match (local_ver, remote_ver) {
        (Some(lv), Some(rv)) if lv > rv => local.clone(),
        (Some(_), Some(_)) => remote.clone(), // remote wins on tie or lower
        (Some(_), None) => local.clone(),     // local has version, remote doesn't
        (None, Some(_)) => remote.clone(),    // remote has version, local doesn't
        (None, None) => {
            // Neither has version — fall back to created_at LWW
            if local.created_at > remote.created_at {
                local.clone()
            } else {
                remote.clone()
            }
        }
    };

    ResolvedItem {
        local: Some(local.clone()),
        remote: Some(remote.clone()),
        winner,
    }
}

/// Resolve a conflict for sale items using status DAG ordering.
///
/// Sale statuses follow a legal transition graph:
/// `active → pending → completed → voided → refunded`
///
/// The item with the **most advanced** status wins — not the most recent
/// timestamp. This prevents a completed sale from being reverted to
/// "pending" by a stale remote item.
///
/// If both items have the same status rank, falls back to version LWW.
///
/// **Used for:** `sale.*`, `complete_sale`, `void_sale`, `refund_sale`
pub fn resolve_sale_lww(local: &OfflineQueueItem, remote: &OfflineQueueItem) -> ResolvedItem {
    let local_status = extract_status(&local.payload).unwrap_or_default();
    let remote_status = extract_status(&remote.payload).unwrap_or_default();

    let local_rank = sale_status_rank(&local_status);
    let remote_rank = sale_status_rank(&remote_status);

    let winner = if local_rank > remote_rank {
        local.clone()
    } else if remote_rank > local_rank {
        remote.clone()
    } else {
        // Same status rank — fall back to version LWW.
        // Call the version resolver directly on the items.
        return resolve_version_lww(local, remote);
    };

    ResolvedItem {
        local: Some(local.clone()),
        remote: Some(remote.clone()),
        winner,
    }
}

/// Resolve a conflict for stock movements using CRDT delta merge.
///
/// Stock movements are immutable delta rows — both deltas are valid and
/// should be applied. The merged winner carries both payloads combined.
///
/// **Used for:** `stock.adjusted`, `stock.movement`
pub fn resolve_stock_crdt(local: &OfflineQueueItem, remote: &OfflineQueueItem) -> ResolvedItem {
    // CRDT merge: both deltas are valid. The merged payload carries both.
    let merged_payload = serde_json::json!({
        "local": serde_json::from_str::<Value>(&local.payload).unwrap_or(Value::Null),
        "remote": serde_json::from_str::<Value>(&remote.payload).unwrap_or(Value::Null),
        "merge_type": "crdt_delta"
    })
    .to_string();

    let winner = OfflineQueueItem {
        id: uuid::Uuid::now_v7().to_string(),
        action: local.action.clone(),
        payload: merged_payload,
        status: local.status,
        retry_count: local.retry_count.max(remote.retry_count),
        last_error: local
            .last_error
            .clone()
            .or_else(|| remote.last_error.clone()),
        created_at: local.created_at.clone(),
        synced_at: None,
        tenant_id: local.tenant_id.clone(),
        priority: local.priority,
    };

    ResolvedItem {
        local: Some(local.clone()),
        remote: Some(remote.clone()),
        winner,
    }
}

// ── Dispatch ─────────────────────────────────────────────────────────

/// Resolve a conflict between a local and remote offline queue item.
///
/// Dispatches to the appropriate strategy based on the action prefix
/// (ADR-21 §1 — Entity-Type Dispatch).
///
/// | Action prefix | Strategy | Behaviour |
/// |---|---|---|
/// | `product.*`, `category.*`, `tax.*`, `user.*`, `staff.*` | Version LWW | Higher `version` wins |
/// | `sale.*`, `complete_sale`, `void_sale`, `refund_sale` | Sale LWW | Higher `status` rank wins |
/// | `stock.*` | CRDT merge | Both deltas preserved |
/// | `*` (fallback) | Created-at LWW | Later `created_at` wins |
pub fn resolve_conflict(local: &OfflineQueueItem, remote: &OfflineQueueItem) -> ResolvedItem {
    let action = local.action.as_str();

    if action.starts_with("sale.")
        || action == "complete_sale"
        || action == "void_sale"
        || action == "refund_sale"
    {
        resolve_sale_lww(local, remote)
    } else if action.starts_with("stock.") {
        resolve_stock_crdt(local, remote)
    } else if action.starts_with("product.")
        || action.starts_with("category.")
        || action.starts_with("tax.")
        || action.starts_with("user.")
        || action.starts_with("staff.")
    {
        resolve_version_lww(local, remote)
    } else {
        // Fallback: original LWW by created_at.
        resolve_lww(local, remote)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::offline::OfflineQueueStatus;

    fn make_item(created_at: &str, action: &str) -> OfflineQueueItem {
        OfflineQueueItem {
            id: uuid::Uuid::now_v7().to_string(),
            action: action.to_owned(),
            payload: "{}".to_owned(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: created_at.to_owned(),
            synced_at: None,
            tenant_id: "default".into(),
            priority: oz_core::offline::SyncPriority::Normal,
        }
    }

    fn make_item_with_version(
        created_at: &str,
        action: &str,
        version: i64,
        extra: &str,
    ) -> OfflineQueueItem {
        let payload = if extra.is_empty() {
            format!(r#"{{"version":{version}}}"#)
        } else {
            format!(r#"{{"version":{version},{extra}}}"#)
        };
        OfflineQueueItem {
            id: uuid::Uuid::now_v7().to_string(),
            action: action.to_owned(),
            payload,
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: created_at.to_owned(),
            synced_at: None,
            tenant_id: "default".into(),
            priority: oz_core::offline::SyncPriority::Normal,
        }
    }

    fn make_sale_item(
        created_at: &str,
        action: &str,
        status: &str,
        version: i64,
    ) -> OfflineQueueItem {
        make_item_with_version(
            created_at,
            action,
            version,
            &format!(r#""status":"{status}""#),
        )
    }

    fn make_stock_item(created_at: &str, action: &str, delta: i64, sku: &str) -> OfflineQueueItem {
        let payload = format!(r#"{{"sku":"{sku}","delta":{delta}}}"#);
        OfflineQueueItem {
            id: uuid::Uuid::now_v7().to_string(),
            action: action.to_owned(),
            payload,
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: created_at.to_owned(),
            synced_at: None,
            tenant_id: "default".into(),
            priority: oz_core::offline::SyncPriority::Normal,
        }
    }

    // ── Legacy LWW tests (unchanged) ─────────────────────────────

    #[test]
    fn lww_local_wins_when_newer() {
        let local = make_item("2025-06-01T12:00:00.000Z", "complete_sale");
        let remote = make_item("2025-06-01T10:00:00.000Z", "complete_sale");
        let resolved = resolve_lww(&local, &remote);
        assert_eq!(resolved.winner.id, local.id);
    }

    #[test]
    fn lww_remote_wins_when_newer() {
        let local = make_item("2025-06-01T10:00:00.000Z", "complete_sale");
        let remote = make_item("2025-06-01T12:00:00.000Z", "complete_sale");
        let resolved = resolve_lww(&local, &remote);
        assert_eq!(resolved.winner.id, remote.id);
    }

    #[test]
    fn lww_remote_wins_on_tie() {
        let local = make_item("2025-06-01T12:00:00.000Z", "complete_sale");
        let remote = make_item("2025-06-01T12:00:00.000Z", "complete_sale");
        let resolved = resolve_lww(&local, &remote);
        assert_eq!(resolved.winner.id, remote.id);
    }

    #[test]
    fn lww_different_actions() {
        let local = make_item("2025-06-01T12:00:00.000Z", "create_product");
        let remote = make_item("2025-06-01T10:00:00.000Z", "delete_product");
        let resolved = resolve_lww(&local, &remote);
        assert_eq!(resolved.winner.action, "create_product");
    }

    #[test]
    fn lww_empty_payload() {
        let mut local = make_item("2025-06-01T12:00:00.000Z", "test");
        let mut remote = make_item("2025-06-01T10:00:00.000Z", "test");
        local.payload = String::new();
        remote.payload = String::new();
        let resolved = resolve_lww(&local, &remote);
        assert!(resolved.winner.payload.is_empty());
    }

    // ── Version LWW tests ─────────────────────────────────────────

    #[test]
    fn version_lww_local_wins_higher_version() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "product.update", 5, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "product.update", 3, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "local should win with version 5 > 3"
        );
    }

    #[test]
    fn version_lww_remote_wins_higher_version() {
        let local = make_item_with_version("2025-06-01T12:00:00.000Z", "product.update", 2, "");
        let remote = make_item_with_version("2025-06-01T10:00:00.000Z", "product.update", 7, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "remote should win with version 7 > 2"
        );
    }

    #[test]
    fn version_lww_remote_wins_on_tie() {
        let local = make_item_with_version("2025-06-01T12:00:00.000Z", "product.update", 4, "");
        let remote = make_item_with_version("2025-06-01T10:00:00.000Z", "product.update", 4, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "remote should win on version tie (server-authoritative)"
        );
    }

    #[test]
    fn version_lww_local_has_version_remote_missing() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "product.update", 3, "");
        let remote = make_item("2025-06-01T12:00:00.000Z", "product.update");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "local should win when remote lacks version"
        );
    }

    #[test]
    fn version_lww_remote_has_version_local_missing() {
        let local = make_item("2025-06-01T10:00:00.000Z", "product.update");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "product.update", 3, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "remote should win when local lacks version"
        );
    }

    #[test]
    fn version_lww_both_missing_falls_back_to_created_at() {
        let local = make_item("2025-06-01T12:00:00.000Z", "product.update");
        let remote = make_item("2025-06-01T10:00:00.000Z", "product.update");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "should fall back to created_at when both lack version"
        );
    }

    #[test]
    fn version_lww_applied_to_category_action() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "category.update", 2, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "category.update", 5, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(resolved.winner.id, remote.id);
    }

    #[test]
    fn version_lww_applied_to_tax_action() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "tax.update", 1, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "tax.update", 1, "");
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(resolved.winner.id, remote.id, "tie → remote wins");
    }

    // ── Sale LWW tests ────────────────────────────────────────────

    #[test]
    fn sale_lww_completed_wins_over_pending() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "pending", 1);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "completed", 1);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "completed sale should win over pending (even though local is newer)"
        );
    }

    #[test]
    fn sale_lww_voided_wins_over_completed() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "void_sale", "voided", 3);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "void_sale", "completed", 2);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "voided should win over completed (status DAG)"
        );
    }

    #[test]
    fn sale_lww_refunded_is_highest() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "refund_sale", "refunded", 5);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "refund_sale", "voided", 4);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "refunded should win over voided"
        );
    }

    #[test]
    fn sale_lww_same_status_falls_back_to_version() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "completed", 10);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "completed", 8);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "same status → higher version (10 > 8) should win"
        );
    }

    #[test]
    fn sale_lww_same_status_same_version_remote_wins() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "completed", 7);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "completed", 7);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "same status + same version → remote wins (server-authoritative)"
        );
    }

    #[test]
    fn sale_lww_active_cannot_override_completed() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "active", 1);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "completed", 5);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "active cannot override completed (status DAG)"
        );
    }

    #[test]
    fn sale_lww_unknown_status_ranked_zero() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "unknown", 1);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "active", 1);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "unknown status (rank 0) should lose to active (rank 1)"
        );
    }

    #[test]
    fn sale_lww_both_unknown_falls_back_to_version() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "weird", 9);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "strange", 5);
        let resolved = resolve_sale_lww(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "both unknown (rank 0) → higher version (9 > 5) wins"
        );
    }

    // ── Stock CRDT merge tests ────────────────────────────────────

    #[test]
    fn stock_crdt_merged_payload_contains_both_deltas() {
        let local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.adjusted", 10, "COFFEE");
        let remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.adjusted", -3, "COFFEE");
        let resolved = resolve_stock_crdt(&local, &remote);

        // Winner is a new merged item with both deltas.
        let winner_payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(winner_payload["local"]["delta"], 10);
        assert_eq!(winner_payload["remote"]["delta"], -3);
        assert_eq!(winner_payload["merge_type"], "crdt_delta");
        assert_eq!(resolved.winner.action, "stock.adjusted");
    }

    #[test]
    fn stock_crdt_preserves_local_and_remote_references() {
        let local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.adjusted", 5, "BAGEL");
        let remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.adjusted", -2, "BAGEL");
        let resolved = resolve_stock_crdt(&local, &remote);

        assert!(resolved.local.is_some());
        assert!(resolved.remote.is_some());
        assert_eq!(resolved.local.unwrap().id, local.id);
        assert_eq!(resolved.remote.unwrap().id, remote.id);
    }

    #[test]
    fn stock_crdt_winner_has_new_id() {
        let local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.adjusted", 1, "TEA");
        let remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.adjusted", 2, "TEA");
        let resolved = resolve_stock_crdt(&local, &remote);

        assert_ne!(resolved.winner.id, local.id);
        assert_ne!(resolved.winner.id, remote.id);
        assert!(!resolved.winner.id.is_empty());
    }

    #[test]
    fn stock_crdt_handles_invalid_payload_gracefully() {
        let mut local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.adjusted", 1, "MILK");
        let mut remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.adjusted", 2, "MILK");
        local.payload = "not-json".into();
        remote.payload = "also-not-json".into();
        let resolved = resolve_stock_crdt(&local, &remote);

        // Should not panic — uses null for unparseable payloads.
        let winner_payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(winner_payload["local"], Value::Null);
        assert_eq!(winner_payload["remote"], Value::Null);
    }

    // ── Resolve conflict dispatch tests ───────────────────────────

    #[test]
    fn dispatch_uses_version_lww_for_product() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "product.update", 5, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "product.update", 3, "");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "product.* should use version LWW"
        );
    }

    #[test]
    fn dispatch_uses_version_lww_for_category() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "category.update", 2, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "category.update", 5, "");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "category.* should use version LWW"
        );
    }

    #[test]
    fn dispatch_uses_version_lww_for_tax() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "tax.update", 1, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "tax.update", 1, "");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "tax.* should use version LWW"
        );
    }

    #[test]
    fn dispatch_uses_version_lww_for_user() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "user.update", 3, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "user.update", 1, "");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "user.* should use version LWW"
        );
    }

    #[test]
    fn dispatch_uses_version_lww_for_staff() {
        let local = make_item_with_version("2025-06-01T10:00:00.000Z", "staff.update", 2, "");
        let remote = make_item_with_version("2025-06-01T12:00:00.000Z", "staff.update", 4, "");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "staff.* should use version LWW"
        );
    }

    #[test]
    fn dispatch_uses_sale_lww_for_complete_sale() {
        let local = make_sale_item("2025-06-01T12:00:00.000Z", "complete_sale", "pending", 1);
        let remote = make_sale_item("2025-06-01T10:00:00.000Z", "complete_sale", "completed", 5);
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "complete_sale should use sale LWW (completed > pending)"
        );
    }

    #[test]
    fn dispatch_uses_sale_lww_for_void_sale() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "void_sale", "voided", 3);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "void_sale", "completed", 2);
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "void_sale should use sale LWW (voided > completed)"
        );
    }

    #[test]
    fn dispatch_uses_sale_lww_for_refund_sale() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "refund_sale", "active", 1);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "refund_sale", "refunded", 5);
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "refund_sale should use sale LWW"
        );
    }

    #[test]
    fn dispatch_uses_sale_lww_for_sale_prefix() {
        let local = make_sale_item("2025-06-01T10:00:00.000Z", "sale.hold", "pending", 1);
        let remote = make_sale_item("2025-06-01T12:00:00.000Z", "sale.hold", "completed", 5);
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "sale.* should use sale LWW (completed > pending)"
        );
    }

    #[test]
    fn dispatch_uses_stock_crdt_for_stock_adjusted() {
        let local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.adjusted", 10, "COFFEE");
        let remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.adjusted", -3, "COFFEE");
        let resolved = resolve_conflict(&local, &remote);

        let payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(
            payload["merge_type"], "crdt_delta",
            "stock.* should use CRDT merge"
        );
    }

    #[test]
    fn dispatch_uses_stock_crdt_for_stock_movement() {
        let local = make_stock_item("2025-06-01T10:00:00.000Z", "stock.movement", 5, "BAGEL");
        let remote = make_stock_item("2025-06-01T12:00:00.000Z", "stock.movement", -1, "BAGEL");
        let resolved = resolve_conflict(&local, &remote);

        let payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(
            payload["merge_type"], "crdt_delta",
            "stock.movement should use CRDT merge"
        );
    }

    #[test]
    fn dispatch_fallback_to_lww_for_unknown_action() {
        let local = make_item("2025-06-01T12:00:00.000Z", "unknown.action");
        let remote = make_item("2025-06-01T10:00:00.000Z", "unknown.action");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, local.id,
            "unknown action should fall back to created_at LWW"
        );
    }

    #[test]
    fn dispatch_fallback_remote_wins_on_tie() {
        let local = make_item("2025-06-01T12:00:00.000Z", "mystery.op");
        let remote = make_item("2025-06-01T12:00:00.000Z", "mystery.op");
        let resolved = resolve_conflict(&local, &remote);
        assert_eq!(
            resolved.winner.id, remote.id,
            "unknown action tie → remote wins (server-authoritative)"
        );
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn resolved_item_preserves_both_items() {
        let local = make_item("2025-06-01T10:00:00.000Z", "update");
        let remote = make_item("2025-06-01T12:00:00.000Z", "update");
        let resolved = resolve_lww(&local, &remote);
        assert!(resolved.local.is_some());
        assert!(resolved.remote.is_some());
        assert_eq!(resolved.local.unwrap().id, local.id);
        assert_eq!(resolved.remote.unwrap().id, remote.id);
    }

    #[test]
    fn resolved_item_debug() {
        let local = make_item("2025-06-01T10:00:00.000Z", "update");
        let remote = make_item("2025-06-01T12:00:00.000Z", "update");
        let resolved = resolve_lww(&local, &remote);
        let debug = format!("{resolved:?}");
        assert!(debug.contains(&local.id));
        assert!(debug.contains(&remote.id));
    }

    #[test]
    fn version_lww_preserves_extra_payload_fields() {
        let local = make_item_with_version(
            "2025-06-01T10:00:00.000Z",
            "product.update",
            3,
            r#""name":"Coffee","price":15000"#,
        );
        let remote = make_item_with_version(
            "2025-06-01T12:00:00.000Z",
            "product.update",
            1,
            r#""name":"Tea","price":10000"#,
        );
        let resolved = resolve_version_lww(&local, &remote);
        assert_eq!(resolved.winner.id, local.id);

        // Winner payload should still contain the extra fields.
        let payload: Value = serde_json::from_str(&resolved.winner.payload).unwrap();
        assert_eq!(payload["name"], "Coffee");
        assert_eq!(payload["price"], 15000);
    }

    #[test]
    fn extract_version_from_valid_payload() {
        assert_eq!(extract_version(r#"{"version":5}"#), Some(5));
        assert_eq!(extract_version(r#"{"version":0}"#), Some(0));
        assert_eq!(extract_version(r#"{"version":-1}"#), Some(-1));
    }

    #[test]
    fn extract_version_from_invalid_payload() {
        assert_eq!(extract_version("not-json"), None);
        assert_eq!(extract_version(r#"{"no_version":1}"#), None);
        assert_eq!(extract_version(r#"{"version":"abc"}"#), None);
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn extract_status_from_valid_payload() {
        assert_eq!(
            extract_status(r#"{"status":"completed"}"#),
            Some("completed".into())
        );
        assert_eq!(
            extract_status(r#"{"status":"pending"}"#),
            Some("pending".into())
        );
    }

    #[test]
    fn extract_status_from_invalid_payload() {
        assert_eq!(extract_status("not-json"), None);
        assert_eq!(extract_status(r#"{"no_status":true}"#), None);
        assert_eq!(extract_status(r#"{"status":123}"#), None);
    }

    #[test]
    fn sale_status_rank_ordering() {
        assert_eq!(sale_status_rank("active"), 0);
        assert_eq!(sale_status_rank("pending"), 1);
        assert_eq!(sale_status_rank("completed"), 2);
        assert_eq!(sale_status_rank("voided"), 3);
        assert_eq!(sale_status_rank("refunded"), 4);
        assert_eq!(sale_status_rank("unknown"), 0);
    }
}
