//! Tauri commands for physical inventory / stock counting.
//!
//! Exposes CRUD for stock counts, lines, and the complete workflow
//! that generates adjustments and updates inventory quantities.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{CountType, StockAdjustment, StockCount, StockCountLine, StockCountStatus, Store};

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct StockCountDto {
    pub id: String,
    pub count_number: String,
    pub status: String,
    pub count_type: String,
    pub notes: String,
    pub counted_by: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

impl From<StockCount> for StockCountDto {
    fn from(c: StockCount) -> Self {
        Self {
            id: c.id,
            count_number: c.count_number,
            status: c.status.as_str().to_string(),
            count_type: c.count_type.as_str().to_string(),
            notes: c.notes,
            counted_by: c.counted_by,
            created_at: c.created_at,
            completed_at: c.completed_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StockCountLineDto {
    pub id: String,
    pub count_id: String,
    pub sku: String,
    pub product_name: String,
    pub expected_qty: i64,
    pub counted_qty: Option<i64>,
    pub difference: i64,
    pub notes: String,
}

impl From<StockCountLine> for StockCountLineDto {
    fn from(l: StockCountLine) -> Self {
        Self {
            id: l.id,
            count_id: l.count_id,
            sku: l.sku,
            product_name: l.product_name,
            expected_qty: l.expected_qty,
            counted_qty: l.counted_qty,
            difference: l.difference,
            notes: l.notes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StockAdjustmentDto {
    pub id: String,
    pub count_id: Option<String>,
    pub sku: String,
    pub product_name: String,
    pub previous_qty: i64,
    pub adjusted_qty: i64,
    pub reason: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

impl From<StockAdjustment> for StockAdjustmentDto {
    fn from(a: StockAdjustment) -> Self {
        Self {
            id: a.id,
            count_id: a.count_id,
            sku: a.sku,
            product_name: a.product_name,
            previous_qty: a.previous_qty,
            adjusted_qty: a.adjusted_qty,
            reason: a.reason,
            created_by: a.created_by,
            created_at: a.created_at,
        }
    }
}

// ── Command args ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateStockCountArgs {
    pub count_type: String,
    pub notes: String,
    pub counted_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddCountLineArgs {
    pub count_id: String,
    pub sku: String,
    pub product_name: String,
    pub expected_qty: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCountLineArgs {
    pub line_id: String,
    pub counted_qty: Option<i64>,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveCountLineArgs {
    pub line_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteStockCountArgs {
    pub count_id: String,
    pub completed_by: Option<String>,
}

// ── Commands ───────────────────────────────────────────────────────────

/// Create a new stock count with an auto-generated count number.
#[command]
pub async fn create_stock_count(
    args: CreateStockCountArgs,
    state: State<'_, AppState>,
) -> Result<StockCountDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let count_number = store.next_count_number()?;
    let count_type = CountType::from_db_str(&args.count_type).unwrap_or(CountType::Full);

    let count = StockCount {
        id,
        count_number,
        status: StockCountStatus::Draft,
        count_type,
        notes: args.notes,
        counted_by: args.counted_by,
        created_at: now.clone(),
        completed_at: None,
        updated_at: now,
    };

    store.create_stock_count(&count)?;

    tracing::info!(count_number = %count.count_number, "stock count created");
    Ok(count.into())
}

/// Fetch a single stock count by id.
#[command]
pub async fn get_stock_count(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<StockCountDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_stock_count(&id)?.map(|c| c.into()))
}

/// List all stock counts, newest first.
#[command]
pub async fn list_stock_counts(state: State<'_, AppState>) -> Result<Vec<StockCountDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let counts = store.list_stock_counts()?;
    Ok(counts.into_iter().map(|c| c.into()).collect())
}

/// Get all lines for a stock count.
#[command]
pub async fn get_count_lines(
    count_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockCountLineDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let lines = store.get_count_lines(&count_id)?;
    Ok(lines.into_iter().map(|l| l.into()).collect())
}

/// Add a line to a stock count.
#[command]
pub async fn add_count_line(
    args: AddCountLineArgs,
    state: State<'_, AppState>,
) -> Result<StockCountLineDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let id = uuid::Uuid::new_v4().to_string();
    let line = StockCountLine {
        id,
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

/// Update a count line (record counted quantity).
#[command]
pub async fn update_count_line(
    args: UpdateCountLineArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Fetch the existing line to compute the difference.
    let existing = match store.get_count_line_by_id(&args.line_id)? {
        Some(l) => l,
        None => return Err(AppError::Invalid("count line not found".into())),
    };

    let difference = args.counted_qty.map_or(0, |cq| cq - existing.expected_qty);

    let updated = StockCountLine {
        id: args.line_id,
        count_id: existing.count_id,
        sku: existing.sku,
        product_name: existing.product_name,
        expected_qty: existing.expected_qty,
        counted_qty: args.counted_qty,
        difference,
        notes: args.notes,
    };

    store.update_count_line(&updated)?;
    Ok(())
}

/// Remove a line from a stock count.
#[command]
pub async fn remove_count_line(
    args: RemoveCountLineArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.remove_count_line(&args.line_id)?;
    Ok(())
}

/// Complete a stock count: create adjustments and update inventory.
#[command]
pub async fn complete_stock_count(
    args: CompleteStockCountArgs,
    state: State<'_, AppState>,
) -> Result<Vec<StockAdjustmentDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let adjustments = store.complete_stock_count(&args.count_id, args.completed_by.as_deref())?;

    tracing::info!(
        count_id = %args.count_id,
        adjustments = %adjustments.len(),
        "stock count completed"
    );

    Ok(adjustments.into_iter().map(|a| a.into()).collect())
}

/// Update a stock count's status (e.g. from draft to in_progress).
#[command]
pub async fn update_stock_count_status(
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let mut count = store
        .get_stock_count(&id)?
        .ok_or_else(|| AppError::Invalid("stock count not found".into()))?;

    let new_status = StockCountStatus::from_db_str(&status)
        .ok_or_else(|| AppError::Invalid(format!("invalid status: {status}")))?;

    count.status = new_status;
    count.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    store.update_stock_count(&count)?;
    Ok(())
}

/// List all stock adjustments.
#[command]
pub async fn list_stock_adjustments(
    state: State<'_, AppState>,
) -> Result<Vec<StockAdjustmentDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let adjustments = store.list_stock_adjustments()?;
    Ok(adjustments.into_iter().map(|a| a.into()).collect())
}
