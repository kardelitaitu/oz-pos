//! Workspace listing and boot-resolution commands for the tablet client.
//!
//! Parity with the desktop client (audit/06 residual): the pre-session
//! workspace picker (`list_workspaces` / `list_workspace_screens`) only
//! accepts the short-lived picker ticket minted by `staff_login` and
//! resolves the caller's REAL user + role from the global identity
//! database — caller-supplied `role_id` / `user_id` are never trusted.

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use tauri::State;

use oz_core::db::Store;
use oz_core::db::workspaces::WorkspaceDto;
use platform_core::StoreDatabaseManager;

use crate::commands::picker_ticket;
use crate::commands::terminals::DEVICE_BINDING_KEYRING_NAME;
use crate::error::AppError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

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

/// Verify a device-binding HMAC signature using constant-time comparison.
///
/// Uses `mac.verify_slice()` which internally uses `subtle::ConstantTimeEq`
/// to prevent timing side-channel attacks (parity with the desktop client).
fn verify_binding_hmac(
    secret: &str,
    terminal_id: &str,
    store_id: &str,
    instance_id: &str,
    hex_signature: &str,
) -> bool {
    let expected_bytes = match hex::decode(hex_signature) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(terminal_id.as_bytes());
    mac.update(b":");
    mac.update(store_id.as_bytes());
    mac.update(b":");
    mac.update(instance_id.as_bytes());
    mac.verify_slice(&expected_bytes).is_ok()
}

/// Resolve the active store at boot time (before authentication).
///
/// Parity with the desktop client: when the device has a stored binding
/// (terminal row + HMAC signature made with the OS-keyring secret), the
/// tablet auto-boots into that store + instance. A missing/tampered
/// binding, an unknown device, or a bound instance that no longer exists
/// all fall back to the primary store profile — a boot can never fail
/// because of a stale binding.
#[tauri::command]
pub async fn resolve_boot_store(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<BootResolution, AppError> {
    let device_id = device_id
        .filter(|d| !d.is_empty())
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .ok()
        })
        .unwrap_or_default(); // A keyring failure must never break boot: without a keyring there is
    // no way to verify a binding, so resolution degrades to primary store.
    // The (non-Send) keyring is acquired only after the lock so no `.await`
    // point holds it — Tauri requires command futures to be Send.
    let db = state.db.lock().await;
    let keyring = oz_security::default_keyring().ok();
    let resolution =
        resolve_boot_store_core(&db, &state.db_manager, &device_id, keyring.as_deref())?;
    drop(db);
    Ok(resolution)
}

