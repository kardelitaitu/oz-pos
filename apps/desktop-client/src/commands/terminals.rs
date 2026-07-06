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
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
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
}
