//! Staff authentication commands — login, logout, session verification.
//!
//! These commands are the IPC surface for `ui/src/features/auth/`. PIN
//! hashing and verification is delegated to `oz_core::auth`.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::auth::LoginSession;
use oz_core::db::Store;
use oz_core::session::SessionContext;

use foundation::validate_not_empty;

use crate::commands::picker_ticket;
use crate::error::AppError;
use crate::state::AppState;

/// Arguments for the `staff_login` command.
#[derive(Debug, Deserialize)]
pub struct StaffLoginArgs {
    /// Staff username (case-sensitive).
    pub username: String,
    /// Plain-text PIN entered by the staff member.
    pub pin: String,
    /// Optional device/terminal identifier for per-device abuse controls
    /// (STAFF-07). When absent the backend derives one from the host name
    /// (`COMPUTERNAME`/`HOSTNAME`) so distributed brute-force from a single
    /// terminal is still bounded.
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Result of a successful staff login.
#[derive(Debug, Serialize)]
pub struct StaffLoginResult {
    /// Session info including user id, display name, and role.
    pub session: LoginSession,
    /// Short-lived picker ticket (audit/06 residual).
    ///
    /// The pre-session `list_workspaces` / `list_workspace_screens`
    /// commands verify this ticket and resolve the caller's REAL role
    /// from the database — caller-supplied `role_id` / `user_id` are
    /// never trusted for the workspace picker.
    pub picker_ticket: String,
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
    /// Always `true`. The pre-check never reveals whether the account
    /// exists or is active (STAFF-06); the real state is written to the
    /// server log only, and the login endpoint reports a uniform failure.
    pub proceed: bool,
}

/// Check a username before the PIN step (STAFF-06).
///
/// Returns a **uniform** pre-auth response so the command cannot be used as
/// an account-enumeration oracle: it always answers `proceed: true` for any
/// syntactically valid username, whether the account exists, is inactive,
/// or is unknown. The actual found/active state is emitted as a server-side
/// trace only. Failed login attempts are handled by `staff_login`, which
/// reports a single uniform error for every bad-credential case.
#[tauri::command]
pub async fn staff_check_username(
    args: CheckUsernameArgs,
    state: State<'_, AppState>,
) -> Result<CheckUsernameResult, AppError> {
    let username = args.username.trim().to_lowercase();
    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let user = store.get_user_by_username(&username)?;
    match &user {
        Some(u) => tracing::debug!(
            username = %username,
            is_active = u.is_active,
            "staff_check_username: account exists (server-side detail only)"
        ),
        None => tracing::debug!(
            username = %username,
            "staff_check_username: no such account (server-side detail only)"
        ),
    }
    drop(db);

    Ok(CheckUsernameResult { proceed: true })
}

/// Authenticate a staff member by username and PIN.
///
/// Looks up the user by username, verifies the PIN against the stored
/// argon2 hash, and returns a [`LoginSession`] on success.
///
/// Rate limiting (STAFF-07) combines per-account, per-device, and global
/// abuse controls with exponential backoff instead of a fixed short lock:
///   - per account: 3 failed attempts in a 60s window
///   - per device:  10 failed attempts in a 60s window (across usernames)
///   - global:      30 failed attempts in a 60s window
/// Backoff doubles per strike (capped at 1h). All rows persist across
/// app restarts.
///
/// # Errors
///
/// Returns `Invalid` for a **uniform** credential failure — unknown user,
/// deactivated account, and wrong PIN all report the same message so the
/// endpoint cannot be used to enumerate accounts or probe account state
/// (STAFF-06/STAFF-07). The rate-limit lockout reports retry-after info
/// only.
#[tauri::command]
pub async fn staff_login(
    args: StaffLoginArgs,
    state: State<'_, AppState>,
) -> Result<StaffLoginResult, AppError> {
    let username = args.username.trim().to_lowercase();
    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;

    // STAFF-07: resolve the device id — prefer the caller's, else the host.
    let device_id = args
        .device_id
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(str::to_owned)
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .or_else(|| std::env::var("HOSTNAME").ok());

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Check rate limiter (persistent — survives app restarts).
    // Records the attempt — on success (PIN correct) we clear the
    // counter; on failure the attempt stays recorded.
    if let Err(retry_after) = store.record_login_attempt_scoped(
        &username,
        device_id.as_deref(),
        oz_core::db::staff::LoginLimits {
            max_attempts: 3,         // per-account max
            window_secs: 60,         // window secs
            device_max_attempts: 10, // per-device max
            global_max_attempts: 30, // global max
            max_backoff_secs: 3600,  // max backoff secs
        },
    )? {
        tracing::warn!(
            username = %username,
            device_id = device_id.as_deref().unwrap_or("unknown"),
            retry_after,
            "staff login rate limit exceeded"
        );
        return Err(AppError::Invalid(format!(
            "Too many attempts. Try again in {retry_after}s."
        )));
    }

    // Look up user by username.
    let user = store
        .get_user_by_username(&username)?
        .ok_or_else(|| AppError::Invalid("invalid username or PIN".into()))?;

    // Uniform failure — do not reveal that the account is deactivated.
    if !user.is_active {
        tracing::debug!(
            username = %username,
            "staff login: account inactive (uniform error returned)"
        );
        return Err(AppError::Invalid("invalid username or PIN".into()));
    }

    // Verify PIN against stored hash.
    // `verify_pin` fails closed (Ok(false)) on malformed/placeholder hashes;
    // the Err arm is retained for future argon2 library errors.
    let valid = oz_core::auth::verify_pin(&args.pin, &user.pin_hash)
        .map_err(|e| AppError::Internal(format!("PIN verification failed: {e}")))?;

    if !valid {
        tracing::debug!(
            username = %username,
            "staff login: wrong PIN (uniform error returned)"
        );
        return Err(AppError::Invalid("invalid username or PIN".into()));
    }

    // PIN correct — clear rate limiter for this user and device.
    store.clear_login_attempts(&username)?;
    if let Some(dev) = device_id.as_deref().filter(|d| !d.is_empty()) {
        store.clear_login_attempts_by_device(dev)?;
    }

    // Look up role for the session.
    let role = store
        .get_role(&user.role_id)?
        .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;

    drop(db);

    // Mint the short-lived picker ticket bound to this authenticated
    // user. It is only valid for the pre-session workspace picker;
    // `create_session` hands out the opaque session token afterwards.
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let picker_ticket = picker_ticket::sign_picker_ticket(
        &state.picker_ticket_secret,
        &user.id,
        now_ts + picker_ticket::PICKER_TICKET_TTL_SECS,
    );

    Ok(StaffLoginResult {
        session: LoginSession {
            user_id: user.id,
            display_name: user.display_name,
            role_name: role.name,
            role_id: role.id,
        },
        picker_ticket,
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
/// The token is a random UUID v7 stored in the in-memory session
/// store. Session TTL (default 24h) is configurable via the
/// `session.ttl_seconds` setting. A background daemon prunes
/// expired tokens every 5 minutes.
#[tauri::command]
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

    // Server-side authorization: verify the user has a valid role assignment
    // for the requested workspace instance (ADR #4 / ADR #7).
    {
        let db = state.db.lock().await;
        let store = oz_core::db::Store::new(&db);
        if !store.verify_instance_access(
            &args.role_id,
            &args.user_id,
            &args.instance_id,
            &args.store_id,
        )? {
            tracing::warn!(
                user_id = %args.user_id,
                role_id = %args.role_id,
                instance_id = %args.instance_id,
                "authorization denied — user has no access to this instance"
            );
            return Err(AppError::Invalid(
                "User does not have access to this workspace instance".into(),
            ));
        }
    }

    let token = uuid::Uuid::now_v7().to_string();

    // Snapshot the current time once for both expiry and creation timestamp.
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Compute session expiry from the cached TTL setting.
    // 0 or negative means no expiry (development mode).
    let expires_at = if state.session_ttl_seconds > 0 {
        Some(now_ts + state.session_ttl_seconds)
    } else {
        None
    };

    {
        let mut session_store = state
            .session_store
            .write()
            .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;

        // Lazy prune: sweep expired sessions when the store is near capacity.
        if session_store.len() >= 200 {
            let before = session_store.len();
            session_store.retain(|_, ctx| !ctx.is_expired());
            let pruned = before - session_store.len();
            if pruned > 0 {
                tracing::info!("lazy prune removed {pruned} expired session(s)");
            }
        }

        // Defensive: log if a UUID collision occurs (astronomically unlikely).
        if session_store.contains_key(&token) {
            tracing::warn!(token = %token, "session token collision detected — overwriting");
        }

        // Enforce a maximum session count with deterministic LRU eviction.
        // Iterates all entries to find the oldest by created_at timestamp.
        // With max 256 entries, this is negligible overhead and guarantees
        // fair eviction (unlike the previous non-deterministic keys().next()).
        const MAX_SESSIONS: usize = 256;
        if session_store.len() >= MAX_SESSIONS {
            let oldest_entry = session_store
                .iter()
                .min_by_key(|(_, ctx)| ctx.created_at)
                .map(|(token, _)| token.clone());

            if let Some(old_token) = oldest_entry {
                session_store.remove(&old_token);
                tracing::warn!(
                    old_token = %old_token,
                    "session store full — evicted oldest session by created_at"
                );
            }
        }

        let context = SessionContext::new(
            args.user_id.clone(),
            args.role_id.clone(),
            args.terminal_id.clone(),
            args.store_id.clone(),
            args.instance_id.clone(),
            args.type_key.clone(),
            expires_at,
            now_ts,
        );
        session_store.insert(token.clone(), context.clone());
    }

    // Invalidate the location cache — a new session means either a fresh
    // login or a workspace switch, so cached location bindings from the
    // previous session should not carry over.
    oz_core::location_resolver::invalidate_location_cache();

    tracing::info!(
        user_id = %args.user_id,
        store_id = %args.store_id,
        instance_id = %args.instance_id,
        ttl_seconds = %state.session_ttl_seconds,
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
#[tauri::command]
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
            device_id: Some("term-1".into()),
        };
        let d = format!("{args:?}");
        assert!(d.contains("u"));
    }

    #[test]
    fn staff_login_args_device_id_defaults_none() {
        // `device_id` is optional — legacy JSON without it must deserialize.
        let json = r##"{"username":"jdoe","pin":"1234"}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.device_id, None);
    }

    #[test]
    fn staff_login_args_device_id_deserializes() {
        let json = r##"{"username":"jdoe","pin":"1234","device_id":"term-7"}"##;
        let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.device_id.as_deref(), Some("term-7"));
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
        let result = StaffLoginResult {
            session,
            picker_ticket: String::new(),
        };
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
        let result = StaffLoginResult {
            session,
            picker_ticket: String::new(),
        };
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
        let result = StaffLoginResult {
            session,
            picker_ticket: String::new(),
        };
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
        let result = StaffLoginResult {
            session,
            picker_ticket: String::new(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["role_name"], "");
        assert_eq!(json["session"]["role_id"], "");
    }

    // ── Session-mint authorization gate (audit/06 residual) ───────────
    //
    // TDD red: `create_session` must fail closed when the caller claims an
    // identity it has not authenticated — unknown user, or a role_id that
    // does not match the user's actual database role. Previously the gate
    // (oz_core `Store::verify_instance_access`) trusted the claimed role
    // and never resolved the user, so a caller who knew an owner's user id
    // could mint a session as that owner and inherit every permission.

    use oz_core::migrations;
    use tauri::Manager as _;

    /// Seed the built-in roles plus one owner user in the GLOBAL identity DB.
    fn seed_owner(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn staff_login_mints_verifiable_picker_ticket() {
        // audit/06: the picker ticket returned by a successful login must
        // verify against the process secret and bind the authenticated user.
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        let hash = oz_core::auth::hash_pin("1234").unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', ?1, 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [hash],
        )
        .unwrap();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = staff_login(
            StaffLoginArgs {
                username: "owner".into(),
                pin: "1234".into(),
                device_id: None,
            },
            app.state(),
        )
        .await
        .unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let state = app.state::<AppState>();
        assert_eq!(
            picker_ticket::verify_picker_ticket(
                &state.picker_ticket_secret,
                &result.picker_ticket,
                now
            )
            .as_deref(),
            Some("user-owner"),
            "login must mint a ticket bound to the authenticated user"
        );
    }

    #[tokio::test]
    async fn create_session_rejects_forged_role_id() {
        // A staff user whose REAL role is role-staff claims role-owner.
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_session(
            CreateSessionArgs {
                user_id: "user-cashier".into(),
                role_id: "role-owner".into(), // forged
                store_id: "default".into(),
                instance_id: "default-restaurant-pos".into(),
                type_key: "restaurant-pos".into(),
                terminal_id: "terminal-1".into(),
            },
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::Invalid(_))),
            "forged role must not mint a session"
        );
        let state = app.state::<AppState>();
        assert_eq!(
            state.session_store.read().unwrap().len(),
            0,
            "no session token may be created for a forged role"
        );
    }

    #[tokio::test]
    async fn create_session_rejects_unknown_user() {
        let conn = migrations::fresh_db();
        seed_owner(&conn);
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_session(
            CreateSessionArgs {
                user_id: "ghost-user".into(),
                role_id: "role-owner".into(),
                store_id: "default".into(),
                instance_id: "default-restaurant-pos".into(),
                type_key: "restaurant-pos".into(),
                terminal_id: "terminal-1".into(),
            },
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::Invalid(_))),
            "unknown user must not be able to open a session"
        );
        let state = app.state::<AppState>();
        assert_eq!(state.session_store.read().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_session_allows_real_owner() {
        let conn = migrations::fresh_db();
        seed_owner(&conn);
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_session(
            CreateSessionArgs {
                user_id: "user-owner".into(),
                role_id: "role-owner".into(),
                store_id: "default".into(),
                instance_id: "default-restaurant-pos".into(),
                type_key: "restaurant-pos".into(),
                terminal_id: "terminal-1".into(),
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.context.role_id, "role-owner");
        assert_eq!(result.context.user_id, "user-owner");
        let state = app.state::<AppState>();
        assert_eq!(state.session_store.read().unwrap().len(), 1);
    }
}
