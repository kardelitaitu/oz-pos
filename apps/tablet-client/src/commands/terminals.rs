//! Terminal management Tauri commands.
//!
//! CRUD operations for registered POS terminals. Each POS device
//! registers itself with a unique name and device identifier.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tauri::{State, command};

use oz_core::{Store, Terminal, TerminalFeatureOverride};

use foundation::validate_not_empty;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Keyring name for the device binding HMAC secret (parity with the
/// desktop client's `DEVICE_BINDING_KEYRING_NAME`).
pub const DEVICE_BINDING_KEYRING_NAME: &str = "oz-pos/device-binding-hmac-key";

/// Compute an HMAC-SHA256 signature for a device binding.
///
/// The signature covers `{terminal_id}:{bound_store_id}:{bound_instance_id}`
/// using a secret stored in the OS keyring. If no secret exists yet, one is
/// generated and stored. Parity with the desktop client's `sign_binding`.
fn sign_binding(
    keyring: &dyn oz_security::Keyring,
    terminal_id: &str,
    store_id: &str,
    instance_id: &str,
) -> Result<String, AppError> {
    let secret = keyring
        .get_secret(DEVICE_BINDING_KEYRING_NAME)
        .map_err(|e| AppError::Internal(format!("keyring read failed: {e}")))?;

    let secret = match secret {
        Some(s) => s,
        None => {
            let new_secret = uuid::Uuid::now_v7().to_string();
            keyring
                .set_secret(DEVICE_BINDING_KEYRING_NAME, &new_secret)
                .map_err(|e| AppError::Internal(format!("keyring write failed: {e}")))?;
            new_secret
        }
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
    mac.update(terminal_id.as_bytes());
    mac.update(b":");
    mac.update(store_id.as_bytes());
    mac.update(b":");
    mac.update(instance_id.as_bytes());

    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

// ── DTOs ──────────────────────────────────────────────────────────────

/// Terminal DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDto {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// ID of the associated device.
    pub device_id: String,
    /// Whether this is active.
    pub is_active: bool,
    /// Last Seen At.
    pub last_seen_at: Option<String>,
    /// Metadata.
    pub metadata: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl From<Terminal> for TerminalDto {
    fn from(t: Terminal) -> Self {
        Self {
            id: t.id,
            name: t.name,
            device_id: t.device_id,
            is_active: t.is_active,
            last_seen_at: t.last_seen_at,
            metadata: t.metadata,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

/// Arguments for registering a new terminal.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTerminalArgs {
    /// Display name.
    pub name: String,
    /// ID of the associated device.
    pub device_id: String,
    /// Terminal Secret.
    pub terminal_secret: Option<String>,
    /// Metadata.
    pub metadata: Option<String>,
}

/// Result of registering a new terminal.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTerminalResult {
    /// Unique identifier.
    pub id: String,
}

/// Arguments for updating a terminal.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminalArgs {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// ID of the associated device.
    pub device_id: Option<String>,
    /// Terminal Secret.
    pub terminal_secret: Option<String>,
    /// Whether this is active.
    pub is_active: Option<bool>,
    /// Metadata.
    pub metadata: Option<String>,
}

/// Result of updating a terminal.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminalResult {
    /// Unique identifier.
    pub id: String,
}

// ── Device binding ────────────────────────────────────────────────────
//
// Parity with the desktop client (audit/06 residual): a tablet can be
// bound to a store+instance so `resolve_boot_store` auto-boots into it.
// The binding signature is an HMAC-SHA256 over
// `{terminal_id}:{bound_store_id}:{bound_instance_id}` keyed by a secret
// stored in the OS keyring — a tampered binding (DB row edited without the
// keyring secret) fails verification and falls back to the primary store.

/// Arguments for setting a terminal's device binding.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceBindingArgs {
    /// ID of the terminal to bind.
    pub terminal_id: String,
    /// ID of the associated bound store.
    pub bound_store_id: String,
    /// ID of the associated bound instance.
    pub bound_instance_id: String,
}

/// Set (or update) a terminal's device binding with HMAC signature.
///
/// The caller identity comes from the explicit `user_id` (legacy terminal
/// command convention on this client). The binding row lives in the GLOBAL
/// identity DB — the same place the tablet's terminal CRUD and
/// `resolve_boot_store` read it.
#[command]
pub async fn set_device_binding(
    user_id: String,
    args: SetDeviceBindingArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    // Acquire the (non-Send) keyring only after the lock so no `.await`
    // point holds it — Tauri requires command futures to be Send.
    let keyring = oz_security::default_keyring()
        .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    run_set_device_binding(&db, keyring.as_ref(), &args)?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        store_id = %args.bound_store_id,
        instance_id = %args.bound_instance_id,
        "device binding set (tablet)"
    );
    Ok(())
}

