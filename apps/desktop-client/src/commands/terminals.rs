//! Terminal management Tauri commands.
//!
//! CRUD operations for registered POS terminals. Each POS device
//! registers itself with a unique name and device identifier.
//!
//! All commands have scoped variants (ADR #7) that use the session token
//! pattern. Old commands are preserved with deprecation notices.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use oz_core::{Store, Terminal, TerminalFeatureOverride, TerminalProfile};

use foundation::validate_not_empty;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Keyring name for the device binding HMAC secret.
pub const DEVICE_BINDING_KEYRING_NAME: &str = "oz-pos/device-binding-hmac-key";

/// Compute an HMAC-SHA256 signature for a device binding.
///
/// The signature covers `{terminal_id}:{bound_store_id}:{bound_instance_id}`
/// using a secret stored in the OS keyring. If no secret exists yet, one is
/// generated and stored.
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

/// Verify a device binding HMAC signature.
fn verify_binding(
    keyring: &dyn oz_security::Keyring,
    terminal_id: &str,
    store_id: &str,
    instance_id: &str,
    signature: &str,
) -> Result<bool, AppError> {
    let expected = sign_binding(keyring, terminal_id, store_id, instance_id)?;
    Ok(expected == signature)
}

// ── DTOs ──────────────────────────────────────────────────────────────

/// Terminal DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDto {
    pub id: String,
    pub name: String,
    pub device_id: String,
    pub is_active: bool,
    pub last_seen_at: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
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
    pub name: String,
    pub device_id: String,
    pub terminal_secret: Option<String>,
    pub metadata: Option<String>,
}

/// Result of registering a new terminal.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTerminalResult {
    pub id: String,
}

/// Arguments for updating a terminal.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminalArgs {
    pub id: String,
    pub name: Option<String>,
    pub device_id: Option<String>,
    pub terminal_secret: Option<String>,
    pub is_active: Option<bool>,
    pub metadata: Option<String>,
}

/// Result of updating a terminal.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminalResult {
    pub id: String,
}

// ── Read Commands ────────────────────────────────────────────────────

/// List all registered terminals.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_terminals_scoped`.
#[command]
pub async fn list_terminals(state: State<'_, AppState>) -> Result<Vec<TerminalDto>, AppError> {
    let db = state.db.lock().await;
    run_list_terminals(&db)
}

/// List terminals from the store resolved from a session token. ADR #7.
#[command]
pub async fn list_terminals_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<TerminalDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let result = run_list_terminals(&db);
    drop(db);
    result
}

fn run_list_terminals(conn: &rusqlite::Connection) -> Result<Vec<TerminalDto>, AppError> {
    let store = Store::new(conn);
    let terminals = store.list_terminals()?;
    let dtos: Vec<TerminalDto> = terminals.into_iter().map(TerminalDto::from).collect();
    Ok(dtos)
}

/// Get a single terminal by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_terminal_scoped`.
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

/// Get a terminal from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_terminal_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TerminalDto>, AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let terminal = store.get_terminal(&id)?;
    drop(db);

    Ok(terminal.map(TerminalDto::from))
}

/// Ping a terminal to update its last_seen_at timestamp.
///
/// **Deprecated for multi-store (ADR #7):** Use `ping_terminal_scoped`.
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

/// Ping a terminal in the store resolved from a session token. ADR #7.
#[command]
pub async fn ping_terminal_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.ping_terminal(&id)?;
    drop(db);

    tracing::debug!(id, "terminal pinged (scoped)");
    Ok(())
}

/// List feature overrides for a terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_terminal_overrides_scoped`.
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

/// List terminal overrides from the store resolved from a session token. ADR #7.
#[command]
pub async fn list_terminal_overrides_scoped(
    session_token: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TerminalFeatureOverride>, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let overrides = store.list_terminal_overrides(&terminal_id)?;
    drop(db);

    Ok(overrides)
}

/// List all terminal profiles.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_terminal_profiles_scoped`.
#[command]
pub async fn list_terminal_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<TerminalProfileDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let profiles = store.list_terminal_profiles()?;
    drop(db);
    Ok(profiles.into_iter().map(TerminalProfileDto::from).collect())
}

/// List terminal profiles from the store resolved from a session token. ADR #7.
#[command]
pub async fn list_terminal_profiles_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<TerminalProfileDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let profiles = store.list_terminal_profiles()?;
    drop(db);
    Ok(profiles.into_iter().map(TerminalProfileDto::from).collect())
}

/// Get the profile for a terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_terminal_profile_scoped`.
#[command]
pub async fn get_terminal_profile(
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TerminalProfileDto>, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let profile = store.get_terminal_profile(&terminal_id)?;
    drop(db);

    Ok(profile.map(TerminalProfileDto::from))
}

