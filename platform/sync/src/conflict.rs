//! Conflict Resolution — strategies for resolving conflicts between local
//! and remote versions of the same data.
//!
//! Initially only Last-Write-Wins (LWW) is implemented, using the
//! `created_at` timestamp to determine the winner.

use crate::queue::ResolvedItem;
use oz_core::offline::OfflineQueueItem;

/// Resolve a conflict using Last-Write-Wins (LWW).
///
/// Compares the `created_at` timestamps of the local and remote items.
/// The item with the later timestamp wins. If timestamps are equal, the
/// remote item wins (server-authoritative).
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

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::offline::OfflineQueueStatus;

    fn make_item(created_at: &str, action: &str) -> OfflineQueueItem {
        OfflineQueueItem {
            id: uuid::Uuid::new_v4().to_string(),
            action: action.to_owned(),
            payload: "{}".to_owned(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: created_at.to_owned(),
            synced_at: None,
        }
    }

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
        // Remote wins on tie (server-authoritative).
        assert_eq!(resolved.winner.id, remote.id);
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
}
