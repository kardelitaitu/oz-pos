/*
last audited 25-07-26 by RSA-Agent (foundation slice B: cart deep read)
crate: foundation (cart.rs) | status: SAFE | lint: CLEAN
findings: exemplary — MONEY-AUDIT-3 fixes verified intact (CartLine::total fails closed on serde-bypassed qty<=0; discount_amount never masks with .or(Some(zero))); fixed discount capped via Money::min; debug_assert currency guards for direct-field mutation; production logic 1-353 is clean; COR-33 pattern: ~850 lines of inline tests (355-1205) in the production file
next: move inline tests to sibling cart_tests.rs (COR-33 family) | perf: discount folds lines once
*/
//! Cart and CartLine — the in-memory sale pipeline.
//!
//! A `Cart` is created with a [`Currency`], lines are added via
//! [`Cart::add_line`], and the total is computed by summing line totals
//! in checked arithmetic.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::money::{Currency, Money};
use crate::percentage::Percentage;
use crate::sku::{LineId, Sku};

/// Unique identifier for a cart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CartId(pub Uuid);

impl CartId {
    /// Create a new cart identifier backed by a UUID v7.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for CartId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CartId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single line in a cart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartLine {
    /// Unique line identifier.
    pub id: LineId,
    /// The product SKU.
    pub sku: Sku,
    /// Quantity ordered (must be > 0).
    pub qty: i64,
    /// Base unit price (before per-line override).
    pub unit_price: Money,
    /// Optional per-line price override.
    pub overridden_price: Option<Money>,
}

impl CartLine {
    /// Construct a new line. `qty` is asserted > 0.
    ///
    /// # Panics
    /// Panics if `qty <= 0`.
    pub fn new(sku: Sku, qty: i64, unit_price: Money) -> Self {
        assert!(qty > 0, "qty must be > 0, got {qty}");
        Self {
            id: LineId::new(),
            sku,
            qty,
            unit_price,
            overridden_price: None,
        }
    }

    /// Total for this line: `unit_price * qty`, in minor units.
    /// If [`overridden_price`](Self::overridden_price) is set, uses that instead.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds, panics if `overridden_price` is set but its currency
    /// does not match `unit_price.currency`. This guards against direct field
    /// mutation bypassing [`set_overridden_price`](Self::set_overridden_price).
    ///
    /// # Errors
    ///
    /// Returns `None` on `i64` overflow **or** when `qty <= 0`. `qty <= 0`
    /// cannot occur through [`CartLine::new`] (which asserts `qty > 0`), but
    /// it CAN occur through `serde` deserialization of a persisted cart (the
    /// fields are public and `Deserialize` does not run the constructor
    /// assert). Returning `None` makes a corrupt persisted line fail closed
    /// instead of silently computing a zero or negative total.
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        if self.qty <= 0 {
            return None;
        }
        let price = self.overridden_price.unwrap_or(self.unit_price);
        debug_assert!(
            price.currency == self.unit_price.currency,
            "CartLine::total: overridden_price currency ({}) does not match unit_price currency ({})",
            price.currency,
            self.unit_price.currency
        );
        price.checked_mul(self.qty)
    }

    /// Override the unit price for this line.
    ///
    /// Returns `Err(CartError::CurrencyMismatch)` if `price.currency`
    /// does not match `self.unit_price.currency`.
    pub fn set_overridden_price(&mut self, price: Money) -> Result<(), CartError> {
        if price.currency != self.unit_price.currency {
            return Err(CartError::CurrencyMismatch {
                cart: self.unit_price.currency.to_string(),
                line: price.currency.to_string(),
            });
        }
        self.overridden_price = Some(price);
        Ok(())
    }
}

/// Failure modes for cart mutations.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CartError {
    /// Line currency does not match the cart currency.
    #[error("currency mismatch: cart is {cart}, line is {line}")]
    CurrencyMismatch {
        /// Cart currency code.
        cart: String,
        /// Line currency code.
        line: String,
    },
    /// Attempted to remove a SKU that is not in the cart.
    #[error("sku not in cart: {0}")]
    SkuNotInCart(String),
}

