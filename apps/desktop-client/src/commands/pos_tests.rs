use super::*;
use oz_core::Currency;
use tauri::Manager as _;

fn usd() -> Currency {
    "USD".parse().unwrap()
}

fn price(minor: i64) -> Money {
    Money {
        minor_units: minor,
        currency: usd(),
    }
}

#[test]
fn start_cart_add_line() {
    let mut cart = oz_core::Cart::new(usd());
    let cart_id = cart.id();

    let line = CartLine::new(Sku::new("COFFEE"), 2, price(350));
    cart.add_line(line).unwrap();

    assert_eq!(cart.line_count(), 1);
    let total = cart.total();
    assert_eq!(total.unwrap().minor_units, 700);
    assert_eq!(total.unwrap().currency, usd());
    assert!(!cart_id.to_string().is_empty());

    let line2 = CartLine::new(Sku::new("BAGEL"), 1, price(450));
    cart.add_line(line2).unwrap();
    assert_eq!(cart.line_count(), 2);
    assert_eq!(cart.total().unwrap().minor_units, 1150);
}

#[test]
fn cart_total_with_fractional_qty() {
    let mut cart = oz_core::Cart::new(usd());
    let line = CartLine::new(Sku::new("TEA"), 3, price(200));
    let line_total = line.total().unwrap();
    cart.add_line(line).unwrap();
    assert_eq!(line_total.minor_units, 600);
    assert_eq!(cart.total().unwrap().minor_units, 600);
}

// ── DTO struct tests ─────────────────────────────────────────────

#[test]
fn set_cart_discount_args_debug() {
    let args = SetCartDiscountArgs {
        cart_id: CartId::new(),
        percent: 10,
        label: Some("Senior".into()),
        user_id: "user-1".into(),
    };
    let debug = format!("{args:?}");
    assert!(debug.contains("Senior"));
    assert!(debug.contains("10"));
}

#[test]
fn start_sale_args_default_currency() {
    let json = r#"{}"#;
    let args: StartSaleArgs = serde_json::from_str(json).unwrap();
    assert!(args.currency.is_empty());
}

#[test]
fn start_sale_result_debug() {
    let cart_id = CartId::new();
    let result = StartSaleResult {
        cart_id,
        deduction_location_id: None,
    };
    let debug = format!("{result:?}");
    assert!(debug.contains("StartSaleResult"));
}

#[test]
fn add_line_args_fields() {
    let args = AddLineArgs {
        cart_id: CartId::new(),
        sku: Sku::new("COFFEE"),
        qty: 3,
        unit_price_minor: 350,
    };
    assert_eq!(args.qty, 3);
    assert_eq!(args.unit_price_minor, 350);
    assert_eq!(args.sku.as_str(), "COFFEE");
}

#[test]
fn serial_number_arg_fields() {
    let arg = SerialNumberArg {
        sku: "LAPTOP".into(),
        serial: "SN12345".into(),
    };
    assert_eq!(arg.sku, "LAPTOP");
    assert_eq!(arg.serial, "SN12345");
}

#[test]
fn hold_cart_args_default_bill_type() {
    let json =
        r#"{"label":"Test","cartData":"{}","itemCount":1,"totalMinor":100,"currency":"USD"}"#;
    let args: HoldCartArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.bill_type, "hold");
}

#[test]
fn complete_sale_result_debug() {
    let result = CompleteSaleResult {
        sale_id: "sale-1".into(),
        total: Some(price(1000)),
        line_count: 2,
    };
    let debug = format!("{result:?}");
    assert!(debug.contains("sale-1"));
    assert!(debug.contains("1000"));
}

// ── Serde regression: all DTOs accept camelCase from JS ────────

#[test]
fn add_line_args_from_camel_case_json() {
    let json = r#"{"cartId":"11111111-1111-1111-1111-111111111111","sku":"BAGEL","qty":2,"unitPriceMinor":500}"#;
    let args: AddLineArgs = serde_json::from_str(json).unwrap();
    assert_eq!(
        args.cart_id.to_string(),
        "11111111-1111-1111-1111-111111111111"
    );
    assert_eq!(args.sku.as_str(), "BAGEL");
    assert_eq!(args.qty, 2);
    assert_eq!(args.unit_price_minor, 500);
}