/// Get a terminal profile from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_terminal_profile_scoped(
    session_token: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TerminalProfileDto>, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let profile = store.get_terminal_profile(&terminal_id)?;
    drop(db);

    Ok(profile.map(TerminalProfileDto::from))
}

/// Get a terminal's device binding and validate its HMAC signature.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_device_binding_scoped`.
#[command]
pub async fn get_device_binding(
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<DeviceBindingDto, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let binding = store.get_terminal_binding(&terminal_id)?;
    drop(db);

    build_device_binding_dto(&terminal_id, binding)
}

/// Get device binding from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_device_binding_scoped(
    session_token: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<DeviceBindingDto, AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let binding = store.get_terminal_binding(&terminal_id)?;
    drop(db);

    build_device_binding_dto(&terminal_id, binding)
}

fn build_device_binding_dto(
    terminal_id: &str,
    binding: Option<(String, String, String)>,
) -> Result<DeviceBindingDto, AppError> {
    match binding {
        None => Ok(DeviceBindingDto {
            bounded: false,
            bound_store_id: None,
            bound_instance_id: None,
            signature_valid: false,
        }),
        Some((store_id, instance_id, signature)) => {
            let keyring = oz_security::default_keyring()
                .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
            let valid = verify_binding(
                keyring.as_ref(),
                terminal_id,
                &store_id,
                &instance_id,
                &signature,
            )
            .unwrap_or(false);

            Ok(DeviceBindingDto {
                bounded: true,
                bound_store_id: Some(store_id),
                bound_instance_id: Some(instance_id),
                signature_valid: valid,
            })
        }
    }
}

// ── Write Commands ───────────────────────────────────────────────────

/// Register a new terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `register_terminal_scoped`.
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

/// Register a terminal in the store resolved from a session token. ADR #7.
#[command]
pub async fn register_terminal_scoped(
    session_token: String,
    args: RegisterTerminalArgs,
    state: State<'_, AppState>,
) -> Result<RegisterTerminalResult, AppError> {
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("device_id", &args.device_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let mut terminal = Terminal::new(args.name, args.device_id);
    if let Some(secret) = args.terminal_secret {
        terminal = terminal.with_secret(secret);
    }
    if let Some(meta) = args.metadata {
        terminal = terminal.with_metadata(meta);
    }

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_REGISTER,
    )?;
    store.create_terminal(&terminal)?;
    drop(db);

    tracing::info!(id = %terminal.id, name = %terminal.name, "terminal registered (scoped)");
    Ok(RegisterTerminalResult { id: terminal.id })
}

/// Update an existing terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_terminal_scoped`.
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

/// Update a terminal in the store resolved from a session token. ADR #7.
#[command]
pub async fn update_terminal_scoped(
    session_token: String,
    args: UpdateTerminalArgs,
    state: State<'_, AppState>,
) -> Result<UpdateTerminalResult, AppError> {
    validate_not_empty("id", &args.id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.update_terminal(&terminal)?;
    drop(db);

    tracing::info!(id = %terminal.id, "terminal updated (scoped)");
    Ok(UpdateTerminalResult { id: terminal.id })
}

/// Delete a terminal by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_terminal_scoped`.
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

/// Delete a terminal in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_terminal_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_DELETE,
    )?;
    store.delete_terminal(&id)?;
    drop(db);

    tracing::info!(id, "terminal deleted (scoped)");
    Ok(())
}

/// Set (upsert) a feature override for a terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `set_terminal_override_scoped`.
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

/// Set a terminal override in the store resolved from a session token. ADR #7.
#[command]
pub async fn set_terminal_override_scoped(
    session_token: String,
    terminal_id: String,
    feature: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("feature", &feature).map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.set_terminal_override(&terminal_id, &feature, enabled)?;
    drop(db);

    tracing::info!(
        terminal_id,
        feature,
        enabled,
        "terminal feature override set (scoped)"
    );
    Ok(())
}

/// Delete a feature override for a terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_terminal_override_scoped`.
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

/// Delete a terminal override in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_terminal_override_scoped(
    session_token: String,
    terminal_id: String,
    feature: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("feature", &feature).map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.delete_terminal_override(&terminal_id, &feature)?;
    drop(db);

    tracing::info!(
        terminal_id,
        feature,
        "terminal feature override deleted (scoped)"
    );
    Ok(())
}

// ── Terminal Profile Commands ──────────────────────────────────────

/// Terminal profile DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalProfileDto {
    pub terminal_id: String,
    pub profile_type: String,
    pub locked_screen: Option<String>,
    pub updated_at: String,
}

impl From<TerminalProfile> for TerminalProfileDto {
    fn from(p: TerminalProfile) -> Self {
        Self {
            terminal_id: p.terminal_id,
            profile_type: p.profile_type,
            locked_screen: p.locked_screen,
            updated_at: p.updated_at,
        }
    }
}