/// An open cart scoped to a single currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cart {
    id: CartId,
    currency: Currency,
    lines: Vec<CartLine>,
    #[serde(default)]
    discount_percent: Percentage,
    #[serde(default)]
    discount_label: Option<String>,
    /// Fixed discount in minor currency units, applied after any percentage discount.
    #[serde(default)]
    fixed_discount_minor: i64,
}

impl Cart {
    /// Create a new empty cart in the given currency.
    #[must_use]
    pub fn new(currency: Currency) -> Self {
        Self {
            id: CartId::new(),
            currency,
            lines: Vec::new(),
            discount_percent: Percentage::zero(),
            discount_label: None,
            fixed_discount_minor: 0,
        }
    }

    /// Return the cart's unique identifier.
    #[must_use]
    pub fn id(&self) -> CartId {
        self.id
    }
    /// Return the currency scoped to this cart.
    #[must_use]
    pub fn currency(&self) -> Currency {
        self.currency
    }
    /// Return a shared reference to the line items.
    #[must_use]
    pub fn lines(&self) -> &[CartLine] {
        &self.lines
    }
    /// Return a mutable reference to the line items.
    pub fn lines_mut(&mut self) -> &mut [CartLine] {
        &mut self.lines
    }
    /// Return the number of line items.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    /// Return the discount percentage as an integer (0–100).
    #[must_use]
    pub fn discount_percent(&self) -> i64 {
        self.discount_percent.get() as i64
    }
    /// Return an optional label for the current discount.
    #[must_use]
    pub fn discount_label(&self) -> Option<&str> {
        self.discount_label.as_deref()
    }

    /// Get the discount as a [`Percentage`] value.
    #[must_use]
    pub fn discount_percentage(&self) -> Percentage {
        self.discount_percent
    }

    /// Set a cart-level percentage discount and clear any fixed discount.
    pub fn set_discount(&mut self, percent: Percentage, label: Option<String>) {
        self.discount_percent = percent;
        self.fixed_discount_minor = 0;
        self.discount_label = if percent.get() == 0 { None } else { label };
    }

    /// Set a fixed cart discount in minor currency units.
    ///
    /// The amount is capped at the cart's payable total when the total is
    /// calculated, so a discount can safely be applied before all lines exist.
    pub fn set_fixed_discount(&mut self, minor_units: i64, label: Option<String>) {
        self.fixed_discount_minor = minor_units.max(0);
        self.discount_label = if self.fixed_discount_minor == 0 {
            None
        } else {
            label
        };
    }

    /// Return the fixed discount in minor currency units.
    #[must_use]
    pub fn fixed_discount_minor(&self) -> i64 {
        self.fixed_discount_minor
    }

    /// Append a line. Returns `Err` on currency mismatch.
    pub fn add_line(&mut self, line: CartLine) -> Result<LineId, CartError> {
        if line.unit_price.currency != self.currency {
            return Err(CartError::CurrencyMismatch {
                cart: currency_summary(&self.currency),
                line: currency_summary(&line.unit_price.currency),
            });
        }
        let id = line.id;
        self.lines.push(line);
        Ok(id)
    }

    /// Remove every line with the given SKU.
    pub fn remove_sku(&mut self, sku: &str) -> Result<usize, CartError> {
        let before = self.lines.len();
        self.lines.retain(|l| l.sku.as_str() != sku);
        let removed = before - self.lines.len();
        if removed == 0 {
            Err(CartError::SkuNotInCart(sku.to_owned()))
        } else {
            Ok(removed)
        }
    }

