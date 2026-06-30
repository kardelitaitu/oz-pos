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
    /// Comma-separated item display names.
    pub items_summary: String,
    /// Total item count.
    pub item_count: i64,
    /// Special notes.
    pub notes: String,
}
