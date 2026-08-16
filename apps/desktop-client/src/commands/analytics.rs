//! Analytics commands (analytics:view — owner/admin/manager only).
//!
//! Per-staff shift + completed-sales aggregates for the session's store,
//! enriched with display names from the GLOBAL identity DB. The gate is
//! scope-aware (ADR #35 D5 / spec 0048): `require_permission_for_session`
//! evaluates the session's store against the caller's assignment, so a
//! scoped member only sees analytics for branches they are assigned to
//! (an out-of-scope session is denied fail-closed).

use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use oz_core::db::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

/// Per-staff analytics row as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct StaffAnalyticsDto {
    /// Staff member id (cashier).
    pub user_id: String,
    /// Display name resolved from the global identity DB.
    pub display_name: String,
    /// Number of shifts opened in the range.
    pub shift_count: i64,
    /// Number of those shifts closed.
    pub closed_shift_count: i64,
    /// Sum of shift `total_sales_minor` in the range.
    pub shift_sales_minor: i64,
    /// Number of completed sales in the range.
    pub sale_count: i64,
    /// Sum of completed `sales.total_minor` in the range.
    pub sale_total_minor: i64,
}

/// Per-day series row for one staff member.
#[derive(Debug, Serialize)]
pub struct StaffAnalyticsDailyDto {
    /// `YYYY-MM-DD`.
    pub day: String,
    /// Completed sales attributed to the staff member that day.
    pub sale_count: i64,
    /// Sum of `sales.total_minor` for those sales.
    pub sale_total_minor: i64,
    /// Shifts opened that day.
    pub shift_count: i64,
    /// Sum of `shifts.total_sales_minor` for those shifts.
    pub shift_sales_minor: i64,
}

/// Per-staff shift + sales summary for the session's store over `[from, to]`.
#[tauri::command]
pub async fn get_staff_analytics_scoped(
    session_token: String,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<Vec<StaffAnalyticsDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::ANALYTICS_VIEW).await?;

    // Staff display names live in the GLOBAL identity DB; the aggregates in
    // the store-scoped DB. Resolve the name map before opening the store.
    let display_names: HashMap<String, String> = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store
            .list_users()?
            .into_iter()
            .map(|u| (u.id, u.display_name))
            .collect()
    };

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.staff_analytics_summary(&from, &to)?;
    drop(db);

    Ok(rows
        .into_iter()
        .map(|r| StaffAnalyticsDto {
            user_id: r.user_id.clone(),
            display_name: display_names.get(&r.user_id).cloned().unwrap_or_else(|| {
                // A user row can be deleted while shifts/sales survive; show
                // the id rather than leaking a fabricated name.
                r.user_id.clone()
            }),
            shift_count: r.shift_count,
            closed_shift_count: r.closed_shift_count,
            shift_sales_minor: r.shift_sales_minor,
            sale_count: r.sale_count,
            sale_total_minor: r.sale_total_minor,
        })
        .collect())
}

