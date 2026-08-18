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
#[cfg(test)]
pub mod test_helpers;
pub mod tls;
#[cfg(target_os = "windows")]
pub mod windows;

use rand::RngCore;
use serde::{Deserialize, Serialize};

pub use error::SecurityError;

// ── RotationInfo ─────────────────────────────────────────────────────

/// Information about a completed key rotation.
///
/// Returned by [`Keyring::rotate_key`] so callers can display the
/// new key's creation date, update their local cache, or log the
/// rotation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationInfo {
    /// The name of the key that was rotated (e.g. `"oz-pos/encryption-key"`).
    pub key_name: String,
    /// ISO 8601 timestamp of when the new key was created.
    pub created_at: String,
    /// Number of bytes in the generated key.
    pub key_bytes: u32,
}

// ── Keyring trait ────────────────────────────────────────────────────

/// OS-level credential store abstraction.
///
/// Implementations store secrets in the platform's native keyring:
/// Windows Credential Manager, Linux Secret Service, or macOS Keychain.
///
/// # Example
///
/// ```no_run
/// # use oz_security::Keyring;
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let keyring = oz_security::default_keyring()?;
/// keyring.set_secret("api-key", "sk_live_abc123")?;
/// let secret = keyring.get_secret("api-key")?;
/// keyring.delete_secret("api-key")?;
/// # Ok(())
/// # }
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

    /// Retrieve the ISO 8601 creation timestamp for a key.
    ///
    /// Returns `None` if the key doesn't exist or no timestamp has
    /// been recorded yet (key was created before rotation tracking
    /// was added).
    fn key_created_at(&self, name: &str) -> Result<Option<String>, SecurityError> {
        self.get_secret(&format!("{name}-created-at"))
    }

    /// Generate a new random 256-bit key, store it under `name`, and
    /// archive the previous key (if any) as `{name}-prev`.
    ///
    /// Uses `get_secret` and `set_secret` for key storage, so
    /// implementors do NOT need to override this unless they need
    /// atomic lock-and-rotate (e.g. [`InMemoryKeyring`]).
    fn rotate_key(&self, name: &str) -> Result<RotationInfo, SecurityError> {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng()
            .try_fill_bytes(&mut key_bytes)
            .map_err(|e| SecurityError::KeyGenerationFailed(format!("rng error: {e}")))?;

        let hex_key = hex::encode(key_bytes);
        let now = chrono::Utc::now().to_rfc3339();

        // Archive existing key as prev
        if let Some(existing) = self.get_secret(name)? {
            self.set_secret(&format!("{name}-prev"), &existing)?;
        }

        self.set_secret(name, &hex_key)?;
        self.set_secret(&format!("{name}-created-at"), &now)?;

        Ok(RotationInfo {
            key_name: name.to_owned(),
            created_at: now,
            key_bytes: 32,
        })
    }
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

    fn rotate_key(&self, name: &str) -> Result<RotationInfo, SecurityError> {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng()
            .try_fill_bytes(&mut key_bytes)
            .map_err(|e| SecurityError::KeyGenerationFailed(format!("rng error: {e}")))?;

        let hex_key = hex::encode(key_bytes);
        let now = chrono::Utc::now().to_rfc3339();

        let mut map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;

        // Archive existing key as prev (clone to release borrow before mutating)
        if let Some(existing) = map.get(name).cloned() {
            map.insert(format!("{name}-prev"), existing);
        }

        map.insert(name.to_owned(), hex_key);
        map.insert(format!("{name}-created-at"), now.clone());

        Ok(RotationInfo {
            key_name: name.to_owned(),
            created_at: now,
            key_bytes: 32,
        })
    }

    fn key_created_at(&self, name: &str) -> Result<Option<String>, SecurityError> {
        let map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;
        Ok(map.get(&format!("{name}-created-at")).cloned())
    }
}

#[cfg(test)] #[path = "lib_tests.rs"] mod tests;
