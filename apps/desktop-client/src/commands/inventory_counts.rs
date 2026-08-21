//! Tauri commands for physical inventory / stock counting.
//!
//! Stock-count commands resolve the store and actor from the opaque session
//! token. The legacy unscoped handlers remain available for compatibility but
//! are not registered by the application; all front-end calls use the scoped
//! handlers below.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::{CountType, StockAdjustment, StockCount, StockCountLine, StockCountStatus, Store};

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
/// A stock-count record returned to the front end.
pub struct StockCountDto {
    /// Unique identifier.
    pub id: String,
    /// Human-readable count number.
    pub count_number: String,
    /// Current lifecycle status.
    pub status: String,
    /// Count type.
    pub count_type: String,
    /// Operator notes.
    pub notes: String,
    /// Authenticated actor who started the count.
    pub counted_by: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 completion timestamp.
    pub completed_at: Option<String>,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl From<StockCount> for StockCountDto {
    fn from(count: StockCount) -> Self {
        Self {
            id: count.id,
            count_number: count.count_number,
            status: count.status.as_str().to_string(),
            count_type: count.count_type.as_str().to_string(),
            notes: count.notes,
            counted_by: count.counted_by,
            created_at: count.created_at,
            completed_at: count.completed_at,
            updated_at: count.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
/// A stock-count line returned to the front end.
pub struct StockCountLineDto {
    /// Unique identifier.
    pub id: String,
    /// Parent stock-count identifier.
    pub count_id: String,
    /// Stock-keeping unit.
    pub sku: String,
    /// Product display name.
    pub product_name: String,
    /// Expected quantity.
    pub expected_qty: i64,
    /// Physical quantity observed so far.
    pub counted_qty: Option<i64>,
    /// Counted minus expected quantity.
    pub difference: i64,
    /// Operator notes.
    pub notes: String,
}

impl From<StockCountLine> for StockCountLineDto {
    fn from(line: StockCountLine) -> Self {
        Self {
            id: line.id,
            count_id: line.count_id,
            sku: line.sku,
            product_name: line.product_name,
            expected_qty: line.expected_qty,
            counted_qty: line.counted_qty,
            difference: line.difference,
            notes: line.notes,
        }
    }
}

#[derive(Debug, Serialize)]
/// A stock adjustment returned to the front end.
pub struct StockAdjustmentDto {
    /// Unique identifier.
    pub id: String,
    /// Parent stock-count identifier, if applicable.
    pub count_id: Option<String>,
    /// Stock-keeping unit.
    pub sku: String,
    /// Product display name.
    pub product_name: String,
    /// Quantity before the adjustment.
    pub previous_qty: i64,
    /// Quantity after the adjustment.
    pub adjusted_qty: i64,
    /// Audit reason.
    pub reason: String,
    /// Authenticated actor who applied the adjustment.
    pub created_by: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl From<StockAdjustment> for StockAdjustmentDto {
    fn from(adjustment: StockAdjustment) -> Self {
        Self {
            id: adjustment.id,
            count_id: adjustment.count_id,
            sku: adjustment.sku,
            product_name: adjustment.product_name,
            previous_qty: adjustment.previous_qty,
            adjusted_qty: adjustment.adjusted_qty,
            reason: adjustment.reason,
            created_by: adjustment.created_by,
            created_at: adjustment.created_at,
        }
    }
}

// ── Command args ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for creating a stock count. The actor is session-derived.
pub struct CreateStockCountArgs {
    /// Count type (`full`, `cyclic`, or `spot`).
    pub count_type: String,
    /// Operator notes.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for adding a stock-count line.
pub struct AddCountLineArgs {
    /// Parent stock-count identifier.
    pub count_id: String,
    /// Stock-keeping unit.
    pub sku: String,
    /// Product display name.
    pub product_name: String,
    /// Expected quantity.
    pub expected_qty: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for updating a stock-count line.
pub struct UpdateCountLineArgs {
    /// Line identifier.
    pub line_id: String,
    /// Observed quantity, or `null` to clear it.
    pub counted_qty: Option<i64>,
    /// Operator notes.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for removing a stock-count line.
pub struct RemoveCountLineArgs {
    /// Line identifier.
    pub line_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for completing a stock count. The actor is session-derived.
pub struct CompleteStockCountArgs {
    /// Parent stock-count identifier.
    pub count_id: String,
}

// ── Shared helpers ────────────────────────────────────────────────────

/// Verify that the authenticated user may manage physical inventory counts.
async fn require_inventory_count_permission(
    state: &AppState,
    user_id: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    crate::commands::authz::require_permission_for_user(
        &store,
        user_id,
        oz_core::permissions::INVENTORY_COUNT,
    )
}

/// Require that a count exists and is still editable.
fn editable_count(store: &Store<'_>, count_id: &str) -> Result<StockCount, AppError> {
    let count = store
        .get_stock_count(count_id)?
        .ok_or_else(|| AppError::Invalid("stock count not found".into()))?;
    if matches!(
        count.status,
        StockCountStatus::Draft | StockCountStatus::InProgress
    ) {
        Ok(count)
    } else {
        Err(AppError::Invalid(
            "stock count is no longer editable".into(),
        ))
    }
}

/// Validate that a stock-count line references a product in this store.
fn validate_product(store: &Store<'_>, sku: &str) -> Result<(), AppError> {
    if sku.trim().is_empty() {
        return Err(AppError::Invalid("sku must not be empty".into()));
    }
    let exists: bool = store.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM products WHERE sku = ?1)",
        [sku],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(AppError::Invalid(format!(
            "product SKU '{sku}' was not found"
        )))
    }
}

/// Validate a physical quantity and reject negative values at the command boundary.
fn validate_quantity(field: &'static str, quantity: i64) -> Result<(), AppError> {
    if quantity >= 0 {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("{field} must be non-negative")))
    }
}

/// Build and persist a count while the caller holds the store connection lock.
fn create_count_in_store(
    store: &Store<'_>,
    args: CreateStockCountArgs,
    actor_id: Option<&str>,
) -> Result<StockCountDto, AppError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let count_type = CountType::from_db_str(&args.count_type)
        .ok_or_else(|| AppError::Invalid(format!("invalid count type: {}", args.count_type)))?;
    let mut count = StockCount {
        id: uuid::Uuid::now_v7().to_string(),
        count_number: String::new(),
        status: StockCountStatus::Draft,
        count_type,
        notes: args.notes,
        counted_by: actor_id.map(str::to_owned),
        created_at: now.clone(),
        completed_at: None,
        updated_at: now,
    };
    store.create_stock_count_with_next_number(&mut count)?;
    Ok(count.into())
}

// ── Session-scoped commands ───────────────────────────────────────────

/// Create a stock count in the session's store and attribute it to the session user.
#[tauri::command]
pub async fn create_stock_count_scoped(
    session_token: String,
    args: CreateStockCountArgs,
    state: State<'_, AppState>,
) -> Result<StockCountDto, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    create_count_in_store(&Store::new(&db), args, Some(&session.user_id))
}

/// Fetch one stock count from the session's store.
#[tauri::command]
pub async fn get_stock_count_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<StockCountDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).get_stock_count(&id)?.map(Into::into))
}

