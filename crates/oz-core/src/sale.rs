//! Sale domain type — the transaction lifecycle.
//!
//! A [`Sale`] represents a completed or in-progress point-of-sale
//! transaction. It is built from a [`Cart`] via [`Sale::from_cart`],
//! progresses through a state machine (`Pending → Active → Completed |
//! Voided`), and maps 1:1 to the `sales` / `sale_lines` tables
//! (migration `001_sales.sql`).

use serde::{Deserialize, Serialize};

use foundation::{Cart, Currency, InvalidTransition, Money, SaleStatus};

/// A single line item within a sale.
///
/// # Schema mapping
///
/// Consolidates the `sale_lines` table columns (`unit_minor` +
/// `currency` → `unit_price: Money`, `line_minor` + `currency` →
/// `line_total: Money`). The database layer handles the mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaleLine {
    /// Internal row id (UUID v4).
    pub id: String,

    /// FK to `sales.id`.
    pub sale_id: String,

    /// The product SKU at time of sale.
    pub sku: String,

    /// Quantity sold (≥ 1).
    pub qty: i64,

    /// Unit price at time of sale (in minor units).
    pub unit_price: Money,

    /// Line total: `unit_price * qty` (in minor units).
    pub line_total: Money,

    /// Ordinal position of this line within the sale (1-indexed).
    pub line_position: i64,

    /// Tax amount for this line (same currency as the sale).
    /// Defaults to zero when no tax has been computed.
    #[serde(default)]
    pub tax_amount: Money,

    /// Tax rate ID applied to this line.
    /// `None` when no tax was applied (e.g. tax-exempt product).
    #[serde(default)]
    pub tax_rate_id: Option<String>,
}

/// A point-of-sale transaction with line items and a state machine.
///
/// # Schema mapping
///
/// Maps to the `sales` table (migrations `001_sales.sql` +
/// `004_sale_status.sql` + `008_payments.sql`). The `total` field
/// consolidates `total_minor` + `currency` into a [`Money`]. Lines
/// are stored in `sale_lines`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sale {
    /// Internal row id (UUID v4).
    pub id: String,

    /// Current state of the sale.
    pub status: SaleStatus,

    /// Grand total, computed from lines.
    pub total: Money,

    /// Number of line items.
    pub line_count: i64,

    /// Currency for all monetary values in this sale.
    pub currency: Currency,

    /// Payment method used ("cash", "card", "other", or `None`).
    pub payment_method: Option<String>,

    /// Amount tendered by the customer in minor units (for cash).
    pub tendered_minor: Option<i64>,

    /// User ID of the cashier who processed this sale.
    #[serde(default)]
    pub user_id: Option<String>,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 timestamp of the last state transition.
    pub updated_at: String,

    /// Line items in positional order.
    pub lines: Vec<SaleLine>,

    /// Discount percentage applied (0-100). 0 means no discount.
    #[serde(default)]
    pub discount_percent: i64,

    /// Human-readable discount label (e.g. "Senior 10%").
    #[serde(default)]
    pub discount_label: Option<String>,

    /// Subtotal before discount (sum of line totals).
    /// Defaults to [`total`] when no subtotal has been computed
    /// (backward-compat with sales created before Phase 3).
    #[serde(default)]
    pub subtotal: Money,

    /// Total tax amount across all line items.
    /// Defaults to zero for sales created before Phase 3.
    #[serde(default)]
    pub tax_total: Money,

    /// Customer ID linked to this sale for loyalty tracking.
    #[serde(default)]
    pub customer_id: Option<String>,
}

impl Sale {
    /// Create a new sale from a [`Cart`].
    ///
    /// Converts every [`CartLine`] into a [`SaleLine`], computes the
    /// grand total, and sets `status = Pending`. Timestamps are set to
    /// the current UTC time.
    ///
    /// # Returns
    ///
    /// `None` if the cart's total overflows `i64` (cart is corrupt).
    pub fn from_cart(cart: &Cart) -> Option<Self> {
        Self::from_cart_with_user(cart, None)
    }

    /// Create a new sale from a [`Cart`], with an optional user_id.
    ///
    /// Like [`from_cart`] but also attaches the user_id of the cashier
    /// who processed the sale.
    pub fn from_cart_with_user(cart: &Cart, user_id: Option<String>) -> Option<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        let total = cart.total()?;
        let currency = cart.currency();
        let line_count = cart.line_count() as i64;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let lines: Vec<SaleLine> = cart
            .lines()
            .iter()
            .enumerate()
            .map(|(i, cl)| {
                let line_total = cl.total()?;
                Some(SaleLine {
                    id: uuid::Uuid::new_v4().to_string(),
                    sale_id: id.clone(),
                    sku: cl.sku.as_str().to_owned(),
                    qty: cl.qty,
                    unit_price: cl.unit_price,
                    line_total,
                    line_position: (i as i64) + 1,
                    tax_amount: Money::zero(currency),
                    tax_rate_id: None,
                })
            })
            .collect::<Option<Vec<_>>>()?;