#[test]
fn complete_sale_args_from_camel_case_json() {
    let json = r#"{"cartId":"22222222-2222-2222-2222-222222222222","paymentMethod":"cash","tenderedMinor":50000,"userId":"user-1"}"#;
    let args: CompleteSaleArgs = serde_json::from_str(json).unwrap();
    assert_eq!(
        args.cart_id.to_string(),
        "22222222-2222-2222-2222-222222222222"
    );
    assert_eq!(args.payment_method, "cash");
    assert_eq!(args.tendered_minor, Some(50000));
    assert_eq!(args.user_id, "user-1");
}

#[test]
fn hold_cart_args_from_camel_case_json() {
    let json = r#"{"label":"Table 5","cartData":"{}","itemCount":3,"totalMinor":15000,"currency":"IDR","customerName":"Budi"}"#;
    let args: HoldCartArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.label, "Table 5");
    assert_eq!(args.item_count, 3);
    assert_eq!(args.total_minor, 15000);
    assert_eq!(args.currency, "IDR");
    assert_eq!(args.customer_name.as_deref(), Some("Budi"));
    assert_eq!(args.bill_type, "hold");
}

// ── Scoped command token rejection tests ───────────────────────

#[test]
fn pos_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[test]
fn complete_sale_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("bad-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_sale_deducts_from_topology_warehouse_not_pos_location() {
    use oz_core::migrations;
    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;

    let store_id = "store-stock-route-e2e";
    let pos_instance_id = "pos-stock-route-e2e";
    let warehouse_instance_id = "warehouse-stock-route-e2e";
    let global = migrations::fresh_db();
    let runtime_key = format!("{TOPOLOGY_RUNTIME_SETTING_KEY}/{store_id}");
    let runtime_plan = serde_json::json!({
        "routes": [{
            "source_instance_id": pos_instance_id,
            "target_instance_id": warehouse_instance_id,
            "from_port_id": "stock-out",
            "to_port_id": "stock-in",
            "relationship_type": "stock-routing"
        }]
    });
    oz_core::Settings::set(&global, &runtime_key, &runtime_plan.to_string()).unwrap();
    {
        let identity_store = Store::new(&global);
        identity_store.seed_default_roles().unwrap();
        global.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('stock-route-user', 'stock-route-user', 'hash', 'Stock Route User', 'role-owner', 1, '2026-08-09T00:00:00Z', '2026-08-09T00:00:00Z')",
            [],
        )
        .unwrap();
    }

    let temp_dir = tempfile::tempdir().unwrap();
    let manager = StoreDatabaseManager::new(temp_dir.path().to_path_buf(), migrations::ALL);
    let store_conn = manager.open_store(store_id).unwrap();
    {
        let db = store_conn.lock().unwrap();
        db.execute_batch(
            "INSERT OR IGNORE INTO store_profiles (id, name, is_primary) VALUES ('store-stock-route-e2e', 'Stock Route E2E', 0);
             INSERT INTO inventory_locations (id, name, type) VALUES
                ('stock-route-pos-location', 'Stock Route POS', 'store'),
                ('stock-route-warehouse-location', 'Stock Route Warehouse', 'warehouse');
             INSERT INTO workspace_instances (id, type_key, store_id, name, bound_location_id)
                VALUES ('pos-stock-route-e2e', 'restaurant-pos', 'store-stock-route-e2e', 'Route POS', 'stock-route-pos-location');
             INSERT INTO workspace_instances (id, type_key, store_id, name, bound_location_id)
                VALUES ('warehouse-stock-route-e2e', 'warehouse', 'store-stock-route-e2e', 'Route Warehouse', 'stock-route-warehouse-location');
             INSERT INTO products (id, sku, name, price_minor, currency, product_type)
                VALUES ('stock-route-product', 'STOCK-ROUTE-COFFEE', 'Stock Route Coffee', 1000, 'USD', 'retail');
             INSERT INTO stock_summary (item_id, location_id, qty)
                VALUES ('stock-route-product', 'stock-route-pos-location', 20),
                       ('stock-route-product', 'stock-route-warehouse-location', 20);",
        )
        .unwrap();
    }

    let mut state = AppState::for_test_with_conn(global);
    state.db_manager = manager;
    state.session_store.write().unwrap().insert(
        "stock-route-token".into(),
        SessionContext::new(
            "stock-route-user".into(),
            "role-owner".into(),
            "stock-route-terminal".into(),
            store_id.into(),
            pos_instance_id.into(),
            "restaurant-pos".into(),
            None,
            0,
        ),
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let started = start_sale_scoped(
        "stock-route-token".into(),
        StartSaleArgs {
            currency: "USD".into(),
        },
        app.state(),
    )
    .await
    .unwrap();
    add_line_scoped(
        "stock-route-token".into(),
        AddLineArgs {
            cart_id: started.cart_id,
            sku: Sku::new("STOCK-ROUTE-COFFEE"),
            qty: 3,
            unit_price_minor: 1000,
        },
        app.state(),
    )
    .await
    .unwrap();
    complete_sale_scoped(
        "stock-route-token".into(),
        CompleteSaleScopedArgs {
            cart_id: started.cart_id,
            payment_method: "cash".into(),
            tendered_minor: Some(3000),
            customer_id: None,
            payment_splits: None,
            customer_name: None,
            serial_numbers: None,
            base_currency: None,
            base_total_minor: None,
            tender_rate_millionths: None,
            tip_minor: None,
            service_charge_minor: None,
        },
        app.state(),
    )
    .await
    .unwrap();

    let state = app.state::<AppState>();
    let store_conn = state.db_manager.open_store(store_id).unwrap();
    let db = store_conn.lock().unwrap();
    let pos_qty: i64 = db
        .query_row(
            "SELECT qty FROM stock_summary WHERE item_id = 'stock-route-product' AND location_id = 'stock-route-pos-location'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let warehouse_qty: i64 = db
        .query_row(
            "SELECT qty FROM stock_summary WHERE item_id = 'stock-route-product' AND location_id = 'stock-route-warehouse-location'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pos_qty, 20, "POS stock must remain untouched by the route");
    assert_eq!(
        warehouse_qty, 17,
        "Warehouse stock must fund the completed sale"
    );
}

#[test]
fn runtime_plan_selects_stock_target_for_pos_source() {
    let plan = serde_json::json!({
        "routes": [{
            "source_instance_id": "pos-main",
            "target_instance_id": "warehouse-main",
            "from_port_id": "stock-out",
            "to_port_id": "stock-in",
            "relationship_type": "stock-routing"
        }]
    });
    assert_eq!(
        runtime_stock_target_instances(&plan, "pos-main"),
        vec!["warehouse-main"]
    );
    assert!(runtime_stock_target_instances(&plan, "other-pos").is_empty());
}

#[test]
fn runtime_plan_uses_retail_operation_route_for_warehouse_stock_target() {
    let plan = serde_json::json!({
        "routes": [{
            "source_instance_id": "pos-main",
            "target_instance_id": "warehouse-main",
            "from_port_id": "operation-out",
            "to_port_id": "operation-in",
            "relationship_type": "generic",
            "target_node_kind": "warehouse"
        }]
    });
    assert_eq!(
        runtime_stock_target_instances(&plan, "pos-main"),
        vec!["warehouse-main"]
    );
}

#[test]
fn runtime_plan_does_not_treat_operation_feed_to_kds_as_stock() {
    let plan = serde_json::json!({
        "routes": [{
            "source_instance_id": "restaurant-pos",
            "target_instance_id": "kds-main",
            "from_port_id": "operation-out",
            "to_port_id": "operation-in",
            "relationship_type": "generic",
            "target_node_kind": "workspace"
        }]
    });
    assert!(runtime_stock_target_instances(&plan, "restaurant-pos").is_empty());
}

#[test]
fn runtime_plan_preserves_distinct_stock_targets_in_route_order() {
    let plan = serde_json::json!({
        "routes": [
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "warehouse-b",
                "from_port_id": "stock-out",
                "to_port_id": "stock-in",
                "relationship_type": "stock-routing"
            },
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "warehouse-a",
                "from_port_id": "stock-out",
                "to_port_id": "stock-in",
                "relationship_type": "stock-routing"
            },
            {
                "source_instance_id": "pos-main",
                "target_instance_id": "warehouse-b",
                "from_port_id": "stock-out",
                "to_port_id": "stock-in",
                "relationship_type": "stock-routing"
            }
        ]
    });
    assert_eq!(
        runtime_stock_target_instances(&plan, "pos-main"),
        vec!["warehouse-b", "warehouse-a"]
    );
}

