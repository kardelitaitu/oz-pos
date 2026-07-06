//! Domain error type for `oz-core`.
//!
//! Library crates in OZ-POS use `thiserror` to define a typed error enum
//! so consumers can match on variants. The enum is `#[non_exhaustive]`
//! so we can add variants without breaking semver.

use serde::Serialize;
use thiserror::Error;

/// Serializable discriminator for [`CoreError`] variants.
///
/// Mirrored on the front-end as `AppError.subKind` so UI code can branch
/// on the specific flavour of core error without parsing the message string.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoreErrorKind {
    /// A database operation failed.
    Db,
    /// A platform infrastructure error.
    Platform,
    /// Money arithmetic overflowed `i64`.
    MoneyOverflow,
    /// Currency code mismatch in an operation requiring equal codes.
    CurrencyMismatch,
    /// A lookup by id returned no row.
    NotFound,
    /// A uniqueness constraint was violated.
    Conflict,
    /// Input validation failure.
    Validation,
    /// Unexpected internal error.
    Internal,
}

/// Errors that can originate in `oz-core` domain logic.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A platform infrastructure error.
    #[error("platform error: {0}")]
    Platform(#[from] platform_core::PlatformError),

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

    /// A uniqueness constraint was violated (duplicate SKU, name, etc.).
    #[error("conflict: {entity} already exists ({field})")]
    Conflict {
        /// The entity type (e.g. "product", "category").
        entity: &'static str,
        /// The field that triggered the conflict (e.g. "sku", "name").
        field: &'static str,
    },

    /// A value failed input validation.
    #[error("validation error on {field}: {message}")]
    Validation {
        /// The field that failed validation.
        field: &'static str,
        /// Human-readable description of the failure.
        message: String,
    },

    /// An unexpected internal error (serialization, crypto, I/O, etc.).
    #[error("internal error: {0}")]
    Internal(String),
}

impl CoreError {
    /// Map a `CoreError` to its [`CoreErrorKind`] discriminator.
    pub fn kind(&self) -> CoreErrorKind {
        match self {
            CoreError::Db(_) => CoreErrorKind::Db,
            CoreError::Platform(_) => CoreErrorKind::Platform,
            CoreError::MoneyOverflow { .. } => CoreErrorKind::MoneyOverflow,
            CoreError::CurrencyMismatch(..) => CoreErrorKind::CurrencyMismatch,
            CoreError::NotFound { .. } => CoreErrorKind::NotFound,
            CoreError::Conflict { .. } => CoreErrorKind::Conflict,
            CoreError::Validation { .. } => CoreErrorKind::Validation,
            CoreError::Internal(_) => CoreErrorKind::Internal,
        }
    }
}
