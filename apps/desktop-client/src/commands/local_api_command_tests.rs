//! Command-level tests for the local API surface. The heavy lifting
//! (bind/serve/mint) is covered in `local_api_tests.rs`; these cover the
//! status projection and setting persistence the IPC layer adds.

use super::*;
use crate::state::AppState;

#[tokio::test]
async fn build_status_reflects_settings_and_slot() {
    let state = AppState::for_test_with_conn(oz_core::migrations::fresh_db());

    // Fresh install: off, not running, default port, no URL.
    let s = build_status(&state).await.unwrap();
    assert!(!s.enabled);
    assert!(!s.running);
    assert_eq!(s.port, local_api::DEFAULT_PORT);
    assert!(s.base_url.is_none());

    // Enabled intent + custom port surface without a running server.
    {
        let db = state.db.lock().await;
        persist_setting(&db, local_api::SETTINGS_ENABLED, "1").unwrap();
        persist_setting(&db, local_api::SETTINGS_PORT, "4010").unwrap();
    }
    let s = build_status(&state).await.unwrap();
    assert!(s.enabled);
    assert!(!s.running);
    assert_eq!(s.port, 4010);
    assert!(s.base_url.is_none());
}

#[tokio::test]
async fn build_status_reports_running_handle() {
    let state = AppState::for_test_with_conn(oz_core::migrations::fresh_db());
    let dir = std::env::temp_dir().join(format!("oz-local-api-cmd-{}", uuid::Uuid::new_v4()));
    let handle = local_api::start(
        state.db.clone(),
        state.db_path.clone(),
        dir.clone(),
        "f".repeat(32),
        0,
    )
    .await
    .unwrap();
    let base_url = handle.base_url.clone();
    *state.local_api.lock().await = Some(handle);

    let s = build_status(&state).await.unwrap();
    assert!(s.running);
    assert_eq!(s.base_url.as_deref(), Some(base_url.as_str()));

    // Stop path mirrors the command's disable branch.
    if let Some(handle) = state.local_api.lock().await.take() {
        handle.stop();
    }
    let s = build_status(&state).await.unwrap();
    assert!(!s.running);
    assert!(s.base_url.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn mint_uses_persisted_secret_stably() {
    let state = AppState::for_test_with_conn(oz_core::migrations::fresh_db());
    let secret = {
        let db = state.db.lock().await;
        local_api::load_or_create_secret(&db).unwrap()
    };
    let first = local_api::mint_token(&secret, "script-a", Some(1)).unwrap();
    let second_secret = {
        let db = state.db.lock().await;
        local_api::load_or_create_secret(&db).unwrap()
    };
    assert_eq!(
        secret, second_secret,
        "secret must not rotate between mints"
    );
    let second = local_api::mint_token(&second_secret, "script-b", Some(1)).unwrap();
    assert_ne!(first.token_id, second.token_id);
    // Both validate under the same per-install secret.
    for t in [&first.token, &second.token] {
        assert!(
            oz_api::auth::validate_token_with_secret(t, Some(&secret))
                .await
                .is_ok()
        );
    }
}
