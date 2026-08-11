//! Intelligence / reporting commands: revenue, heatmap, top products, alerts.
//!
//! These commands expose the `oz_core::db::reports` Store methods as
//! Tauri IPC handlers for the dashboard and analytics front-end.

use tauri::State;

use oz_core::db::Store;
use oz_core::db::popularity::{CategoryPopularityRow, CategoryTrendPoint};
use oz_core::db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    TopProductRow, WeeklyRevenueRow,
};
use oz_core::export::{CustomReportRequest, CustomReportResponse};
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

const MAX_TOP_PRODUCTS: i64 = 100;

async fn resolve_report_scope(
    state: &AppState,
    session_token: &str,
    permission: &str,
) -> Result<std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>, AppError> {
    let session = state.resolve_session(session_token)?;
    {
        let db = state.db.lock().await;
        let identity_store = Store::new(&db);
        require_permission_for_user(&identity_store, &session.user_id, permission)?;
    }
    state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))
}

fn validate_top_product_limit(limit: i64) -> Result<(), AppError> {
    if !(1..=MAX_TOP_PRODUCTS).contains(&limit) {
        return Err(AppError::Invalid(format!(
            "top product limit must be between 1 and {MAX_TOP_PRODUCTS}"
        )));
    }
    Ok(())
}

/// Per-category popularity limits: a category's leaderboard needs only a
/// handful of entries (the UI shows the top 3).
const MAX_CATEGORY_TOP: i64 = 20;

fn validate_category_top(top_per_category: i64) -> Result<(), AppError> {
    if !(1..=MAX_CATEGORY_TOP).contains(&top_per_category) {
        return Err(AppError::Invalid(format!(
            "top per category must be between 1 and {MAX_CATEGORY_TOP}"
        )));
    }
    Ok(())
}

/// Trend series limit: the chart shows one line per category, so more than
/// a handful of series becomes unreadable.
const MAX_TREND_CATEGORIES: i64 = 10;

fn validate_trend_args(granularity: &str, top_categories: i64) -> Result<(), AppError> {
    if !oz_core::db::popularity::TREND_GRANULARITIES.contains(&granularity) {
        return Err(AppError::Invalid(format!(
            "granularity must be one of {:?}",
            oz_core::db::popularity::TREND_GRANULARITIES
        )));
    }
    if !(1..=MAX_TREND_CATEGORIES).contains(&top_categories) {
        return Err(AppError::Invalid(format!(
            "top categories must be between 1 and {MAX_TREND_CATEGORIES}"
        )));
    }
    Ok(())
}

/// The top-products ranking keys accepted by the command layer (whitelist
/// — the store query falls back to revenue for anything else).
fn validate_top_product_order(order_by: &str) -> Result<(), AppError> {
    if !matches!(order_by, "revenue" | "profit") {
        return Err(AppError::Invalid(format!(
            "top product order must be 'revenue' or 'profit', got '{order_by}'"
        )));
    }
    Ok(())
}

#[tauri::command]
/// Get menu engineering from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_menu_engineering_scoped`.
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

#[tauri::command]
/// Get menu engineering for the session's store.
#[allow(clippy::too_many_arguments)]
pub async fn get_menu_engineering_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<oz_reporting::menu_engineering::MenuEngineeringResult, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(oz_reporting::menu_engineering::query_menu_engineering(
        &db,
        &start_date,
        &end_date,
    )?)
}

#[tauri::command]
/// Get per-line cost and margin for a single sale (HPP exposure).
///
/// Enriches every line of the sale with the product's current cost, the
/// line margin, and the margin percentage (see `oz_reporting::margin`).
pub async fn get_sale_line_margins_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_reporting::margin::SaleLineMargin>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(oz_reporting::margin::query_sale_lines_with_margin(
        &db, &sale_id,
    )?)
}

#[tauri::command]
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

#[tauri::command]
/// Get daily revenue for the session's store.
pub async fn get_daily_revenue_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<Vec<DailyRevenueRow>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).daily_revenue(&start_date, &end_date)?)
}

#[tauri::command]
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

#[tauri::command]
/// Get weekly revenue for the session's store.
pub async fn get_weekly_revenue_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<Vec<WeeklyRevenueRow>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).weekly_revenue(&start_date, &end_date)?)
}

#[tauri::command]
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

#[tauri::command]
/// Get monthly revenue for the session's store.
pub async fn get_monthly_revenue_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<Vec<MonthlyRevenueRow>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).monthly_revenue(&start_date, &end_date)?)
}

