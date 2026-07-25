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
    use crate::test_helpers::{CredentialGuard, set_and_verify, unique_test_name};

    fn test_keyring() -> MacOsKeychain {
        MacOsKeychain::new().expect("failed to create keyring")
    }

    #[test]
    fn macos_roundtrip() {
        let k = test_keyring();
        let name = unique_test_name("oz-pos-test-macos-roundtrip");
        let _guard = CredentialGuard::new(name.clone(), &k);

        assert_eq!(k.get_secret(&name).unwrap(), None);

        set_and_verify(&k, &name, "s3kr3t!");

        assert!(k.delete_secret(&name).unwrap());
        assert_eq!(k.get_secret(&name).unwrap(), None);
    }

    #[test]
    fn macos_delete_nonexistent_returns_false() {
        let k = test_keyring();
        let name = unique_test_name("oz-pos-test-nonexistent-del-mac");
        let _guard = CredentialGuard::new(name.clone(), &k);
        assert!(!k.delete_secret(&name).unwrap());
    }

    #[test]
    fn macos_overwrite_existing() {
        let k = test_keyring();
        let name = unique_test_name("oz-pos-test-overwrite-mac");
        let _guard = CredentialGuard::new(name.clone(), &k);

        // Retry the writes until each value is observed. The macOS
        // keychain can be asynchronous about writes, so polling is more
        // robust than a single write/read.
        set_and_verify(&k, &name, "first");
        set_and_verify(&k, &name, "second");
    }
}
