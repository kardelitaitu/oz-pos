use super::*;
use oz_core::session::SessionContext;
use oz_core::{CreateKdsOrderInput, Currency, Money, Sale, SaleStatus};
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

// ── Existing tests (preserved) ────────────────────────────────────

#[test]
fn empty_runtime_kds_targets_disable_ticket_creation() {
    let conn = oz_core::migrations::fresh_db();
    let key = format!("{TOPOLOGY_RUNTIME_SETTING_KEY}/store-1");
    oz_core::Settings::set(&conn, &key, r#"{"routes":[]}"#).unwrap();

    let runtime_targets = resolve_runtime_kds_plan(&conn, "store-1")
        .unwrap()
        .map(|plan| runtime_kds_target_instances(&plan, "pos-main"));
    assert_eq!(runtime_targets, Some(Vec::<String>::new()));
    assert!(!should_create_kds_tickets(runtime_targets.as_deref()));
    assert!(should_create_kds_tickets(None));
}

fn test_kds_order(id: &str) -> KdsOrder {
    KdsOrder {
        id: id.into(),
        sale_id: format!("sale-{id}"),
        store_id: Some("store-1".into()),
        target_instance_id: Some("kds-main".into()),
        status: "pending".into(),
        items_summary: "Burger".into(),
        item_count: 1,
        display_number: Some(1),
        received_at: "2026-08-09T12:00:00.000Z".into(),
        started_at: None,
        ready_at: None,
        served_at: None,
        prep_time_seconds: 0,
        kitchen_zone: None,
        notes: String::new(),
        table_number: None,
        priority: false,
    }
}

#[test]
fn runtime_plan_maps_each_kds_target_to_its_hardware() {
    let plan = serde_json::json!({
        "routes": [
            {
                "source_instance_id": "kds-main",
                "target_instance_id": "printer-grill",
                "from_port_id": "ticket-out",
                "to_port_id": "ticket-in",
                "relationship_type": "ticket-routing"
            },
            {
                "source_instance_id": "kds-expediter",
                "target_instance_id": "printer-pass",
                "from_port_id": "ticket-out",
                "to_port_id": "ticket-in",
                "relationship_type": "ticket-routing"
            },
            {
                "source_instance_id": "kds-main",
                "target_instance_id": "printer-grill",
                "from_port_id": "ticket-out",
                "to_port_id": "ticket-in",
                "relationship_type": "ticket-routing"
            }
        ]
    });
    let kds_targets = vec!["kds-main".into(), "kds-expediter".into()];
    assert_eq!(
        runtime_kds_hardware_targets(&plan, &kds_targets),
        vec![
            ("kds-main".into(), "printer-grill".into()),
            ("kds-expediter".into(), "printer-pass".into()),
        ]
    );
    let jobs = build_kds_chit_jobs(&[test_kds_order("order-1")], &kds_targets, &plan);
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].hardware_instance_id, "printer-grill");
    assert_eq!(jobs[1].hardware_instance_id, "printer-pass");
}

#[tokio::test]
async fn target_aware_chit_jobs_print_to_separate_registered_printers() {
    let registry = oz_hal::DriverRegistry::default();
    let grill = Arc::new(oz_hal::drivers::mock::MockReceiptPrinter::new());
    let pass = Arc::new(oz_hal::drivers::mock::MockReceiptPrinter::new());
    registry
        .register_printer("printer-grill", grill.clone())
        .await;
    registry
        .register_printer("printer-pass", pass.clone())
        .await;
    let plan = serde_json::json!({
        "routes": [
            {"source_instance_id":"kds-main","target_instance_id":"printer-grill","from_port_id":"ticket-out","to_port_id":"ticket-in","relationship_type":"ticket-routing"},
            {"source_instance_id":"kds-expediter","target_instance_id":"printer-pass","from_port_id":"ticket-out","to_port_id":"ticket-in","relationship_type":"ticket-routing"}
        ]
    });
    let orders = vec![test_kds_order("order-1")];
    let kds_targets = vec!["kds-main".into(), "kds-expediter".into()];

    try_auto_print_kds_chit_jobs(&orders, &kds_targets, &plan, &registry, None).await;

    assert_eq!(grill.printed_raw.lock().unwrap().len(), 1);
    assert_eq!(pass.printed_raw.lock().unwrap().len(), 1);
}