/// Set a device binding with the caller resolved from a session token.
///
/// ADR #7 variant: the session token binds the caller instead of a
/// client-supplied `user_id`. The binding row still lives in the GLOBAL
/// identity DB where the tablet's `resolve_boot_store` reads it.
#[command]
pub async fn set_device_binding_scoped(
    session_token: String,
    args: SetDeviceBindingArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    // Acquire the (non-Send) keyring only after the lock so no `.await`
    // point holds it — Tauri requires command futures to be Send.
    let keyring = oz_security::default_keyring()
        .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    run_set_device_binding(&db, keyring.as_ref(), &args)?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        store_id = %args.bound_store_id,
        instance_id = %args.bound_instance_id,
        "device binding set (tablet, scoped)"
    );
    Ok(())
}

/// Shared binding write for `set_device_binding*` (extracted for testing).
fn run_set_device_binding(
    conn: &rusqlite::Connection,
    keyring: &dyn oz_security::Keyring,
    args: &SetDeviceBindingArgs,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &args.terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_store_id", &args.bound_store_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_instance_id", &args.bound_instance_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let signature = sign_binding(
        keyring,
        &args.terminal_id,
        &args.bound_store_id,
        &args.bound_instance_id,
    )?;

    let store = Store::new(conn);
    store.update_terminal_binding(
        &args.terminal_id,
        &args.bound_store_id,
        &args.bound_instance_id,
        &signature,
    )?;
    Ok(())
}

// ── Commands ──────────────────────────────────────────────────────────

/// List all registered terminals.
#[command]
pub async fn list_terminals(state: State<'_, AppState>) -> Result<Vec<TerminalDto>, AppError> {
    let db = state.db.lock().await;
    run_list_terminals(&db)
}

fn run_list_terminals(conn: &rusqlite::Connection) -> Result<Vec<TerminalDto>, AppError> {
    let store = Store::new(conn);
    let terminals = store.list_terminals()?;
    let dtos: Vec<TerminalDto> = terminals.into_iter().map(TerminalDto::from).collect();
    Ok(dtos)
}

/// Get a single terminal by id.
#[command]
pub async fn get_terminal(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TerminalDto>, AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let terminal = store.get_terminal(&id)?;
    drop(db);

    Ok(terminal.map(TerminalDto::from))
}

