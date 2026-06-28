//! Windows Credential Manager implementation of [`Keyring`].
//!
//! Wraps `wincred` / `CredWriteW` / `CredReadW` / `CredDeleteW`
//! from `advapi32.dll` to store secrets.

use crate::error::SecurityError;
use crate::Keyring;

/// Windows Credential Manager keyring.
///
/// Stores secrets in the Windows Credential Manager under the
/// target name `OZ-POS:{name}`.
pub struct WindowsCredentialManager {
    // Placeholder for future win32 API bindings.
    _private: (),
}

impl WindowsCredentialManager {
    /// Create a new Windows Credential Manager keyring instance.
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self { _private: () })
    }
}

impl Keyring for WindowsCredentialManager {
    fn get_secret(&self, _name: &str) -> Result<Option<String>, SecurityError> {
        // TODO: Implement CredReadW
        Err(SecurityError::KeyUnavailable(
            "Windows Credential Manager not yet implemented".into(),
        ))
    }

    fn set_secret(&self, _name: &str, _value: &str) -> Result<(), SecurityError> {
        // TODO: Implement CredWriteW
        Err(SecurityError::KeyUnavailable(
            "Windows Credential Manager not yet implemented".into(),
        ))
    }

    fn delete_secret(&self, _name: &str) -> Result<bool, SecurityError> {
        // TODO: Implement CredDeleteW
        Err(SecurityError::KeyUnavailable(
            "Windows Credential Manager not yet implemented".into(),
        ))
    }
}
