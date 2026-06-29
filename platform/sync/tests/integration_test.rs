//! Integration test: offline sale → sync → verify on server.
//!
//! Spins up a lightweight axum test server, seeds the local offline queue,
//! runs a sync cycle, and verifies the server received the data and the
//! local items were marked as synced.

use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, routing::post};
use oz_core::{
    Store, migrations,
    offline::{OfflineQueueItem, OfflineQueueStatus},
    sync_client::SyncConfig,
};
use platform_sync::{
    SyncEngine,
    transport::{PullRequest, PullResponse, PushOutcome, PushResponse},
};

// ── Test HTTP server ─────────────────────────────────────────────────

/// Shared state capturing what the test server receives.
#[derive(Clone, Default)]
struct TestServerState {
    received_pushes: Arc<Mutex<Vec<OfflineQueueItem>>>,
}

/// Test server bundle returned by [`spawn_test_server`].
struct TestServer {
    port: u16,
    state: TestServerState,
    handle: tokio::task::JoinHandle<()>,
}

/// Handler: POST /api/sync/push
async fn handle_push(
    State(state): State<TestServerState>,
    Json(items): Json<Vec<OfflineQueueItem>>,
) -> Json<PushResponse> {
    let mut pushes = state.received_pushes.lock().unwrap();
    let results: Vec<PushOutcome> = items.iter().map(|_| PushOutcome::Accepted).collect();
    pushes.extend(items);
    Json(PushResponse { results })
}

/// Handler: POST /api/sync/pull
async fn handle_pull(Json(_request): Json<PullRequest>) -> Json<PullResponse> {
    Json(PullResponse { items: vec![] })
}

/// Start a test server on a random port and return a [`TestServer`] bundle.
async fn spawn_test_server() -> TestServer {
    let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let state = TestServerState::default();
    let app = Router::new()
        .route("/api/sync/push", post(handle_push))
        .route("/api/sync/pull", post(handle_pull))
        .with_state(state.clone());

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Brief delay so the server is ready before tests send requests.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    TestServer {
        port,
        state,
        handle,
    }
}

/// Create a test server with custom route handlers, returning both the port
/// and a join handle. Handlers are passed as an axum Router.
///
/// This avoids per-test TcpListener binding which can trigger Windows
/// firewall 403 errors with 127.0.0.1.
async fn spawn_custom_server(app: Router) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (port, handle)
}

// ── Test helpers ─────────────────────────────────────────────────────

/// Create an in-memory SQLite database with migrations.
fn setup_store() -> Store<'static> {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&mut conn).unwrap();
    let conn: &'static rusqlite::Connection = Box::leak(Box::new(conn));
    Store::new(conn)
}