/// Arguments for `set_terminal_profile`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTerminalProfileArgs {
    pub terminal_id: String,
    pub profile_type: String,
    pub locked_screen: Option<String>,
}

/// Set (upsert) the profile for a terminal.
///
/// **Deprecated for multi-store (ADR #7):** Use `set_terminal_profile_scoped`.
#[command]
pub async fn set_terminal_profile(
    user_id: String,
    args: SetTerminalProfileArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &args.terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("profile_type", &args.profile_type)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.set_terminal_profile(
        &args.terminal_id,
        &args.profile_type,
        args.locked_screen.as_deref(),
    )?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        profile_type = %args.profile_type,
        "terminal profile set"
    );
    Ok(())
}

/// Set a terminal profile in the store resolved from a session token. ADR #7.
#[command]
pub async fn set_terminal_profile_scoped(
    session_token: String,
    args: SetTerminalProfileArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &args.terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("profile_type", &args.profile_type)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.set_terminal_profile(
        &args.terminal_id,
        &args.profile_type,
        args.locked_screen.as_deref(),
    )?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        profile_type = %args.profile_type,
        "terminal profile set (scoped)"
    );
    Ok(())
}

/// Delete a terminal's profile.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_terminal_profile_scoped`.
#[command]
pub async fn delete_terminal_profile(
    user_id: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.delete_terminal_profile(&terminal_id)?;
    drop(db);

    tracing::info!(terminal_id, "terminal profile deleted");
    Ok(())
}

/// Delete a terminal profile in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_terminal_profile_scoped(
    session_token: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.delete_terminal_profile(&terminal_id)?;
    drop(db);

    tracing::info!(terminal_id, "terminal profile deleted (scoped)");
    Ok(())
}

// ── Device Binding Commands ────────────────────────────────────────

/// Arguments for setting a device binding.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceBindingArgs {
    pub terminal_id: String,
    pub bound_store_id: String,
    pub bound_instance_id: String,
}

/// Set (or update) a terminal's device binding with HMAC signature.
///
/// **Deprecated for multi-store (ADR #7):** Use `set_device_binding_scoped`.
#[command]
pub async fn set_device_binding(
    user_id: String,
    args: SetDeviceBindingArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &args.terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_store_id", &args.bound_store_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_instance_id", &args.bound_instance_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let signature = {
        let keyring = oz_security::default_keyring()
            .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
        sign_binding(
            keyring.as_ref(),
            &args.terminal_id,
            &args.bound_store_id,
            &args.bound_instance_id,
        )?
    };

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.update_terminal_binding(
        &args.terminal_id,
        &args.bound_store_id,
        &args.bound_instance_id,
        &signature,
    )?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        store_id = %args.bound_store_id,
        instance_id = %args.bound_instance_id,
        "device binding set"
    );
    Ok(())
}

/// Set a device binding in the store resolved from a session token. ADR #7.
#[command]
pub async fn set_device_binding_scoped(
    session_token: String,
    args: SetDeviceBindingArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &args.terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_store_id", &args.bound_store_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("bound_instance_id", &args.bound_instance_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;

    let signature = {
        let keyring = oz_security::default_keyring()
            .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
        sign_binding(
            keyring.as_ref(),
            &args.terminal_id,
            &args.bound_store_id,
            &args.bound_instance_id,
        )?
    };

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.update_terminal_binding(
        &args.terminal_id,
        &args.bound_store_id,
        &args.bound_instance_id,
        &signature,
    )?;
    drop(db);

    tracing::info!(
        terminal_id = %args.terminal_id,
        store_id = %args.bound_store_id,
        instance_id = %args.bound_instance_id,
        "device binding set (scoped)"
    );
    Ok(())
}

/// DTO for device binding info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBindingDto {
    pub bounded: bool,
    pub bound_store_id: Option<String>,
    pub bound_instance_id: Option<String>,
    pub signature_valid: bool,
}

/// Clear a terminal's device binding.
///
/// **Deprecated for multi-store (ADR #7):** Use `clear_device_binding_scoped`.
#[command]
pub async fn clear_device_binding(
    user_id: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TERMINALS_EDIT)?;
    store.clear_terminal_binding(&terminal_id)?;
    drop(db);

    tracing::info!(terminal_id, "device binding cleared");
    Ok(())
}

/// Clear a device binding in the store resolved from a session token. ADR #7.
#[command]
pub async fn clear_device_binding_scoped(
    session_token: String,
    terminal_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_not_empty("terminal_id", &terminal_id)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::TERMINALS_EDIT,
    )?;
    store.clear_terminal_binding(&terminal_id)?;
    drop(db);

    tracing::info!(terminal_id, "device binding cleared (scoped)");
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
    fn terminals_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
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
}
