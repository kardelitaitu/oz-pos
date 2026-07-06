//! Physical inventory / stock-counting domain types.
//!
//! [`StockCount`], [`StockCountLine`], and [`StockAdjustment`] represent
//! a cycle-counting workflow: create a count, add lines with expected vs.
//! counted quantities, complete the count to generate adjustments, and
//! reconcile inventory.

use serde::{Deserialize, Serialize};

/// The status of a stock count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockCountStatus {
    /// Newly created, not yet being counted.
    Draft,
    /// Actively being counted in the field.
    InProgress,
    /// Count completed and adjustments applied.
    Completed,
    /// Count cancelled, no adjustments made.
    Cancelled,
}

impl StockCountStatus {
    /// Return the serialized string used in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from a database string.
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// The type of stock count being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountType {
    /// Count every product in the store.
    Full,
    /// Count a subset of products (e.g., rotating schedule).
    Cyclic,
    /// Quick spot-check on a handful of SKUs.
    Spot,
}

impl CountType {
    /// Return the serialized string used in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Cyclic => "cyclic",
            Self::Spot => "spot",
        }
    }

    /// Parse from a database string.
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "cyclic" => Some(Self::Cyclic),
            "spot" => Some(Self::Spot),
            _ => None,
        }
    }
}

/// A physical inventory count session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockCount {
    /// UUID primary key.
    pub id: String,
    /// Human-readable count number (e.g., "CNT-20260706-001").
    pub count_number: String,
    /// Current status of the count.
    pub status: StockCountStatus,
    /// Type of count: full, cyclic, or spot.
    pub count_type: CountType,
    /// Free-text notes for this count session.
    pub notes: String,
    /// FK to `users.id` — who performed / is performing the count.
    pub counted_by: Option<String>,
    /// ISO-8601 timestamp of creation.
    pub created_at: String,
    /// ISO-8601 timestamp when the count was completed (null if not done).
    pub completed_at: Option<String>,
    /// ISO-8601 timestamp of last modification.
    pub updated_at: String,
}

/// A single line within a stock count — one SKU, expected vs counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockCountLine {
    /// UUID primary key.
    pub id: String,
    /// FK to `stock_counts.id`.
    pub count_id: String,
    /// The product SKU being counted.
    pub sku: String,
    /// Product display name (denormalised for convenience).
    pub product_name: String,
    /// Quantity expected based on current inventory.
    pub expected_qty: i64,
    /// Quantity actually counted (None = not yet counted).
    pub counted_qty: Option<i64>,
    /// Difference = counted_qty - expected_qty (0 if not counted).
    pub difference: i64,
    /// Per-line notes.
    pub notes: String,
}

/// A stock adjustment record created when a count is completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockAdjustment {
    /// UUID primary key.
    pub id: String,
    /// FK to `stock_counts.id` (nullable if adjustment is standalone).
    pub count_id: Option<String>,
    /// SKU of the adjusted product.
    pub sku: String,
    /// Product display name (denormalised).
    pub product_name: String,
    /// Quantity before adjustment.
    pub previous_qty: i64,
    /// New quantity after adjustment.
    pub adjusted_qty: i64,
    /// Reason for the adjustment.
    pub reason: String,
    /// FK to `users.id` — who created the adjustment.
    pub created_by: Option<String>,
    /// ISO-8601 timestamp.
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `StockCountStatus::as_str` and `from_db_str` are inverses for every variant.
    #[test]
    fn stock_count_status_roundtrip() {
        for s in [
            StockCountStatus::Draft,
            StockCountStatus::InProgress,
            StockCountStatus::Completed,
            StockCountStatus::Cancelled,
        ] {
            assert_eq!(StockCountStatus::from_db_str(s.as_str()), Some(s));
        }
    }

    /// Unrecognised strings must return `None`, not panic.
    #[test]
    fn stock_count_status_unknown_is_none() {
        assert_eq!(StockCountStatus::from_db_str(""), None);
        assert_eq!(StockCountStatus::from_db_str("Draft"), None); // case-sensitive
        assert_eq!(StockCountStatus::from_db_str("approved"), None);
    }

    /// `CountType::as_str` and `from_db_str` are inverses for every variant.
    #[test]
    fn count_type_roundtrip() {
        for c in [CountType::Full, CountType::Cyclic, CountType::Spot] {
            assert_eq!(CountType::from_db_str(c.as_str()), Some(c));
        }
    }

    /// Unrecognised strings must return `None`.
    #[test]
    fn count_type_unknown_is_none() {
        assert_eq!(CountType::from_db_str(""), None);
        assert_eq!(CountType::from_db_str("FULL"), None);
        assert_eq!(CountType::from_db_str("random"), None);
    }
}
