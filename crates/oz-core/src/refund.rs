//! Refund domain type — a refund linked to a completed sale.

use crate::Money;
use serde::{Deserialize, Serialize};

/// A refund against a completed sale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refund {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to the original sale.
    pub sale_id: String,
    /// Total refund amount in minor units.
    pub total: Money,
    /// Reason for the refund.
    pub reason: String,
    /// Internal note about the refund.
    pub note: String,
    /// User ID of the staff member who processed the refund.
    pub processed_by: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Line items being refunded.
    pub lines: Vec<RefundLine>,
}

/// A single line item within a refund.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefundLine {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to the refund.
    pub refund_id: String,
    /// FK to the original sale line.
    pub sale_line_id: String,
    /// SKU of the refunded product.
    pub sku: String,
    /// Quantity refunded.
    pub qty: i64,
    /// Unit price at time of refund.
    pub unit_price: Money,
    /// Line total (unit_price * qty).
    pub line_total: Money,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl Refund {
    /// Create a new refund for the given sale.
    pub fn new(
        sale_id: impl Into<String>,
        total: Money,
        reason: impl Into<String>,
        note: impl Into<String>,
        processed_by: impl Into<String>,
        lines: Vec<RefundLine>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            sale_id: sale_id.into(),
            total,
            reason: reason.into(),
            note: note.into(),
            processed_by: processed_by.into(),
            created_at: now.clone(),
            lines: lines
                .into_iter()
                .map(|mut l| {
                    l.refund_id = id.clone();
                    if l.created_at.is_empty() {
                        l.created_at = now.clone();
                    }
                    l
                })
                .collect(),
        }
    }
}