/// Register a new terminal.
#[command]
pub async fn register_terminal(
    user_id: String,
    args: RegisterTerminalArgs,
    state: State<'_, AppState>,
) -> Result<RegisterTerminalResult, AppError> {
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("device_id", &args.device_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let mut terminal = Terminal::new(args.name, args.device_id);
    if let Some(secret) = args.terminal_secret {
        terminal = terminal.with_secret(secret);
    }
    if let Some(meta) = args.metadata {
        terminal = terminal.with_metadata(meta);
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_REGISTER)?;
    store.create_terminal(&terminal)?;
    drop(db);

    tracing::info!(id = %terminal.id, name = %terminal.name, "terminal registered");
    Ok(RegisterTerminalResult { id: terminal.id })
}

/// Update an existing terminal.
#[command]
pub async fn update_terminal(
    user_id: String,
    args: UpdateTerminalArgs,
    state: State<'_, AppState>,
) -> Result<UpdateTerminalResult, AppError> {
    validate_not_empty("id", &args.id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    let mut terminal = store
        .get_terminal(&args.id)?
        .ok_or_else(|| AppError::Invalid(format!("terminal '{}' not found", args.id)))?;

    if let Some(name) = args.name {
        validate_not_empty("name", &name).map_err(|e| AppError::Invalid(e.to_string()))?;
        terminal.name = name;
    }
    if let Some(device_id) = args.device_id {
        validate_not_empty("device_id", &device_id)
            .map_err(|e| AppError::Invalid(e.to_string()))?;
        terminal.device_id = device_id;
    }
    if let Some(secret) = args.terminal_secret {
        terminal.terminal_secret = Some(secret);
    }
    if let Some(active) = args.is_active {
        terminal.is_active = active;
    }
    if let Some(meta) = args.metadata {
        terminal.metadata = Some(meta);
    }

    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.update_terminal(&terminal)?;
    drop(db);

    tracing::info!(id = %terminal.id, "terminal updated");
    Ok(UpdateTerminalResult { id: terminal.id })
}

/// Update a terminal's last_seen_at timestamp (heartbeat).
#[command]
pub async fn ping_terminal(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.ping_terminal(&id)?;
    drop(db);

    tracing::debug!(id, "terminal pinged");
    Ok(())
}

/// Delete a terminal by id.
#[command]
pub async fn delete_terminal(
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_DELETE)?;
    store.delete_terminal(&id)?;
    drop(db);

    tracing::info!(id, "terminal deleted");
    Ok(())
}

/// List all feature overrides for a terminal.
#[command]
pub async fn list_terminal_overrides(
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TerminalFeatureOverride>, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let overrides = store.list_terminal_overrides(&terminal_id)?;
    drop(db);

    Ok(overrides)
}

/// Set (upsert) a feature override for a terminal.
#[command]
pub async fn set_terminal_override(
    user_id: String,
    terminal_id: String,
    feature: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("feature", &feature).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.set_terminal_override(&terminal_id, &feature, enabled)?;
    drop(db);

    tracing::info!(
        terminal_id,
        feature,
        enabled,
        "terminal feature override set"
    );
    Ok(())
}

/// Delete a single feature override for a terminal.
#[command]
pub async fn delete_terminal_override(
    user_id: String,
    terminal_id: String,
    feature: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("feature", &feature).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.delete_terminal_override(&terminal_id, &feature)?;
    drop(db);

    tracing::info!(terminal_id, feature, "terminal feature override deleted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    #[test]
    fn list_terminals_empty_db() {
        let conn = fresh_conn();
        let terminals = run_list_terminals(&conn).unwrap();
        assert!(terminals.is_empty());
    }

    #[test]
    fn list_terminals_with_seeded_data() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t1 = Terminal::new("Front Counter", "host-01");
        store.create_terminal(&t1).unwrap();
        let t2 = Terminal::new("Drive-Thru", "host-02");
        store.create_terminal(&t2).unwrap();

        let terminals = run_list_terminals(&conn).unwrap();
        assert_eq!(terminals.len(), 2);
        // Ordered by name: Drive-Thru, Front Counter
        assert_eq!(terminals[0].name, "Drive-Thru");
        assert_eq!(terminals[1].name, "Front Counter");
    }

    #[test]
    fn register_and_get_terminal() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Back Office", "host-03")
            .with_secret("s3cr3t")
            .with_metadata(r#"{"os":"windows"}"#);
        store.create_terminal(&t).unwrap();

        let loaded = store.get_terminal(&t.id).unwrap().unwrap();
        assert_eq!(loaded.name, "Back Office");
        assert_eq!(loaded.device_id, "host-03");
        assert_eq!(loaded.terminal_secret, Some("s3cr3t".into()));
        assert!(loaded.is_active);
    }

    #[test]
    fn get_terminal_by_device_id() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Counter", "host-04");
        store.create_terminal(&t).unwrap();

        let loaded = store.get_terminal_by_device_id("host-04").unwrap().unwrap();
        assert_eq!(loaded.id, t.id);
        assert_eq!(loaded.name, "Counter");
    }

    #[test]
    fn get_terminal_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = store.get_terminal("nonexistent").unwrap();
        assert!(t.is_none());
    }

    #[test]
    fn update_terminal_fields() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Old Name", "host-05");
        store.create_terminal(&t).unwrap();

        let mut updated = t.clone();
        updated.name = "New Name".into();
        store.update_terminal(&updated).unwrap();

        let loaded = store.get_terminal(&t.id).unwrap().unwrap();
        assert_eq!(loaded.name, "New Name");
    }

    #[test]
    fn update_terminal_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Ghost", "ghost");
        let err = store.update_terminal(&t).unwrap_err();
        assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
    }

    #[test]
    fn ping_terminal_updates_timestamp() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Counter", "host-06");
        store.create_terminal(&t).unwrap();

        // Initially last_seen_at is None.
        assert!(
            store
                .get_terminal(&t.id)
                .unwrap()
                .unwrap()
                .last_seen_at
                .is_none()
        );

        store.ping_terminal(&t.id).unwrap();
        let loaded = store.get_terminal(&t.id).unwrap().unwrap();
        assert!(
            loaded.last_seen_at.is_some(),
            "ping should set last_seen_at"
        );
    }

    #[test]
    fn ping_terminal_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let err = store.ping_terminal("nope").unwrap_err();
        assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
    }

    // ── Terminal Feature Override tests ────────────────────────────

    #[test]
    fn list_terminal_overrides_empty() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        let overrides = store.list_terminal_overrides(&t.id).unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn list_terminal_overrides_with_data() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        store
            .set_terminal_override(&t.id, "card-payment", false)
            .unwrap();
        store
            .set_terminal_override(&t.id, "receipt-printing", true)
            .unwrap();

        let overrides = store.list_terminal_overrides(&t.id).unwrap();
        assert_eq!(overrides.len(), 2);
        // Ordered by feature ASC.
        assert_eq!(overrides[0].feature, "card-payment");
        assert!(!overrides[0].enabled);
        assert_eq!(overrides[1].feature, "receipt-printing");
        assert!(overrides[1].enabled);
        assert_eq!(overrides[0].terminal_id, t.id);
        assert_eq!(overrides[1].terminal_id, t.id);
    }

    #[test]
    fn list_terminal_overrides_scoped_by_terminal() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t1 = Terminal::new("Term-1", "host-1");
        let t2 = Terminal::new("Term-2", "host-2");
        store.create_terminal(&t1).unwrap();
        store.create_terminal(&t2).unwrap();

        store
            .set_terminal_override(&t1.id, "card-payment", true)
            .unwrap();
        store
            .set_terminal_override(&t2.id, "card-payment", false)
            .unwrap();

        let t1_overrides = store.list_terminal_overrides(&t1.id).unwrap();
        assert_eq!(t1_overrides.len(), 1);
        assert!(t1_overrides[0].enabled);

        let t2_overrides = store.list_terminal_overrides(&t2.id).unwrap();
        assert_eq!(t2_overrides.len(), 1);
        assert!(!t2_overrides[0].enabled);
    }

    #[test]
    fn set_terminal_override_insert() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        store
            .set_terminal_override(&t.id, "cash-payment", false)
            .unwrap();

        let o = store
            .get_terminal_override(&t.id, "cash-payment")
            .unwrap()
            .unwrap();
        assert_eq!(o.feature, "cash-payment");
        assert!(!o.enabled);
        assert_eq!(o.terminal_id, t.id);
        assert!(!o.created_at.is_empty());
        assert!(!o.updated_at.is_empty());
    }

    #[test]
    fn set_terminal_override_update_existing() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        store
            .set_terminal_override(&t.id, "card-payment", false)
            .unwrap();
        store
            .set_terminal_override(&t.id, "card-payment", true)
            .unwrap();

        let o = store
            .get_terminal_override(&t.id, "card-payment")
            .unwrap()
            .unwrap();
        assert!(o.enabled);
    }

    #[test]
    fn delete_terminal_override_removes_row() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        store
            .set_terminal_override(&t.id, "card-payment", false)
            .unwrap();
        store
            .delete_terminal_override(&t.id, "card-payment")
            .unwrap();

        let o = store.get_terminal_override(&t.id, "card-payment").unwrap();
        assert!(o.is_none());
    }

    #[test]
    fn set_terminal_override_nonexistent_terminal_fails() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        // No terminal created — FK constraint should reject.
        let err = store
            .set_terminal_override("no-such-terminal", "card-payment", true)
            .unwrap_err();
        assert!(matches!(err, oz_core::CoreError::Db(_)));
    }

    #[test]
    fn delete_terminal_override_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Test", "host-override");
        store.create_terminal(&t).unwrap();

        let err = store
            .delete_terminal_override(&t.id, "nonexistent")
            .unwrap_err();
        assert!(
            matches!(err, oz_core::CoreError::NotFound { entity, .. } if entity == "terminal_feature_override")
        );
    }

    // ── delete_terminal ──────────────────────────────────────────────

    #[test]
    fn delete_terminal_removes_row() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let t = Terminal::new("Temp", "host-07");
        store.create_terminal(&t).unwrap();
        store.delete_terminal(&t.id).unwrap();

        let loaded = store.get_terminal(&t.id).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn delete_terminal_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let err = store.delete_terminal("nope").unwrap_err();
        assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
    }

    // -- DTO struct tests --

    #[test]
    fn terminal_dto_debug() {
        let dto = TerminalDto {
            id: "t1".into(),
            name: "Front Counter".into(),
            device_id: "host-01".into(),
            is_active: true,
            last_seen_at: None,
            metadata: None,
            created_at: "2025-01-01".into(),
            updated_at: "2025-01-01".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Front Counter"));
    }

    #[test]
    fn terminal_dto_serialize() {
        let dto = TerminalDto {
            id: "t2".into(),
            name: "Drive-Thru".into(),
            device_id: "host-02".into(),
            is_active: false,
            last_seen_at: Some("2025-06-01".into()),
            metadata: Some(r#"{"os":"linux"}"#.into()),
            created_at: "2025-01-01".into(),
            updated_at: "2025-01-01".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "Drive-Thru");
        assert_eq!(json["isActive"], false);
    }

    #[test]
    fn register_terminal_args_deserialize() {
        let json = r##"{"name":"POS-1","deviceId":"host-03"}"##;
        let args: RegisterTerminalArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "POS-1");
        assert_eq!(args.terminal_secret, None);
    }

    #[test]
    fn register_terminal_args_debug() {
        let args = RegisterTerminalArgs {
            name: "N".into(),
            device_id: "D".into(),
            terminal_secret: None,
            metadata: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("N"));
    }

    #[test]
    fn register_terminal_result_serialize() {
        let result = RegisterTerminalResult { id: "t99".into() };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["id"], "t99");
    }

    #[test]
    fn register_terminal_result_debug() {
        let result = RegisterTerminalResult { id: "t42".into() };
        let d = format!("{result:?}");
        assert!(d.contains("t42"));
    }

    #[test]
    fn update_terminal_args_deserialize_minimal() {
        let json = r##"{"id":"t1"}"##;
        let args: UpdateTerminalArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "t1");
        assert_eq!(args.name, None);
        assert_eq!(args.is_active, None);
    }

    #[test]
    fn update_terminal_args_debug() {
        let args = UpdateTerminalArgs {
            id: "x".into(),
            name: None,
            device_id: None,
            terminal_secret: None,
            is_active: None,
            metadata: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("x"));
    }

    #[test]
    fn update_terminal_result_serialize() {
        let result = UpdateTerminalResult { id: "t-up".into() };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["id"], "t-up");
    }

    // ── Device binding (parity with desktop client) ─────────────────────

    #[test]
    fn sign_binding_roundtrip_matches() {
        let keyring = oz_security::InMemoryKeyring::new();
        let sig = sign_binding(&keyring, "term-1", "store-a", "ws-a-1").unwrap();
        assert!(!sig.is_empty());
        assert_eq!(
            sign_binding(&keyring, "term-1", "store-a", "ws-a-1").unwrap(),
            sig,
            "signing the same payload with the same keyring must be stable"
        );
    }

    #[test]
    fn sign_binding_different_secret_differs() {
        let signer = oz_security::InMemoryKeyring::new();
        let other = oz_security::InMemoryKeyring::new();
        let sig = sign_binding(&signer, "term-1", "store-a", "ws-a-1").unwrap();
        assert_ne!(
            sign_binding(&other, "term-1", "store-a", "ws-a-1").unwrap(),
            sig,
            "a signature from a different keyring secret must not match"
        );
    }

    #[test]
    fn sign_binding_differs_for_wrong_payload() {
        let keyring = oz_security::InMemoryKeyring::new();
        let sig = sign_binding(&keyring, "term-1", "store-a", "ws-a-1").unwrap();
        assert_ne!(
            sign_binding(&keyring, "term-1", "store-a", "ws-a-2").unwrap(),
            sig,
            "signature for a different instance must not match"
        );
    }

    #[test]
    fn run_set_device_binding_writes_verifiable_binding() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let t = Terminal::new("Counter", "host-bind");
        store.create_terminal(&t).unwrap();

        // `bound_store_id` is FK-enforced against the global `store_profiles`.
        let now = "2026-07-31T00:00:00.000Z";
        conn.execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES ('store-a', 'Store A', '', '', 'USD', 'UTC', 0, ?1, ?1)",
            [now],
        )
        .unwrap();

        let keyring = oz_security::InMemoryKeyring::new();
        run_set_device_binding(
            &conn,
            &keyring,
            &SetDeviceBindingArgs {
                terminal_id: t.id.clone(),
                bound_store_id: "store-a".into(),
                bound_instance_id: "ws-a-1".into(),
            },
        )
        .unwrap();
        let (store_id, instance_id, sig) = store.get_terminal_binding(&t.id).unwrap().unwrap();
        assert_eq!(store_id, "store-a");
        assert_eq!(instance_id, "ws-a-1");
        assert_eq!(
            sign_binding(&keyring, &t.id, &store_id, &instance_id).unwrap(),
            sig,
            "persisted binding must match the same keyring's signature"
        );
    }

    #[test]
    fn set_device_binding_args_deserialize() {
        let json = r##"{"terminalId":"t1","boundStoreId":"store-a","boundInstanceId":"ws-a-1"}"##;
        let args: SetDeviceBindingArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.terminal_id, "t1");
        assert_eq!(args.bound_store_id, "store-a");
        assert_eq!(args.bound_instance_id, "ws-a-1");
    }

    #[test]
    fn set_device_binding_args_debug() {
        let args = SetDeviceBindingArgs {
            terminal_id: "t1".into(),
            bound_store_id: "store-a".into(),
            bound_instance_id: "ws-a-1".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("store-a"));
        assert!(d.contains("ws-a-1"));
    }
}