#[test]
fn runtime_plan_selects_all_kds_targets_for_pos_source() {
    let plan = serde_json::json!({
        "routes": [
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "kds-main",
                "from_port_id": "operation-out",
                "to_port_id": "operation-in",
                "relationship_type": "generic"
            },
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "kds-expediter",
                "from_port_id": "operation-out",
                "to_port_id": "operation-in",
                "relationship_type": "generic"
            },
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "kds-main",
                "from_port_id": "operation-out",
                "to_port_id": "operation-in",
                "relationship_type": "generic"
            }
        ]
    });
    assert_eq!(
        runtime_kds_target_instances(&plan, "pos-main"),
        vec!["kds-main", "kds-expediter"]
    );
    assert!(runtime_kds_target_instances(&plan, "other-pos").is_empty());
}

// ── Helpers ───────────────────────────────────────────────────────

fn seed_owner(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
}

fn scoped_state(
    conn: rusqlite::Connection,
    token: &str,
    user_id: &str,
    role_id: &str,
    store_id: &str,
    instance_id: &str,
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
            instance_id.into(),
            "pos".into(),
            None,
            0,
        ),
    );
    state
}

fn create_sale_in_store(state: &AppState, sale_id: &str) {
    let store_db = state.db_manager.open_store("s1").unwrap();
    let db = store_db.lock().unwrap();
    let s = Store::new(&db);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let usd: Currency = "USD".parse().unwrap();
    let zero = Money {
        minor_units: 0,
        currency: usd.clone(),
    };
    let sale = Sale {
        id: sale_id.into(),
        status: SaleStatus::Pending,
        total: zero.clone(),
        line_count: 0,
        currency: usd,
        payment_method: None,
        tendered_minor: None,
        user_id: Some("user-owner".into()),
        created_at: now.clone(),
        updated_at: now,
        lines: Vec::new(),
        discount_percent: 0,
        discount_label: None,
        subtotal: zero.clone(),
        tax_total: zero,
        customer_id: None,
        version: 1,
    };
    s.create_sale(&sale).unwrap();
}

fn create_kds_order_in_store(state: &AppState, order: &KdsOrder) -> KdsOrder {
    // First create the FK sale in the same store-DB.
    create_sale_in_store(state, &order.sale_id);
    let store_db = state.db_manager.open_store("s1").unwrap();
    let db = store_db.lock().unwrap();
    let s = Store::new(&db);
    let input = CreateKdsOrderInput {
        sale_id: order.sale_id.clone(),
        store_id: order.store_id.clone(),
        items_summary: order.items_summary.clone(),
        item_count: order.item_count,
        kitchen_zone: order.kitchen_zone.clone(),
        notes: order.notes.clone(),
        table_number: order.table_number.clone(),
        priority: order.priority,
    };
    s.create_kds_order_routed(input, order.target_instance_id.as_deref())
        .unwrap()
}

// ── Session validation ────────────────────────────────────────────

#[tokio::test]
async fn scoped_list_kds_orders_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_kds_orders_scoped("bad-token".into(), None, app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_get_kds_queue_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = get_kds_queue_scoped("bad-token".into(), None, app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_get_kds_order_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = get_kds_order_scoped("bad-token".into(), "order-1".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

// ── CRUD operations ───────────────────────────────────────────────

#[tokio::test]
async fn owner_can_list_kds_orders_empty() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let orders = list_kds_orders_scoped("tok".into(), None, app.state())
        .await
        .unwrap();
    assert!(orders.is_empty());
}

#[tokio::test]
async fn owner_can_list_kds_orders_with_data() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    let order = test_kds_order("o1");
    let created = create_kds_order_in_store(&state, &order);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let orders = list_kds_orders_scoped("tok".into(), None, app.state())
        .await
        .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].id, created.id);
}