    /// Sum of all line totals, minus any discount.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds, panics if any line's effective currency (unit_price
    /// or overridden_price) does not match the cart's currency.
    ///
    /// Returns `None` on overflow or currency mismatch (the latter is
    /// caught by [`Money::checked_add`] in release builds).
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        let mut acc = Money::zero(self.currency);
        for line in &self.lines {
            debug_assert!(
                line.unit_price.currency == self.currency,
                "Cart::total: line unit_price currency ({}) does not match cart currency ({})",
                line.unit_price.currency,
                self.currency
            );
            let t = line.total()?;
            acc = acc.checked_add(t)?;
        }
        if self.discount_percent.get() > 0 {
            acc = self.discount_percent.complement_apply_to(acc)?;
        }
        if self.fixed_discount_minor > 0 {
            // Cap the fixed discount at the payable total. Both amounts are
            // in `self.currency`, so `Money::min` cannot return `None` here;
            // the `?` propagates a mismatch the same way `checked_sub` does.
            let fixed = Money {
                minor_units: self.fixed_discount_minor,
                currency: self.currency,
            };
            acc = acc.checked_sub(fixed.min(acc)?)?;
        }
        Some(acc)
    }

    /// The discount amount in minor units, or 0 if no discount.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds, panics if any line's effective currency does not
    /// match the cart's currency.
    ///
    /// Returns `None` on overflow, currency mismatch, or a corrupt line
    /// (`qty <= 0` deserialized from a persisted cart) — the same
    /// fail-closed contract as [`Cart::total`](Self::total).
    #[must_use]
    pub fn discount_amount(&self) -> Option<Money> {
        let mut subtotal = Money::zero(self.currency);
        for line in &self.lines {
            debug_assert!(
                line.unit_price.currency == self.currency,
                "Cart::discount_amount: line unit_price currency ({}) does not match cart currency ({})",
                line.unit_price.currency,
                self.currency
            );
            let t = line.total()?;
            subtotal = subtotal.checked_add(t)?;
        }
        if self.discount_percent.get() == 0 && self.fixed_discount_minor == 0 {
            return Some(Money::zero(self.currency));
        }
        let discounted = if self.discount_percent.get() > 0 {
            self.discount_percent.complement_apply_to(subtotal)?
        } else {
            subtotal
        };
        let fixed = Money {
            minor_units: self.fixed_discount_minor,
            currency: self.currency,
        };
        // Both subtractions are on amounts capped by `fixed.min(discounted)`
        // (which cannot fail here: both operands are in `self.currency`), so
        // they cannot underflow. `?` propagates any overflow as `None` — we
        // deliberately do NOT fall back to a zero discount here, because a
        // failure means the discount amount is unknown, not zero.
        let capped = fixed.min(discounted)?;
        let total_after_discount = discounted.checked_sub(capped)?;
        subtotal.checked_sub(total_after_discount)
    }
}

