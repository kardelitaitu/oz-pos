use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;

/// Build a test AppState with a session and a fresh temp-dir db_manager.
fn scoped_state(
    conn: rusqlite::Connection,
    token: &str,
    user_id: &str,
    role_id: &str,
    store_id: &str,
) -> AppState {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut state = AppState::for_test_with_conn(conn);
    state.db_manager =
        StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
    state.session_store.write().unwrap().insert(
        token.into(),
        SessionContext::new(
            user_id.into(),
            role_id.into(),
            "terminal-1".into(),
            store_id.into(),
            "instance-1".into(),
            "pos".into(),
            None,
            0,
        ),
    );
    state
}

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

    let state = scoped_state(conn, "custom-token", "user-custom", "role-owner", "store-1");
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

// ── validate_category_top ──────────────────────────────────────────

#[test]
fn category_top_accepts_bounded_values() {
    assert!(validate_category_top(1).is_ok());
    assert!(validate_category_top(MAX_CATEGORY_TOP).is_ok());
}

#[test]
fn category_top_rejects_unbounded_values() {
    for val in [0, -1, MAX_CATEGORY_TOP + 1] {
        assert!(validate_category_top(val).is_err());
    }
}

// ── validate_trend_args ────────────────────────────────────────────

#[test]
fn trend_args_accepts_valid_granularity_and_top() {
    for g in oz_core::db::popularity::TREND_GRANULARITIES {
        assert!(validate_trend_args(g, 1).is_ok());
        assert!(validate_trend_args(g, MAX_TREND_CATEGORIES).is_ok());
    }
}

#[test]
fn trend_args_rejects_invalid_granularity() {
    assert!(validate_trend_args("invalid", 5).is_err());
    assert!(validate_trend_args("", 5).is_err());
}

#[test]
fn trend_args_rejects_unbounded_top_categories() {
    for val in [0, -1, MAX_TREND_CATEGORIES + 1] {
        assert!(validate_trend_args("daily", val).is_err());
    }
}

// ── resolve_report_scope edge cases ────────────────────────────────

#[tokio::test]
async fn scoped_report_rejects_empty_token() {
    let state = AppState::for_test();
    let result = resolve_report_scope(&state, "", permissions::REPORTS_VIEW).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_report_returns_conn_for_valid_session() {
    let conn = oz_core::migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-rpt', 'rpt', 'hash', 'Reporter', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();

    let state = scoped_state(conn, "rpt-token", "user-rpt", "role-owner", "store-1");
    let result = resolve_report_scope(&state, "rpt-token", permissions::REPORTS_VIEW).await;
    assert!(result.is_ok(), "valid session should resolve scope");
}