/// List stock counts from the session's store.
#[tauri::command]
pub async fn list_stock_counts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockCountDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db)
        .list_stock_counts()?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Fetch lines from a count in the session's store.
#[tauri::command]
pub async fn get_count_lines_scoped(
    session_token: String,
    count_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockCountLineDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    editable_or_readable_count(&store, &count_id)?;
    Ok(store
        .get_count_lines(&count_id)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Read-only existence check used by line reads, including completed counts.
fn editable_or_readable_count(store: &Store<'_>, count_id: &str) -> Result<StockCount, AppError> {
    store
        .get_stock_count(count_id)?
        .ok_or_else(|| AppError::Invalid("stock count not found".into()))
}

/// Add a line to an editable count in the session's store.
#[tauri::command]
pub async fn add_count_line_scoped(
    session_token: String,
    args: AddCountLineArgs,
    state: State<'_, AppState>,
) -> Result<StockCountLineDto, AppError> {
    validate_quantity("expected_qty", args.expected_qty)?;
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    editable_count(&store, &args.count_id)?;
    validate_product(&store, &args.sku)?;
    let line = StockCountLine {
        id: uuid::Uuid::now_v7().to_string(),
        count_id: args.count_id,
        sku: args.sku,
        product_name: args.product_name,
        expected_qty: args.expected_qty,
        counted_qty: None,
        difference: 0,
        notes: String::new(),
    };
    store.add_count_line(&line)?;
    Ok(line.into())
}

/// Update a line belonging to an editable count in the session's store.
#[tauri::command]
pub async fn update_count_line_scoped(
    session_token: String,
    args: UpdateCountLineArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if let Some(quantity) = args.counted_qty {
        validate_quantity("counted_qty", quantity)?;
    }
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let existing = store
        .get_count_line_by_id(&args.line_id)?
        .ok_or_else(|| AppError::Invalid("count line not found".into()))?;
    editable_count(&store, &existing.count_id)?;
    let difference = args
        .counted_qty
        .map(|quantity| {
            quantity
                .checked_sub(existing.expected_qty)
                .ok_or_else(|| AppError::Invalid("counted_qty difference overflow".into()))
        })
        .transpose()?
        .unwrap_or(0);
    store.update_count_line(&StockCountLine {
        id: existing.id,
        count_id: existing.count_id,
        sku: existing.sku,
        product_name: existing.product_name,
        expected_qty: existing.expected_qty,
        counted_qty: args.counted_qty,
        difference,
        notes: args.notes,
    })?;
    Ok(())
}

/// Remove a line belonging to an editable count in the session's store.
#[tauri::command]
pub async fn remove_count_line_scoped(
    session_token: String,
    args: RemoveCountLineArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let existing = store
        .get_count_line_by_id(&args.line_id)?
        .ok_or_else(|| AppError::Invalid("count line not found".into()))?;
    editable_count(&store, &existing.count_id)?;
    store.remove_count_line(&args.line_id)?;
    Ok(())
}

/// Complete a count and attribute generated adjustments to the session user.
#[tauri::command]
pub async fn complete_stock_count_scoped(
    session_token: String,
    args: CompleteStockCountArgs,
    state: State<'_, AppState>,
) -> Result<Vec<StockAdjustmentDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    editable_count(&store, &args.count_id)?;
    Ok(store
        .complete_stock_count(&args.count_id, Some(&session.user_id))?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Move an editable count to `in_progress` or `cancelled`.
#[tauri::command]
pub async fn update_stock_count_status_scoped(
    session_token: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let mut count = editable_count(&store, &id)?;
    let new_status = StockCountStatus::from_db_str(&status)
        .ok_or_else(|| AppError::Invalid(format!("invalid status: {status}")))?;
    let allowed = matches!(
        (count.status, new_status),
        (StockCountStatus::Draft, StockCountStatus::Draft)
            | (StockCountStatus::Draft, StockCountStatus::InProgress)
            | (StockCountStatus::Draft, StockCountStatus::Cancelled)
            | (StockCountStatus::InProgress, StockCountStatus::InProgress)
            | (StockCountStatus::InProgress, StockCountStatus::Cancelled)
    );
    if !allowed {
        return Err(AppError::Invalid(
            "invalid stock count status transition".into(),
        ));
    }
    count.status = new_status;
    count.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    store.update_stock_count(&count)?;
    Ok(())
}

/// List adjustments from the session's store.
#[tauri::command]
pub async fn list_stock_adjustments_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockAdjustmentDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_count_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db)
        .list_stock_adjustments()?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[cfg(test)]
#[path = "inventory_counts_tests.rs"]
mod tests;
