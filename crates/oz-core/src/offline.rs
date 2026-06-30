//! Offline queue domain type — queued transactions for later sync.

use serde::{Deserialize, Serialize};

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
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 sync timestamp.
    pub synced_at: Option<String>,
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
    /// Create a new offline queue item.
    pub fn new(action: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action: action.into(),
            payload: payload.into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            synced_at: None,
        }
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
}
