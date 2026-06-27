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
