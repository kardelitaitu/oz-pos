//! Table management Tauri commands.
//!
//! CRUD for restaurant floor-plan tables plus section listing.
//!
//! All commands have scoped variants (ADR #7) that use the session token
//! pattern. Old commands are preserved with deprecation notices.

use tauri::{State, command};

use oz_core::Table;
use oz_core::db::Store;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── Read Commands ────────────────────────────────────────────────────

/// List tables, optionally filtered by section.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_tables_scoped`.
#[command]
pub async fn list_tables(
    section: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Table>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let tables = store.list_tables(section.as_deref())?;
    drop(db);
    Ok(tables)
}

/// List tables for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_tables_scoped(
    session_token: String,
    section: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Table>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let tables = store.list_tables(section.as_deref())?;
    drop(db);
    Ok(tables)
}

/// Get a single table by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_table_scoped`.
#[command]
pub async fn get_table(id: String, state: State<'_, AppState>) -> Result<Option<Table>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let table = store.get_table(&id)?;
    drop(db);
    Ok(table)
}

/// Get a table from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_table_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Table>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let table = store.get_table(&id)?;
    drop(db);
    Ok(table)
}

/// List all section names.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_sections_scoped`.
#[command]
pub async fn list_sections(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sections = store.list_sections()?;
    drop(db);
    Ok(sections)
}

/// List sections for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_sections_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let sections = store.list_sections()?;
    drop(db);
    Ok(sections)
}

// ── Write Commands ───────────────────────────────────────────────────

/// Create a new table.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_table_scoped`.
#[command]
pub async fn create_table(
    user_id: String,
    args: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CREATE)?;
    let table = store.create_table(&args)?;
    drop(db);
    Ok(table)
}

/// Create a table in the store resolved from a session token. ADR #7.
#[command]
pub async fn create_table_scoped(
    session_token: String,
    table: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
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
        oz_core::permissions::TABLES_CREATE,
    )?;
    let result = store.create_table(&table)?;
    drop(db);
    Ok(result)
}

/// Update an existing table.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_table_scoped`.
#[command]
pub async fn update_table(
    user_id: String,
    table: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_EDIT)?;
    let result = store.update_table(&table)?;
    drop(db);
    Ok(result)
}

/// Update a table in the store resolved from a session token. ADR #7.
#[command]
pub async fn update_table_scoped(
    session_token: String,
    table: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, oz_core::permissions::TABLES_EDIT)?;
    let result = store.update_table(&table)?;
    drop(db);
    Ok(result)
}

/// Delete a table by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_table_scoped`.
#[command]
pub async fn delete_table(
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_DELETE)?;
    store.delete_table(&id)?;
    drop(db);
    Ok(())
}

/// Delete a table in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_table_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
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
        oz_core::permissions::TABLES_DELETE,
    )?;
    store.delete_table(&id)?;
    drop(db);
    Ok(())
}

/// Update a table's status (e.g. "occupied", "available").
///
/// **Deprecated for multi-store (ADR #7):** Use `update_table_status_scoped`.
#[command]
pub async fn update_table_status(
    user_id: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.update_table_status(&id, &status)?;
    drop(db);
    Ok(table)
}

/// Update a table's status in the store resolved from a session token. ADR #7.
#[command]
pub async fn update_table_status_scoped(
    session_token: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.update_table_status(&id, &status)?;
    drop(db);
    Ok(table)
}

/// Assign an order/sale to a table.
///
/// **Deprecated for multi-store (ADR #7):** Use `assign_table_order_scoped`.
#[command]
pub async fn assign_table_order(
    user_id: String,
    table_id: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_ASSIGN)?;
    let table = store.assign_table_order(&table_id, &sale_id)?;
    drop(db);
    Ok(table)
}

/// Assign an order to a table in the store resolved from a session token. ADR #7.
#[command]
pub async fn assign_table_order_scoped(
    session_token: String,
    table_id: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
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
        oz_core::permissions::TABLES_ASSIGN,
    )?;
    let table = store.assign_table_order(&table_id, &sale_id)?;
    drop(db);
    Ok(table)
}

/// Release a table (clear its order assignment).
///
/// **Deprecated for multi-store (ADR #7):** Use `release_table_scoped`.
#[command]
pub async fn release_table(
    user_id: String,
    table_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.release_table(&table_id)?;
    drop(db);
    Ok(table)
}

/// Release a table in the store resolved from a session token. ADR #7.
#[command]
pub async fn release_table_scoped(
    session_token: String,
    table_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.release_table(&table_id)?;
    drop(db);
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}
