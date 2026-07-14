//! Offline queue domain type — queued transactions for later sync.

use serde::{Deserialize, Serialize};

/// Sync priority tier for offline queue items (P-2 spec §Priority tiers).
///
/// Lower numeric values indicate higher priority. Items are sorted by
/// priority before batching so Critical items always transmit before
/// Normal items, which always transmit before Low items.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(i32)]
pub enum SyncPriority {
    /// Sale completions, voids — must propagate before anything else.
    Critical = 0,
    /// Product creation, stock adjustments, inventory changes.
    Normal = 1,
    /// Settings changes, branding updates, low-urgency metadata.
    Low = 2,
}

/// Default priority for new queue items.
fn default_priority() -> SyncPriority {
    SyncPriority::Normal
}

impl From<i32> for SyncPriority {
    fn from(v: i32) -> Self {
        match v {
            0 => SyncPriority::Critical,
            2 => SyncPriority::Low,
            _ => SyncPriority::Normal,
        }
    }
}

/// A queued offline transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineQueueItem {
    /// Internal row id (UUID v4).
    pub id: String,
    /// The action to perform (e.g. "complete_sale", "void_sale").
    pub action: String,
    /// JSON-serialized payload for the action.
    pub payload: String,
    /// Queue status: pending, synced, or failed.
    pub status: OfflineQueueStatus,
    /// Number of retry attempts.
    pub retry_count: i64,
    /// Last error message, if any.
    pub last_error: Option<String>,
    /// Tenant / store ID for multi-tenant cloud isolation.
    /// Defaults to "default" for single-store deployments.
    #[serde(default = "default_tenant_id")]
    pub tenant_id: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 sync timestamp.
    pub synced_at: Option<String>,
    /// Sync priority tier (P-2). Critical items transmit before Normal/Low.
    #[serde(default = "default_priority")]
    pub priority: SyncPriority,
}

/// Status of an offline queue item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineQueueStatus {
    /// Waiting to be synced.
    Pending,
    /// Successfully synced to the server.
    Synced,
    /// Sync failed after multiple retries.
    Failed,
}

/// Default tenant ID for single-store deployments.
fn default_tenant_id() -> String {
    String::from("default")
}

impl OfflineQueueStatus {
    /// Return the status as a stored string value.
    pub fn as_stored_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Synced => "synced",
            Self::Failed => "failed",
        }
    }

    /// Parse a stored string value into a status.
    pub fn from_stored_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "synced" => Some(Self::Synced),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