#[tauri::command]
/// Get top products.
pub async fn get_top_products(
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
    limit: i64,
    order_by: String,
) -> Result<Vec<TopProductRow>, AppError> {
    validate_top_product_order(&order_by)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.top_products(&start_date, &end_date, limit, &order_by)?;
    drop(db);
    Ok(rows)
}

#[tauri::command]
/// Get top products for the session's store with a bounded limit.
pub async fn get_top_products_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    limit: i64,
    order_by: String,
    state: State<'_, AppState>,
) -> Result<Vec<TopProductRow>, AppError> {
    validate_top_product_limit(limit)?;
    validate_top_product_order(&order_by)?;
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).top_products(&start_date, &end_date, limit, &order_by)?)
}

#[tauri::command]
/// Get per-category popularity standings for the session's store: each
/// category's mean score, its ratio to the catalog average, and its
/// top products ranked by popularity (ADR #37 per-category evolution).
pub async fn get_category_popularity_scoped(
    session_token: String,
    top_per_category: i64,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryPopularityRow>, AppError> {
    validate_category_top(top_per_category)?;
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).category_popularity(top_per_category)?)
}

#[tauri::command]
/// Get the per-period popularity trend for the session's store: each of the
/// top categories' score over `start_date..=end_date`, bucketed by
/// `granularity` (`daily` | `weekly` | `monthly`) — the same ADR #37 blend
/// as the materialized scores, so the lines read against current standings.
pub async fn get_category_popularity_trend_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    granularity: String,
    top_categories: i64,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryTrendPoint>, AppError> {
    validate_trend_args(&granularity, top_categories)?;
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).category_popularity_trend(
        &start_date,
        &end_date,
        &granularity,
        top_categories,
    )?)
}

#[tauri::command]
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

#[tauri::command]
/// Get hourly heatmap for the session's store.
pub async fn get_hourly_heatmap_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<Vec<HourlyHeatmapRow>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).hourly_heatmap(&start_date, &end_date)?)
}

#[tauri::command]
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

#[tauri::command]
/// Get low stock alerts for the session's default store location.
pub async fn get_low_stock_alerts_scoped(
    session_token: String,
    threshold: i64,
    state: State<'_, AppState>,
) -> Result<Vec<LowStockAlert>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).low_stock_alerts_at_location(
        oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        threshold,
    )?)
}

#[tauri::command]
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

#[tauri::command]
/// Get category breakdown for the session's store.
pub async fn get_category_breakdown_scoped(
    session_token: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryBreakdownRow>, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).category_breakdown(&start_date, &end_date)?)
}

/// Build a custom report from user-selected columns and filters.
///
/// **Deprecated for multi-store (ADR #7):** Use `build_custom_report_scoped`.
#[tauri::command]
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

/// Build a custom report for the session's store.
///
/// Custom reports can expose customer and staff data, so exporting them
/// requires the stronger `reports:export` permission.
#[tauri::command]
pub async fn build_custom_report_scoped(
    session_token: String,
    request: CustomReportRequest,
    state: State<'_, AppState>,
) -> Result<CustomReportResponse, AppError> {
    let conn = resolve_report_scope(&state, &session_token, permissions::REPORTS_EXPORT).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).build_custom_report(request)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::session::SessionContext;

    #[tokio::test]
    async fn scoped_report_rejects_invalid_session() {
        let state = AppState::for_test();
        let result = resolve_report_scope(&state, "missing-token", permissions::REPORTS_VIEW).await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn scoped_report_denies_user_without_reports_permission() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-custom', 'custom', 'hash', 'Custom', 'role-custom', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let state = AppState::for_test_with_conn(conn);
        state.session_store.write().unwrap().insert(
            "custom-token".into(),
            SessionContext::new(
                "user-custom".into(),
                "role-owner".into(),
                "terminal-1".into(),
                "store-1".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );

        let result = resolve_report_scope(&state, "custom-token", permissions::REPORTS_VIEW).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[test]
    fn top_product_limit_accepts_bounded_values() {
        assert!(validate_top_product_limit(1).is_ok());
        assert!(validate_top_product_limit(MAX_TOP_PRODUCTS).is_ok());
    }

    #[test]
    fn top_product_limit_rejects_unbounded_values() {
        for limit in [0, -1, MAX_TOP_PRODUCTS + 1, i64::MAX] {
            assert!(validate_top_product_limit(limit).is_err());
        }
    }

    #[test]
    fn top_product_order_accepts_revenue_and_profit() {
        assert!(validate_top_product_order("revenue").is_ok());
        assert!(validate_top_product_order("profit").is_ok());
    }

    #[test]
    fn top_product_order_rejects_unknown_values() {
        for bad in ["", "quantity", "margin", "revenue DESC"] {
            assert!(validate_top_product_order(bad).is_err());
        }
    }
}
