//! Intelligence / reporting commands: revenue, heatmap, top products, alerts.
//!
//! These commands expose the `oz_core::db::reports` Store methods as
//! Tauri IPC handlers for the dashboard and analytics front-end.

use tauri::{State, command};

use oz_core::db::Store;
use oz_core::db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    TopProductRow, WeeklyRevenueRow,
};
use oz_core::export::{CustomReportRequest, CustomReportResponse};

use crate::error::AppError;
use crate::state::AppState;

#[command]
/// Get menu engineering.
pub async fn get_menu_engineering(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<oz_reporting::menu_engineering::MenuEngineeringResult, AppError> {
    let db = state.db.lock().await;
    let result =
        oz_reporting::menu_engineering::query_menu_engineering(&db, &start_date, &end_date)?;
    drop(db);
    Ok(result)
}

#[command]
/// Get daily revenue.
pub async fn get_daily_revenue(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<DailyRevenueRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.daily_revenue(&start_date, &end_date)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get weekly revenue.
pub async fn get_weekly_revenue(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<WeeklyRevenueRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.weekly_revenue(&start_date, &end_date)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get monthly revenue.
pub async fn get_monthly_revenue(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<MonthlyRevenueRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.monthly_revenue(&start_date, &end_date)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get top products.
pub async fn get_top_products(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
    limit: i64,
) -> Result<Vec<TopProductRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.top_products(&start_date, &end_date, limit)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get hourly heatmap.
pub async fn get_hourly_heatmap(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<HourlyHeatmapRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.hourly_heatmap(&start_date, &end_date)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get low stock alerts.
#[allow(deprecated)]
pub async fn get_low_stock_alerts(
    state: State<'_, AppState>,
    threshold: i64,
) -> Result<Vec<LowStockAlert>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.low_stock_alerts(threshold)?;
    drop(db);
    Ok(rows)
}

#[command]
/// Get category breakdown.
pub async fn get_category_breakdown(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<CategoryBreakdownRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.category_breakdown(&start_date, &end_date)?;
    drop(db);
    Ok(rows)
}

/// Build a custom report from user-selected columns and filters.
///
/// The backend validates column names against a per-dataset whitelist
/// to prevent SQL injection — only recognised columns are included.
/// Supported datasets: "sales" (5 columns, date filter), "inventory" (5 columns).
#[command]
pub async fn build_custom_report(
    state: State<'_, AppState>,
    request: CustomReportRequest,
) -> Result<CustomReportResponse, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.build_custom_report(request)?;
    drop(db);
    Ok(result)
}