/// Core boot-resolution logic (extracted for testing).
///
/// `keyring` is `None` when the OS keyring is unavailable or when the
/// caller only wants the primary-store fallback; the binding path is only
/// reachable with a keyring present.
fn resolve_boot_store_core(
    conn: &rusqlite::Connection,
    db_manager: &StoreDatabaseManager,
    device_id: &str,
    keyring: Option<&dyn oz_security::Keyring>,
) -> Result<BootResolution, AppError> {
    let primary_store = |conn: &rusqlite::Connection| -> Result<BootResolution, AppError> {
        let store = Store::new(conn);
        let primary = store
            .get_primary_store()?
            .ok_or_else(|| AppError::Internal("no primary store found".into()))?;
        tracing::info!(
            store_id = %primary.id,
            "tablet boot resolution: primary store"
        );
        Ok(BootResolution {
            is_bound: false,
            store_id: primary.id,
            instance_id: None,
        })
    };

    if device_id.is_empty() {
        return primary_store(conn);
    }
    let Some(keyring) = keyring else {
        return primary_store(conn);
    };

    let binding_info: Option<(String, String, String, String)> = {
        let store = Store::new(conn);
        store
            .get_terminal_by_device_id(device_id)?
            .and_then(|terminal| {
                let tid = terminal.id;
                store
                    .get_terminal_binding(&tid)
                    .ok()
                    .flatten()
                    .map(|(s, i, sig)| (tid, s, i, sig))
            })
    };

    if let Some((terminal_id, bound_store_id, bound_instance_id, signature)) = binding_info {
        let secret = keyring
            .get_secret(DEVICE_BINDING_KEYRING_NAME)
            .map_err(|e| AppError::Internal(format!("keyring read failed: {e}")))?;
        let signature_valid = match secret {
            Some(secret) => verify_binding_hmac(
                &secret,
                &terminal_id,
                &bound_store_id,
                &bound_instance_id,
                &signature,
            ),
            None => false,
        };

        if !signature_valid {
            tracing::warn!(
                terminal_id = %terminal_id,
                bound_store_id = %bound_store_id,
                "tablet device binding HMAC validation failed — falling back to primary store"
            );
        } else {
            let instance_exists = {
                db_manager
                    .open_store(&bound_store_id)
                    .ok()
                    .and_then(|db_arc| {
                        let db = db_arc.lock().ok()?;
                        let store = Store::new(&db);
                        store
                            .get_workspace_instance(&bound_instance_id, None)
                            .ok()
                            .map(|_| true)
                    })
                    .unwrap_or(false)
            };

            if !instance_exists {
                tracing::warn!(
                    terminal_id = %terminal_id,
                    bound_store_id = %bound_store_id,
                    bound_instance_id = %bound_instance_id,
                    "tablet bound instance not found or not active — falling back to primary store"
                );
            } else {
                tracing::info!(
                    terminal_id = %terminal_id,
                    store_id = %bound_store_id,
                    instance_id = %bound_instance_id,
                    "tablet device binding resolved — auto-booting into bound workspace"
                );
                return Ok(BootResolution {
                    is_bound: true,
                    store_id: bound_store_id,
                    instance_id: Some(bound_instance_id),
                });
            }
        }
    }

    primary_store(conn)
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

    // ── Device binding auto-boot (parity with desktop client) ───────────

    use oz_core::Terminal;
    use oz_security::Keyring as _;

    /// HMAC-SHA256 hex over `{terminal}:{store}:{instance}` with a fixed
    /// secret — mirrors `sign_binding` so tests can forge bindings.
    fn hmac_hex(secret: &str, terminal_id: &str, store_id: &str, instance_id: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(terminal_id.as_bytes());
        mac.update(b":");
        mac.update(store_id.as_bytes());
        mac.update(b":");
        mac.update(instance_id.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Global DB with a bound terminal (device "tablet-1" → store-a/ws-a-1,
    /// signed with a known in-memory keyring secret) + a store-a DB with the
    /// instance. Primary store row exists in the global DB for fallbacks.
    fn binding_state() -> (AppState, tempfile::TempDir, oz_security::InMemoryKeyring) {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let keyring = oz_security::InMemoryKeyring::new();
        keyring
            .set_secret(
                crate::commands::terminals::DEVICE_BINDING_KEYRING_NAME,
                "test-binding-secret",
            )
            .unwrap();

        let terminal = Terminal::new("Tablet-1", "tablet-1");
        store.create_terminal(&terminal).unwrap();
        // `bound_store_id` is FK-enforced against the global `store_profiles`,
        // and `resolve_boot_store` reads the primary from the same table.
        let now = "2026-07-31T00:00:00.000Z";
        conn.execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES ('store-a', 'Store A', '', '', 'USD', 'UTC', 0, ?1, ?1), ('store-main', 'Main', '', '', 'USD', 'UTC', 1, ?1, ?1)",
            [now],
        )
        .unwrap();
        let sig = hmac_hex("test-binding-secret", &terminal.id, "store-a", "ws-a-1");
        store
            .update_terminal_binding(&terminal.id, "store-a", "ws-a-1", &sig)
            .unwrap();

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
        (state, temp_dir, keyring)
    }

    #[tokio::test]
    async fn resolve_boot_store_autoboots_into_bound_instance() {
        let (state, _dir, keyring) = binding_state();
        let db = state.db.lock().await;
        let resolution =
            resolve_boot_store_core(&db, &state.db_manager, "tablet-1", Some(&keyring)).unwrap();
        assert!(resolution.is_bound, "valid binding must auto-boot");
        assert_eq!(resolution.store_id, "store-a");
        assert_eq!(resolution.instance_id.as_deref(), Some("ws-a-1"));
    }

    #[tokio::test]
    async fn resolve_boot_store_tampered_binding_falls_back_to_primary() {
        let (state, _dir, _keyring) = binding_state();
        // A DIFFERENT keyring secret — the DB row was not signed by this
        // device's secret, so the HMAC must fail and resolution degrades.
        let other = oz_security::InMemoryKeyring::new();
        other
            .set_secret(
                crate::commands::terminals::DEVICE_BINDING_KEYRING_NAME,
                "attacker-secret",
            )
            .unwrap();
        let db = state.db.lock().await;
        let resolution =
            resolve_boot_store_core(&db, &state.db_manager, "tablet-1", Some(&other)).unwrap();
        assert!(!resolution.is_bound);
        assert_eq!(resolution.store_id, "store-main");
    }

    #[tokio::test]
    async fn resolve_boot_store_bound_instance_missing_falls_back_to_primary() {
        let (state, _dir, keyring) = binding_state();
        // Valid signature, but the bound instance was archived/deleted.
        {
            let conn = state.db_manager.open_store("store-a").unwrap();
            let db = conn.lock().unwrap();
            let store = Store::new(&db);
            store.archive_instance("ws-a-1").unwrap();
            drop(db);
        }
        let db = state.db.lock().await;
        let resolution =
            resolve_boot_store_core(&db, &state.db_manager, "tablet-1", Some(&keyring)).unwrap();
        assert!(!resolution.is_bound);
        assert_eq!(resolution.store_id, "store-main");
    }

    #[tokio::test]
    async fn resolve_boot_store_unknown_device_falls_back_to_primary() {
        let (state, _dir, keyring) = binding_state();
        let db = state.db.lock().await;
        let resolution =
            resolve_boot_store_core(&db, &state.db_manager, "ghost-device", Some(&keyring))
                .unwrap();
        assert!(!resolution.is_bound);
        assert_eq!(resolution.store_id, "store-main");
    }

    #[test]
    fn verify_binding_hmac_valid_signature_passes() {
        let sig = hmac_hex("secret", "term-1", "store-a", "ws-a-1");
        assert!(verify_binding_hmac(
            "secret", "term-1", "store-a", "ws-a-1", &sig
        ));
    }

    #[test]
    fn verify_binding_hmac_tampered_signature_fails() {
        let sig = hmac_hex("secret", "term-1", "store-a", "ws-a-1");
        assert!(!verify_binding_hmac(
            "secret", "term-1", "store-a", "ws-a-2", &sig
        ));
        assert!(!verify_binding_hmac(
            "other", "term-1", "store-a", "ws-a-1", &sig
        ));
    }

    #[test]
    fn verify_binding_hmac_garbage_hex_fails() {
        assert!(!verify_binding_hmac(
            "secret", "term-1", "store-a", "ws-a-1", "not-hex!"
        ));
        assert!(!verify_binding_hmac(
            "secret", "term-1", "store-a", "ws-a-1", ""
        ));
    }
}
