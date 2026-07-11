//! Cart and CartLine — the in-memory sale pipeline.
//!
//! A `Cart` is created with a [`Currency`], lines are added via
//! [`Cart::add_line`], and the total is computed by summing line totals
//! in checked arithmetic.

#![allow(missing_docs)]

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
    pub id: LineId,
    pub sku: Sku,
    pub qty: i64,
    pub unit_price: Money,
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
    /// Returns `None` on `i64` overflow.
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        let price = self.overridden_price.unwrap_or(self.unit_price);
        price.checked_mul(self.qty)
    }

    /// Override the unit price for this line.
    pub fn set_overridden_price(&mut self, price: Money) {
        self.overridden_price = Some(price);
    }
}

/// Failure modes for cart mutations.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CartError {
    #[error("currency mismatch: cart is {cart}, line is {line}")]
    CurrencyMismatch { cart: String, line: String },
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
        }
    }

    #[must_use]
    pub fn id(&self) -> CartId {
        self.id
    }
    #[must_use]
    pub fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub fn lines(&self) -> &[CartLine] {
        &self.lines
    }
    pub fn lines_mut(&mut self) -> &mut [CartLine] {
        &mut self.lines
    }
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    #[must_use]
    pub fn discount_percent(&self) -> i64 {
        self.discount_percent.get() as i64
    }
    #[must_use]
    pub fn discount_label(&self) -> Option<&str> {
        self.discount_label.as_deref()
    }

    /// Get the discount as a [`Percentage`] value.
    #[must_use]
    pub fn discount_percentage(&self) -> Percentage {
        self.discount_percent
    }

    /// Set a cart-level percentage discount.
    pub fn set_discount(&mut self, percent: Percentage, label: Option<String>) {
        self.discount_percent = percent;
        self.discount_label = if percent.get() == 0 { None } else { label };
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

    /// Sum of all line totals, minus any discount. Returns `None` on overflow.
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        let mut acc = Money::zero(self.currency);
        for line in &self.lines {
            let t = line.total()?;
            acc = acc.checked_add(t)?;
        }
        if self.discount_percent.get() > 0 {
            acc = self.discount_percent.complement_apply_to(acc)?;
        }
        Some(acc)
    }

    /// The discount amount in minor units, or 0 if no discount.
    #[must_use]
    pub fn discount_amount(&self) -> Option<Money> {
        if self.discount_percent.get() == 0 {
            return Some(Money::zero(self.currency));
        }
        let mut subtotal = Money::zero(self.currency);
        for line in &self.lines {
            let t = line.total()?;
            subtotal = subtotal.checked_add(t)?;
        }
        self.discount_percent.apply_to(subtotal)
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
        // Setting a discount on an overflowing subtotal should propagate the overflow
        cart.set_discount(Percentage::new(50).unwrap(), None);
        assert!(cart.discount_amount().is_none());
        assert!(cart.total().is_none());
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
        });
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
        });
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
        });
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
        cart.lines_mut()[0].set_overridden_price(Money {
            minor_units: 50,
            currency: usd(),
        });
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
        assert_eq!(back.discount_label(), Some("sale"));
    }
}
