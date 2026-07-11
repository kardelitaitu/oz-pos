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
/// Stockcountdto.
pub struct StockCountDto {
    /// Unique identifier.
    pub id: String,
    /// Count Number.
    pub count_number: String,
    /// Current status.
    pub status: String,
    /// Count Type.
    pub count_type: String,
    /// Notes.
    pub notes: String,
    /// Counted By.
    pub counted_by: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Completed At.
    pub completed_at: Option<String>,
    /// ISO-8601 last-update timestamp.
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
/// Stockcountlinedto.
pub struct StockCountLineDto {
    /// Unique identifier.
    pub id: String,
    /// ID of the associated count.
    pub count_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Product Name.
    pub product_name: String,
    /// Expected Qty.
    pub expected_qty: i64,
    /// Counted Qty.
    pub counted_qty: Option<i64>,
    /// Difference.
    pub difference: i64,
    /// Notes.
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
/// Stockadjustmentdto.
pub struct StockAdjustmentDto {
    /// Unique identifier.
    pub id: String,
    /// ID of the associated count.
    pub count_id: Option<String>,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Product Name.
    pub product_name: String,
    /// Previous Qty.
    pub previous_qty: i64,
    /// Adjusted Qty.
    pub adjusted_qty: i64,
    /// Reason.
    pub reason: String,
    /// Created By.
    pub created_by: Option<String>,
    /// ISO-8601 creation timestamp.
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
/// Createstockcountargs.
pub struct CreateStockCountArgs {
    /// Count Type.
    pub count_type: String,
    /// Notes.
    pub notes: String,
    /// Counted By.
    pub counted_by: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Addcountlineargs.
pub struct AddCountLineArgs {
    /// ID of the associated count.
    pub count_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Product Name.
    pub product_name: String,
    /// Expected Qty.
    pub expected_qty: i64,
}

#[derive(Debug, Deserialize)]
/// Updatecountlineargs.
pub struct UpdateCountLineArgs {
    /// ID of the associated line.
    pub line_id: String,
    /// Counted Qty.
    pub counted_qty: Option<i64>,
    /// Notes.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
/// Removecountlineargs.
pub struct RemoveCountLineArgs {
    /// ID of the associated line.
    pub line_id: String,
}

#[derive(Debug, Deserialize)]
/// Completestockcountargs.
pub struct CompleteStockCountArgs {
    /// ID of the associated count.
    pub count_id: String,
    /// Completed By.
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

    let id = uuid::Uuid::now_v7().to_string();
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

    let id = uuid::Uuid::now_v7().to_string();
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── StockCountDto ───────────────────────────────────────────────────

    #[test]
    fn stock_count_dto_debug() {
        let dto = StockCountDto {
            id: "c1".into(),
            count_number: "CNT-001".into(),
            status: "draft".into(),
            count_type: "full".into(),
            notes: "Monthly count".into(),
            counted_by: Some("user1".into()),
            created_at: "2025-01-01T00:00:00.000Z".into(),
            completed_at: None,
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("CNT-001"));
        assert!(d.contains("draft"));
    }