#[tokio::test]
async fn list_kds_orders_filters_by_status() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    // Create two orders (both start as "pending" per DB default).
    let order1 = create_kds_order_in_store(&state, &test_kds_order("o1"));
    let order2 = create_kds_order_in_store(&state, &test_kds_order("o2"));

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Move order2 to "ready" via the status update command.
    update_kds_status_scoped("tok".into(), order2.id.clone(), "ready".into(), app.state())
        .await
        .unwrap();

    // Now filter by status.
    let pending_orders = list_kds_orders_scoped("tok".into(), Some("pending".into()), app.state())
        .await
        .unwrap();
    assert_eq!(pending_orders.len(), 1);
    assert_eq!(pending_orders[0].id, order1.id);

    let ready_orders = list_kds_orders_scoped("tok".into(), Some("ready".into()), app.state())
        .await
        .unwrap();
    assert_eq!(ready_orders.len(), 1);
    assert_eq!(ready_orders[0].id, order2.id);
}

#[tokio::test]
async fn owner_can_get_kds_order_by_id() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    let order = test_kds_order("o1");
    let created = create_kds_order_in_store(&state, &order);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let fetched = get_kds_order_scoped("tok".into(), created.id.clone(), app.state()).await;
    assert!(fetched.is_ok());
    assert!(fetched.unwrap().is_some());
}

#[tokio::test]
async fn get_kds_order_returns_none_for_unknown() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = get_kds_order_scoped("tok".into(), "nonexistent".into(), app.state())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn owner_can_get_kds_queue() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    let order = test_kds_order("o1");
    let created = create_kds_order_in_store(&state, &order);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let queue = get_kds_queue_scoped("tok".into(), None, app.state())
        .await
        .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, created.id);
}

#[tokio::test]
async fn owner_can_update_kds_status() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    let order = test_kds_order("o1");
    let created = create_kds_order_in_store(&state, &order);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let updated = update_kds_status_scoped(
        "tok".into(),
        created.id.clone(),
        "preparing".into(),
        app.state(),
    )
    .await;
    if let Err(ref e) = updated {
        eprintln!("update_kds_status error: {e:?}");
    }
    assert!(updated.is_ok(), "owner should update KDS status");
    assert_eq!(updated.unwrap().status, "preparing");
}

#[tokio::test]
async fn update_kds_status_returns_error_for_unknown_order() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = update_kds_status_scoped(
        "tok".into(),
        "nonexistent".into(),
        "ready".into(),
        app.state(),
    )
    .await;
    assert!(result.is_err());
}

// ── Instance isolation ────────────────────────────────────────────

#[tokio::test]
async fn kds_orders_scoped_to_instance() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1", "kds-main");

    // Order for kds-main — should be visible.
    let mut order_main = test_kds_order("o1");
    order_main.target_instance_id = Some("kds-main".into());
    let created_main = create_kds_order_in_store(&state, &order_main);

    // Order for kds-expediter — should NOT be visible.
    let mut order_exp = test_kds_order("o2");
    order_exp.target_instance_id = Some("kds-expediter".into());
    create_kds_order_in_store(&state, &order_exp);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let orders = list_kds_orders_scoped("tok".into(), None, app.state())
        .await
        .unwrap();
    assert_eq!(orders.len(), 1, "only kds-main orders should be visible");
    assert_eq!(orders[0].id, created_main.id);
}

// ── Staff permission tests ────────────────────────────────────────

#[tokio::test]
async fn staff_can_list_kds_orders() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1", "kds-main");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_kds_orders_scoped("tok".into(), None, app.state()).await;
    assert!(result.is_ok(), "staff has KDS_VIEW permission");
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn staff_can_update_kds_status() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1", "kds-main");

    let order = test_kds_order("o1");
    let created = create_kds_order_in_store(&state, &order);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = update_kds_status_scoped(
        "tok".into(),
        created.id.clone(),
        "preparing".into(),
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "staff has KDS_UPDATE permission");
}
