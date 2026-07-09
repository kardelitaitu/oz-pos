use tauri::{State, command};

use oz_core::Table;
use oz_core::db::Store;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

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

#[command]
pub async fn get_table(id: String, state: State<'_, AppState>) -> Result<Option<Table>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let table = store.get_table(&id)?;
    drop(db);
    Ok(table)
}

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

#[command]
pub async fn list_sections(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sections = store.list_sections()?;
    drop(db);
    Ok(sections)
}
