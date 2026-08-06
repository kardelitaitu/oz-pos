//! Workspace listing and boot-resolution commands for the tablet client.
//!
//! Parity with the desktop client (audit/06 residual): the pre-session
//! workspace picker (`list_workspaces` / `list_workspace_screens`) only
//! accepts the short-lived picker ticket minted by `staff_login` and
//! resolves the caller's REAL user + role from the global identity
//! database — caller-supplied `role_id` / `user_id` are never trusted.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use oz_core::db::Store;
use oz_core::db::workspaces::WorkspaceDto;

use crate::commands::picker_ticket;
use crate::error::AppError;
use crate::state::AppState;

/// Screen within a workspace as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct WorkspaceScreenDto {
    /// Screen Key.
    pub screen_key: String,
    /// Display sort order.
    pub sort_order: i32,
}

/// List workspace instances for the pre-session workspace picker.
///
/// Parity with the desktop client: the caller presents the short-lived
/// picker ticket minted by `staff_login` (audit/06 residual); the REAL
/// user is resolved from the global identity database and the REAL role
/// is used for the listing. The requested store is opened through
/// `StoreDatabaseManager` so this read cannot accidentally query the
/// global identity database or another store's connection.
#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
    ticket: String,
    store_id: String,
) -> Result<Vec<WorkspaceDto>, AppError> {
    // 1. Verify the ticket — uniform denial for forged/expired/malformed.
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let user_id = picker_ticket::verify_picker_ticket(&state.picker_ticket_secret, &ticket, now_ts)
        .ok_or_else(|| AppError::PermissionDenied("invalid or expired picker session".into()))?;

    // 2. Resolve the REAL user + role from the global identity DB. The ticket
    //    binds the user; the role is derived from the DB, never the claim.
    let (real_role_id, real_user_id) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let user = store.get_user(&user_id)?.ok_or_else(|| {
            AppError::PermissionDenied("picker session user no longer exists".into())
        })?;
        if !user.is_active {
            return Err(AppError::PermissionDenied(
                "picker session user is inactive".into(),
            ));
        }
        let role = store
            .get_role(&user.role_id)?
            .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;
        (role.id, user.id)
    };

    // 3. List instances in the requested store using the REAL role + user.
    //    `list_workspaces` applies the owner bypass, `user_store_access`
    //    (multi-store), explicit instance assignment, and role workspace types.
    let conn = state
        .db_manager
        .open_store(&store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspaces(&real_role_id, Some(&real_user_id), &store_id)?;
    drop(db);
    Ok(rows)
}

/// List screens (nav items) for a workspace type during boot/workspace
/// selection. The store ID is explicit so the read is routed to the correct
/// store database.
///
/// Parity with the desktop client: the picker ticket (audit/06 residual)
/// proves the caller completed a real login before this bootstrap read can
/// touch any store database.
#[tauri::command]
pub async fn list_workspace_screens(
    state: State<'_, AppState>,
    ticket: String,
    type_key: String,
    store_id: String,
) -> Result<Vec<WorkspaceScreenDto>, AppError> {
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    picker_ticket::verify_picker_ticket(&state.picker_ticket_secret, &ticket, now_ts)
        .ok_or_else(|| AppError::PermissionDenied("invalid or expired picker session".into()))?;
    let conn = state
        .db_manager
        .open_store(&store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspace_type_screens(&type_key)?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceScreenDto {
            screen_key: r.screen_key,
            sort_order: r.sort_order,
        })
        .collect())
}

/// DTO returned by `resolve_boot_store`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootResolution {
    /// Whether this is bound.
    pub is_bound: bool,
    /// ID of the associated store.
    pub store_id: String,
    /// ID of the associated instance.
    pub instance_id: Option<String>,
}

