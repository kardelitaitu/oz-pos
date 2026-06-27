//! Domain error type for `oz-core`.
//!
//! Library crates in OZ-POS use `thiserror` to define a typed error enum
//! so consumers can match on variants. The enum is `#[non_exhaustive]`
//! so we can add variants without breaking semver.

use thiserror::Error;

/// Errors that can originate in `oz-core` domain logic.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// Adding two [`crate::Money`] values overflowed `i64`.
    #[error("money overflow: {left} {currency} + {right}")]
    MoneyOverflow {
        /// Left-hand minor-unit operand.
        left: i64,
        /// Right-hand minor-unit operand.
        right: i64,
        /// ISO-4217 currency code, uppercased.
        currency: String,
    },

    /// A currency mismatch was passed to a function that requires equality.
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(String, String),

    /// A lookup by id returned no row.
    #[error("not found: {entity} {id}")]
    NotFound {
        /// The kind of entity that was being looked up.
        entity: &'static str,
        /// The id that was looked up.
        id: String,
    },
}
