//! Error type for `platform-core`.
//!
//! Uses `thiserror` so consumers can match on variants.
//! The enum is `#[non_exhaustive]` to allow adding variants
//! without breaking semver.

use thiserror::Error;

/// Errors that can originate in `platform-core` infrastructure.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A setting key was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// An internal error (serialization, crypto, I/O, etc.).
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_error_display() {
        let err = PlatformError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        let msg = err.to_string();
        assert!(
            msg.starts_with("database error:"),
            "expected db error prefix, got: {msg}"
        );
    }

    #[test]
    fn not_found_error_display() {
        let err = PlatformError::NotFound("setting.key".into());
        assert_eq!(err.to_string(), "not found: setting.key");
    }

    #[test]
    fn internal_error_display() {
        let err = PlatformError::Internal("something went wrong".into());
        assert_eq!(err.to_string(), "internal error: something went wrong");
    }

    #[test]
    fn platform_error_debug() {
        let err = PlatformError::Internal("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn from_rusqlite_error() {
        let inner = rusqlite::Error::InvalidColumnName("col".into());
        let err: PlatformError = inner.into();
        assert!(err.to_string().contains("database error"));
    }

    #[test]
    fn platform_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PlatformError>();
    }

    #[test]
    fn platform_error_variants_are_distinct() {
        let db_err = PlatformError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        let nf_err = PlatformError::NotFound("x".into());
        let int_err = PlatformError::Internal("x".into());
        assert_ne!(format!("{db_err:?}"), format!("{nf_err:?}"));
        assert_ne!(format!("{nf_err:?}"), format!("{int_err:?}"));
    }
}
