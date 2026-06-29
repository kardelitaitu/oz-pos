//! Cash Payout (Safe Drop) domain type.
//!
//! A cash payout represents cash physically removed from the drawer
//! mid-shift (e.g. a bank drop or manager pickup). Payouts reduce the
//! expected cash calculation at shift close.

use serde::{Deserialize, Serialize};

/// A mid-shift cash removal from the drawer (safe drop).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashPayout {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to `shifts.id`.
    pub shift_id: String,
    /// Amount removed in minor units (must be > 0).
    pub amount_minor: i64,
    /// Reason for the payout (e.g. "bank drop", "manager pickup").
    pub reason: String,
    /// ISO-8601 timestamp.
    pub created_at: String,
}

impl CashPayout {
    /// Create a new CashPayout with a generated UUID.
    pub fn new(shift_id: impl Into<String>, amount_minor: i64, reason: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            shift_id: shift_id.into(),
            amount_minor,
            reason: reason.into(),
            created_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cash_payout_has_generated_id() {
        let p = CashPayout::new("shift-1", 5000, "bank drop");
        assert!(!p.id.is_empty());
        assert_eq!(p.shift_id, "shift-1");
        assert_eq!(p.amount_minor, 5000);
        assert_eq!(p.reason, "bank drop");
        assert!(p.created_at.contains('T'));
    }

    #[test]
    fn serde_roundtrip() {
        let p = CashPayout::new("shift-1", 10000, "manager pickup");
        let json = serde_json::to_string(&p).unwrap();
        let back: CashPayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, p.id);
        assert_eq!(back.amount_minor, 10000);
        assert_eq!(back.reason, "manager pickup");
    }
}