impl OfflineQueueItem {
    /// Create a new offline queue item with the default tenant ("default").
    pub fn new(action: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            action: action.into(),
            payload: payload.into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            synced_at: None,
            tenant_id: String::from("default"),
            priority: SyncPriority::Normal,
        }
    }

    /// Create a new offline queue item scoped to the given tenant.
    pub fn with_tenant(
        action: impl Into<String>,
        payload: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        let mut item = Self::new(action, payload);
        item.tenant_id = tenant_id.into();
        item
    }

    /// Create a new queue item with a specific sync priority.
    pub fn with_priority(
        action: impl Into<String>,
        payload: impl Into<String>,
        priority: SyncPriority,
    ) -> Self {
        let mut item = Self::new(action, payload);
        item.priority = priority;
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_queue_item_sets_fields() {
        let item = OfflineQueueItem::new("complete_sale", r#"{"sale_id":"abc"}"#);
        assert_eq!(item.action, "complete_sale");
        assert_eq!(item.payload, r#"{"sale_id":"abc"}"#);
        assert!(!item.id.is_empty());
        assert!(item.created_at.contains('T'));
    }

    #[test]
    fn queue_item_defaults_to_pending() {
        let item = OfflineQueueItem::new("void_sale", "{}");
        assert_eq!(item.status, OfflineQueueStatus::Pending);
        assert_eq!(item.retry_count, 0);
        assert!(item.last_error.is_none());
        assert!(item.synced_at.is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let item = OfflineQueueItem::new("complete_sale", r#"{"total":1000}"#);
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: OfflineQueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.action, item.action);
        assert_eq!(deserialized.payload, item.payload);
        assert_eq!(deserialized.status, item.status);
    }

    #[test]
    fn status_roundtrip() {
        for (s, expected) in &[
            ("pending", OfflineQueueStatus::Pending),
            ("synced", OfflineQueueStatus::Synced),
            ("failed", OfflineQueueStatus::Failed),
        ] {
            assert_eq!(OfflineQueueStatus::from_stored_str(s), Some(*expected));
            assert_eq!(expected.as_stored_str(), *s);
        }
        assert_eq!(OfflineQueueStatus::from_stored_str("unknown"), None);
    }

    // ── OfflineQueueItem additional tests ────────────────────────────

    #[test]
    fn queue_item_debug_output() {
        let item = OfflineQueueItem::new("complete_sale", r#"{"total":1000}"#);
        let debug = format!("{item:?}");
        assert!(debug.contains("complete_sale"));
        assert!(debug.contains("Pending"));
        assert!(debug.contains(&item.id));
    }

    #[test]
    fn queue_item_clone_eq() {
        let item = OfflineQueueItem::new("void_sale", "{}");
        let cloned = item.clone();
        assert_eq!(item, cloned);
        assert_eq!(item.id, cloned.id);
        assert_eq!(item.action, cloned.action);
        assert_eq!(item.payload, cloned.payload);
        assert_eq!(item.status, cloned.status);
        assert_eq!(item.retry_count, cloned.retry_count);
        assert_eq!(item.last_error, cloned.last_error);
    }

    #[test]
    fn queue_item_json_field_names() {
        let item = OfflineQueueItem::new("complete_sale", r#"{"total":1000}"#);
        let json = serde_json::to_value(&item).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("action"));
        assert!(obj.contains_key("payload"));
        assert!(obj.contains_key("status"));
        assert!(obj.contains_key("retry_count"));
        assert!(obj.contains_key("last_error"));
        assert!(obj.contains_key("created_at"));
        assert!(obj.contains_key("synced_at"));
    }

    #[test]
    fn queue_item_with_error_field() {
        let mut item = OfflineQueueItem::new("void_sale", "{}");
        item.last_error = Some("timeout".into());
        let json = serde_json::to_string(&item).unwrap();
        let roundtripped: OfflineQueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.last_error, Some("timeout".into()));
    }

    // ── OfflineQueueStatus additional tests ──────────────────────────

    #[test]
    fn status_debug_output() {
        assert!(format!("{:?}", OfflineQueueStatus::Pending).contains("Pending"));
        assert!(format!("{:?}", OfflineQueueStatus::Synced).contains("Synced"));
        assert!(format!("{:?}", OfflineQueueStatus::Failed).contains("Failed"));
    }

    #[test]
    fn status_serde_json_format() {
        assert_eq!(
            serde_json::to_value(OfflineQueueStatus::Pending).unwrap(),
            "pending"
        );
        assert_eq!(
            serde_json::to_value(OfflineQueueStatus::Synced).unwrap(),
            "synced"
        );
        assert_eq!(
            serde_json::to_value(OfflineQueueStatus::Failed).unwrap(),
            "failed"
        );
    }

    #[test]
    fn status_serde_roundtrip() {
        for status in &[
            OfflineQueueStatus::Pending,
            OfflineQueueStatus::Synced,
            OfflineQueueStatus::Failed,
        ] {
            let json = serde_json::to_string(status).unwrap();
            let rt: OfflineQueueStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*status, rt);
        }
    }

    #[test]
    fn status_from_stored_str_invalid_cases() {
        assert_eq!(OfflineQueueStatus::from_stored_str(""), None);
        assert_eq!(OfflineQueueStatus::from_stored_str("PENDING"), None);
        assert_eq!(OfflineQueueStatus::from_stored_str("  pending  "), None);
    }

    #[test]
    fn status_as_stored_str_all_variants() {
        assert_eq!(OfflineQueueStatus::Pending.as_stored_str(), "pending");
        assert_eq!(OfflineQueueStatus::Synced.as_stored_str(), "synced");
        assert_eq!(OfflineQueueStatus::Failed.as_stored_str(), "failed");
    }

    #[test]
    fn queue_item_new_generates_unique_ids() {
        let a = OfflineQueueItem::new("act", "{}");
        let b = OfflineQueueItem::new("act", "{}");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn queue_item_new_has_rfc3339_timestamp() {
        let item = OfflineQueueItem::new("act", "{}");
        assert!(item.created_at.contains('T'));
        assert!(item.created_at.ends_with('Z'));
    }
}
