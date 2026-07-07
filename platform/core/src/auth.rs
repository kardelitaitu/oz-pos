//! Authentication primitives — PIN hashing and verification.
//!
//! Uses the `argon2` crate for password hashing with Argon2id.
//! The default configuration provides reasonable security for a
//! local POS terminal PIN while keeping verification fast.

use crate::error::PlatformError;

/// Hash a PIN/password for storage.
///
/// Returns the PHC-formatted hash string (e.g.
/// `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`).
///
/// # Errors
///
/// Returns [`PlatformError::Internal`] if the argon2 library fails.
pub fn hash_pin(pin: &str) -> Result<String, PlatformError> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };

    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| PlatformError::Internal(format!("argon2 hash failed: {e}")))?;

    Ok(hash.to_string())
}

/// Verify a PIN against a stored PHC-formatted hash.
///
/// Returns `true` if the PIN matches the hash, `false` otherwise.
///
/// # Errors
///
/// Returns [`PlatformError::Internal`] if the argon2 library fails.
pub fn verify_pin(pin: &str, hash: &str) -> Result<bool, PlatformError> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };

    let parsed = PasswordHash::new(hash)
        .map_err(|e| PlatformError::Internal(format!("invalid password hash: {e}")))?;

    let argon2 = Argon2::default();
    Ok(argon2.verify_password(pin.as_bytes(), &parsed).is_ok())
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
        let h1 = hash_pin("0000").unwrap();
        let h2 = hash_pin("0000").unwrap();
        assert_ne!(h1, h2);
        assert!(verify_pin("0000", &h1).unwrap());
        assert!(verify_pin("0000", &h2).unwrap());
    }

    #[test]
    fn verify_invalid_hash_format() {
        let result = verify_pin("1234", "not-a-valid-hash");
        assert!(result.is_err());
    }

    #[test]
    fn login_session_serde_roundtrip() {
        let session = LoginSession {
            user_id: "u1".into(),
            display_name: "Alice".into(),
            role_name: "cashier".into(),
            role_id: "role-cashier".into(),
        };
        let json = serde_json::to_string(&session).unwrap();
        let back: LoginSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_id, "u1");
        assert_eq!(back.display_name, "Alice");
        assert_eq!(back.role_name, "cashier");
    }

    #[test]
    fn login_session_debug() {
        let session = LoginSession {
            user_id: "u1".into(),
            display_name: "Alice".into(),
            role_name: "manager".into(),
            role_id: "role-manager".into(),
        };
        let debug = format!("{session:?}");
        assert!(debug.contains("u1"));
        assert!(debug.contains("Alice"));
        assert!(debug.contains("manager"));
    }

    #[test]
    fn login_session_clone_eq() {
        let s1 = LoginSession {
            user_id: "u1".into(),
            display_name: "Bob".into(),
            role_name: "owner".into(),
            role_id: "role-owner".into(),
        };
        let s2 = s1.clone();
        assert_eq!(s1.user_id, s2.user_id);
        assert_eq!(s1.display_name, s2.display_name);
        assert_eq!(s1.role_name, s2.role_name);
        assert_eq!(s1.role_id, s2.role_id);
    }

    #[test]
    fn login_session_json_field_names() {
        let session = LoginSession {
            user_id: "u1".into(),
            display_name: "Alice".into(),
            role_name: "cashier".into(),
            role_id: "role-cashier".into(),
        };
        let json = serde_json::to_value(&session).unwrap();
        assert_eq!(json["user_id"], "u1");
        assert_eq!(json["display_name"], "Alice");
        assert_eq!(json["role_name"], "cashier");
        assert_eq!(json["role_id"], "role-cashier");
    }
}