        Some(Self {
            id,
            status: SaleStatus::Pending,
            total,
            line_count,
            currency,
            payment_method: None,
            tendered_minor: None,
            user_id,
            customer_id: None,
            created_at: now.clone(),
            updated_at: now,
            lines,
            discount_percent: cart.discount_percent(),
            discount_label: cart.discount_label().map(String::from),
            subtotal: Money::zero(currency),
            tax_total: Money::zero(currency),
        })
    }

    /// Transition to a new state.
    ///
    /// # Valid transitions
    ///
    /// | From      | To          |
    /// |-----------|-------------|
    /// | Pending   | Active      |
    /// | Active    | Completed   |
    /// | Active    | Voided      |
    ///
    /// All other transitions return `Err(InvalidTransition)`.
    pub fn transition_to(&mut self, to: SaleStatus) -> Result<(), InvalidTransition> {
        let from = self.status;
        let valid = matches!(
            (from, to),
            (SaleStatus::Pending, SaleStatus::Active)
                | (SaleStatus::Active, SaleStatus::Completed)
                | (SaleStatus::Active, SaleStatus::Voided)
        );

        if valid {
            self.status = to;
            Ok(())
        } else {
            Err(InvalidTransition { from, to })
        }
    }

    /// True when the sale cannot be modified further.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sku;

    fn usd() -> crate::Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn make_cart() -> Cart {
        use crate::CartLine;
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();
        cart
    }

    // ── Construction ─────────────────────────────────────────────

    #[test]
    fn from_cart_builds_sale() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        assert_eq!(sale.status, SaleStatus::Pending);
        assert_eq!(sale.total.minor_units, 1150); // 2×350 + 1×450
        assert_eq!(sale.currency, usd());
        assert_eq!(sale.line_count, 2);
        assert_eq!(sale.lines.len(), 2);
    }

    #[test]
    fn from_cart_lines_have_correct_positions() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        assert_eq!(sale.lines[0].line_position, 1);
        assert_eq!(sale.lines[1].line_position, 2);
    }

    #[test]
    fn from_cart_lines_link_to_sale() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        for line in &sale.lines {
            assert_eq!(line.sale_id, sale.id);
        }
    }

    #[test]
    fn from_cart_empty_returns_some() {
        let cart = Cart::new(usd());
        let sale = Sale::from_cart(&cart).unwrap();
        assert_eq!(sale.line_count, 0);
        assert_eq!(sale.total.minor_units, 0);
    }

    #[test]
    fn from_cart_sets_timestamps() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        assert!(!sale.created_at.is_empty(), "timestamps should be set");
        assert!(!sale.updated_at.is_empty(), "timestamps should be set");
        assert_eq!(sale.created_at, sale.updated_at);
    }

    // ── State machine ────────────────────────────────────────────

    #[test]
    fn pending_to_active() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        assert!(sale.transition_to(SaleStatus::Active).is_ok());
        assert_eq!(sale.status, SaleStatus::Active);
    }

    #[test]
    fn pending_to_completed_rejected() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let err = sale.transition_to(SaleStatus::Completed).unwrap_err();
        assert_eq!(err.from, SaleStatus::Pending);
        assert_eq!(err.to, SaleStatus::Completed);
        assert_eq!(sale.status, SaleStatus::Pending); // unchanged
    }

    #[test]
    fn active_to_completed() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.transition_to(SaleStatus::Active).unwrap();
        assert!(sale.transition_to(SaleStatus::Completed).is_ok());
        assert_eq!(sale.status, SaleStatus::Completed);
    }

    #[test]
    fn active_to_voided() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.transition_to(SaleStatus::Active).unwrap();
        assert!(sale.transition_to(SaleStatus::Voided).is_ok());
        assert_eq!(sale.status, SaleStatus::Voided);
    }

    #[test]
    fn completed_is_terminal() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.transition_to(SaleStatus::Active).unwrap();
        sale.transition_to(SaleStatus::Completed).unwrap();
        assert!(sale.is_terminal());
        assert!(sale.transition_to(SaleStatus::Voided).is_err());
        assert!(sale.transition_to(SaleStatus::Active).is_err());
    }

    #[test]
    fn voided_is_terminal() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.transition_to(SaleStatus::Active).unwrap();
        sale.transition_to(SaleStatus::Voided).unwrap();
        assert!(sale.is_terminal());
        assert!(sale.transition_to(SaleStatus::Completed).is_err());
    }

    #[test]
    fn self_transition_is_always_invalid() {
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        assert!(sale.transition_to(SaleStatus::Pending).is_err());
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        let json = serde_json::to_string(&sale).unwrap();
        let back: Sale = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sale);
    }

    #[test]
    fn status_serializes_as_kebab_case() {
        let json = serde_json::to_string(&SaleStatus::Pending).unwrap();
        assert_eq!(json, "\"pending\"");
        let json = serde_json::to_string(&SaleStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
    }

    // ── SaleLine totals ──────────────────────────────────────────

    #[test]
    fn sale_line_total_matches_unit_times_qty() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        let coffee = &sale.lines[0];
        assert_eq!(coffee.unit_price.minor_units, 350);
        assert_eq!(coffee.qty, 2);
        assert_eq!(coffee.line_total.minor_units, 700);
    }

    #[test]
    fn grand_total_matches_sum_of_lines() {
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        let sum: i64 = sale.lines.iter().map(|l| l.line_total.minor_units).sum();
        assert_eq!(sale.total.minor_units, sum);
    }
}