// ── Scoped command integration tests ─────────────────────────────

use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;

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

// ── Session validation ────────────────────────────────────────────

#[tokio::test]
async fn scoped_hold_cart_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = hold_cart_scoped(
        "bad-token".into(),
        HoldCartArgs {
            label: "Test".into(),
            cart_data: "{}".into(),
            item_count: 1,
            total_minor: 500,
            currency: "USD".into(),
            bill_type: "regular".into(),
            customer_name: None,
            deduction_location_id: None,
        },
        app.state(),
    )
    .await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_list_held_carts_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_held_carts_scoped("bad-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

// ── Owner hold_cart CRUD ─────────────────────────────────────────

#[tokio::test]
async fn owner_can_hold_cart() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = hold_cart_scoped(
        "tok".into(),
        HoldCartArgs {
            label: "Table 5".into(),
            cart_data: r#"{"lines":[]}"#.into(),
            item_count: 2,
            total_minor: 1500,
            currency: "USD".into(),
            bill_type: "regular".into(),
            customer_name: None,
            deduction_location_id: None,
        },
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "owner should hold a cart");
}

#[tokio::test]
async fn owner_can_list_held_carts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Hold two carts.
    for i in 0..2 {
        hold_cart_scoped(
            "tok".into(),
            HoldCartArgs {
                label: format!("Table {i}"),
                cart_data: "{}".into(),
                item_count: 1,
                total_minor: 500,
                currency: "USD".into(),
                bill_type: "regular".into(),
                customer_name: None,
                deduction_location_id: None,
            },
            app.state(),
        )
        .await
        .unwrap();
    }

    let carts = list_held_carts_scoped("tok".into(), app.state())
        .await
        .unwrap();
    assert_eq!(carts.len(), 2);
}

#[tokio::test]
async fn list_held_carts_empty_when_none() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let carts = list_held_carts_scoped("tok".into(), app.state())
        .await
        .unwrap();
    assert!(carts.is_empty());
}

// ── Owner open_bills ─────────────────────────────────────────────

#[tokio::test]
async fn owner_can_list_open_bills_empty() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let bills = list_open_bills_scoped("tok".into(), app.state())
        .await
        .unwrap();
    assert!(bills.is_empty());
}

// ── Permission matrix: staff (has SALES_PROCESS) ─────────────────

#[tokio::test]
async fn staff_can_hold_cart() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = hold_cart_scoped(
        "tok".into(),
        HoldCartArgs {
            label: "Staff hold".into(),
            cart_data: "{}".into(),
            item_count: 1,
            total_minor: 500,
            currency: "USD".into(),
            bill_type: "regular".into(),
            customer_name: None,
            deduction_location_id: None,
        },
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "staff has SALES_PROCESS permission");
}

#[tokio::test]
async fn staff_can_list_held_carts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_held_carts_scoped("tok".into(), app.state()).await;
    assert!(result.is_ok(), "staff has SALES_PROCESS permission");
    assert!(result.unwrap().is_empty());
}
