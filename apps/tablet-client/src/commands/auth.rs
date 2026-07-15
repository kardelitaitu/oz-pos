//! Staff authentication commands — login, logout, session verification.
//!
//! These commands are the IPC surface for `ui/src/features/auth/`. PIN
//! hashing and verification is delegated to `oz_core::auth`.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::LoginSession;
use oz_core::db::Store;
use oz_core::session::SessionContext;

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
#[command]
pub async fn staff_check_username(
    args: CheckUsernameArgs,
    state: State<'_, AppState>,
) -> Result<CheckUsernameResult, AppError> {
    let username = args.username.trim().to_lowercase();
    if username.is_empty() {
        return Err(AppError::Invalid("username must not be empty".into()));
    }

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
    if username.is_empty() {
        return Err(AppError::Invalid("username must not be empty".into()));
    }

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

/// Result of `create_session` — returns the opaque session token.
#[derive(Debug, Serialize)]
pub struct CreateSessionResult {
    /// Session Token.
    pub session_token: String,
    /// Context.
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
/// ADR #4 / ADR #7: Called after login + workspace selection.
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

        if store.contains_key(&token) {
            tracing::warn!(token = %token, "session token collision detected — overwriting");
        }

        const MAX_SESSIONS: usize = 256;
        if store.len() >= MAX_SESSIONS
            && let Some(old_token) = store.keys().next().cloned()
        {
            store.remove(&old_token);
            tracing::warn!(old_token = %old_token, "session store full — evicted oldest session");
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

    tracing::info!(
        user_id = %args.user_id,
        store_id = %args.store_id,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staff_login_args_deserialize() {
        let json = r#"{"username":"cashier1","pin":"1234"}"#;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "cashier1");
        assert_eq!(args.pin, "1234");
    }

    #[test]
    fn staff_login_args_debug() {
        let args = StaffLoginArgs {
            username: "admin".into(),
            pin: "9999".into(),
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("admin"));
    }

    #[test]
    fn staff_login_result_serialize() {
        let result = StaffLoginResult {
            session: LoginSession {
                user_id: "u1".into(),
                display_name: "Alice".into(),
                role_name: "Manager".into(),
                role_id: "r1".into(),
            },
        };
        let json = serde_json::to_value(&result).unwrap();
        let session = &json["session"];
        assert_eq!(session["user_id"], "u1");
        assert_eq!(session["display_name"], "Alice");
        assert_eq!(session["role_name"], "Manager");
    }

    #[test]
    fn staff_login_result_debug() {
        let result = StaffLoginResult {
            session: LoginSession {
                user_id: "u1".into(),
                display_name: "Bob".into(),
                role_name: "Cashier".into(),
                role_id: "r2".into(),
            },
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("Bob"));
    }
}
