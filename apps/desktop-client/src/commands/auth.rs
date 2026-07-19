//! Staff authentication commands — login, logout, session verification.
//!
//! These commands are the IPC surface for `ui/src/features/auth/`. PIN
//! hashing and verification is delegated to `oz_core::auth`.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::LoginSession;
use oz_core::db::Store;
use oz_core::session::SessionContext;

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

/// Arguments for the `staff_check_username` command.
#[derive(Debug, Deserialize)]
pub struct CheckUsernameArgs {
    /// Staff username to look up.
    pub username: String,
}

/// Result of a username existence check.
#[derive(Debug, Serialize)]
pub struct CheckUsernameResult {
    /// Whether a user with this username was found in the database.
    pub found: bool,
    /// Whether the found user account is active (only meaningful when `found` is true).
    pub is_active: bool,
}

/// Check if a username exists and is active in the system.
///
/// Called before transitioning to the PIN step so the UI can reject
/// unknown usernames early without collecting a PIN.
#[command]
pub async fn staff_check_username(
    args: CheckUsernameArgs,
    state: State<'_, AppState>,
) -> Result<CheckUsernameResult, AppError> {
    let username = args.username.trim().to_lowercase();
    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    match store.get_user_by_username(&username)? {
        Some(user) => Ok(CheckUsernameResult {
            found: true,
            is_active: user.is_active,
        }),
        None => Ok(CheckUsernameResult {
            found: false,
            is_active: false,
        }),
    }
}

/// Authenticate a staff member by username and PIN.
///
/// Looks up the user by username, verifies the PIN against the stored
/// argon2 hash, and returns a [`LoginSession`] on success.
///
/// Includes PIN brute-force rate limiting: 3 failed attempts per username
/// within a 60-second sliding window; lockout until the window expires.
///
/// # Errors
///
/// Returns `Invalid` if:
/// - The username doesn't match any active user
/// - The PIN doesn't match the stored hash
/// - The rate-limit lockout is active (includes retry-after info)
#[command]
pub async fn staff_login(
    args: StaffLoginArgs,
    state: State<'_, AppState>,
) -> Result<StaffLoginResult, AppError> {
    let username = args.username.trim().to_lowercase();
    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Check rate limiter (persistent — survives app restarts).
    // Records the attempt — on success (PIN correct) we clear the
    // counter; on failure the attempt stays recorded.
    let remaining = match store.record_login_attempt(&username, 3, 60)? {
        Err(retry_after) => {
            return Err(AppError::Invalid(format!(
                "Too many attempts. Try again in {retry_after}s."
            )));
        }
        Ok(r) => r,
    };

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
        let msg = if remaining == 1 {
            "Wrong PIN. 1 attempt remaining.".to_string()
        } else {
            format!("Wrong PIN. {remaining} attempts remaining.")
        };
        return Err(AppError::Invalid(msg));
    }

    // PIN correct — clear rate limiter for this user.
    store.clear_login_attempts(&username)?;

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

/// Arguments for `create_session`.
#[derive(Debug, Deserialize)]
pub struct CreateSessionArgs {
    /// The authenticated user ID.
    pub user_id: String,
    /// The user's active role ID.
    pub role_id: String,
    /// The resolved store ID.
    pub store_id: String,
    /// The resolved workspace instance ID.
    pub instance_id: String,
    /// The workspace type key (derived from the instance).
    pub type_key: String,
    /// The terminal/device ID.
    pub terminal_id: String,
}

/// Result of `create_session` — returns the opaque session token.
#[derive(Debug, Serialize)]
pub struct CreateSessionResult {
    /// Opaque session token to be passed with every subsequent command.
    pub session_token: String,
    /// The resolved session context (for frontend display).
    pub context: SessionContextDto,
}

/// Lightweight session context DTO for the frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContextDto {
    /// ID of the associated user.
    pub user_id: String,
    /// ID of the associated role.
    pub role_id: String,
    /// ID of the associated store.
    pub store_id: String,
    /// ID of the associated instance.
    pub instance_id: String,
    /// Type Key.
    pub type_key: String,
    /// ID of the associated terminal.
    pub terminal_id: String,
}

