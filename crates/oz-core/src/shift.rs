//! Shift domain type — cashier shift with cash reconciliation.
//!
//! A [`Shift`] tracks the open/close lifecycle of a cashier session:
//! opening and closing cash balances, expected cash from sales, and
//! aggregated sales breakdowns by payment method.

use serde::{Deserialize, Serialize};

/// A cashier shift with opening/closing balance and sales aggregation.
///
/// # Schema mapping
///
/// Maps 1:1 to the `shifts` table (migration `021_shifts.sql`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shift {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to `users.id` — the staff member who opened the shift.
    pub user_id: String,
    /// FK to `terminals.id` — which terminal this shift was opened on.
    pub terminal_id: Option<String>,
    /// ISO-8601 timestamp when the shift was opened.
    pub opened_at: String,
    /// ISO-8601 timestamp when the shift was closed (None if still open).
    pub closed_at: Option<String>,
    /// Cash in the drawer at shift open (minor units).
    pub opening_balance_minor: i64,
    /// Cash counted at shift close (minor units). None if shift is still open.
    pub closing_balance_minor: Option<i64>,
    /// Expected cash: opening + cash sales - cash payouts (minor units).
    pub expected_cash_minor: Option<i64>,
    /// Difference: closing - expected (positive = over, negative = short).
    pub cash_difference_minor: Option<i64>,
    /// Total sales amount during the shift (minor units).
    pub total_sales_minor: i64,
    /// Cash sales amount (minor units).
    pub total_cash_minor: i64,
    /// Card sales amount (minor units).
    pub total_card_minor: i64,
    /// Other payment method sales (minor units).
    pub total_other_minor: i64,
    /// Total voided amount during the shift (minor units).
    pub total_voids_minor: i64,
    /// Total refunded amount during the shift (minor units).
    pub total_refunds_minor: i64,
    /// Total amount of cash payouts (safe drops) during the shift.
    pub total_payouts_minor: i64,
    /// Optional manager notes about the shift.
    pub notes: String,
    /// Shift status: `"open"` or `"closed"`.
    pub status: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl Shift {
    /// Create a new shift with the given user and terminal.
    ///
    /// Generates a fresh UUID for `id`. All monetary fields default to 0.
    /// Status is `"open"`. `opened_at` is auto-generated.
    ///
    /// # Panics
    ///
    /// Panics if `user_id` is empty after trimming.
    pub fn new(
        user_id: impl Into<String>,
        terminal_id: Option<impl Into<String>>,
        opening_balance_minor: i64,
    ) -> Self {
        let user_id = user_id.into().trim().to_owned();
        assert!(!user_id.is_empty(), "user_id must not be empty");
        assert!(
            opening_balance_minor >= 0,
            "opening_balance_minor must be ≥ 0"
        );

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            terminal_id: terminal_id.map(|t| t.into()),
            opened_at: now.clone(),
            closed_at: None,
            opening_balance_minor,
            closing_balance_minor: None,
            expected_cash_minor: None,
            cash_difference_minor: None,
            total_sales_minor: 0,
            total_cash_minor: 0,
            total_card_minor: 0,
            total_other_minor: 0,
            total_voids_minor: 0,
            total_refunds_minor: 0,
            total_payouts_minor: 0,
            notes: String::new(),
            status: "open".to_owned(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Whether this shift is currently open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == "open"
    }

    /// Whether this shift is closed.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.status == "closed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_shift_has_generated_id() {
        let s = Shift::new("user-1", Some("term-1"), 100);
        assert!(!s.id.is_empty());
        assert_eq!(s.user_id, "user-1");
        assert_eq!(s.terminal_id.as_deref(), Some("term-1"));
        assert_eq!(s.opening_balance_minor, 100);
        assert!(s.is_open());
        assert!(!s.is_closed());
        assert!(s.closed_at.is_none());
        assert!(s.closing_balance_minor.is_none());
        assert_eq!(s.total_sales_minor, 0);
        assert_eq!(s.total_cash_minor, 0);
        assert_eq!(s.total_card_minor, 0);
        assert_eq!(s.total_other_minor, 0);
        assert_eq!(s.total_voids_minor, 0);
        assert_eq!(s.total_refunds_minor, 0);
        assert!(s.opened_at.contains('T'));
        assert!(s.opened_at.ends_with('Z'));
    }

    #[test]
    fn new_shift_without_terminal() {
        let s = Shift::new("user-1", None::<String>, 0);
        assert!(s.terminal_id.is_none());
    }

    #[test]
    fn new_shift_default_opening_balance() {
        let s = Shift::new("user-2", None::<String>, 0);
        assert_eq!(s.opening_balance_minor, 0);
    }

    #[test]
    #[should_panic(expected = "user_id must not be empty")]
    fn new_shift_panics_on_empty_user() {
        Shift::new("", None::<String>, 0);
    }

    #[test]
    #[should_panic(expected = "opening_balance_minor must be ≥ 0")]
    fn new_shift_panics_on_negative_balance() {
        Shift::new("user-1", None::<String>, -1);
    }

    #[test]
    fn is_open_and_is_closed() {
        let s = Shift::new("user-1", None::<String>, 0);
        assert!(s.is_open());
        assert!(!s.is_closed());
    }

    #[test]
    fn serde_roundtrip() {
        let s = Shift::new("user-1", Some("term-1"), 500);
        let json = serde_json::to_string(&s).unwrap();
        let back: Shift = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, s.id);
        assert_eq!(back.user_id, s.user_id);
        assert_eq!(back.opening_balance_minor, 500);
        assert_eq!(back.total_payouts_minor, 0);
        assert_eq!(back.status, "open");
    }
}
