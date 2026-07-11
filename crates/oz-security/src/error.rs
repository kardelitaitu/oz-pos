//! Error type for `oz-security`.

use thiserror::Error;

/// Errors that can originate in the security subsystem.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SecurityError {
    /// A key-encryption-key (KEK) was missing or inaccessible.
    #[error("key unavailable: {0}")]
    KeyUnavailable(String),

    /// A secret's ciphertext failed authentication on decrypt.
    #[error("decryption failed: ciphertext is corrupt or has been tampered with")]
    DecryptionFailed,

    /// The caller does not have the required permission.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_unavailable_display() {
        let err = SecurityError::KeyUnavailable("kek-1234".into());
        assert_eq!(err.to_string(), "key unavailable: kek-1234");
    }

    #[test]
    fn decryption_failed_display() {
        let err = SecurityError::DecryptionFailed;
        assert_eq!(
            err.to_string(),
            "decryption failed: ciphertext is corrupt or has been tampered with"
        );
    }

    #[test]
    fn permission_denied_display() {
        let err = SecurityError::PermissionDenied("admin access required".into());
        assert_eq!(err.to_string(), "permission denied: admin access required");
    }

    #[test]
    fn debug_output() {
        let err = SecurityError::KeyUnavailable("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn implements_std_error() {
        let err = SecurityError::DecryptionFailed;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SecurityError>();
    }

    #[test]
    fn variants_are_distinct() {
        let a = format!("{:?}", SecurityError::KeyUnavailable("x".into()));
        let b = format!("{:?}", SecurityError::DecryptionFailed);
        assert_ne!(a, b);
    }
}
