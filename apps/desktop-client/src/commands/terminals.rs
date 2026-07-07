//! Terminal management Tauri commands.
//!
//! CRUD operations for registered POS terminals. Each POS device
//! registers itself with a unique name and device identifier.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Store, Terminal, TerminalFeatureOverride};

use foundation::validate_not_empty;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

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