fn currency_summary(c: &Currency) -> String {
    std::str::from_utf8(&c.0).unwrap_or("???").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }
    fn eur() -> Currency {
        "EUR".parse().unwrap()
    }

    #[test]
    fn empty_cart_has_zero_total() {
        let cart = Cart::new(usd());
        assert_eq!(cart.total().unwrap().minor_units, 0);
        assert_eq!(cart.line_count(), 0);
    }

    #[test]
    fn add_line_appends_and_returns_id() {
        let mut cart = Cart::new(usd());
        let line = CartLine::new(
            Sku::new("COFFEE"),
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
        );
        let id = cart.add_line(line).unwrap();
        assert_eq!(cart.line_count(), 1);
        assert_eq!(cart.lines()[0].id, id);
        assert_eq!(cart.total().unwrap().minor_units, 700);
    }

    #[test]
    fn add_line_currency_mismatch_rejected() {
        let mut cart = Cart::new(usd());
        let bad = CartLine::new(
            Sku::new("TEA"),
            1,
            Money {
                minor_units: 200,
                currency: eur(),
            },
        );
        assert!(matches!(
            cart.add_line(bad),
            Err(CartError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn remove_sku_drops_matching_lines() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("B"),
            1,
            Money {
                minor_units: 200,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 150,
                currency: usd(),
            },
        ))
        .unwrap();
        let removed = cart.remove_sku("A").unwrap();
        assert_eq!(removed, 2);
        assert_eq!(cart.line_count(), 1);
    }

    #[test]
    fn total_overflow_returns_none() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("BIG"),
            1,
            Money {
                minor_units: i64::MAX,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("PLUS"),
            1,
            Money {
                minor_units: 1,
                currency: usd(),
            },
        ))
        .unwrap();
        assert!(cart.total().is_none());
    }

    #[test]
    fn cart_id_new_generates_unique_ids() {
        let a = CartId::new();
        let b = CartId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn cart_id_default() {
        let id = CartId::default();
        assert!(!format!("{id}").is_empty());
    }

    #[test]
    fn cart_id_display() {
        let id = CartId::new();
        let display = format!("{id}");
        assert!(!display.is_empty());
        // UUID format: 8-4-4-4-12 hex chars
        assert_eq!(display.len(), 36);
    }

    #[test]
    fn cart_id_serializes_as_uuid_string() {
        let id = CartId::new();
        let json = serde_json::to_string(&id).unwrap();
        // CartId is a newtype over Uuid — serializes as bare UUID string.
        assert!(
            json.starts_with('"'),
            "cart ID should serialize as a bare string, got: {json}"
        );
        let back: CartId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn cart_currency_accessor() {
        let cart = Cart::new(usd());
        assert_eq!(cart.currency(), usd());
    }

    #[test]
    fn cart_id_accessor() {
        let cart = Cart::new(usd());
        assert!(!format!("{}", cart.id()).is_empty());
    }

    #[test]
    fn cart_default_discount() {
        let cart = Cart::new(usd());
        assert_eq!(cart.discount_percent(), 0);
        assert!(cart.discount_label().is_none());
    }

    #[test]
    fn set_discount_valid_range() {
        let mut cart = Cart::new(usd());
        cart.set_discount(Percentage::new(10).unwrap(), Some("VIP 10% off".into()));
        assert_eq!(cart.discount_percent(), 10);
        assert_eq!(cart.discount_label(), Some("VIP 10% off"));
    }

    #[test]
    fn set_discount_zero_clears_label() {
        let mut cart = Cart::new(usd());
        cart.set_discount(Percentage::new(10).unwrap(), Some("sale".into()));
        cart.set_discount(Percentage::zero(), None);
        assert_eq!(cart.discount_percent(), 0);
        assert!(cart.discount_label().is_none());
    }

    #[test]
    fn discount_applied_to_total() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("ITEM"),
            2,
            Money {
                minor_units: 1000,
                currency: usd(),
            },
        ))
        .unwrap();
        // 2 x 1000 = 2000, with 10% discount = 1800
        cart.set_discount(Percentage::new(10).unwrap(), None);
        assert_eq!(cart.total().unwrap().minor_units, 1800);
    }

    #[test]
    fn discount_amount_calculated_correctly() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("ITEM"),
            3,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        // 3 x 500 = 1500, with 10% discount = 150 discount
        cart.set_discount(Percentage::new(10).unwrap(), Some("10%".into()));
        assert_eq!(cart.discount_amount().unwrap().minor_units, 150);
    }

    #[test]
    fn discount_amount_zero_when_no_discount() {
        let cart = Cart::new(usd());
        assert_eq!(cart.discount_amount().unwrap().minor_units, 0);
    }

    #[test]
    fn fixed_discount_is_capped_and_survives_serialization() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("ITEM"),
            1,
            Money {
                minor_units: 1000,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_fixed_discount(1500, Some("Loyalty points".into()));
        assert_eq!(cart.total().unwrap().minor_units, 0);
        assert_eq!(cart.discount_amount().unwrap().minor_units, 1000);

        let json = serde_json::to_string(&cart).unwrap();
        let restored: Cart = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.fixed_discount_minor(), 1500);
        assert_eq!(restored.total().unwrap().minor_units, 0);
    }

    #[test]
    fn discount_overflow_returns_none() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("BIG"),
            1,
            Money {
                minor_units: i64::MAX,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("HUGE"),
            1,
            Money {
                minor_units: i64::MAX,
                currency: usd(),
            },
        ))
        .unwrap();
        // Subtotal = i64::MAX + i64::MAX overflows → total and discount_amount
        // must be None. (A single line with 50% discount succeeds now — the
        // overflow-free Percentage decomposition computes i64::MAX * 50 / 100
        // = i64::MAX/2 without overflowing the intermediate product.)
        cart.set_discount(Percentage::new(50).unwrap(), None);
        assert!(cart.total().is_none());
        assert!(cart.discount_amount().is_none());
    }

    #[test]
    fn remove_sku_not_found_returns_error() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        ))
        .unwrap();
        assert!(matches!(
            cart.remove_sku("Z"),
            Err(CartError::SkuNotInCart(..))
        ));
    }

    #[test]
    fn cart_line_new_panics_on_zero_qty() {
        use std::panic::catch_unwind;
        let sku = Sku::new("TEST");
        let price = Money {
            minor_units: 100,
            currency: usd(),
        };
        let result = catch_unwind(|| CartLine::new(sku, 0, price));
        assert!(result.is_err());
    }

    #[test]
    fn cart_line_new_panics_on_negative_qty() {
        use std::panic::catch_unwind;
        let sku = Sku::new("TEST");
        let price = Money {
            minor_units: 100,
            currency: usd(),
        };
        let result = catch_unwind(|| CartLine::new(sku, -1, price));
        assert!(result.is_err());
    }

    #[test]
    fn cart_line_total_calculated() {
        let line = CartLine::new(
            Sku::new("TEA"),
            3,
            Money {
                minor_units: 150,
                currency: usd(),
            },
        );
        assert_eq!(line.total().unwrap().minor_units, 450);
    }

    #[test]
    fn cart_line_total_overflow_returns_none() {
        let line = CartLine::new(
            Sku::new("BIG"),
            2,
            Money {
                minor_units: i64::MAX,
                currency: usd(),
            },
        );
        assert!(line.total().is_none());
    }

    /// MONEY-AUDIT-3: `CartLine::total()` must return `None` (fail closed)
    /// when `qty <= 0` arrives via serde deserialization — `CartLine::new`
    /// asserts `qty > 0`, but the public fields + `Deserialize` bypass the
    /// constructor, so a corrupt persisted cart could otherwise silently
    /// compute a zero or negative line total.
    #[test]
    fn cart_line_total_fails_closed_on_zero_or_negative_qty_from_serde() {
        // The JSON payloads below hard-code the unit price, so no local
        // `unit_price` binding is needed (the MONEY-AUDIT-3 invariant is
        // about qty, not price).
        // qty = 0 via JSON (would be free money without the guard).
        let json_zero = r#"{"id":"00000000-0000-0000-0000-000000000001","sku":"TEA","qty":0,"unit_price":{"minor_units":500,"currency":"USD"},"overridden_price":null}"#;
        let line: CartLine = serde_json::from_str(json_zero).unwrap();
        assert_eq!(line.qty, 0);
        assert!(line.total().is_none(), "qty=0 must fail closed");

        // qty = -2 via JSON (would be a negative total / money creation).
        let json_neg = r#"{"id":"00000000-0000-0000-0000-000000000002","sku":"TEA","qty":-2,"unit_price":{"minor_units":500,"currency":"USD"},"overridden_price":null}"#;
        let line: CartLine = serde_json::from_str(json_neg).unwrap();
        assert_eq!(line.qty, -2);
        assert!(line.total().is_none(), "qty<0 must fail closed");
    }

    /// MONEY-AUDIT-3b: `Cart::total()` propagates the fail-closed `None`
    /// from a corrupted line (qty=0 via serde) instead of summing a free
    /// item into the cart total.
    #[test]
    fn cart_total_fails_closed_when_serde_line_has_zero_qty() {
        let json = r#"{"id":"00000000-0000-0000-0000-000000000003","currency":"USD","lines":[{"id":"00000000-0000-0000-0000-000000000004","sku":"TEA","qty":0,"unit_price":{"minor_units":500,"currency":"USD"},"overridden_price":null}],"discount_percent":0,"discount_label":null,"fixed_discount_minor":0}"#;
        let cart: Cart = serde_json::from_str(json).unwrap();
        assert!(cart.total().is_none(), "corrupt line must fail closed");
        assert!(
            cart.discount_amount().is_none(),
            "corrupt line must fail closed in discount_amount too"
        );
    }

    #[test]
    fn cart_error_display_currency_mismatch() {
        let err = CartError::CurrencyMismatch {
            cart: "USD".into(),
            line: "EUR".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("USD"), "msg should contain USD, got: {msg}");
        assert!(msg.contains("EUR"), "msg should contain EUR, got: {msg}");
    }

    #[test]
    fn cart_error_display_sku_not_in_cart() {
        let err = CartError::SkuNotInCart("XYZ".into());
        assert_eq!(err.to_string(), "sku not in cart: XYZ");
    }

    #[test]
    fn cart_error_debug() {
        let err = CartError::SkuNotInCart("TEST".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn cart_error_partial_eq() {
        assert_eq!(
            CartError::SkuNotInCart("A".into()),
            CartError::SkuNotInCart("A".into()),
        );
        assert_ne!(
            CartError::SkuNotInCart("A".into()),
            CartError::SkuNotInCart("B".into()),
        );
    }

    // ── CartLine overridden price ──

    #[test]
    fn cartline_overridden_price_none_by_default() {
        let line = CartLine::new(
            Sku::new("TEA"),
            2,
            Money {
                minor_units: 150,
                currency: usd(),
            },
        );
        assert!(line.overridden_price.is_none());
    }

    #[test]
    fn cartline_set_overridden_price() {
        let mut line = CartLine::new(
            Sku::new("TEA"),
            2,
            Money {
                minor_units: 150,
                currency: usd(),
            },
        );
        line.set_overridden_price(Money {
            minor_units: 100,
            currency: usd(),
        })
        .unwrap();
        assert_eq!(line.overridden_price.unwrap().minor_units, 100);
    }

    #[test]
    fn cartline_total_uses_overridden_price() {
        let mut line = CartLine::new(
            Sku::new("TEA"),
            3,
            Money {
                minor_units: 200,
                currency: usd(),
            },
        );
        // Without override: 3 x 200 = 600
        assert_eq!(line.total().unwrap().minor_units, 600);
        // With override: 3 x 150 = 450
        line.set_overridden_price(Money {
            minor_units: 150,
            currency: usd(),
        })
        .unwrap();
        assert_eq!(line.total().unwrap().minor_units, 450);
    }

    #[test]
    fn cartline_clone_preserves_fields() {
        let mut line = CartLine::new(
            Sku::new("LATTE"),
            1,
            Money {
                minor_units: 450,
                currency: usd(),
            },
        );
        line.set_overridden_price(Money {
            minor_units: 400,
            currency: usd(),
        })
        .unwrap();
        let clone = line.clone();
        assert_eq!(clone.sku, line.sku);
        assert_eq!(clone.qty, line.qty);
        assert_eq!(clone.unit_price, line.unit_price);
        assert_eq!(clone.overridden_price, line.overridden_price);
    }

    #[test]
    fn cartline_serialization_roundtrip() {
        let line = CartLine::new(
            Sku::new("MOCHA"),
            2,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        );
        let json = serde_json::to_string(&line).unwrap();
        let back: CartLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, line.sku);
        assert_eq!(back.qty, line.qty);
        assert_eq!(back.unit_price, line.unit_price);
    }

    #[test]
    fn cartline_set_overridden_price_currency_mismatch_returns_error() {
        let mut line = CartLine::new(
            Sku::new("TEA"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        );
        let eur_price = Money {
            minor_units: 90,
            currency: eur(),
        };
        let result = line.set_overridden_price(eur_price);
        assert!(matches!(result, Err(CartError::CurrencyMismatch { .. })));
        // The original price should not have been overwritten
        assert!(line.overridden_price.is_none());
    }
    #[test]
    fn cartline_total_debug_assert_currency_mismatch_on_direct_mutation() {
        // set_overridden_price now validates currency, but the fields are pub
        // so a caller could bypass it with direct mutation. The debug_assert!
        // in total() catches this in debug/test builds.
        let mut line = CartLine::new(
            Sku::new("TEA"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        );
        // Direct mutation bypasses set_overridden_price validation
        line.overridden_price = Some(Money {
            minor_units: 90,
            currency: eur(),
        });
        // debug_assert! should fire — verify with catch_unwind
        let result = std::panic::catch_unwind(|| {
            let _ = line.total();
        });
        assert!(
            result.is_err(),
            "debug_assert! should have panicked on currency mismatch from direct field mutation"
        );
    }

    #[test]
    fn cart_total_debug_assert_on_line_currency_mismatch() {
        // Cart::total() checks every line's unit_price.currency matches the cart.
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("ITEM"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        ))
        .unwrap();
        // Direct mutation: change unit_price currency via pub field
        cart.lines_mut()[0].unit_price.currency = eur();
        let result = std::panic::catch_unwind(|| {
            let _ = cart.total();
        });
        assert!(
            result.is_err(),
            "debug_assert! should have panicked on line currency mismatch in Cart::total()"
        );
    }

    #[test]
    fn cart_discount_amount_debug_assert_on_line_currency_mismatch() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("ITEM"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_discount(Percentage::new(10).unwrap(), None);
        // Direct mutation
        cart.lines_mut()[0].unit_price.currency = eur();
        let result = std::panic::catch_unwind(|| {
            let _ = cart.discount_amount();
        });
        assert!(
            result.is_err(),
            "debug_assert! should have panicked on line currency mismatch in Cart::discount_amount()"
        );
    }

    // ── Cart accessors & serialization ──

    #[test]
    fn cart_discount_percentage_accessor() {
        let mut cart = Cart::new(usd());
        cart.set_discount(Percentage::new(15).unwrap(), None);
        assert_eq!(cart.discount_percentage().get(), 15);
    }

    #[test]
    fn cart_lines_and_lines_mut() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 100,
                currency: usd(),
            },
        ))
        .unwrap();
        assert_eq!(cart.lines().len(), 1);
        cart.lines_mut()[0]
            .set_overridden_price(Money {
                minor_units: 50,
                currency: usd(),
            })
            .unwrap();
        assert_eq!(cart.lines()[0].overridden_price.unwrap().minor_units, 50);
    }

    #[test]
    fn cart_serialization_roundtrip() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            2,
            Money {
                minor_units: 300,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_discount(Percentage::new(10).unwrap(), Some("sale".into()));
        let json = serde_json::to_string(&cart).unwrap();
        let back: Cart = serde_json::from_str(&json).unwrap();
        assert_eq!(back.line_count(), 1);
        assert_eq!(back.currency(), usd());
        assert_eq!(back.discount_percent(), 10);
        assert_eq!(back.fixed_discount_minor(), 0);
        assert_eq!(back.discount_label(), Some("sale"));
    }

    // ── Additional Cart edge-case tests ──────────────────────────────────

    #[test]
    fn fixed_discount_capped_to_total() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        // Total is 500. Fixed discount of 1000 should be capped to 500.
        cart.set_fixed_discount(1000, Some("Over-cap".into()));
        assert_eq!(cart.fixed_discount_minor(), 1000);
        // ...but total() caps the effective discount to the available amount.
        assert_eq!(cart.total().unwrap().minor_units, 0);
    }

    #[test]
    fn fixed_discount_exactly_equals_total() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_fixed_discount(500, Some("Exact".into()));
        assert_eq!(cart.total().unwrap().minor_units, 0);
    }

    #[test]
    fn fixed_discount_less_than_total() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 1000,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_fixed_discount(300, Some("Partial".into()));
        assert_eq!(cart.total().unwrap().minor_units, 700);
    }

    #[test]
    fn discount_amount_with_fixed_discount() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            2,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        // Total = 1000. Fixed discount = 250.
        cart.set_fixed_discount(250, Some("Coupon".into()));
        assert_eq!(cart.discount_amount().unwrap().minor_units, 250);
    }

    #[test]
    fn discount_amount_with_percentage_discount() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 1000,
                currency: usd(),
            },
        ))
        .unwrap();
        // 15% of 1000 = 150
        cart.set_discount(Percentage::new(15).unwrap(), Some("VIP".into()));
        assert_eq!(cart.discount_amount().unwrap().minor_units, 150);
    }

    #[test]
    fn cart_with_multiple_lines_total() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            2,
            Money {
                minor_units: 300,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("B"),
            1,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        // 2*300 + 1*500 = 1100
        assert_eq!(cart.total().unwrap().minor_units, 1100);
        assert_eq!(cart.line_count(), 2);
    }

    #[test]
    fn cart_total_with_all_lines_removed() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("A"),
            1,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("B"),
            1,
            Money {
                minor_units: 300,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.remove_sku("A").unwrap();
        cart.remove_sku("B").unwrap();
        assert_eq!(cart.line_count(), 0);
        assert_eq!(cart.total().unwrap().minor_units, 0);
    }

    #[test]
    fn set_fixed_discount_zero_clears_label() {
        let mut cart = Cart::new(usd());
        cart.set_fixed_discount(100, Some("Test".into()));
        assert!(cart.discount_label().is_some());
        cart.set_fixed_discount(0, None);
        assert!(cart.discount_label().is_none());
        assert_eq!(cart.fixed_discount_minor(), 0);
    }

    #[test]
    fn set_fixed_discount_negative_treated_as_zero() {
        let mut cart = Cart::new(usd());
        cart.set_fixed_discount(-500, Some("Neg".into()));
        assert_eq!(cart.fixed_discount_minor(), 0);
        assert!(cart.discount_label().is_none());
    }

    #[test]
    fn discount_percentage_accessor() {
        let mut cart = Cart::new(usd());
        assert_eq!(cart.discount_percentage().get(), 0);
        cart.set_discount(Percentage::new(25).unwrap(), None);
        assert_eq!(cart.discount_percentage().get(), 25);
    }

    #[test]
    fn set_discount_clears_fixed_discount() {
        let mut cart = Cart::new(usd());
        cart.set_fixed_discount(500, Some("Fixed".into()));
        assert_eq!(cart.fixed_discount_minor(), 500);
        cart.set_discount(Percentage::new(10).unwrap(), None);
        assert_eq!(cart.fixed_discount_minor(), 0);
    }

    #[test]
    fn cart_serialization_roundtrip_with_lines_and_discount() {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("SKU-1"),
            3,
            Money {
                minor_units: 250,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.set_discount(Percentage::new(5).unwrap(), Some("test".into()));
        let json = serde_json::to_string(&cart).unwrap();
        let back: Cart = serde_json::from_str(&json).unwrap();
        assert_eq!(back.line_count(), 1);
        // 3*250 = 750. 5% discount via complement_apply: 750 * 95 / 100 = 712
        assert_eq!(back.total().unwrap().minor_units, 712);
        assert_eq!(back.discount_percentage().get(), 5);
    }
}