    #[test]
    fn stock_count_dto_serialize() {
        let dto = StockCountDto {
            id: "c2".into(),
            count_number: "CNT-002".into(),
            status: "in_progress".into(),
            count_type: "cyclic".into(),
            notes: String::new(),
            counted_by: None,
            created_at: "2025-02-01T00:00:00.000Z".into(),
            completed_at: None,
            updated_at: "2025-02-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["count_number"], "CNT-002");
        assert!(json["counted_by"].is_null());
        assert!(json["completed_at"].is_null());
    }

    // ── StockCountLineDto ───────────────────────────────────────────────

    #[test]
    fn stock_count_line_dto_debug() {
        let dto = StockCountLineDto {
            id: "l1".into(),
            count_id: "c1".into(),
            sku: "SKU-A".into(),
            product_name: "Widget".into(),
            expected_qty: 100,
            counted_qty: Some(95),
            difference: -5,
            notes: "Short".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("SKU-A"));
        assert!(d.contains("Widget"));
    }

    #[test]
    fn stock_count_line_dto_serialize() {
        let dto = StockCountLineDto {
            id: "l2".into(),
            count_id: "c2".into(),
            sku: "SKU-B".into(),
            product_name: "Gadget".into(),
            expected_qty: 50,
            counted_qty: None,
            difference: 0,
            notes: String::new(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["sku"], "SKU-B");
        assert_eq!(json["expected_qty"], 50);
        assert!(json["counted_qty"].is_null());
    }

    // ── StockAdjustmentDto ──────────────────────────────────────────────

    #[test]
    fn stock_adjustment_dto_debug() {
        let dto = StockAdjustmentDto {
            id: "a1".into(),
            count_id: Some("c1".into()),
            sku: "SKU-A".into(),
            product_name: "Widget".into(),
            previous_qty: 100,
            adjusted_qty: 95,
            reason: "Cycle count adjustment".into(),
            created_by: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("SKU-A"));
    }

    #[test]
    fn stock_adjustment_dto_serialize() {
        let dto = StockAdjustmentDto {
            id: "a2".into(),
            count_id: None,
            sku: "SKU-C".into(),
            product_name: "Manual adj".into(),
            previous_qty: 200,
            adjusted_qty: 210,
            reason: "Correction".into(),
            created_by: Some("admin".into()),
            created_at: "2025-03-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["sku"], "SKU-C");
        assert!(json["count_id"].is_null());
    }

    // ── CreateStockCountArgs ────────────────────────────────────────────

    #[test]
    fn create_stock_count_args_deserialize() {
        let json = r#"{"count_type":"full","notes":"Q1 count","counted_by":"user1"}"#;
        let args: CreateStockCountArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.count_type, "full");
        assert_eq!(args.notes, "Q1 count");
        assert_eq!(args.counted_by.as_deref(), Some("user1"));
    }

    #[test]
    fn create_stock_count_args_debug() {
        let args = CreateStockCountArgs {
            count_type: "spot".into(),
            notes: "Quick check".into(),
            counted_by: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("spot"));
    }

    // ── AddCountLineArgs ────────────────────────────────────────────────

    #[test]
    fn add_count_line_args_deserialize() {
        let json = r#"{"count_id":"c1","sku":"SKU-A","product_name":"Widget","expected_qty":100}"#;
        let args: AddCountLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.count_id, "c1");
        assert_eq!(args.sku, "SKU-A");
        assert_eq!(args.expected_qty, 100);
    }

    #[test]
    fn add_count_line_args_debug() {
        let args = AddCountLineArgs {
            count_id: "c2".into(),
            sku: "SKU-B".into(),
            product_name: "Gadget".into(),
            expected_qty: 50,
        };
        let d = format!("{args:?}");
        assert!(d.contains("Gadget"));
    }

    // ── UpdateCountLineArgs ─────────────────────────────────────────────

    #[test]
    fn update_count_line_args_deserialize() {
        let json = r#"{"line_id":"l1","counted_qty":95,"notes":"Found 95"}"#;
        let args: UpdateCountLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.line_id, "l1");
        assert_eq!(args.counted_qty, Some(95));
        assert_eq!(args.notes, "Found 95");
    }

    #[test]
    fn update_count_line_args_deserialize_no_counted_qty() {
        let json = r#"{"line_id":"l2","notes":"Skipped"}"#;
        let args: UpdateCountLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.counted_qty, None);
    }

    #[test]
    fn update_count_line_args_debug() {
        let args = UpdateCountLineArgs {
            line_id: "l3".into(),
            counted_qty: Some(10),
            notes: String::new(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("l3"));
    }

    // ── RemoveCountLineArgs ─────────────────────────────────────────────

    #[test]
    fn remove_count_line_args_deserialize() {
        let json = r#"{"line_id":"l99"}"#;
        let args: RemoveCountLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.line_id, "l99");
    }

    #[test]
    fn remove_count_line_args_debug() {
        let args = RemoveCountLineArgs {
            line_id: "l42".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("l42"));
    }

    // ── CompleteStockCountArgs ──────────────────────────────────────────

    #[test]
    fn complete_stock_count_args_deserialize() {
        let json = r#"{"count_id":"c1","completed_by":"user1"}"#;
        let args: CompleteStockCountArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.count_id, "c1");
        assert_eq!(args.completed_by.as_deref(), Some("user1"));
    }

    #[test]
    fn complete_stock_count_args_deserialize_no_completed_by() {
        let json = r#"{"count_id":"c2"}"#;
        let args: CompleteStockCountArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.completed_by, None);
    }

    #[test]
    fn complete_stock_count_args_debug() {
        let args = CompleteStockCountArgs {
            count_id: "c3".into(),
            completed_by: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("c3"));
    }
}
