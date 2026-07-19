//! Security commands — key rotation, key age, and related PCI-DSS
//! compliance operations.
//!
//! These commands expose the [`oz_security::Keyring`] trait to the
//! front-end so users can rotate encryption keys and monitor key age
//! from the Settings page.

use oz_security::RotationInfo;
use serde::Serialize;
use tauri::command;

use crate::error::AppError;

/// Key name used for the primary encryption key in the OS keyring.
pub const ENCRYPTION_KEY_NAME: &str = "oz-pos/encryption-key";

/// Response for the key rotation status query.
#[derive(Debug, Serialize)]
pub struct KeyRotationStatus {
    /// Whether a key has been created/rotated at least once.
    pub has_key: bool,
    /// ISO 8601 timestamp of when the current key was created.
    /// `None` if no key exists or timestamp is missing.
    pub created_at: Option<String>,
    /// Number of days since the key was created.
    /// `None` if the key age is unknown.
    pub age_days: Option<i64>,
}

/// Get the current key rotation status (key age, creation timestamp).
///
/// Returns the status without exposing the key material itself.
#[command]
pub async fn get_key_rotation_info() -> Result<KeyRotationStatus, AppError> {
    let keyring = oz_security::default_keyring()
        .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;

    let created_at: Option<String> = keyring.key_created_at(ENCRYPTION_KEY_NAME)?;

    let age_days = created_at.as_ref().and_then(|ts| {
        let created = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(created);
        Some(duration.num_days())
    });

    Ok(KeyRotationStatus {
        has_key: keyring.get_secret(ENCRYPTION_KEY_NAME)?.is_some(),
        created_at,
        age_days,
    })
}

/// Rotate (re-generate) the encryption key.
///
/// Generates a new random 256-bit AES key, archives the previous key,
/// and stores the creation timestamp. Returns the [`RotationInfo`] with
/// the new key's metadata.
#[command]
pub async fn rotate_encryption_key() -> Result<RotationInfo, AppError> {
    let keyring = oz_security::default_keyring()
        .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;

    let info = keyring.rotate_key(ENCRYPTION_KEY_NAME)?;

    tracing::info!(
        key_name = %info.key_name,
        created_at = %info.created_at,
        "encryption key rotated successfully"
    );

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_name_is_constant() {
        assert_eq!(ENCRYPTION_KEY_NAME, "oz-pos/encryption-key");
    }

    #[test]
    fn rotation_status_defaults() {
        // When there's no key, status should reflect that.
        let keyring = oz_security::InMemoryKeyring::new();
        assert_eq!(keyring.key_created_at("test").unwrap(), None);
    }

    #[tokio::test]
    async fn get_key_rotation_info_returns_status() {
        // Use the in-memory keyring (no platform dependencies)
        let status = get_key_rotation_info().await.unwrap();
        // Initially no key exists
        assert!(!status.has_key);
        assert!(status.created_at.is_none());
        assert!(status.age_days.is_none());
    }
}
