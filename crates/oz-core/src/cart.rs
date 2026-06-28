//! Cart and CartLine — the in-memory sale pipeline.
//!
//! A `Cart` is created with a [`Currency`], lines are added via
//! [`Cart::add_line`], and the cart's [`Cart::total`] is computed by
//! summing line totals in checked arithmetic. Mismatched-currency adds
//! return [`CartError::CurrencyMismatch`] instead of panicking, keeping
//! the "never panic in library code" invariant from the `rust-backend`
//! skill.
//!
//! `Cart` is the only mutable type here. It is `Send + Sync` only when
//! wrapped in a `Mutex` or `RwLock`; the Tauri command layer uses
//! `tokio::sync::Mutex<HashMap<CartId, Cart>>` for the in-memory store.

// Scaffold: a few accessors and field docs are still TODO. The full
// doc pass is tracked in a followup; for now allow the warnings so the
// scaffold compiles under `clippy -- -D warnings`.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::money::Money;
use crate::sku::{LineId, Sku};

/// Unique identifier for a cart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CartId(pub Uuid);

impl CartId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
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
        }
    }

    /// Total for this line: `unit_price * qty`, in minor units.
    ///
    /// Returns `None` on `i64` overflow.
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        self.unit_price
            .minor_units
            .checked_mul(self.qty)
            .map(|minor_units| Money {
                minor_units,
                currency: self.unit_price.currency,
            })
    }
}

/// Failure modes for cart mutations.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CartError {
    /// Tried to add a line whose currency differs from the cart's.
    #[error("currency mismatch: cart is {cart}, line is {line}")]
    CurrencyMismatch { cart: String, line: String },
    /// Tried to remove a SKU that isn't in the cart.
    #[error("sku not in cart: {0}")]
    SkuNotInCart(String),
    /// Invalid discount percentage (must be 0-100).
    #[error("invalid discount percentage: {0} (must be 0-100)")]
    InvalidDiscount(i64),
}

/// An open cart scoped to a single currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cart {
    id: CartId,
    currency: crate::money::Currency,
    lines: Vec<CartLine>,
    /// Discount percentage (0-100). 0 means no discount.
    #[serde(default)]
    discount_percent: i64,
    /// Optional human-readable label for the discount (e.g. "Senior 10%").
    #[serde(default)]
    discount_label: Option<String>,
}

impl Cart {
    /// Create a new empty cart in the given currency.
    #[must_use]
    pub fn new(currency: crate::money::Currency) -> Self {
        Self {
            id: CartId::new(),
            currency,
            lines: Vec::new(),
            discount_percent: 0,
            discount_label: None,
        }
    }

    #[must_use]
    pub fn id(&self) -> CartId {
        self.id
    }
    #[must_use]
    pub fn currency(&self) -> crate::money::Currency {
        self.currency
    }
    #[must_use]
    pub fn lines(&self) -> &[CartLine] {
        &self.lines
    }
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get the current discount percentage.
    #[must_use]
    pub fn discount_percent(&self) -> i64 {
        self.discount_percent
    }

    /// Get the discount label.
    #[must_use]
    pub fn discount_label(&self) -> Option<&str> {
        self.discount_label.as_deref()
    }

    /// Set a cart-level discount.
    ///
    /// `percent` must be between 0 and 100 inclusive.
    /// `label` is an optional human-readable description.
    /// Pass `percent = 0` to clear the discount.
    #[must_use]
    pub fn set_discount(&mut self, percent: i64, label: Option<String>) -> Result<(), CartError> {
        if !(0..=100).contains(&percent) {
            return Err(CartError::InvalidDiscount(percent));
        }
        self.discount_percent = percent;
        self.discount_label = if percent == 0 { None } else { label };
        Ok(())
    }

    /// Append a line. Returns `Err(CartError::CurrencyMismatch)` if the
    /// line's `unit_price.currency` differs from the cart's currency.
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

    /// Remove every line with the given SKU. Returns `Err` if none found.
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

    /// Sum of all line totals, minus any discount. Returns `None` on `i64` overflow.
    ///
    /// Applies the discount percentage to the subtotal. For example, a 10%
    /// discount on a $10.00 subtotal gives a $9.00 total.
    #[must_use]
    pub fn total(&self) -> Option<Money> {
        let mut acc = Money::zero(self.currency);
        for line in &self.lines {
            let t = line.total()?;
            acc = acc.checked_add(t)?;
        }
        if self.discount_percent > 0 {
            // Apply percentage: total * (100 - discount_percent) / 100
            let discount_multiplier = 100 - self.discount_percent;
            let discounted = acc.minor_units.checked_mul(discount_multiplier)? / 100;
            acc = Money {
                minor_units: discounted,
                currency: self.currency,
            };
        }
        Some(acc)
    }

    /// The discount amount in minor units, or 0 if no discount is applied.
    #[must_use]
    pub fn discount_amount(&self) -> Option<Money> {
        if self.discount_percent == 0 {
            return Some(Money::zero(self.currency));
        }
        let mut subtotal = Money::zero(self.currency);
        for line in &self.lines {
            let t = line.total()?;
            subtotal = subtotal.checked_add(t)?;
        }
        let discounted = subtotal.minor_units.checked_mul(self.discount_percent)? / 100;
        Some(Money {
            minor_units: discounted,
            currency: self.currency,
        })
    }
}

/// Render a 3-byte currency code as a UTF-8 string (lossy for non-ASCII).
fn currency_summary(c: &crate::money::Currency) -> String {
    std::str::from_utf8(&c.0).unwrap_or("???").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;

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
        match cart.add_line(bad) {
            Err(CartError::CurrencyMismatch { .. }) => {}
            other => panic!("expected CurrencyMismatch, got {other:?}"),
        }
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
}
