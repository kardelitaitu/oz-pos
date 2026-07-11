//! Kitchen Display System (KDS) domain types.
//!
//! Types for order tickets that route completed sales to the kitchen
//! display system with status tracking and timestamps.

use serde::{Deserialize, Serialize};

/// Status of a KDS order in the kitchen workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KdsStatus {
    /// Order received, not yet being worked on.
    Pending,
    /// Kitchen is actively preparing the order.
    Preparing,
    /// Order is ready to be served.
    Ready,
    /// Order has been served to the customer.
    Served,
    /// Order was cancelled.
    Cancelled,
}

impl KdsStatus {
    /// Serialize to the database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Preparing => "preparing",
            Self::Ready => "ready",
            Self::Served => "served",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from a database string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "preparing" => Some(Self::Preparing),
            "ready" => Some(Self::Ready),
            "served" => Some(Self::Served),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// A KDS order ticket displayed in the kitchen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdsOrder {
    /// Primary key (UUID v4).
    pub id: String,
    /// FK to the originating sale.
    pub sale_id: String,
    /// The store where the order belongs (ADR #8).
    ///
    /// Populated from the sale's store context. Used by KDS tablets
    /// to filter orders for defense-in-depth in multi-store deployments.
    pub store_id: Option<String>,
    /// Current kitchen status ("pending", "preparing", "ready", "served", "cancelled").
    pub status: String,
    /// Comma-separated item names for display.
    pub items_summary: String,
    /// Total number of items in the order.
    pub item_count: i64,
    /// Human-readable display number (auto-increment per day).
    pub display_number: Option<i64>,
    /// ISO-8601 timestamp of when the order was received.
    pub received_at: String,
    /// ISO-8601 timestamp of when preparation started.
    pub started_at: Option<String>,
    /// ISO-8601 timestamp of when preparation finished.
    pub ready_at: Option<String>,
    /// ISO-8601 timestamp of when the order was served.
    pub served_at: Option<String>,
    /// Estimated preparation time in seconds.
    pub prep_time_seconds: i64,
    /// Special notes from the POS (e.g., "no onions").
    pub notes: String,
}

/// Input for creating a KDS order from a completed sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKdsOrderInput {
    /// FK to the originating sale.
    pub sale_id: String,
    /// The store where this order belongs (ADR #8).
    pub store_id: Option<String>,
    /// Comma-separated item display names.
    pub items_summary: String,
    /// Total item count.
    pub item_count: i64,
    /// Special notes.
    pub notes: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── KdsStatus as_str ───────────────────────────────────────────

    #[test]
    fn status_as_str_all_variants() {
        assert_eq!(KdsStatus::Pending.as_str(), "pending");
        assert_eq!(KdsStatus::Preparing.as_str(), "preparing");
        assert_eq!(KdsStatus::Ready.as_str(), "ready");
        assert_eq!(KdsStatus::Served.as_str(), "served");
        assert_eq!(KdsStatus::Cancelled.as_str(), "cancelled");
    }

    // ── KdsStatus from_str ─────────────────────────────────────────

    #[test]
    fn status_from_str_all_variants() {
        assert_eq!(KdsStatus::from_str("pending"), Some(KdsStatus::Pending));
        assert_eq!(KdsStatus::from_str("preparing"), Some(KdsStatus::Preparing));
        assert_eq!(KdsStatus::from_str("ready"), Some(KdsStatus::Ready));
        assert_eq!(KdsStatus::from_str("served"), Some(KdsStatus::Served));
        assert_eq!(KdsStatus::from_str("cancelled"), Some(KdsStatus::Cancelled));
    }

    #[test]
    fn status_from_str_invalid() {
        assert_eq!(KdsStatus::from_str("bogus"), None);
        assert_eq!(KdsStatus::from_str(""), None);
        assert_eq!(KdsStatus::from_str("PENDING"), None);
    }

    #[test]
    fn status_from_str_roundtrip() {
        for s in &[
            KdsStatus::Pending,
            KdsStatus::Preparing,
            KdsStatus::Ready,
            KdsStatus::Served,
            KdsStatus::Cancelled,
        ] {
            assert_eq!(KdsStatus::from_str(s.as_str()), Some(s.clone()));
        }
    }

    // ── Serde roundtrips ───────────────────────────────────────────

    #[test]
    fn kds_status_serde_roundtrip() {
        let status = KdsStatus::Ready;
        let json = serde_json::to_string(&status).unwrap();
        let back: KdsStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, KdsStatus::Ready);
    }

    #[test]
    fn kds_order_serde_roundtrip() {
        let order = KdsOrder {
            id: "o-1".into(),
            sale_id: "s-1".into(),
            store_id: Some("store-default".into()),
            status: "pending".into(),
            items_summary: "Coffee x2, Bagel".into(),
            item_count: 3,
            display_number: Some(1),
            received_at: "2025-01-01T12:00:00.000Z".into(),
            started_at: None,
            ready_at: None,
            served_at: None,
            prep_time_seconds: 300,
            notes: "No onions".into(),
        };
        let json = serde_json::to_string(&order).unwrap();
        let back: KdsOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, order.id);
        assert_eq!(back.sale_id, order.sale_id);
        assert_eq!(back.status, order.status);
        assert_eq!(back.items_summary, order.items_summary);
        assert_eq!(back.item_count, order.item_count);
        assert_eq!(back.prep_time_seconds, order.prep_time_seconds);
        assert_eq!(back.notes, order.notes);
    }

    #[test]
    fn create_kds_order_input_serde_roundtrip() {
        let input = CreateKdsOrderInput {
            sale_id: "s-1".into(),
            store_id: None,
            items_summary: "Tea".into(),
            item_count: 1,
            notes: String::new(),
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: CreateKdsOrderInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sale_id, "s-1");
        assert_eq!(back.items_summary, "Tea");
        assert_eq!(back.item_count, 1);
        assert_eq!(back.notes, "");
    }

    #[test]
    fn kds_order_optional_timestamps() {
        let order = KdsOrder {
            id: "o-2".into(),
            sale_id: "s-2".into(),
            store_id: None,
            status: "served".into(),
            items_summary: "Done".into(),
            item_count: 1,
            display_number: None,
            received_at: "2025-01-01T12:00:00.000Z".into(),
            started_at: Some("2025-01-01T12:05:00.000Z".into()),
            ready_at: Some("2025-01-01T12:10:00.000Z".into()),
            served_at: Some("2025-01-01T12:12:00.000Z".into()),
            prep_time_seconds: 720,
            notes: String::new(),
        };
        assert_eq!(
            order.started_at.as_deref(),
            Some("2025-01-01T12:05:00.000Z")
        );
        assert_eq!(order.ready_at.as_deref(), Some("2025-01-01T12:10:00.000Z"));
        assert_eq!(order.served_at.as_deref(), Some("2025-01-01T12:12:00.000Z"));
        assert!(order.display_number.is_none());
    }
}
