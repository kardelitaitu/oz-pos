//! Error type for `oz-reporting`.

use thiserror::Error;

/// Errors that can originate in a reporting query or export.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReportingError {
    /// The underlying SQLite query failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// The requested time window is invalid (e.g., end before start).
    #[error("invalid time window: {0}")]
    InvalidWindow(String),

    /// A CSV export could not be written to disk.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_display() {
        let err = ReportingError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        assert!(err.to_string().contains("database error:"));
    }

    #[test]
    fn db_source() {
        let err = ReportingError::Db(rusqlite::Error::InvalidParameterName("x".into()));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn invalid_window_display() {
        let err = ReportingError::InvalidWindow("end before start".into());
        assert_eq!(err.to_string(), "invalid time window: end before start");
    }

    #[test]
    fn io_display_and_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ReportingError::Io(io);
        assert!(err.to_string().contains("i/o error:"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn debug_output() {
        let err = ReportingError::InvalidWindow("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn implements_std_error() {
        let err = ReportingError::InvalidWindow("test".into());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ReportingError>();
    }

    #[test]
    fn variants_are_distinct() {
        let a = format!("{:?}", ReportingError::InvalidWindow("x".into()));
        let b = format!(
            "{:?}",
            ReportingError::Db(rusqlite::Error::InvalidParameterName("x".into()))
        );
        assert_ne!(a, b);
    }
}
