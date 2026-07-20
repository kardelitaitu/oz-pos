//! macOS Keychain implementation of [`Keyring`].
//!
//! Wraps the Security framework (`Security.framework`) to store
//! secrets in the user's default keychain.

use crate::Keyring;
use crate::error::SecurityError;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

/// macOS Keychain keyring.
///
/// Stores secrets in the user's default login keychain using the
/// Security framework's generic password API with service `OZ-POS`
/// and account name `{name}`.
pub struct MacOsKeychain;

impl MacOsKeychain {
    /// Create a new macOS Keychain instance.
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self)
    }
}

impl Keyring for MacOsKeychain {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecurityError> {
        match get_generic_password("OZ-POS", name) {
            Ok(bytes) => {
                let s = String::from_utf8(bytes).map_err(|e| {
                    SecurityError::KeyUnavailable(format!("keychain password not valid UTF-8: {e}"))
                })?;
                Ok(Some(s))
            }
            Err(e) if e.code() < 0 => {
                // errSecItemNotFound = -25300, errSecUnimplemented = -128.
                // The security-framework crate can surface either code;
                // check the string description as a fallback.
                if format!("{e:?}").contains("item not found")
                    || format!("{e:?}").contains("-25300")
                    || format!("{e:?}").contains("-128")
                {
                    return Ok(None);
                }
                Err(SecurityError::KeyUnavailable(format!(
                    "get_generic_password failed: {e}"
                )))
            }
            Err(e) => Err(SecurityError::KeyUnavailable(format!(
                "get_generic_password failed: {e}"
            ))),
        }
    }

    fn set_secret(&self, name: &str, value: &str) -> Result<(), SecurityError> {
        set_generic_password("OZ-POS", name, value.as_bytes())
            .map_err(|e| SecurityError::KeyUnavailable(format!("set_generic_password failed: {e}")))
    }

    fn delete_secret(&self, name: &str) -> Result<bool, SecurityError> {
        match delete_generic_password("OZ-POS", name) {
            Ok(()) => Ok(true),
            Err(e) => {
                let msg = format!("{e:?}");
                if msg.contains("item not found") || msg.contains("-25300") || msg.contains("-128")
                {
                    return Ok(false);
                }
                Err(SecurityError::KeyUnavailable(format!(
                    "delete_generic_password failed: {e}"
                )))
            }
        }
    }

    // `rotate_key` and `key_created_at` use the default implementations
    // from the `Keyring` trait.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keyring() -> MacOsKeychain {
        MacOsKeychain::new().expect("failed to create keyring")
    }

    #[test]
    fn macos_roundtrip() {
        let k = test_keyring();
        let name = "oz-pos-test-macos-roundtrip";
        let _ = k.delete_secret(name);

        assert_eq!(k.get_secret(name).unwrap(), None);

        k.set_secret(name, "s3kr3t!").unwrap();
        assert_eq!(k.get_secret(name).unwrap(), Some("s3kr3t!".into()));

        assert!(k.delete_secret(name).unwrap());
        assert_eq!(k.get_secret(name).unwrap(), None);
    }

    #[test]
    fn macos_delete_nonexistent_returns_false() {
        let k = test_keyring();
        assert!(!k.delete_secret("oz-pos-test-nonexistent-del-mac").unwrap());
    }

    #[test]
    fn macos_overwrite_existing() {
        let k = test_keyring();
        let name = "oz-pos-test-overwrite-mac";
        let _ = k.delete_secret(name);

        k.set_secret(name, "first").unwrap();
        k.set_secret(name, "second").unwrap();
        assert_eq!(k.get_secret(name).unwrap(), Some("second".into()));

        k.delete_secret(name).unwrap();
    }
}
