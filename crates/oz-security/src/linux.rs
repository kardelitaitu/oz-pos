//! Linux Secret Service (libsecret / DBus) implementation of [`Keyring`].
//!
//! Talks to the org.freedesktop.secrets DBus service to store and
//! retrieve secrets.

use crate::error::SecurityError;
use crate::Keyring;

/// Linux Secret Service keyring.
///
/// Stores secrets in the GNOME/FreeDesktop Secret Service using
/// the `org.freedesktop.secrets` DBus API.
pub struct LibSecretKeyring {
    // Placeholder for future DBus bindings.
    _private: (),
}

impl LibSecretKeyring {
    /// Create a new libsecret keyring instance.
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self { _private: () })
    }
}

impl Keyring for LibSecretKeyring {
    fn get_secret(&self, _name: &str) -> Result<Option<String>, SecurityError> {
        // TODO: Implement Secret Service lookup via DBus
        Err(SecurityError::KeyUnavailable(
            "Linux Secret Service not yet implemented".into(),
        ))
    }

    fn set_secret(&self, _name: &str, _value: &str) -> Result<(), SecurityError> {
        // TODO: Implement Secret Service store via DBus
        Err(SecurityError::KeyUnavailable(
            "Linux Secret Service not yet implemented".into(),
        ))
    }

    fn delete_secret(&self, _name: &str) -> Result<bool, SecurityError> {
        // TODO: Implement Secret Service delete via DBus
        Err(SecurityError::KeyUnavailable(
            "Linux Secret Service not yet implemented".into(),
        ))
    }
}
