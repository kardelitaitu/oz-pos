//! macOS Keychain implementation of [`Keyring`].
//!
//! Wraps the Security framework (`Security.framework`) to store
//! secrets in the user's default keychain.

use crate::error::SecurityError;
use crate::Keyring;

/// macOS Keychain keyring.
///
/// Stores secrets in the user's default login keychain using the
/// Security framework's `SecItemAdd`, `SecItemCopyMatching`, and
/// `SecItemDelete` APIs.
pub struct MacOsKeychain {
    // Placeholder for future Security framework bindings.
    _private: (),
}

impl MacOsKeychain {
    /// Create a new macOS Keychain instance.
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self { _private: () })
    }
}

impl Keyring for MacOsKeychain {
    fn get_secret(&self, _name: &str) -> Result<Option<String>, SecurityError> {
        // TODO: Implement SecItemCopyMatching
        Err(SecurityError::KeyUnavailable(
            "macOS Keychain not yet implemented".into(),
        ))
    }

    fn set_secret(&self, _name: &str, _value: &str) -> Result<(), SecurityError> {
        // TODO: Implement SecItemAdd
        Err(SecurityError::KeyUnavailable(
            "macOS Keychain not yet implemented".into(),
        ))
    }

    fn delete_secret(&self, _name: &str) -> Result<bool, SecurityError> {
        // TODO: Implement SecItemDelete
        Err(SecurityError::KeyUnavailable(
            "macOS Keychain not yet implemented".into(),
        ))
    }
}
