//! Session-scoped Tauri commands for physical inventory / stock counting.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{CountType, StockAdjustment, StockCount, StockCountLine, StockCountStatus, Store};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
/// A stock-count record returned to the front end.
pub struct StockCountDto {
    /// Unique identifier.
    pub id: String,
    /// Human-readable count number.
    pub count_number: String,
    /// Lifecycle status.
    pub status: String,
    /// Count type.
    pub count_type: String,
    /// Operator notes.
    pub notes: String,
    /// Authenticated actor who started the count.
    pub counted_by: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Completion timestamp.
    pub completed_at: Option<String>,
    /// Last-update timestamp.
    pub updated_at: String,
}

impl From<StockCount> for StockCountDto {
    fn from(count: StockCount) -> Self {
        Self {
            id: count.id,
            count_number: count.count_number,
            status: count.status.as_str().to_owned(),
            count_type: count.count_type.as_str().to_owned(),
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
    /// Parent count identifier.
    pub count_id: String,
    /// Stock-keeping unit.
    pub sku: String,
    /// Product display name.
    pub product_name: String,
    /// Expected quantity.
    pub expected_qty: i64,
    /// Counted quantity.
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
    /// Parent count identifier, if applicable.
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
    /// Creation timestamp.
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for creating a count. The actor is derived from the session.
pub struct CreateStockCountArgs {
    /// Count type (`full`, `cyclic`, or `spot`).
    pub count_type: String,
    /// Operator notes.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for adding a count line.
pub struct AddCountLineArgs {
    /// Parent count identifier.
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
/// Arguments for updating a count line.
pub struct UpdateCountLineArgs {
    /// Line identifier.
    pub line_id: String,
    /// Counted quantity, or null to clear it.
    pub counted_qty: Option<i64>,
    /// Operator notes.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for removing a count line.
pub struct RemoveCountLineArgs {
    /// Line identifier.
    pub line_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Arguments for completing a count. The actor is derived from the session.
pub struct CompleteStockCountArgs {
    /// Parent count identifier.
    pub count_id: String,
}

async fn require_inventory_count_permission(
    state: &AppState,
    user_id: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    require_permission_for_user(
        &Store::new(&db),
        user_id,
        oz_core::permissions::INVENTORY_COUNT,
    )
}

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

fn validate_quantity(field: &'static str, quantity: i64) -> Result<(), AppError> {
    if quantity >= 0 {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("{field} must be non-negative")))
    }
}

fn get_count(store: &Store<'_>, id: &str) -> Result<StockCount, AppError> {
    store
        .get_stock_count(id)?
        .ok_or_else(|| AppError::Invalid("stock count not found".into()))
}

fn editable_count(store: &Store<'_>, id: &str) -> Result<StockCount, AppError> {
    let count = get_count(store, id)?;
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

fn difference(counted_qty: Option<i64>, expected_qty: i64) -> Result<i64, AppError> {
    counted_qty
        .map(|quantity| {
            quantity
                .checked_sub(expected_qty)
                .ok_or_else(|| AppError::Invalid("counted_qty difference overflow".into()))
        })
        .transpose()
        .map(|value| value.unwrap_or(0))
}

fn create_count(
    store: &Store<'_>,
    args: CreateStockCountArgs,
    actor: &str,
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
        counted_by: Some(actor.to_owned()),
        created_at: now.clone(),
        completed_at: None,
        updated_at: now,
    };
    store.create_stock_count_with_next_number(&mut count)?;
    Ok(count.into())
}

#[command]
/// Create a stock count in the session's store.
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
    create_count(&Store::new(&db), args, &session.user_id)
}

#[command]
/// Get a stock count from the session's store.
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

#[command]
/// List stock counts from the session's store.
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

#[command]
/// Get count lines from the session's store.
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
    get_count(&store, &count_id)?;
    Ok(store
        .get_count_lines(&count_id)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[command]
/// Add a line to an editable count in the session's store.
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

#[command]
/// Update a line in an editable count in the session's store.
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
    let updated = StockCountLine {
        counted_qty: args.counted_qty,
        difference: difference(args.counted_qty, existing.expected_qty)?,
        notes: args.notes,
        ..existing
    };
    store.update_count_line(&updated)?;
    Ok(())
}

#[command]
/// Remove a line from an editable count in the session's store.
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

#[command]
/// Complete a count and attribute adjustments to the session user.
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

#[command]
/// Update a stock-count lifecycle status in the session's store.
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
    let next = StockCountStatus::from_db_str(&status)
        .ok_or_else(|| AppError::Invalid(format!("invalid status: {status}")))?;
    let allowed = matches!(
        (count.status, next),
        (StockCountStatus::Draft, StockCountStatus::InProgress)
            | (StockCountStatus::Draft, StockCountStatus::Cancelled)
            | (StockCountStatus::InProgress, StockCountStatus::Cancelled)
            | (StockCountStatus::InProgress, StockCountStatus::InProgress)
    );
    if !allowed {
        return Err(AppError::Invalid(
            "invalid stock count status transition".into(),
        ));
    }
    count.status = next;
    count.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    store.update_stock_count(&count)?;
    Ok(())
}

#[command]
/// List adjustments from the session's store.
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
mod tests {
    use super::*;

    #[test]
    fn quantities_reject_negative_values() {
        assert!(validate_quantity("counted_qty", -1).is_err());
        assert!(validate_quantity("counted_qty", 0).is_ok());
    }

    #[test]
    fn difference_is_checked() {
        assert_eq!(difference(Some(8), 10).unwrap(), -2);
        assert_eq!(difference(None, 10).unwrap(), 0);
        assert!(difference(Some(i64::MAX), i64::MIN).is_err());
    }

    #[test]
    fn create_args_ignore_no_actor_field() {
        let args: CreateStockCountArgs =
            serde_json::from_str(r#"{"countType":"full","notes":"cycle","countedBy":"forged"}"#)
                .unwrap();
        assert_eq!(args.count_type, "full");
        assert_eq!(args.notes, "cycle");
    }
}