/// Resolve the active store at boot time (before authentication).
///
/// Parity with the desktop client's boot flow: falls back to the primary
/// store profile. Device binding is not implemented on the tablet client,
/// so the resolution is never `is_bound: true` here.
#[tauri::command]
pub async fn resolve_boot_store(
    state: State<'_, AppState>,
    _device_id: Option<String>,
) -> Result<BootResolution, AppError> {
    let primary_id = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let primary = store
            .get_primary_store()?
            .ok_or_else(|| AppError::Internal("no primary store found".into()))?;
        primary.id
    };
    tracing::info!(
        store_id = %primary_id,
        "tablet boot resolution: primary store"
    );
    Ok(BootResolution {
        is_bound: false,
        store_id: primary_id,
        instance_id: None,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use oz_core::StoreProfile;
    use oz_core::migrations;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    /// Seed the GLOBAL identity DB with an owner and a cashier.
    fn seed_global_users(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-owner',   'owner',   'hash', 'Owner',   'role-owner',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-cashier', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
    }

    fn make_profile(id: &str, name: &str) -> StoreProfile {
        StoreProfile {
            id: id.to_owned(),
            name: name.to_owned(),
            address: String::new(),
            tax_id: String::new(),
            currency: "USD".to_owned(),
            timezone: "UTC".to_owned(),
            is_primary: false,
            created_at: "2026-07-01T10:00:00Z".to_owned(),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
        }
    }

    /// Build an AppState with a global identity DB (users seeded) and a
    /// temp-dir store manager with store-a (1 instance) so role binding can
    /// be exercised.
    fn picker_state() -> (AppState, tempfile::TempDir) {
        let conn = migrations::fresh_db();
        seed_global_users(&conn);
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), migrations::ALL);

        let conn = state.db_manager.open_store("store-a").unwrap();
        let db = conn.lock().unwrap();
        let store = Store::new(&db);
        store
            .create_store_profile(&make_profile("store-a", "Store A"))
            .unwrap();
        store
            .create_workspace_instance("ws-a-1", "store-pos", "store-a", "POS", "", None)
            .unwrap();
        drop(db);
        (state, temp_dir)
    }

    fn sign_ticket_for(state: &AppState, user_id: &str, ttl_offset: i64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        picker_ticket::sign_picker_ticket(&state.picker_ticket_secret, user_id, now + ttl_offset)
    }

    #[tokio::test]
    async fn list_workspaces_rejects_forged_or_missing_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        for ticket in [
            String::new(),
            "garbage-not-a-ticket".into(),
            picker_ticket::sign_picker_ticket(
                b"attacker-secret",
                "user-owner",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
                    + 300,
            ),
        ] {
            let result = list_workspaces(app.state(), ticket.clone(), "store-a".into()).await;
            assert!(
                matches!(result, Err(AppError::PermissionDenied(_))),
                "ticket {ticket:?} must be denied"
            );
        }
    }

    #[tokio::test]
    async fn list_workspaces_rejects_expired_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let expired = sign_ticket_for(&app.state(), "user-owner", -1);
        let result = list_workspaces(app.state(), expired, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspaces_rejects_unknown_user_even_with_valid_signature() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let ghost = sign_ticket_for(&app.state(), "user-deleted", 300);
        let result = list_workspaces(app.state(), ghost, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspaces_uses_real_role_not_claimed_role() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let owner_ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let owner_rows = list_workspaces(app.state(), owner_ticket, "store-a".into())
            .await
            .unwrap();
        assert!(owner_rows.iter().any(|d| d.instance_id == "ws-a-1"));

        let cashier_ticket = sign_ticket_for(&app.state(), "user-cashier", 300);
        let cashier_rows = list_workspaces(app.state(), cashier_ticket, "store-a".into())
            .await
            .unwrap();
        assert!(
            cashier_rows.is_empty(),
            "cashier role must not see owner-level instances, got {cashier_rows:?}"
        );
    }

    #[tokio::test]
    async fn list_workspaces_inactive_user_is_denied() {
        let (state, _dir) = picker_state();
        {
            let db = state.db.lock().await;
            db.execute("UPDATE users SET is_active = 0 WHERE id = 'user-owner'", [])
                .unwrap();
        }
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let result = list_workspaces(app.state(), ticket, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspace_screens_requires_valid_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let denied = list_workspace_screens(
            app.state(),
            "bogus".into(),
            "store-pos".into(),
            "store-a".into(),
        )
        .await;
        assert!(matches!(denied, Err(AppError::PermissionDenied(_))));

        let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let allowed =
            list_workspace_screens(app.state(), ticket, "store-pos".into(), "store-a".into()).await;
        assert!(allowed.is_ok());
    }

    #[tokio::test]
    async fn resolve_boot_store_returns_primary_store() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let now = "2026-07-31T00:00:00.000Z";
        conn.execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES ('store-main', 'Main', '', '', 'USD', 'UTC', 1, ?1, ?1)",
            [now],
        )
        .unwrap();
        let _ = store;
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let resolution = resolve_boot_store(app.state(), None).await.unwrap();
        assert_eq!(resolution.store_id, "store-main");
        assert!(!resolution.is_bound);
        assert!(resolution.instance_id.is_none());
    }
}