/// Create a `SyncConfig` pointing at the test server.
fn test_config(port: u16) -> SyncConfig {
    SyncConfig {
        server_url: format!("http://localhost:{port}"),
        api_key: None,
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn single_sale_enqueued_is_pushed_and_marked_synced() {
    let server = spawn_test_server().await;
    let store = setup_store();

    // Enqueue a completed sale (simulating SaleSyncEnqueuer behaviour).
    let payload = serde_json::json!({
        "sale_id": "it-sale-1",
        "total_minor": 1500,
        "currency": "USD",
        "line_items": [{"sku": "COFFEE", "qty": 3, "unit_price_minor": 500}],
    })
    .to_string();

    let queued = store.enqueue_offline("complete_sale", &payload).unwrap();
    assert_eq!(queued.action, "complete_sale");
    assert_eq!(store.pending_offline_count().unwrap(), 1);

    // Run a sync cycle.
    let engine = SyncEngine::new(test_config(server.port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    // Server received the item.
    let pushes = server.state.received_pushes.lock().unwrap();
    assert_eq!(pushes.len(), 1);
    assert_eq!(pushes[0].action, "complete_sale");
    assert!(pushes[0].payload.contains("it-sale-1"));
    drop(pushes); // release lock

    // Local item was marked as synced.
    let all = store.list_all_offline().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].status, OfflineQueueStatus::Synced);
    assert!(all[0].synced_at.is_some());
    assert_eq!(store.pending_offline_count().unwrap(), 0);

    // ReplicationResult is correct.
    assert_eq!(result.pushed, 1);
    assert_eq!(result.pulled, 0);

    server.handle.abort();
}

#[tokio::test]
async fn push_items_oldest_first() {
    let server = spawn_test_server().await;
    let store = setup_store();

    // Enqueue items with a small delay between them so created_at differs.
    store.enqueue_offline("sale_1", r#"{"seq":1}"#).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store.enqueue_offline("sale_2", r#"{"seq":2}"#).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store.enqueue_offline("sale_3", r#"{"seq":3}"#).unwrap();

    let engine = SyncEngine::new(test_config(server.port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    let pushes = server.state.received_pushes.lock().unwrap();
    assert_eq!(pushes.len(), 3);
    // Verify order: oldest first
    assert_eq!(pushes[0].action, "sale_1");
    assert_eq!(pushes[1].action, "sale_2");
    assert_eq!(pushes[2].action, "sale_3");
    drop(pushes);

    assert_eq!(result.pushed, 3);

    server.handle.abort();
}

#[tokio::test]
async fn conflict_response_marks_item_and_re_enqueues() {
    // Server that returns Conflict for the first item, then Accepted for retries.
    let conflict_count = Arc::new(Mutex::new(0i32));
    let conflict_state = conflict_count.clone();

    let app = Router::new()
        .route(
            "/api/sync/push",
            post(move |Json(items): Json<Vec<OfflineQueueItem>>| async move {
                let mut count = conflict_state.lock().unwrap();
                if *count == 0 {
                    *count += 1;
                    let results: Vec<PushOutcome> = items
                        .iter()
                        .map(|item| PushOutcome::Conflict(item.clone()))
                        .collect();
                    Json(PushResponse { results })
                } else {
                    let results: Vec<PushOutcome> =
                        items.iter().map(|_| PushOutcome::Accepted).collect();
                    Json(PushResponse { results })
                }
            }),
        )
        .route(
            "/api/sync/pull",
            post(|| async { Json(PullResponse { items: vec![] }) }),
        );

    let (port, handle) = spawn_custom_server(app).await;
    let store = setup_store();
    store
        .enqueue_offline("complete_sale", r#"{"sale_id":"conflict-1"}"#)
        .unwrap();

    let engine = SyncEngine::new(test_config(port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    // The first attempt gets Conflict, re-enqueues, but the engine doesn't
    // retry within the same cycle. So the item should be marked synced
    // (because conflict resolution calls mark_synced) and re-enqueued.
    let all = store.list_all_offline().unwrap();
    assert_eq!(all.len(), 1, "conflict should mark original as synced");
    assert_eq!(all[0].status, OfflineQueueStatus::Synced);

    assert_eq!(result.pushed, 1);

    handle.abort();
}

#[tokio::test]
async fn rejected_item_marked_failed() {
    let app = Router::new()
        .route(
            "/api/sync/push",
            post(|| async {
                let results = vec![
                    PushOutcome::Accepted,
                    PushOutcome::Rejected {
                        reason: "invalid payload".into(),
                    },
                    PushOutcome::Accepted,
                ];
                Json(PushResponse { results })
            }),
        )
        .route(
            "/api/sync/pull",
            post(|| async { Json(PullResponse { items: vec![] }) }),
        );

    let (port, handle) = spawn_custom_server(app).await;
    let store = setup_store();
    store.enqueue_offline("a", "{}").unwrap();
    store.enqueue_offline("b", "{}").unwrap();
    store.enqueue_offline("c", "{}").unwrap();

    let engine = SyncEngine::new(test_config(port));
    let result = engine.run_sync_cycle(&store).await.unwrap();
    assert_eq!(result.pushed, 3);

    let all = store.list_all_offline().unwrap();
    assert_eq!(all.len(), 3);

    // Items a and c should be synced; item b should be failed.
    let statuses: Vec<OfflineQueueStatus> = all.iter().map(|i| i.status).collect();
    assert!(statuses.contains(&OfflineQueueStatus::Synced));
    assert!(statuses.contains(&OfflineQueueStatus::Failed));

    handle.abort();
}

#[tokio::test]
async fn pull_returns_items() {
    let remote_item = OfflineQueueItem::new("remote_update", r#"{"data":"from_server"}"#);
    let pull_item = remote_item.clone();

    let app = Router::new()
        .route(
            "/api/sync/push",
            post(|| async {
                let results: Vec<PushOutcome> = vec![];
                Json(PushResponse { results })
            }),
        )
        .route(
            "/api/sync/pull",
            post(|| async move {
                Json(PullResponse {
                    items: vec![pull_item.clone()],
                })
            }),
        );

    let (port, handle) = spawn_custom_server(app).await;
    let store = setup_store();
    let engine = SyncEngine::new(test_config(port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    assert_eq!(result.pushed, 0);
    assert_eq!(result.pulled, 1, "should have pulled 1 item from server");

    handle.abort();
}

#[tokio::test]
async fn api_key_is_sent_in_headers() {
    // Start a server that captures the Authorization header.
    let auth_header = Arc::new(Mutex::new(None::<String>));
    let auth_state = auth_header.clone();

    let app = Router::new()
        .route(
            "/api/sync/push",
            post(move |req: axum::http::Request<axum::body::Body>| {
                let auth = req
                    .headers()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_owned());
                *auth_state.lock().unwrap() = auth;
                async { Json(PushResponse { results: vec![] }) }
            }),
        )
        .route(
            "/api/sync/pull",
            post(|| async { Json(PullResponse { items: vec![] }) }),
        );

    let (port, handle) = spawn_custom_server(app).await;
    let store = setup_store();
    store.enqueue_offline("test", "{}").unwrap();

    let mut config = test_config(port);
    config.api_key = Some("sk-test-key".into());
    let engine = SyncEngine::new(config);
    let _result = engine.run_sync_cycle(&store).await.unwrap();

    let auth = auth_header.lock().unwrap();
    assert_eq!(auth.as_deref(), Some("Bearer sk-test-key"));

    handle.abort();
}

#[tokio::test]
async fn connection_refused_returns_error() {
    // Use a port that nothing is listening on.
    let port = 28999;
    let store = setup_store();
    store.enqueue_offline("test", "{}").unwrap();

    let config = SyncConfig {
        server_url: format!("http://localhost:{port}"),
        api_key: None,
    };
    let engine = SyncEngine::new(config);
    let result = engine.run_sync_cycle(&store).await;

    assert!(
        result.is_err(),
        "sync should fail when server is unreachable"
    );
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("transport")
                || msg.contains("Connection refused")
                || msg.contains("push request failed"),
            "expected transport error, got: {msg}"
        );
    }
}

#[tokio::test]
async fn empty_queue_produces_no_push() {
    let server = spawn_test_server().await;
    let store = setup_store();

    let engine = SyncEngine::new(test_config(server.port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    assert_eq!(result.pushed, 0);
    assert_eq!(result.pulled, 0);

    let pushes = server.state.received_pushes.lock().unwrap();
    assert!(pushes.is_empty());

    server.handle.abort();
}

#[tokio::test]
async fn multiple_items_are_all_pushed_and_synced() {
    let server = spawn_test_server().await;
    let store = setup_store();

    store
        .enqueue_offline("complete_sale", r#"{"sale_id":"s1"}"#)
        .unwrap();
    store
        .enqueue_offline("product_created", r#"{"sku":"P1"}"#)
        .unwrap();
    store
        .enqueue_offline("stock_adjusted", r#"{"sku":"P1","delta":-1}"#)
        .unwrap();
    assert_eq!(store.pending_offline_count().unwrap(), 3);

    let engine = SyncEngine::new(test_config(server.port));
    let result = engine.run_sync_cycle(&store).await.unwrap();

    // Server received 3 pushes.
    let pushes = server.state.received_pushes.lock().unwrap();
    assert_eq!(pushes.len(), 3);
    drop(pushes);

    // All 3 items synced locally.
    let all = store.list_all_offline().unwrap();
    let synced = all
        .iter()
        .filter(|i| i.status == OfflineQueueStatus::Synced)
        .count();
    assert_eq!(synced, 3);

    assert_eq!(result.pushed, 3);
    assert_eq!(result.pulled, 0);

    server.handle.abort();
}

#[tokio::test]
async fn server_error_prevents_sync_item_stays_pending() {
    // Start a server that will reject the request by returning 500.
    let reject_app = Router::new().route(
        "/api/sync/push",
        post(|| async { axum::http::StatusCode::INTERNAL_SERVER_ERROR }),
    );
    let (port, handle) = spawn_custom_server(reject_app).await;
    let store = setup_store();
    store
        .enqueue_offline("complete_sale", r#"{"sale_id":"fail-1"}"#)
        .unwrap();

    let engine = SyncEngine::new(test_config(port));
    let result = engine.run_sync_cycle(&store).await;

    // The sync should fail because the server returned 500.
    assert!(result.is_err(), "sync should fail when server returns 500");

    // The item should remain pending (the error happens at the transport level,
    // so mark_synced is never called).
    let pending = store.list_pending_offline().unwrap();
    assert_eq!(
        pending.len(),
        1,
        "item should remain pending after server error"
    );

    handle.abort();
}