impl RefundLine {
    /// Create a new refund line.
    pub fn new(
        sale_line_id: impl Into<String>,
        sku: impl Into<String>,
        qty: i64,
        unit_price: Money,
        line_total: Money,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            refund_id: String::new(), // filled by Refund::new
            sale_line_id: sale_line_id.into(),
            sku: sku.into(),
            qty,
            unit_price,
            line_total,
            created_at: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Currency;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[test]
    fn test_new_refund_sets_fields() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 1000,
                currency: usd(),
            },
            "broken",
            "note here",
            "user-1",
            vec![],
        );
        assert_eq!(r.sale_id, "sale-1");
        assert_eq!(r.total.minor_units, 1000);
        assert_eq!(r.reason, "broken");
        assert_eq!(r.note, "note here");
        assert_eq!(r.processed_by, "user-1");
        assert!(!r.id.is_empty());
        assert!(!r.created_at.is_empty());
        assert!(r.lines.is_empty());
    }

    #[test]
    fn test_refund_line_created() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        assert_eq!(line.sale_line_id, "sl-1");
        assert_eq!(line.sku, "COFFEE");
        assert_eq!(line.qty, 2);
        assert_eq!(line.unit_price.minor_units, 350);
        assert_eq!(line.line_total.minor_units, 700);
        assert!(!line.id.is_empty());
    }

    #[test]
    fn test_serde_roundtrip() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 700,
                currency: usd(),
            },
            "broken",
            "",
            "user-1",
            vec![line],
        );
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: Refund = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, r.id);
        assert_eq!(deserialized.sale_id, r.sale_id);
        assert_eq!(deserialized.total.minor_units, r.total.minor_units);
        assert_eq!(deserialized.reason, r.reason);
        assert_eq!(deserialized.processed_by, r.processed_by);
        assert_eq!(deserialized.lines.len(), 1);
        assert_eq!(deserialized.lines[0].sku, "COFFEE");
        assert_eq!(deserialized.lines[0].qty, 2);
    }

    #[test]
    fn test_refund_empty_lines() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 0,
                currency: usd(),
            },
            "no refund",
            "",
            "user-1",
            vec![],
        );
        assert!(r.lines.is_empty());
        assert_eq!(r.total.minor_units, 0);
    }

    #[test]
    fn test_refund_multiple_lines() {
        let line1 = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        let line2 = RefundLine::new(
            "sl-2",
            "BAGEL",
            1,
            Money {
                minor_units: 450,
                currency: usd(),
            },
            Money {
                minor_units: 450,
                currency: usd(),
            },
        );
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 1150,
                currency: usd(),
            },
            "customer returned items",
            "",
            "user-1",
            vec![line1, line2],
        );
        assert_eq!(r.lines.len(), 2);
        assert_eq!(r.lines[0].sku, "COFFEE");
        assert_eq!(r.lines[1].sku, "BAGEL");
        assert_eq!(r.total.minor_units, 1150);
    }

    #[test]
    fn test_refund_assigns_refund_id_to_lines() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            1,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 350,
                currency: usd(),
            },
        );
        assert!(line.refund_id.is_empty()); // not yet assigned
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 350,
                currency: usd(),
            },
            "test",
            "",
            "user-1",
            vec![line],
        );
        assert_eq!(r.lines[0].refund_id, r.id);
    }

    #[test]
    fn test_refund_assigns_timestamps_to_lines() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            1,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 350,
                currency: usd(),
            },
        );
        assert!(line.created_at.is_empty());
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 350,
                currency: usd(),
            },
            "test",
            "",
            "user-1",
            vec![line],
        );
        assert!(
            !r.lines[0].created_at.is_empty(),
            "timestamp should be assigned"
        );
        assert!(r.lines[0].created_at.contains('T'));
    }

    #[test]
    fn test_refund_generates_uuid() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 100,
                currency: usd(),
            },
            "test",
            "",
            "user-1",
            vec![],
        );
        assert_eq!(r.id.len(), 36);
        assert_eq!(r.id.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn test_refund_debug_output() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 100,
                currency: usd(),
            },
            "reason",
            "note",
            "user-1",
            vec![],
        );
        let debug = format!("{:?}", r);
        assert!(debug.contains("sale-1"));
        assert!(debug.contains("reason"));
        assert!(debug.contains("note"));
    }

    #[test]
    fn test_refund_zero_total() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 0,
                currency: usd(),
            },
            "free return",
            "",
            "user-1",
            vec![],
        );
        assert_eq!(r.total.minor_units, 0);
    }

    #[test]
    fn test_refund_empty_reason() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 500,
                currency: usd(),
            },
            "",
            "",
            "user-1",
            vec![],
        );
        assert_eq!(r.reason, "");
    }

    #[test]
    fn test_refund_empty_note() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 500,
                currency: usd(),
            },
            "broken",
            "",
            "user-1",
            vec![],
        );
        assert_eq!(r.note, "");
    }

    #[test]
    fn test_refund_large_total() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: i64::MAX,
                currency: usd(),
            },
            "test",
            "",
            "user-1",
            vec![],
        );
        assert_eq!(r.total.minor_units, i64::MAX);
    }

    // ── RefundLine edge cases ──────────────────────────────────

    #[test]
    fn test_refund_line_debug_output() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        let debug = format!("{:?}", line);
        assert!(debug.contains("COFFEE"));
        assert!(debug.contains("sl-1"));
    }

    #[test]
    fn test_refund_line_zero_qty() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            0,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 0,
                currency: usd(),
            },
        );
        assert_eq!(line.qty, 0);
        assert_eq!(line.line_total.minor_units, 0);
    }

    #[test]
    fn test_refund_line_large_qty() {
        let line = RefundLine::new(
            "sl-1",
            "BULK",
            999,
            Money {
                minor_units: 100,
                currency: usd(),
            },
            Money {
                minor_units: 99900,
                currency: usd(),
            },
        );
        assert_eq!(line.qty, 999);
        assert_eq!(line.line_total.minor_units, 99900);
    }

    #[test]
    fn test_refund_line_empty_sku() {
        let line = RefundLine::new(
            "sl-1",
            "",
            1,
            Money {
                minor_units: 0,
                currency: usd(),
            },
            Money {
                minor_units: 0,
                currency: usd(),
            },
        );
        assert_eq!(line.sku, "");
    }

    #[test]
    fn test_refund_line_serde_roundtrip() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        let json = serde_json::to_string(&line).unwrap();
        let back: RefundLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sale_line_id, "sl-1");
        assert_eq!(back.sku, "COFFEE");
        assert_eq!(back.qty, 2);
        assert_eq!(back.unit_price.minor_units, 350);
        assert_eq!(back.line_total.minor_units, 700);
    }

    #[test]
    fn test_refund_line_clone_eq() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
            Money {
                minor_units: 700,
                currency: usd(),
            },
        );
        let cloned = line.clone();
        assert_eq!(line, cloned);
    }

    #[test]
    fn test_refund_line_generates_uuid() {
        let line = RefundLine::new(
            "sl-1",
            "COFFEE",
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
            Money {
                minor_units: 100,
                currency: usd(),
            },
        );
        assert_eq!(line.id.len(), 36);
        assert_eq!(line.id.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn test_refund_line_sale_line_id_empty() {
        let line = RefundLine::new(
            "",
            "COFFEE",
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
            Money {
                minor_units: 100,
                currency: usd(),
            },
        );
        assert_eq!(line.sale_line_id, "");
    }

    #[test]
    fn test_refund_two_unique_ids() {
        let r1 = Refund::new(
            "sale-1",
            Money {
                minor_units: 100,
                currency: usd(),
            },
            "a",
            "",
            "user-1",
            vec![],
        );
        let r2 = Refund::new(
            "sale-2",
            Money {
                minor_units: 200,
                currency: usd(),
            },
            "b",
            "",
            "user-1",
            vec![],
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn test_refund_serde_empty_reason() {
        let r = Refund::new(
            "sale-1",
            Money {
                minor_units: 500,
                currency: usd(),
            },
            "",
            "",
            "user-1",
            vec![],
        );
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["reason"], "");
        assert_eq!(json["note"], "");
    }
}
