//! Security commands — key rotation, key age, and related PCI-DSS
//! compliance operations.
//!
//! These commands expose the [`oz_security::Keyring`] trait to the
//! front-end so users can rotate encryption keys and monitor key age
//! from the Settings page.

use oz_core::permissions;
use oz_security::{Keyring, RotationInfo};
use serde::Serialize;
use tauri::State;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

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

/// Run a keyring operation outside the Tokio runtime context.
///
/// The Linux Secret Service implementation owns a private Tokio runtime and
/// calls `block_on` for its synchronous [`Keyring`] methods. Running the
/// complete operation on a dedicated OS thread prevents that private runtime
/// from being nested inside Tauri's runtime (including `spawn_blocking`, whose
/// threads still belong to the Tokio runtime).
async fn with_keyring<T, C, F>(create: C, operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    C: FnOnce() -> Result<Box<dyn Keyring>, AppError> + Send + 'static,
    F: FnOnce(&dyn Keyring) -> Result<T, AppError> + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let result = (|| {
            let keyring = create()?;
            operation(keyring.as_ref())
        })();

        // The command may be cancelled while the keyring operation is still
        // running; in that case there is no receiver left to notify.
        let _ = sender.send(result);
    });

    receiver
        .await
        .map_err(|_| AppError::Internal("keyring worker stopped unexpectedly".into()))?
}

async fn key_rotation_info_with<C>(create: C) -> Result<KeyRotationStatus, AppError>
where
    C: FnOnce() -> Result<Box<dyn Keyring>, AppError> + Send + 'static,
{
    with_keyring(create, key_rotation_status).await
}

fn key_rotation_status(keyring: &dyn Keyring) -> Result<KeyRotationStatus, AppError> {
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

/// Get the current key rotation status (key age, creation timestamp).
///
/// Returns the status without exposing the key material itself.
#[tauri::command]
pub async fn get_key_rotation_info() -> Result<KeyRotationStatus, AppError> {
    key_rotation_info_with(|| {
        oz_security::default_keyring()
            .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))
    })
    .await
}

fn rotate_key(keyring: &dyn Keyring) -> Result<RotationInfo, AppError> {
    let info = keyring.rotate_key(ENCRYPTION_KEY_NAME)?;

    tracing::info!(
        key_name = %info.key_name,
        created_at = %info.created_at,
        "encryption key rotated successfully"
    );

    Ok(info)
}

/// Rotate (re-generate) the encryption key.
///
/// Generates a new random 256-bit AES key, archives the previous key,
/// and stores the creation timestamp. Returns the [`RotationInfo`] with
/// the new key's metadata.
#[tauri::command]
pub async fn rotate_encryption_key() -> Result<RotationInfo, AppError> {
    with_keyring(
        || {
            oz_security::default_keyring()
                .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))
        },
        rotate_key,
    )
    .await
}

/// Session-scoped variant of [`get_key_rotation_info`].
#[tauri::command]
pub async fn get_key_rotation_info_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<KeyRotationStatus, AppError> {
    let session = state.resolve_session(&session_token)?;
    // F-017: key age/state is crypto-compliance data — explicit permission.
    require_permission_for_session(&state, &session, permissions::SECURITY_MANAGE).await?;
    get_key_rotation_info().await
}

/// Session-scoped variant of [`rotate_encryption_key`].
#[tauri::command]
pub async fn rotate_encryption_key_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<RotationInfo, AppError> {
    let session = state.resolve_session(&session_token)?;
    // F-017: rotating the at-rest key invalidates every archived key —
    // crypto administration — sensitive key, explicit permission.
    require_permission_for_session(&state, &session, permissions::SECURITY_MANAGE).await?;
    rotate_encryption_key().await
}

#[cfg(test)]
#[path = "security_tests.rs"]
mod tests;
