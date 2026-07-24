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
    fn in_memory_rotate_key_roundtrip() {
        let k = InMemoryKeyring::new();

        // First rotation — no existing key, no prev
        let info = k.rotate_key("test-key").unwrap();
        assert_eq!(info.key_name, "test-key");
        assert_eq!(info.key_bytes, 32);
        assert!(!info.created_at.is_empty());

        // Key should be set
        let secret = k.get_secret("test-key").unwrap().unwrap();
        assert_eq!(secret.len(), 64); // 32 bytes = 64 hex chars

        // Timestamp should be set
        let created = k.key_created_at("test-key").unwrap().unwrap();
        assert_eq!(created, info.created_at);

        // Second rotation — previous key should be archived
        let _info2 = k.rotate_key("test-key").unwrap();
        let prev = k.get_secret("test-key-prev").unwrap().unwrap();
        assert_eq!(prev, secret);
    }

    #[test]
    fn in_memory_key_created_at_missing() {
        let k = InMemoryKeyring::new();
        assert_eq!(k.key_created_at("nonexistent").unwrap(), None);
    }

    #[test]
    fn in_memory_rotate_key_returns_distinct_keys() {
        let k = InMemoryKeyring::new();

        k.rotate_key("k1").unwrap();
        k.rotate_key("k2").unwrap();

        let s1 = k.get_secret("k1").unwrap().unwrap();
        let s2 = k.get_secret("k2").unwrap().unwrap();
        assert_ne!(s1, s2, "two rotations should produce different keys");
    }

    #[test]
    fn default_keyring_returns_something() {
        // Should not panic, though on non-native platforms it'll be
        // InMemoryKeyring.
        let _k = default_keyring().unwrap();
    }

    // ── Boundary / invariant tests for InMemoryKeyring ───────────────

    /// Secret names are treated as opaque strings — the empty string
    /// is a valid name (though semantically dubious). Pins the
    /// behavior so a future change adding "name validation" is
    /// deliberate, not accidental.
    #[test]
    fn in_memory_empty_name_roundtrip() {
        let k = InMemoryKeyring::new();
        k.set_secret("", "empty-name-secret").unwrap();
        assert_eq!(k.get_secret("").unwrap(), Some("empty-name-secret".into()));
        assert!(k.delete_secret("").unwrap());
        assert_eq!(k.get_secret("").unwrap(), None);
    }

    /// Secret values accept the empty string. Pins behavior for
    /// callers that store presence-flags rather than actual data.
    #[test]
    fn in_memory_empty_value_is_valid() {
        let k = InMemoryKeyring::new();
        k.set_secret("flag", "").unwrap();
        assert_eq!(k.get_secret("flag").unwrap(), Some(String::new()));
    }

    /// Secret names containing path separators, slashes, dots, and
    /// Unicode are stored verbatim — the keyring does not interpret
    /// or sanitize names. Pins the API contract.
    #[test]
    fn in_memory_special_char_names_are_opaque() {
        let k = InMemoryKeyring::new();
        let weird_names = [
            "with/slash",
            "with\\backslash",
            "with.dot",
            "\u{4E2D}\u{6587}", // Chinese
            "with-trailing-space ",
            " leading-space",
            "name\twith\ttab",
        ];
        for (i, name) in weird_names.iter().enumerate() {
            let value = format!("v{i}");
            k.set_secret(name, &value).unwrap();
            assert_eq!(
                k.get_secret(name).unwrap(),
                Some(value.clone()),
                "name {name:?} should round-trip"
            );
        }
        // Names are case-sensitive: "Foo" != "foo".
        k.set_secret("Foo", "upper").unwrap();
        k.set_secret("foo", "lower").unwrap();
        assert_eq!(k.get_secret("Foo").unwrap(), Some("upper".into()));
        assert_eq!(k.get_secret("foo").unwrap(), Some("lower".into()));
    }

    /// Large name (1 KiB) and large value (100 KiB) roundtrip.
    /// Pins behavior at non-trivial sizes; the in-memory store has
    /// no implicit size limits.
    #[test]
    fn in_memory_large_name_and_value() {
        let k = InMemoryKeyring::new();
        let long_name: String = "a".repeat(1024);
        let long_value: String = "v".repeat(100 * 1024);
        k.set_secret(&long_name, &long_value).unwrap();
        let got = k.get_secret(&long_name).unwrap().unwrap();
        assert_eq!(got.len(), long_value.len());
        assert_eq!(got, long_value);
    }

    /// Repeated `set_secret` calls with the same name must overwrite
    /// atomically — the final `get_secret` returns the last value.
    /// Pins that there's no "append" semantic.
    #[test]
    fn in_memory_repeated_set_keeps_last_value() {
        let k = InMemoryKeyring::new();
        k.set_secret("k", "v1").unwrap();
        k.set_secret("k", "v2").unwrap();
        k.set_secret("k", "v3").unwrap();
        k.set_secret("k", "v4").unwrap();
        assert_eq!(k.get_secret("k").unwrap(), Some("v4".into()));
    }

    /// Deleting a key twice returns true then false — unlike the
    /// "delete is idempotent" semantics some KV stores adopt.
    /// Pins the current behavior so any future change is intentional.
    #[test]
    fn in_memory_double_delete_distinguishes_presence() {
        let k = InMemoryKeyring::new();
        k.set_secret("k", "v").unwrap();
        assert!(k.delete_secret("k").unwrap()); // first delete: was present
        assert!(!k.delete_secret("k").unwrap()); // second delete: was absent
    }

    /// Three consecutive rotations must produce three distinct
    /// 64-char hex keys, and the `prev` slot must hold the
    /// SECOND-most-recent value (not the first or the latest).
    #[test]
    fn in_memory_three_rotations_chain_prev_correctly() {
        let k = InMemoryKeyring::new();
        _ = k.rotate_key("chain").unwrap();
        let k1 = k.get_secret("chain").unwrap().unwrap();

        _ = k.rotate_key("chain").unwrap();
        let k2 = k.get_secret("chain").unwrap().unwrap();

        _ = k.rotate_key("chain").unwrap();
        let k3 = k.get_secret("chain").unwrap().unwrap();

        // All three keys are distinct (256-bit random — collision ~ 2^-128).
        assert_ne!(k1, k2);
        assert_ne!(k2, k3);
        assert_ne!(k1, k3);

        // All three keys are 64 hex chars (32 bytes).
        assert_eq!(k1.len(), 64);
        assert_eq!(k2.len(), 64);
        assert_eq!(k3.len(), 64);

        // All three keys parse as valid lowercase hex.
        for key in [&k1, &k2, &k3] {
            assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        }

        // After three rotations: `chain` = k3, `chain-prev` = k2.
        // k1 was clobbered when rotation #2 archived k1 into "prev",
        // then rotation #3 archived k2 into "prev" — so k1 is gone.
        assert_eq!(k.get_secret("chain").unwrap().unwrap(), k3);
        assert_eq!(k.get_secret("chain-prev").unwrap().unwrap(), k2);
    }

    /// Successive rotation timestamps must be distinct. Pins the
    /// invariant that `chrono::Utc::now()` has sub-millisecond
    /// resolution — failures here would indicate a clock-source
    /// regression on the platform. 25ms sleep is well above all
    /// platforms' clock resolution (Linux clock_gettime, Windows
    /// GetSystemTimeAsFileTime at 100ns, macOS walltime_ns) while
    /// keeping the suite fast on heavily-loaded CI runners.
    #[test]
    fn in_memory_rotation_timestamps_advance() {
        let k = InMemoryKeyring::new();
        let t1 = k.rotate_key("clk").unwrap().created_at;
        std::thread::sleep(std::time::Duration::from_millis(25));
        let t2 = k.rotate_key("clk").unwrap().created_at;
        assert_ne!(t1, t2, "two rotations should have distinct timestamps");
        // ISO 8601 round-trips through `chrono::DateTime::parse_from_rfc3339`.
        assert!(chrono::DateTime::parse_from_rfc3339(&t1).is_ok());
        assert!(chrono::DateTime::parse_from_rfc3339(&t2).is_ok());
    }

    /// `key_created_at` only returns a timestamp for keys that went
    /// through `rotate_key`. A key inserted via `set_secret` has no
    /// associated `-created-at` companion, so `key_created_at` returns
    /// `None`. Pins that the default impl's suffix-append behavior is
    /// the SOLE producer of timestamps.
    #[test]
    fn in_memory_key_created_at_requires_rotate() {
        let k = InMemoryKeyring::new();
        // Manual set_secret does NOT populate `-created-at`.
        k.set_secret("manually-set", "x").unwrap();
        assert_eq!(k.key_created_at("manually-set").unwrap(), None);

        // rotate_key DOES populate `-created-at`.
        let info = k.rotate_key("rotated-key").unwrap();
        assert_eq!(
            k.key_created_at("rotated-key").unwrap(),
            Some(info.created_at.clone())
        );

        // The raw key still has the 64-hex-char secret value, not a timestamp.
        let raw = k.get_secret("rotated-key").unwrap().unwrap();
        assert_eq!(raw.len(), 64);
    }

    /// 8 threads × 100 ops on disjoint keys → no panic, no race.
    /// Pins the `InMemoryKeyring` Mutex-safety: 800 ops in parallel
    /// must NOT race, lose writes, or panic.
    #[test]
    fn in_memory_concurrent_disjoint_keys_no_panic() {
        use std::sync::Arc;
        use std::thread;

        let k = Arc::new(InMemoryKeyring::new());
        let mut handles = Vec::new();

        for thread_idx in 0..8 {
            let k = Arc::clone(&k);
            handles.push(thread::spawn(move || {
                for op_idx in 0..100 {
                    let name = format!("t{thread_idx}-k{op_idx}");
                    let value = format!("v{thread_idx}-{op_idx}");
                    k.set_secret(&name, &value).unwrap();
                    let got = k.get_secret(&name).unwrap().unwrap();
                    assert_eq!(got, value);
                    if op_idx % 2 == 0 {
                        assert!(k.delete_secret(&name).unwrap());
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("thread must not panic");
        }

        // After 800 ops: 50 even indices per thread were deleted
        // (8 threads × 50 = 400 deletions); 50 odd indices per
        // thread remain (8 × 50 = 400 secrets).
        for thread_idx in 0..8 {
            for op_idx in (0..100).filter(|i| i % 2 != 0) {
                let name = format!("t{thread_idx}-k{op_idx}");
                let value = format!("v{thread_idx}-{op_idx}");
                assert_eq!(
                    k.get_secret(&name).unwrap(),
                    Some(value),
                    "missing key {name} after concurrent storm"
                );
            }
        }
    }
}