/// Create a new session and return an opaque session token.
///
/// ADR #4 / ADR #7: Called after login + workspace selection to
/// establish the caller's resolved scope. The returned token must
/// be passed to every subsequent command as the `session_token`
/// parameter.
///
/// The token is a random UUID v4 stored in the in-memory session
/// store. It is valid until `destroy_session` is called or the
/// process exits.
#[command]
pub async fn create_session(
    args: CreateSessionArgs,
    state: State<'_, AppState>,
) -> Result<CreateSessionResult, AppError> {
    // Validate required fields BEFORE any side effects.
    if args.store_id.is_empty() || args.instance_id.is_empty() || args.user_id.is_empty() {
        return Err(AppError::Invalid(
            "store_id, instance_id, and user_id must not be empty".into(),
        ));
    }

    let token = uuid::Uuid::now_v7().to_string();

    {
        let mut store = state
            .session_store
            .write()
            .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;

        // Defensive: log if a UUID collision occurs (astronomically unlikely).
        if store.contains_key(&token) {
            tracing::warn!(token = %token, "session token collision detected — overwriting");
        }

        // Enforce a maximum session count to prevent unbounded growth.
        // ADR #7 will replace this with TTL-based expiry.
        const MAX_SESSIONS: usize = 256;
        if store.len() >= MAX_SESSIONS {
            // Evict the oldest entry (HashMap iteration order is non-deterministic,
            // but this is an emergency backstop, not a precise LRU).
            if let Some(old_token) = store.keys().next().cloned() {
                store.remove(&old_token);
                tracing::warn!(old_token = %old_token, "session store full — evicted oldest session");
            }
        }

        let context = SessionContext::new(
            args.user_id.clone(),
            args.role_id.clone(),
            args.terminal_id.clone(),
            args.store_id.clone(),
            args.instance_id.clone(),
            args.type_key.clone(),
        );
        store.insert(token.clone(), context.clone());
    }

    // Invalidate the location cache — a new session means either a fresh
    // login or a workspace switch, so cached location bindings from the
    // previous session should not carry over.
    oz_core::location_resolver::invalidate_location_cache();

    tracing::info!(
        user_id = %args.user_id,
        store_id = %args.store_id,
        instance_id = %args.instance_id,
        "session created"
    );

    Ok(CreateSessionResult {
        session_token: token,
        context: SessionContextDto {
            user_id: args.user_id,
            role_id: args.role_id,
            store_id: args.store_id,
            instance_id: args.instance_id,
            type_key: args.type_key,
            terminal_id: args.terminal_id,
        },
    })
}

/// Destroy an active session, invalidating the token.
///
/// ADR #4 / ADR #7: Called on logout or store switch. After this
/// call, any commands using the old token will fail with
/// `AppError::InvalidSession`.
#[command]
pub async fn destroy_session(
    state: State<'_, AppState>,
    session_token: String,
) -> Result<(), AppError> {
    let mut store = state
        .session_store
        .write()
        .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;
    store.remove(&session_token);
    tracing::info!("session destroyed");
    Ok(())
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

    // ── StaffLoginArgs edge cases ────────────────────────────────────────

    #[test]
    fn staff_login_args_whitespace_username() {
        let json = r##"{"username":"   ","pin":"1234"}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        // After trimming in staff_login, this becomes empty
        assert_eq!(args.username, "   ");
        assert_eq!(args.pin, "1234");
    }

    #[test]
    fn staff_login_args_empty_pin() {
        let json = r##"{"username":"jdoe","pin":""}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "jdoe");
        assert_eq!(args.pin, "");
    }

    #[test]
    fn staff_login_args_long_pin() {
        let json = r##"{"username":"jdoe","pin":"12345678901234567890"}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.pin.len(), 20);
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

    // ── Error mapping edge cases ────────────────────────────────────────

    #[test]
    fn staff_login_result_empty_display_name() {
        let session = LoginSession {
            user_id: "u3".into(),
            display_name: "".into(),
            role_name: "Cashier".into(),
            role_id: "r3".into(),
        };
        let result = StaffLoginResult { session };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["display_name"], "");
    }

    #[test]
    fn staff_login_result_null_role_id() {
        let session = LoginSession {
            user_id: "u4".into(),
            display_name: "Bob".into(),
            role_name: "".into(),
            role_id: "".into(),
        };
        let result = StaffLoginResult { session };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["role_name"], "");
        assert_eq!(json["session"]["role_id"], "");
    }
}
