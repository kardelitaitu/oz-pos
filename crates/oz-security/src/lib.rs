/*
last audited 19-07-26 by RSA-Agent
crate: oz-security | status: SAFE (crate root) | lint: CLEAN
findings: #![deny(unsafe_code)] at crate root. windows.rs module has #![allow(unsafe_code)] override for FFI — see stamp in windows.rs. SAFETY comments added 19-07-26.
next: none | perf: N/A
*/

//! Encryption, secrets, and PCI-DSS helpers for OZ-POS.
//!
//! `oz-security` is responsible for at-rest encryption, secret
//! management, key rotation, and the small set of PCI-DSS-related
//! utilities the cashier flow needs (masked PAN display, audit
//! logging, etc.).
//!
//! # Keyring
//!
//! The [`Keyring`] trait provides an OS-credential-store abstraction:
//!
//! - **Windows**: Credential Manager (`wincred`)
//! - **Linux**: Secret Service (libsecret / DBus)
//! - **macOS**: Keychain (Security framework)
//! - **Fallback**: In-memory store (development only)

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod mask;
pub mod tls;
#[cfg(target_os = "windows")]
pub mod windows;

pub use error::SecurityError;

// ── Keyring trait ────────────────────────────────────────────────────

/// OS-level credential store abstraction.
///
/// Implementations store secrets in the platform's native keyring:
/// Windows Credential Manager, Linux Secret Service, or macOS Keychain.
///
/// # Example
///
/// ```ignore
/// use oz_security::Keyring;
///
/// let keyring = oz_security::default_keyring()?;
/// keyring.set_secret("api-key", "sk_live_abc123")?;
/// let secret = keyring.get_secret("api-key")?;
/// keyring.delete_secret("api-key")?;
/// ```
pub trait Keyring {
    /// Retrieve a secret by name. Returns `None` if the secret doesn't
    /// exist.
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecurityError>;

    /// Store a secret, overwriting any existing value with the same
    /// name.
    fn set_secret(&self, name: &str, value: &str) -> Result<(), SecurityError>;

    /// Delete a secret by name. Returns `true` if the secret existed
    /// and was removed.
    fn delete_secret(&self, name: &str) -> Result<bool, SecurityError>;
}

// ── Default keyring ─────────────────────────────────────────────────

/// Create the platform-native keyring.
///
/// - Windows → `WindowsCredentialManager`
/// - Linux → `LibSecretKeyring`
/// - macOS → `MacOsKeychain`
/// - Other → `InMemoryKeyring` (dev fallback)
pub fn default_keyring() -> Result<Box<dyn Keyring>, SecurityError> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsCredentialManager::new()?))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LibSecretKeyring::new()?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOsKeychain::new()?))
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Ok(Box::new(InMemoryKeyring::new()))
    }
}

// ── In-memory keyring (dev fallback) ─────────────────────────────────

/// In-memory-only credential store.
///
/// **This is NOT secure.** Use [`default_keyring`] in production.
/// The in-memory store is intended for development and testing where
/// the platform keyring is unavailable (e.g. CI, WASM, embedded).
#[derive(Debug, Default)]
pub struct InMemoryKeyring {
    secrets: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemoryKeyring {
    /// Create a new empty in-memory keyring.
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Keyring for InMemoryKeyring {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecurityError> {
        let map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;
        Ok(map.get(name).cloned())
    }

    fn set_secret(&self, name: &str, value: &str) -> Result<(), SecurityError> {
        let mut map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;
        map.insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete_secret(&self, name: &str) -> Result<bool, SecurityError> {
        let mut map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;
        Ok(map.remove(name).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_roundtrip() {
        let k = InMemoryKeyring::new();
        assert_eq!(k.get_secret("foo").unwrap(), None);

        k.set_secret("foo", "bar").unwrap();
        assert_eq!(k.get_secret("foo").unwrap(), Some("bar".into()));

        assert!(k.delete_secret("foo").unwrap());
        assert_eq!(k.get_secret("foo").unwrap(), None);
    }

    #[test]
    fn in_memory_delete_nonexistent_returns_false() {
        let k = InMemoryKeyring::new();
        assert!(!k.delete_secret("nope").unwrap());
    }

    #[test]
    fn in_memory_overwrite_existing() {
        let k = InMemoryKeyring::new();
        k.set_secret("key", "v1").unwrap();
        k.set_secret("key", "v2").unwrap();
        assert_eq!(k.get_secret("key").unwrap(), Some("v2".into()));
    }

    #[test]
    fn in_memory_multiple_secrets() {
        let k = InMemoryKeyring::new();
        k.set_secret("a", "1").unwrap();
        k.set_secret("b", "2").unwrap();
        k.set_secret("c", "3").unwrap();
        assert_eq!(k.get_secret("a").unwrap(), Some("1".into()));
        assert_eq!(k.get_secret("b").unwrap(), Some("2".into()));
        assert_eq!(k.get_secret("c").unwrap(), Some("3".into()));
    }

    #[test]
    fn default_keyring_returns_something() {
        // Should not panic, though on non-native platforms it'll be
        // InMemoryKeyring.
        let _k = default_keyring().unwrap();
    }
}