/// Per-day shift + sales series for one staff member over `[from, to]`.
#[tauri::command]
pub async fn get_staff_analytics_daily_scoped(
    session_token: String,
    user_id: String,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<Vec<StaffAnalyticsDailyDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::ANALYTICS_VIEW).await?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.staff_analytics_daily(&user_id, &from, &to)?;
    drop(db);

    Ok(rows
        .into_iter()
        .map(|r| StaffAnalyticsDailyDto {
            day: r.day,
            sale_count: r.sale_count,
            sale_total_minor: r.sale_total_minor,
            shift_count: r.shift_count,
            shift_sales_minor: r.shift_sales_minor,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::db::assignments::{AssignmentSpec, ScopeMode};
    use oz_core::migrations;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    /// Global identity DB (owner / manager / staff presets) + a temp-dir
    /// store manager with store-a seeded with one staff's shifts + sales.
    fn analytics_state() -> (AppState, tempfile::TempDir) {
        let conn = migrations::fresh_db();
        {
            let store = Store::new(&conn);
            store.seed_default_roles().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                    ('user-owner',   'owner',   'hash', 'Owner',   'role-owner',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                    ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                    ('user-staff',   'staff',   'hash', 'Staff',   'role-staff',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
            )
            .unwrap();
        }
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), migrations::ALL);

        // Store-a: the store DB has no identity rows, but shifts.user_id FKs
        // to users(id) — seed the store-side user rows (as the shift open
        // path does) plus one shift + completed sales for the analytics.
        let conn = state.db_manager.open_store("store-a").unwrap();
        let db = conn.lock().unwrap();
        db.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-staff', 'Staff', 'Staff', '[]', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, user_id, created_at) VALUES
                ('s1', 12000, 'USD', 1, 'completed', 'user-staff', '2026-07-10T09:00:00Z'),
                ('s2', 8000,  'USD', 1, 'completed', 'user-staff', '2026-07-10T14:00:00Z');
             INSERT INTO shifts (id, user_id, opened_at, closed_at, status, total_sales_minor, created_at, updated_at) VALUES
                ('sh1', 'user-staff', '2026-07-10T08:00:00Z', '2026-07-10T16:00:00Z', 'closed', 20000, '2026-07-10T08:00:00Z', '2026-07-10T16:00:00Z');",
        )
        .unwrap();
        drop(db);

        (state, temp_dir)
    }

    fn mint_session(state: &mut AppState, token: &str, user: &str, role: &str, store: &str) {
        state.session_store.write().unwrap().insert(
            token.into(),
            oz_core::session::SessionContext::new(
                user.into(),
                role.into(),
                "terminal-1".into(),
                store.into(),
                "ws-a-1".into(),
                "store-pos".into(),
                None,
                0,
            ),
        );
    }

    #[tokio::test]
    async fn staff_role_cannot_view_analytics() {
        let (mut state, _dir) = analytics_state();
        mint_session(
            &mut state,
            "staff-token",
            "user-staff",
            "role-staff",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = get_staff_analytics_scoped(
            "staff-token".into(),
            "2026-07-01".into(),
            "2026-07-31".into(),
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "role-staff lacks analytics:view, got {result:?}"
        );
    }

    #[tokio::test]
    async fn owner_views_staff_analytics_with_display_names() {
        let (mut state, _dir) = analytics_state();
        mint_session(
            &mut state,
            "owner-token",
            "user-owner",
            "role-owner",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let rows = get_staff_analytics_scoped(
            "owner-token".into(),
            "2026-07-01".into(),
            "2026-07-31".into(),
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_id, "user-staff");
        // Enriched from the GLOBAL identity DB, not the store-side row.
        assert_eq!(rows[0].display_name, "Staff");
        assert_eq!(rows[0].shift_count, 1);
        assert_eq!(rows[0].closed_shift_count, 1);
        assert_eq!(rows[0].shift_sales_minor, 20000);
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].sale_total_minor, 20000);
    }

    #[tokio::test]
    async fn manager_views_daily_series_for_a_staff_member() {
        let (mut state, _dir) = analytics_state();
        mint_session(
            &mut state,
            "manager-token",
            "user-manager",
            "role-manager",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let rows = get_staff_analytics_daily_scoped(
            "manager-token".into(),
            "user-staff".into(),
            "2026-07-01".into(),
            "2026-07-31".into(),
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].day, "2026-07-10");
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].sale_total_minor, 20000);
        assert_eq!(rows[0].shift_count, 1);
        assert_eq!(rows[0].shift_sales_minor, 20000);
    }

    #[tokio::test]
    async fn scoped_manager_session_out_of_scope_store_is_denied() {
        let (mut state, _dir) = analytics_state();
        // Manager scoped to branch store-a only — a session minted for
        // store-b is out of scope and must be denied fail-closed before any
        // store DB is touched (ADR #35 D5 / spec 0048).
        {
            let db = state.db.lock().await;
            Store::new(&db)
                .set_assignment(
                    "user-manager",
                    "role-manager",
                    &AssignmentSpec {
                        scope_mode: ScopeMode::Scoped,
                        branches_all: false,
                        branches: vec!["store-a".into()],
                        workspaces_all: true,
                        workspaces: vec![],
                    },
                )
                .unwrap();
        }
        mint_session(
            &mut state,
            "manager-token",
            "user-manager",
            "role-manager",
            "store-b",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = get_staff_analytics_scoped(
            "manager-token".into(),
            "2026-07-01".into(),
            "2026-07-31".into(),
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "out-of-scope session must be denied, got {result:?}"
        );
    }

    #[tokio::test]
    async fn analytics_rejects_invalid_session() {
        let state = AppState::for_test();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = get_staff_analytics_scoped(
            "missing-token".into(),
            "2026-07-01".into(),
            "2026-07-31".into(),
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}
