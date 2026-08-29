use tauri::{State, command};

use oz_core::Table;
use oz_core::db::Store;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

#[command]
/// List tables.
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

#[command]
/// Get table.
pub async fn get_table(id: String, state: State<'_, AppState>) -> Result<Option<Table>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let table = store.get_table(&id)?;
    drop(db);
    Ok(table)
}

#[command]
/// Create table.
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

#[command]
/// Update table.
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

#[command]
/// Delete table.
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

#[command]
/// Update table status.
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

#[command]
/// Assign table order.
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

#[command]
/// Release table.
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

#[command]
/// List sections.
pub async fn list_sections(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sections = store.list_sections()?;
    drop(db);
    Ok(sections)
}

/// Session-scoped variant of `list_tables`.
#[command]
pub async fn list_tables_scoped(
    session_token: String,
    section: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Table>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let tables = store.list_tables(section.as_deref())?;
    drop(db);
    Ok(tables)
}

/// Session-scoped variant of `get_table`.
#[command]
pub async fn get_table_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Table>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let table = store.get_table(&id)?;
    drop(db);
    Ok(table)
}

/// Session-scoped variant of `create_table`.
#[command]
pub async fn create_table_scoped(
    session_token: String,
    user_id: String,
    args: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CREATE)?;
    let table = store.create_table(&args)?;
    drop(db);
    Ok(table)
}

/// Session-scoped variant of `update_table`.
#[command]
pub async fn update_table_scoped(
    session_token: String,
    user_id: String,
    table: Table,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_EDIT)?;
    let result = store.update_table(&table)?;
    drop(db);
    Ok(result)
}

/// Session-scoped variant of `delete_table`.
#[command]
pub async fn delete_table_scoped(
    session_token: String,
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_DELETE)?;
    store.delete_table(&id)?;
    drop(db);
    Ok(())
}

/// Session-scoped variant of `update_table_status`.
#[command]
pub async fn update_table_status_scoped(
    session_token: String,
    user_id: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.update_table_status(&id, &status)?;
    drop(db);
    Ok(table)
}

/// Session-scoped variant of `assign_table_order`.
#[command]
pub async fn assign_table_order_scoped(
    session_token: String,
    user_id: String,
    table_id: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_ASSIGN)?;
    let table = store.assign_table_order(&table_id, &sale_id)?;
    drop(db);
    Ok(table)
}

/// Session-scoped variant of `release_table`.
#[command]
pub async fn release_table_scoped(
    session_token: String,
    user_id: String,
    table_id: String,
    state: State<'_, AppState>,
) -> Result<Table, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::TABLES_CLOSE)?;
    let table = store.release_table(&table_id)?;
    drop(db);
    Ok(table)
}

/// Session-scoped variant of `list_sections`.
#[command]
pub async fn list_sections_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let sections = store.list_sections()?;
    drop(db);
    Ok(sections)
}
