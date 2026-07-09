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

    // ── StockCountStatus ─────────────────────────────────────────

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

    #[test]
    fn stock_count_status_as_str_values() {
        assert_eq!(StockCountStatus::Draft.as_str(), "draft");
        assert_eq!(StockCountStatus::InProgress.as_str(), "in_progress");
        assert_eq!(StockCountStatus::Completed.as_str(), "completed");
        assert_eq!(StockCountStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn stock_count_status_serde_snake_case() {
        let json = serde_json::to_string(&StockCountStatus::InProgress).unwrap();
        assert_eq!(json, "\"in_progress\"");
        let json = serde_json::to_string(&StockCountStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn stock_count_status_serde_roundtrip() {
        for s in [
            StockCountStatus::Draft,
            StockCountStatus::InProgress,
            StockCountStatus::Completed,
            StockCountStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: StockCountStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn stock_count_status_debug() {
        assert!(!format!("{:?}", StockCountStatus::Draft).is_empty());
        assert!(!format!("{:?}", StockCountStatus::InProgress).is_empty());
    }

    // ── CountType ────────────────────────────────────────────────

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

    #[test]
    fn count_type_as_str_values() {
        assert_eq!(CountType::Full.as_str(), "full");
        assert_eq!(CountType::Cyclic.as_str(), "cyclic");
        assert_eq!(CountType::Spot.as_str(), "spot");
    }

    #[test]
    fn count_type_serde_snake_case() {
        let json = serde_json::to_string(&CountType::Full).unwrap();
        assert_eq!(json, "\"full\"");
        let json = serde_json::to_string(&CountType::Cyclic).unwrap();
        assert_eq!(json, "\"cyclic\"");
    }

    #[test]
    fn count_type_serde_roundtrip() {
        for c in [CountType::Full, CountType::Cyclic, CountType::Spot] {
            let json = serde_json::to_string(&c).unwrap();
            let back: CountType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn count_type_debug() {
        assert!(!format!("{:?}", CountType::Full).is_empty());
        assert!(!format!("{:?}", CountType::Spot).is_empty());
    }

    // ── StockCount struct ────────────────────────────────────────

    fn make_stock_count() -> StockCount {
        StockCount {
            id: "cnt-1".into(),
            count_number: "CNT-20260706-001".into(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "Monthly full count".into(),
            counted_by: None,
            created_at: "2026-07-06T08:00:00Z".into(),
            completed_at: None,
            updated_at: "2026-07-06T08:00:00Z".into(),
        }
    }

    #[test]
    fn stock_count_serde_roundtrip() {
        let sc = make_stock_count();
        let json = serde_json::to_string(&sc).unwrap();
        let back: StockCount = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sc);
    }

    #[test]
    fn stock_count_serde_in_progress() {
        let sc = StockCount {
            status: StockCountStatus::InProgress,
            counted_by: Some("user-1".into()),
            ..make_stock_count()
        };
        let json = serde_json::to_string(&sc).unwrap();
        let back: StockCount = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, StockCountStatus::InProgress);
        assert_eq!(back.counted_by, Some("user-1".into()));
    }

    #[test]
    fn stock_count_serde_completed() {
        let sc = StockCount {
            status: StockCountStatus::Completed,
            completed_at: Some("2026-07-06T14:00:00Z".into()),
            ..make_stock_count()
        };
        let json = serde_json::to_string(&sc).unwrap();
        let back: StockCount = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, StockCountStatus::Completed);
        assert_eq!(back.completed_at, Some("2026-07-06T14:00:00Z".into()));
    }

    #[test]
    fn stock_count_debug() {
        let sc = make_stock_count();
        let debug = format!("{:?}", sc);
        assert!(debug.contains("CNT-20260706-001"));
        assert!(debug.contains("Draft"));
    }

    // ── StockCountLine struct ────────────────────────────────────

    fn make_count_line() -> StockCountLine {
        StockCountLine {
            id: "line-1".into(),
            count_id: "cnt-1".into(),
            sku: "COFFEE".into(),
            product_name: "Espresso".into(),
            expected_qty: 50,
            counted_qty: Some(48),
            difference: -2,
            notes: "Found 2 damaged".into(),
        }
    }

    #[test]
    fn stock_count_line_serde_roundtrip() {
        let line = make_count_line();
        let json = serde_json::to_string(&line).unwrap();
        let back: StockCountLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back, line);
    }

    #[test]
    fn stock_count_line_not_yet_counted() {
        let line = StockCountLine {
            counted_qty: None,
            difference: 0,
            ..make_count_line()
        };
        let json = serde_json::to_string(&line).unwrap();
        let back: StockCountLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back.counted_qty, None);
        assert_eq!(back.difference, 0);
    }

    #[test]
    fn stock_count_line_overcount() {
        let line = StockCountLine {
            expected_qty: 10,
            counted_qty: Some(15),
            difference: 5,
            ..make_count_line()
        };
        let json = serde_json::to_string(&line).unwrap();
        let back: StockCountLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back.counted_qty, Some(15));
        assert_eq!(back.difference, 5);
    }

    #[test]
    fn stock_count_line_debug() {
        let line = make_count_line();
        let debug = format!("{:?}", line);
        assert!(debug.contains("COFFEE"));
        assert!(debug.contains("Espresso"));
    }

    // ── StockAdjustment struct ───────────────────────────────────

    fn make_adjustment() -> StockAdjustment {
        StockAdjustment {
            id: "adj-1".into(),
            count_id: Some("cnt-1".into()),
            sku: "COFFEE".into(),
            product_name: "Espresso".into(),
            previous_qty: 50,
            adjusted_qty: 48,
            reason: "Stock count correction".into(),
            created_by: Some("user-1".into()),
            created_at: "2026-07-06T14:00:00Z".into(),
        }
    }

    #[test]
    fn stock_adjustment_serde_roundtrip() {
        let adj = make_adjustment();
        let json = serde_json::to_string(&adj).unwrap();
        let back: StockAdjustment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, adj);
    }

    #[test]
    fn stock_adjustment_standalone_no_count() {
        let adj = StockAdjustment {
            count_id: None,
            created_by: None,
            ..make_adjustment()
        };
        let json = serde_json::to_string(&adj).unwrap();
        let back: StockAdjustment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.count_id, None);
        assert_eq!(back.created_by, None);
    }

    #[test]
    fn stock_adjustment_debug() {
        let adj = make_adjustment();
        let debug = format!("{:?}", adj);
        assert!(debug.contains("COFFEE"));
        assert!(debug.contains("correction"));
    }
}
