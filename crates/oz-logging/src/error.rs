//! Error type for `oz-logging`.

use thiserror::Error;

/// Errors that can originate in the logging subsystem.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoggingError {
    /// The log file could not be opened for writing.
    #[error("could not open log file: {0}")]
    OpenFile(#[from] std::io::Error),

    /// The configured log level is invalid.
    #[error("invalid log level: {0}")]
    InvalidLevel(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_file_display() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = LoggingError::OpenFile(io);
        assert!(err.to_string().contains("could not open log file:"));
    }

    #[test]
    fn open_file_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = LoggingError::OpenFile(io);
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn invalid_level_display() {
        let err = LoggingError::InvalidLevel("TRACE".into());
        assert_eq!(err.to_string(), "invalid log level: TRACE");
    }

    #[test]
    fn debug_output() {
        let err = LoggingError::InvalidLevel("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn implements_std_error() {
        let err = LoggingError::InvalidLevel("test".into());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LoggingError>();
    }

    #[test]
    fn variants_are_distinct() {
        let a = format!("{:?}", LoggingError::InvalidLevel("x".into()));
        let b = format!("{:?}", LoggingError::OpenFile(std::io::Error::other("x")));
        assert_ne!(a, b);
    }
}
