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
    /// Subscription limit exceeded (ADR #5).
    SubscriptionLimitExceeded,
    /// Invalid subscription signature (ADR #5).
    InvalidSubscriptionSignature,
    /// Workspace type requires a higher subscription tier (ADR #5).
    SubscriptionUpgradeRequired,
    /// System clock tampering detected (ADR #5).
    SystemClockTampered,
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

    /// A subscription limit was exceeded (ADR #5).
    #[error("subscription limit exceeded: {0}")]
    SubscriptionLimitExceeded(String),

    /// The subscription signature is invalid or tampered (ADR #5).
    #[error("invalid subscription signature: {0}")]
    InvalidSubscriptionSignature(String),

    /// The workspace type requires a higher subscription tier (ADR #5).
    #[error("subscription upgrade required: {0}")]
    SubscriptionUpgradeRequired(String),

    /// System clock rollback detected — possible tampering (ADR #5).
    #[error("system clock tampered: {0}")]
    SystemClockTampered(String),
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
            CoreError::SubscriptionLimitExceeded(_) => CoreErrorKind::SubscriptionLimitExceeded,
            CoreError::InvalidSubscriptionSignature(_) => {
                CoreErrorKind::InvalidSubscriptionSignature
            }
            CoreError::SubscriptionUpgradeRequired(_) => CoreErrorKind::SubscriptionUpgradeRequired,
            CoreError::SystemClockTampered(_) => CoreErrorKind::SystemClockTampered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_error_kind() {
        let err = CoreError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        assert!(matches!(err.kind(), CoreErrorKind::Db));
        assert!(err.to_string().contains("database error"));
    }

    #[test]
    fn money_overflow_kind_and_display() {
        let err = CoreError::MoneyOverflow {
            left: 1_000_000,
            right: 500_000,
            currency: "IDR".into(),
        };
        assert!(matches!(err.kind(), CoreErrorKind::MoneyOverflow));
        let msg = err.to_string();
        assert!(msg.contains("money overflow"));
        assert!(msg.contains("IDR"));
        assert!(msg.contains("1000000"));
        assert!(msg.contains("500000"));
    }

    #[test]
    fn currency_mismatch_kind_and_display() {
        let err = CoreError::CurrencyMismatch("USD".into(), "IDR".into());
        assert!(matches!(err.kind(), CoreErrorKind::CurrencyMismatch));
        let msg = err.to_string();
        assert!(msg.contains("currency mismatch"));
        assert!(msg.contains("USD"));
        assert!(msg.contains("IDR"));
    }

    #[test]
    fn not_found_kind_and_display() {
        let err = CoreError::NotFound {
            entity: "product",
            id: "prod-1".into(),
        };
        assert!(matches!(err.kind(), CoreErrorKind::NotFound));
        let msg = err.to_string();
        assert!(msg.contains("not found"));
        assert!(msg.contains("product"));
        assert!(msg.contains("prod-1"));
    }

    #[test]
    fn conflict_kind_and_display() {
        let err = CoreError::Conflict {
            entity: "category",
            field: "name",
        };
        assert!(matches!(err.kind(), CoreErrorKind::Conflict));
        let msg = err.to_string();
        assert!(msg.contains("conflict"));
        assert!(msg.contains("category"));
        assert!(msg.contains("name"));
    }

    #[test]
    fn validation_kind_and_display() {
        let err = CoreError::Validation {
            field: "price",
            message: "must be positive".into(),
        };
        assert!(matches!(err.kind(), CoreErrorKind::Validation));
        let msg = err.to_string();
        assert!(msg.contains("validation error"));
        assert!(msg.contains("price"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn internal_kind_and_display() {
        let err = CoreError::Internal("something went wrong".into());
        assert!(matches!(err.kind(), CoreErrorKind::Internal));
        let msg = err.to_string();
        assert!(msg.contains("internal error"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn platform_error_kind() {
        let err = CoreError::Platform(platform_core::PlatformError::Internal("test".into()));
        assert!(matches!(err.kind(), CoreErrorKind::Platform));
    }
}
