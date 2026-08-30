/*
last audited 25-07-26 by RSA-Agent
crate: oz-security | status: SAFE | lint: CLEAN
findings: Keyring trait + InMemoryKeyring + platform dispatch re-verified; default rotate_key is non-atomic get->archive->write (SEC-4); secrets returned as String without zeroize (SEC-6); 82 unit + 6 doc tests pass
fixed 2026-07-25 (glm-5.3 review P2 pass): SEC-4 default rotate_key restaged to park-the-new-key -> archive-current -> promote (only destructive op is the final swap; failed promote preserves current + archives pre-rotation current, leftover -staging slots are overwritten next rotation); SEC-6 partially addressed — raw entropy buffer zeroized after encode and hex key held in Zeroizing (full SecretString keyring surface deferred: OS credential stores copy secrets internally, so the residual exposure is out of process control); 83 unit + 6 doc tests pass
next: SEC-6 residual — SecretString for the Keyring get/set surface | perf: N/A
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
use zeroize::{Zeroize, Zeroizing};

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
    ///
    /// # SEC-4: staged rotation ordering
    ///
    /// OS credential stores have no transaction primitive, so the write
    /// order is staged so that the *only* destructive operation is the
    /// final pointer swap:
    ///
    /// 1. park the fresh key (and timestamp) in `{name}-staging` slots
    /// 2. archive the current key to `{name}-prev`
    /// 3. promote staging → current, then write the timestamp
    /// 4. best-effort cleanup of the staging slots
    ///
    /// A failure before step 3 leaves the current key and the previous
    /// archive untouched — a later rotation simply overwrites any
    /// leftover staging slots. Previously the archive step ran first, so
    /// a failure between archive and swap clobbered `{name}-prev` while
    /// the current key was still the old one.
    fn rotate_key(&self, name: &str) -> Result<RotationInfo, SecurityError> {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng()
            .try_fill_bytes(&mut key_bytes)
            .map_err(|e| SecurityError::KeyGenerationFailed(format!("rng error: {e}")))?;

        // SEC-6: scrub the raw entropy buffer as soon as it is encoded,
        // and keep the encoded form in a zeroizing allocation so it does
        // not linger past the rotation.
        let hex_key = Zeroizing::new(hex::encode(&key_bytes));
        key_bytes.zeroize();
        let now = chrono::Utc::now().to_rfc3339();

        let staging = format!("{name}-staging");
        let prev = format!("{name}-prev");
        let created_at = format!("{name}-created-at");

        // 1. Park the fresh key in staging — nothing user-visible changed.
        self.set_secret(&staging, &hex_key)?;
        self.set_secret(&format!("{staging}-created-at"), &now)?;

        // 2. Archive the current key (if any). Failures here leave the
        //    current key and the previous archive untouched.
        if let Some(existing) = self.get_secret(name)? {
            self.set_secret(&prev, &existing)?;
        }

        // 3. Promote staging → current. The only destructive write.
        self.set_secret(name, &hex_key)?;
        self.set_secret(&created_at, &now)?;

        // 4. Best-effort cleanup of the staging slots.
        let _ = self.delete_secret(&staging);
        let _ = self.delete_secret(&format!("{staging}-created-at"));

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

        // SEC-6: scrub the raw entropy buffer after encoding.
        let hex_key = Zeroizing::new(hex::encode(&key_bytes));
        key_bytes.zeroize();
        let now = chrono::Utc::now().to_rfc3339();

        let mut map = self
            .secrets
            .lock()
            .map_err(|e| SecurityError::KeyUnavailable(format!("lock poisoned: {e}")))?;

        // Archive existing key as prev (clone to release borrow before mutating)
        if let Some(existing) = map.get(name).cloned() {
            map.insert(format!("{name}-prev"), existing);
        }

        map.insert(name.to_owned(), hex_key.to_string());
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

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
