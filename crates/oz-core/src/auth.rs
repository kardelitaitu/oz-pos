//! Authentication utilities — PIN hashing and verification.
//!
//! Uses the `argon2` crate (already a workspace dependency) for
//! password hashing. The default configuration is:
//!
//! - Algorithm: Argon2id
//! - Memory: 19 MiB
//! - Iterations: 2
//! - Parallelism: 1
//!
//! This provides reasonable security for a local POS terminal PIN
//! while keeping verification fast (< 100 ms on modern hardware).

use crate::CoreError;

/// Hash a PIN/password for storage.
///
/// Returns the PHC-formatted hash string (e.g.
/// `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`).
///
/// # Errors
///
/// Returns [`CoreError::Internal`] if the argon2 library fails
/// (e.g. insufficient memory).
pub fn hash_pin(pin: &str) -> Result<String, CoreError> {
    use argon2::{
        Argon2,
        password_hash::{rand_core::OsRng, SaltString, PasswordHasher},
    };

    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| CoreError::Internal(format!("argon2 hash failed: {e}")))?;

    Ok(hash.to_string())
}

/// Verify a PIN against a stored PHC-formatted hash.
///
/// Returns `true` if the PIN matches the hash, `false` otherwise.
///
/// # Errors
///
/// Returns [`CoreError::Internal`] if the argon2 library fails
/// unexpectedly.
pub fn verify_pin(pin: &str, hash: &str) -> Result<bool, CoreError> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };

    let parsed = PasswordHash::new(hash)
        .map_err(|e| CoreError::Internal(format!("invalid password hash: {e}")))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok())
}

/// Result of a successful staff login.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginSession {
    /// The logged-in user's id.
    pub user_id: String,
    /// Display name shown on the UI.
    pub display_name: String,
    /// Role name (e.g. "owner", "manager", "cashier").
    pub role_name: String,
    /// Role id.
    pub role_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_correct_pin() {
        let pin = "1234";
        let hash = hash_pin(pin).unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_pin(pin, &hash).unwrap());
    }

    #[test]
    fn verify_wrong_pin_returns_false() {
        let hash = hash_pin("1234").unwrap();
        assert!(!verify_pin("5678", &hash).unwrap());
    }

    #[test]
    fn verify_empty_pin() {
        let hash = hash_pin("").unwrap();
        assert!(verify_pin("", &hash).unwrap());
        assert!(!verify_pin(" ", &hash).unwrap());
    }

    #[test]
    fn hash_is_deterministic_with_different_salts() {
        // Same PIN should produce different hashes (different salt each time).
        let h1 = hash_pin("0000").unwrap();
        let h2 = hash_pin("0000").unwrap();
        assert_ne!(h1, h2);
        // Both should verify correctly.
        assert!(verify_pin("0000", &h1).unwrap());
        assert!(verify_pin("0000", &h2).unwrap());
    }

    #[test]
    fn verify_invalid_hash_format() {
        let result = verify_pin("1234", "not-a-valid-hash");
        assert!(result.is_err());
    }
}
