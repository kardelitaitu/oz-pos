//! Staff authentication commands — login, logout, session verification.
//!
//! These commands are the IPC surface for `ui/src/features/auth/`. PIN
//! hashing and verification is delegated to `oz_core::auth`.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::LoginSession;
use oz_core::db::Store;

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

/// Arguments for the `staff_login` command.
#[derive(Debug, Deserialize)]
pub struct StaffLoginArgs {
    /// Staff username (case-sensitive).
    pub username: String,
    /// Plain-text PIN entered by the staff member.
    pub pin: String,
}

/// Result of a successful staff login.
#[derive(Debug, Serialize)]
pub struct StaffLoginResult {
    /// Session info including user id, display name, and role.
    pub session: LoginSession,
}

/// Authenticate a staff member by username and PIN.
///
/// Looks up the user by username, verifies the PIN against the stored
/// argon2 hash, and returns a [`LoginSession`] on success.
///
/// # Errors
///
/// Returns `Invalid` if:
/// - The username doesn't match any active user
/// - The PIN doesn't match the stored hash
#[command]
pub async fn staff_login(
    args: StaffLoginArgs,
    state: State<'_, AppState>,
) -> Result<StaffLoginResult, AppError> {
    let username = args.username.trim().to_lowercase();
    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Look up user by username.
    let user = store
        .get_user_by_username(&username)?
        .ok_or_else(|| AppError::Invalid("invalid username or PIN".into()))?;

    // Check if user is active.
    if !user.is_active {
        return Err(AppError::Invalid("account is deactivated".into()));
    }

    // Verify PIN against stored hash.
    let valid = oz_core::auth::verify_pin(&args.pin, &user.pin_hash)
        .map_err(|e| AppError::Internal(format!("PIN verification failed: {e}")))?;

    if !valid {
        return Err(AppError::Invalid("invalid username or PIN".into()));
    }

    // Look up role for the session.
    let role = store
        .get_role(&user.role_id)?
        .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;

    drop(db);

    Ok(StaffLoginResult {
        session: LoginSession {
            user_id: user.id,
            display_name: user.display_name,
            role_name: role.name,
            role_id: role.id,
        },
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── StaffLoginArgs ──────────────────────────────────────────────────

    #[test]
    fn staff_login_args_deserialize() {
        let json = r##"{"username":"jdoe","pin":"1234"}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "jdoe");
        assert_eq!(args.pin, "1234");
    }

    #[test]
    fn staff_login_args_debug() {
        let args = StaffLoginArgs {
            username: "u".into(),
            pin: "0000".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("u"));
    }

    // ── StaffLoginResult ────────────────────────────────────────────────

    #[test]
    fn staff_login_result_serialize() {
        let session = LoginSession {
            user_id: "u1".into(),
            display_name: "John".into(),
            role_name: "Manager".into(),
            role_id: "r1".into(),
        };
        let result = StaffLoginResult { session };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["user_id"], "u1");
        assert_eq!(json["session"]["role_name"], "Manager");
    }

    #[test]
    fn staff_login_result_debug() {
        let session = LoginSession {
            user_id: "u2".into(),
            display_name: "Alice".into(),
            role_name: "Cashier".into(),
            role_id: "r2".into(),
        };
        let result = StaffLoginResult { session };
        let d = format!("{result:?}");
        assert!(d.contains("Alice"));
    }
}
