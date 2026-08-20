use super::*;
use crate::migrations;
use crate::{Cart, CartLine, Money, Sale, Sku};
use rusqlite::Connection;

fn fresh() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn usd() -> crate::Currency {
    "USD".parse().unwrap()
}

fn seed_terminal(conn: &Connection, id: &str, name: &str, device_id: &str) {
    conn.execute(
        "INSERT INTO terminals (id, name, device_id, is_active) VALUES (?1, ?2, ?3, 1)",
        rusqlite::params![id, name, device_id],
    )
    .unwrap();
}

fn price(minor: i64) -> Money {
    Money {
        minor_units: minor,
        currency: usd(),
    }
}

fn seed_product(conn: &Connection, sku: &str, name: &str) {
    let s = store(conn);
    s.create_product(sku, name, price(500), None, None, 100, Some("restaurant"))
        .unwrap();
}

#[test]
fn create_and_get_kds_order() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "COFFEE", "Coffee");

    // Create a minimal sale.
    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id.clone(),
            store_id: None,
            items_summary: "Coffee x2, Bagel".into(),
            item_count: 3,
            kitchen_zone: None,
            notes: "No onions".into(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    assert_eq!(order.sale_id, sale_id);
    assert_eq!(order.items_summary, "Coffee x2, Bagel");
    assert_eq!(order.item_count, 3);
    assert_eq!(order.notes, "No onions");
    assert_eq!(order.status, "pending");
    assert!(order.display_number.is_some());
    assert!(order.display_number.unwrap() >= 1);

    let fetched = s.get_kds_order(&order.id).unwrap().unwrap();
    assert_eq!(fetched.id, order.id);
}

#[test]
fn get_kds_order_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.get_kds_order("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn get_kds_order_by_sale() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "TEA", "Tea");

    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id.clone(),
            store_id: None,
            items_summary: "Tea".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let by_sale = s.get_kds_order_by_sale(&sale_id).unwrap().unwrap();
    assert_eq!(by_sale.id, order.id);
}

#[test]
fn update_kds_status_sets_timestamps() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id,
            store_id: None,
            items_summary: "Test".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    // Pending -> Preparing
    let updated = s.update_kds_status(&order.id, "preparing").unwrap();
    assert_eq!(updated.status, "preparing");
    assert!(updated.started_at.is_some());

    // Preparing -> Ready
    let updated = s.update_kds_status(&order.id, "ready").unwrap();
    assert_eq!(updated.status, "ready");
    assert!(updated.ready_at.is_some());

    // Ready -> Served
    let updated = s.update_kds_status(&order.id, "served").unwrap();
    assert_eq!(updated.status, "served");
    assert!(updated.served_at.is_some());
}

#[test]
fn update_kds_status_invalid() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id,
            store_id: None,
            items_summary: "Test".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let err = s.update_kds_status(&order.id, "bogus").unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
}

#[test]
fn list_kds_orders_empty() {
    let conn = fresh();
    let s = store(&conn);
    let orders = s.list_kds_orders(None).unwrap();
    assert!(orders.is_empty());
}

#[test]
fn list_kds_orders_with_status_filter() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id1 = uuid::Uuid::now_v7().to_string();
    let sale_id2 = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    for sid in [&sale_id1, &sale_id2] {
        let test_sale = Sale {
            id: sid.to_string(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
    }

    s.create_kds_order(CreateKdsOrderInput {
        sale_id: sale_id1,
        store_id: None,
        items_summary: "Order 1".into(),
        item_count: 1,
        kitchen_zone: None,
        notes: String::new(),
        table_number: None,
        priority: false,
    })
    .unwrap();

    s.create_kds_order(CreateKdsOrderInput {
        sale_id: sale_id2,
        store_id: None,
        items_summary: "Order 2".into(),
        item_count: 2,
        kitchen_zone: None,
        notes: String::new(),
        table_number: None,
        priority: false,
    })
    .unwrap();

    let all = s.list_kds_orders(None).unwrap();
    assert_eq!(all.len(), 2);

    let pending = s.list_kds_orders(Some("pending")).unwrap();
    assert_eq!(pending.len(), 2);

    let ready = s.list_kds_orders(Some("ready")).unwrap();
    assert_eq!(ready.len(), 0);
}

#[test]
fn get_kds_queue_returns_pending_and_preparing() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id1 = uuid::Uuid::now_v7().to_string();
    let sale_id2 = uuid::Uuid::now_v7().to_string();
    let sale_id3 = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    for sid in [&sale_id1, &sale_id2, &sale_id3] {
        let test_sale = Sale {
            id: sid.to_string(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
    }

    let _o1 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id1,
            store_id: None,
            items_summary: "Pending".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let o2 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id2,
            store_id: None,
            items_summary: "Preparing".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let o3 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id3,
            store_id: None,
            items_summary: "Served".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    s.update_kds_status(&o2.id, "preparing").unwrap();
    s.update_kds_status(&o3.id, "served").unwrap();

    let queue = s.get_kds_queue(None).unwrap();
    // Queue should include pending + preparing + ready (but not served/cancelled).
    assert_eq!(queue.len(), 2);
    assert!(
        queue
            .iter()
            .all(|o| o.status == "pending" || o.status == "preparing" || o.status == "ready")
    );
}

#[test]
fn complete_sale_to_kds_creates_order() {
    let conn = fresh();
    let s = store(&conn);

    seed_product(&conn, "COFFEE", "Fresh Coffee");
    seed_product(&conn, "BAGEL", "Everything Bagel");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
        .unwrap();
    cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
        .unwrap();

    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
    assert_eq!(orders.len(), 1);
    let order = &orders[0];
    assert_eq!(order.sale_id, sale.id);
    assert_eq!(order.status, "pending");
    assert!(order.items_summary.contains("Coffee"));
    assert!(order.items_summary.contains("Bagel"));
    assert_eq!(order.item_count, 3);
}

#[test]
fn display_number_increments_per_day() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id1 = uuid::Uuid::now_v7().to_string();
    let sale_id2 = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    for sid in [&sale_id1, &sale_id2] {
        let test_sale = Sale {
            id: sid.to_string(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
    }

    let o1 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id1,
            store_id: None,
            items_summary: "First".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let o2 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id2,
            store_id: None,
            items_summary: "Second".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    assert_eq!(o1.display_number, Some(1));
    assert_eq!(o2.display_number, Some(2));
}

// ── CHECK constraint tests ──────────────────────────────────────

#[test]
fn kds_status_check_rejects_invalid_status_on_insert() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    // Attempt a raw INSERT with an invalid status — should fail the CHECK constraint.
    let id = uuid::Uuid::now_v7().to_string();
    let result = s.conn.execute(
        "INSERT INTO kds_orders (id, sale_id, status, items_summary, item_count, notes)
         VALUES (?1, ?2, 'bogus', 'Test', 1, '')",
        params![id, sale_id],
    );

    assert!(
        result.is_err(),
        "expected CHECK constraint error for invalid status"
    );
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("CHECK") || msg.contains("constraint") || msg.contains("abort"),
        "expected constraint violation message, got: {msg}"
    );
}

#[test]
fn kds_status_check_accepts_valid_statuses() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Insert orders with each valid status. Each needs its own sale_id
    // because kds_orders.sale_id has a UNIQUE constraint.
    for status in &["pending", "preparing", "ready", "served", "cancelled"] {
        let sale_id = uuid::Uuid::now_v7().to_string();
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        let order_id = uuid::Uuid::now_v7().to_string();
        s.conn
            .execute(
                "INSERT INTO kds_orders (id, sale_id, status, items_summary, item_count, notes)
             VALUES (?1, ?2, ?3, 'Test', 1, '')",
                params![order_id, sale_id, status],
            )
            .unwrap();
        let fetched = s.get_kds_order(&order_id).unwrap().unwrap();
        assert_eq!(fetched.status, *status);
    }
}

// ── Additional edge cases ─────────────────────────────────────

#[test]
fn update_kds_status_nonexistent_order_fails() {
    let conn = fresh();
    let s = store(&conn);
    let err = s.update_kds_status("no-such-order", "pending").unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));
}

#[test]
fn get_kds_queue_excludes_served_and_cancelled() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Create 4 orders with different statuses.
    let mut ids = Vec::new();
    for st in &["pending", "preparing", "served", "cancelled"] {
        let sale_id = uuid::Uuid::now_v7().to_string();
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
        let order = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id,
                store_id: None,
                items_summary: st.to_string(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
                table_number: None,
                priority: false,
            })
            .unwrap();
        if *st != "pending" {
            s.update_kds_status(&order.id, st).unwrap();
        }
        ids.push(order.id);
    }

    let queue = s.get_kds_queue(None).unwrap();
    assert_eq!(queue.len(), 2, "should only have pending + preparing");
    assert!(queue.iter().any(|o| o.status == "pending"));
    assert!(queue.iter().any(|o| o.status == "preparing"));
}

#[test]
fn get_kds_queue_with_zone_filter() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    for zone in &["grill", "salad"] {
        let sale_id = uuid::Uuid::now_v7().to_string();
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
        s.create_kds_order(CreateKdsOrderInput {
            sale_id,
            store_id: None,
            items_summary: format!("Order in {zone}"),
            item_count: 1,
            kitchen_zone: Some(zone.to_string()),
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();
    }

    let grill = s.get_kds_queue(Some("grill")).unwrap();
    assert_eq!(grill.len(), 1);
    assert!(grill[0].items_summary.contains("grill"));

    let salad = s.get_kds_queue(Some("salad")).unwrap();
    assert_eq!(salad.len(), 1);
    assert!(salad[0].items_summary.contains("salad"));
}

#[test]
fn get_kds_queue_empty_zone_returns_unzoned_orders() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // One order with zone, one without.
    for (suffix, zone) in &[("zoned", Some("grill")), ("unzoned", None)] {
        let sale_id = uuid::Uuid::now_v7().to_string();
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();
        s.create_kds_order(CreateKdsOrderInput {
            sale_id,
            store_id: None,
            items_summary: format!("Order {suffix}"),
            item_count: 1,
            kitchen_zone: zone.map(|z| z.to_string()),
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();
    }

    let unzoned = s.get_kds_queue(Some("")).unwrap();
    assert_eq!(unzoned.len(), 1);
    assert!(unzoned[0].items_summary.contains("unzoned"));
}

#[test]
fn complete_sale_to_kds_no_restaurant_lines_returns_empty() {
    let conn = fresh();
    let s = store(&conn);

    // Seed a retail-type product.
    s.create_product(
        "RETAIL-1",
        "Retail Item",
        price(500),
        None,
        None,
        100,
        Some("retail"),
    )
    .unwrap();

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("RETAIL-1"), 1, price(500)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
    assert!(orders.is_empty(), "no KDS orders for retail-only sale");
}

fn seed_product_with_zone(conn: &Connection, sku: &str, name: &str, zone: &str) {
    let s = store(conn);
    s.create_product(sku, name, price(500), None, None, 100, Some("restaurant"))
        .unwrap();
    // Set kitchen_zone directly via SQL (not exposed on create_product API).
    conn.execute(
        "UPDATE products SET kitchen_zone = ?1 WHERE sku = ?2",
        params![zone, sku],
    )
    .unwrap();
}

#[test]
fn complete_sale_to_kds_groups_same_zone_items() {
    let conn = fresh();
    let s = store(&conn);

    // Seed products in the SAME zone (schema has UNIQUE constraint on sale_id).
    seed_product_with_zone(&conn, "STEAK", "Steak", "grill");
    seed_product_with_zone(&conn, "BURGER", "Burger", "grill");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("STEAK"), 2, price(500)))
        .unwrap();
    cart.add_line(CartLine::new(Sku::new("BURGER"), 3, price(300)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
    // One order because both products are in the same zone.
    assert_eq!(orders.len(), 1, "same zone items grouped into one order");
    let order = &orders[0];
    assert_eq!(order.kitchen_zone.as_deref(), Some("grill"));
    assert_eq!(order.item_count, 5);
    assert!(order.items_summary.contains("Steak"));
    assert!(order.items_summary.contains("Burger"));
}

#[test]
fn complete_sale_to_kds_with_store_id() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "BURGER", "Burger");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("BURGER"), 1, price(500)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let orders = s
        .complete_sale_to_kds_routed(&sale.id, Some("store-1"), Some("kds-main"))
        .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].store_id, Some("store-1".to_string()));
    assert_eq!(orders[0].target_instance_id, Some("kds-main".to_string()));
}

#[test]
fn complete_sale_to_kds_fanout_targets_one_order_to_multiple_instances() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "BURGER", "Burger");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("BURGER"), 1, price(500)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let targets = vec!["kds-main".to_owned(), "kds-expediter".to_owned()];
    let orders = s
        .complete_sale_to_kds_fanout(&sale.id, Some("store-1"), &targets)
        .unwrap();
    assert_eq!(orders.len(), 1, "fan-out must not duplicate the sale order");
    assert_eq!(orders[0].target_instance_id.as_deref(), Some("kds-main"));

    let target_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM kds_order_targets WHERE kds_order_id = ?1",
            params![orders[0].id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(target_count, 2);
    assert_eq!(
        s.get_kds_queue_for_instance(None, "kds-main")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        s.get_kds_queue_for_instance(None, "kds-expediter")
            .unwrap()
            .len(),
        1
    );
    assert!(
        s.get_kds_queue_for_instance(None, "kds-other")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn scoped_kds_commands_reject_cross_instance_targeted_order() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "BURGER", "Burger");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("BURGER"), 1, price(500)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    let order = s
        .complete_sale_to_kds_fanout(&sale.id, Some("store-1"), &["kds-main".to_owned()])
        .unwrap()
        .remove(0);
    let line = s
        .create_kds_line_items(
            &order.id,
            &[CreateKdsLineItemInput {
                sku: "BURGER".into(),
                display_name: "Burger".into(),
                qty: 1,
                course: Some("main".into()),
                modifiers: vec![],
            }],
        )
        .unwrap()
        .remove(0);

    // The print command uses this scoped lookup before it touches a printer.
    assert!(
        s.get_kds_order_for_instance(&order.id, "kds-other")
            .unwrap()
            .is_none()
    );
    let err = s
        .ensure_kds_order_visible_to_instance(&order.id, "kds-other")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));

    // Status and whole-order edit commands cannot mutate another display's ticket.
    let err = s
        .update_kds_status_for_instance(&order.id, "preparing", "kds-other")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));
    let err = s
        .update_kds_order_items_for_instance(
            crate::UpdateKdsOrderItemsInput {
                id: order.id.clone(),
                items_summary: "Tampered".into(),
                item_count: 1,
                line_items: None,
            },
            "kds-other",
        )
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));

    // Both line-item read and update commands enforce the parent order scope.
    let err = s
        .get_kds_order_lines_for_instance(&order.id, "kds-other")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));
    let err = s
        .update_kds_line_item_status_for_instance(&line.id, "ready", "kds-other")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));

    let unchanged = s.get_kds_order(&order.id).unwrap().unwrap();
    assert_eq!(unchanged.status, "pending");
    assert_eq!(unchanged.items_summary, "Burger");
    let unchanged_line = s.get_kds_order_lines(&order.id).unwrap().remove(0);
    assert_eq!(unchanged_line.item_status, "pending");
}

#[test]
fn kds_order_has_runtime_target_instance_column() {
    let conn = fresh();
    let s = store(&conn);
    seed_product(&conn, "BURGER", "Burger");

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("BURGER"), 1, price(500)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale.id,
            store_id: Some("store-1".into()),
            items_summary: "Burger".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let target: Option<String> = conn
        .query_row(
            "SELECT target_instance_id FROM kds_orders WHERE id = ?1",
            params![order.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(target, None);
}

#[test]
fn kds_order_targets_support_multiple_instances() {
    let conn = fresh();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'kds_order_targets'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(exists, 1);
}

#[test]
fn get_kds_order_by_sale_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.get_kds_order_by_sale("no-such-sale").unwrap();
    assert!(result.is_none());
}

#[test]
fn list_kds_orders_ordered_by_received_at_desc() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Create two orders with distinct timing (sleep to separate timestamps).
    let sale_id1 = uuid::Uuid::now_v7().to_string();
    let ts1 = Sale {
        id: sale_id1.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&ts1).unwrap();
    let o1 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id1,
            store_id: None,
            items_summary: "First".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(5));

    let sale_id2 = uuid::Uuid::now_v7().to_string();
    let ts2 = Sale {
        id: sale_id2.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&ts2).unwrap();
    let o2 = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id2,
            store_id: None,
            items_summary: "Second".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let all = s.list_kds_orders(None).unwrap();
    assert_eq!(all.len(), 2);
    // Most recent first.
    assert_eq!(all[0].id, o2.id);
    assert_eq!(all[1].id, o1.id);
}

// ── update_kds_order_items tests ─────────────────────────────────

#[test]
fn update_kds_order_items_updates_summary_and_count() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let sale_id = uuid::Uuid::now_v7().to_string();
    let test_sale = Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: price(0),
        currency: usd(),
        line_count: 0,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&test_sale).unwrap();

    let order = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id,
            store_id: None,
            items_summary: "Coffee x2".into(),
            item_count: 2,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    // Update items.
    let updated = s
        .update_kds_order_items(crate::UpdateKdsOrderItemsInput {
            id: order.id.clone(),
            items_summary: "Coffee x2, Bagel x1".into(),
            item_count: 3,
            line_items: None,
        })
        .unwrap();

    assert_eq!(updated.items_summary, "Coffee x2, Bagel x1");
    assert_eq!(updated.item_count, 3);
    assert_eq!(updated.status, "pending"); // Other fields unchanged
}

#[test]
fn update_kds_order_items_nonexistent_order_fails() {
    let conn = fresh();
    let s = store(&conn);

    let err = s
        .update_kds_order_items(crate::UpdateKdsOrderItemsInput {
            id: "no-such-order".into(),
            items_summary: "New items".into(),
            item_count: 1,
            line_items: None,
        })
        .unwrap_err();

    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));
}

#[test]
fn update_kds_order_items_rejects_empty_summary() {
    let conn = fresh();
    let s = store(&conn);

    let err = s
        .update_kds_order_items(crate::UpdateKdsOrderItemsInput {
            id: "any-id".into(),
            items_summary: "".into(),
            item_count: 1,
            line_items: None,
        })
        .unwrap_err();

    assert!(matches!(err, CoreError::Validation { field, .. } if field == "items_summary"));
}

#[test]
fn update_kds_order_items_rejects_zero_count() {
    let conn = fresh();
    let s = store(&conn);

    let err = s
        .update_kds_order_items(crate::UpdateKdsOrderItemsInput {
            id: "any-id".into(),
            items_summary: "Items".into(),
            item_count: 0,
            line_items: None,
        })
        .unwrap_err();

    assert!(matches!(err, CoreError::Validation { field, .. } if field == "item_count"));
}

// ── KDS order input validation ──────────────────────────────────────

#[test]
fn create_kds_order_rejects_empty_sale_id() {
    let conn = fresh();
    let s = store(&conn);
    let err = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: "".into(),
            store_id: None,
            items_summary: "Items".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "sale_id",
            ..
        }
    ));
}

#[test]
fn create_kds_order_rejects_empty_items_summary() {
    let conn = fresh();
    let s = store(&conn);
    let err = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: "sale-1".into(),
            store_id: None,
            items_summary: "".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "items_summary",
            ..
        }
    ));
}

#[test]
fn create_kds_order_rejects_zero_item_count() {
    let conn = fresh();
    let s = store(&conn);
    let err = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: "sale-1".into(),
            store_id: None,
            items_summary: "Items".into(),
            item_count: 0,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "item_count",
            ..
        }
    ));
}

#[test]
fn create_kds_order_rejects_negative_item_count() {
    let conn = fresh();
    let s = store(&conn);
    let err = s
        .create_kds_order(CreateKdsOrderInput {
            sale_id: "sale-1".into(),
            store_id: None,
            items_summary: "Items".into(),
            item_count: -1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "item_count",
            ..
        }
    ));
}

// ── KDS Device CRUD ────────────────────────────────────────────

use crate::kds::{KdsConnectionStatus, RegisterKdsDeviceInput};

#[test]
fn register_kds_device_and_retrieve() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS", "dev-resto-1");
    let s = store(&conn);
    let input = RegisterKdsDeviceInput {
        name: "Expo Screen".into(),
        restaurant_pos_id: "resto-1".into(),
        station_ids: vec!["station-grill".into(), "station-bar".into()],
        pairing_token_hash: "hash-abc".into(),
        pairing_expires_at: "2099-01-01T00:00:00.000Z".into(),
    };
    let device = s.register_kds_device(input).unwrap();
    assert!(!device.id.is_empty());
    assert_eq!(device.name, "Expo Screen");
    assert_eq!(device.restaurant_pos_id, "resto-1");
    assert_eq!(device.station_ids, vec!["station-grill", "station-bar"]);
    assert!(device.is_active);
    assert_eq!(device.connection_status, KdsConnectionStatus::Disconnected);

    let loaded = s.get_kds_device(&device.id).unwrap().unwrap();
    assert_eq!(loaded.id, device.id);
    assert_eq!(loaded.name, "Expo Screen");
}

#[test]
fn get_kds_device_returns_none_for_missing() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS", "dev-resto-1");
    let s = store(&conn);
    assert!(s.get_kds_device("nope").unwrap().is_none());
}

#[test]
fn list_kds_devices_for_restaurant() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS A", "dev-1");
    seed_terminal(&conn, "resto-2", "Restaurant POS B", "dev-2");
    let s = store(&conn);
    s.register_kds_device(RegisterKdsDeviceInput {
        name: "Screen A".into(),
        restaurant_pos_id: "resto-1".into(),
        station_ids: vec![],
        pairing_token_hash: "h1".into(),
        pairing_expires_at: "2099-01-01".into(),
    })
    .unwrap();
    s.register_kds_device(RegisterKdsDeviceInput {
        name: "Screen B".into(),
        restaurant_pos_id: "resto-1".into(),
        station_ids: vec![],
        pairing_token_hash: "h2".into(),
        pairing_expires_at: "2099-01-01".into(),
    })
    .unwrap();
    s.register_kds_device(RegisterKdsDeviceInput {
        name: "Other Screen".into(),
        restaurant_pos_id: "resto-2".into(),
        station_ids: vec![],
        pairing_token_hash: "h3".into(),
        pairing_expires_at: "2099-01-01".into(),
    })
    .unwrap();

    let devices = s.list_kds_devices_for_restaurant("resto-1").unwrap();
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().all(|d| d.restaurant_pos_id == "resto-1"));
}

#[test]
fn update_kds_device_status_connected() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS", "dev-resto-1");
    let s = store(&conn);
    let device = s
        .register_kds_device(RegisterKdsDeviceInput {
            name: "Test".into(),
            restaurant_pos_id: "resto-1".into(),
            station_ids: vec![],
            pairing_token_hash: "h".into(),
            pairing_expires_at: "2099-01-01".into(),
        })
        .unwrap();

    s.update_kds_device_status(&device.id, KdsConnectionStatus::Connected)
        .unwrap();
    let loaded = s.get_kds_device(&device.id).unwrap().unwrap();
    assert_eq!(loaded.connection_status, KdsConnectionStatus::Connected);
    assert!(loaded.last_seen_at.is_some());
}

#[test]
fn update_kds_device_status_not_found() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS", "dev-resto-1");
    let s = store(&conn);
    let err = s
        .update_kds_device_status("bad-id", KdsConnectionStatus::Connected)
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "kds_device",
            ..
        }
    ));
}

#[test]
fn deactivate_kds_device() {
    let conn = fresh();
    seed_terminal(&conn, "resto-1", "Restaurant POS", "dev-resto-1");
    let s = store(&conn);
    let device = s
        .register_kds_device(RegisterKdsDeviceInput {
            name: "Test".into(),
            restaurant_pos_id: "resto-1".into(),
            station_ids: vec![],
            pairing_token_hash: "h".into(),
            pairing_expires_at: "2099-01-01".into(),
        })
        .unwrap();

    s.deactivate_kds_device(&device.id).unwrap();
    let loaded = s.get_kds_device(&device.id).unwrap().unwrap();
    assert!(!loaded.is_active);
}

// ── Order Acknowledgment ───────────────────────────────────────

#[test]
fn ack_order_first_device_wins() {
    let conn = fresh();
    let s = store(&conn);
    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("SKU-1"), 1, price(100)))
        .unwrap();
    let sale = Sale::from_cart_with_user(&cart, None).unwrap();
    s.create_sale(&sale).unwrap();
    let kds_order = s
        .create_kds_order(crate::CreateKdsOrderInput {
            sale_id: sale.id.clone(),
            store_id: None,
            items_summary: "Item".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let result = s.ack_kds_order(&kds_order.id, "device-a").unwrap();
    assert!(result, "first ack should succeed");
}

#[test]
fn ack_order_second_device_loses() {
    let conn = fresh();
    let s = store(&conn);
    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("SKU-1"), 1, price(100)))
        .unwrap();
    let sale = Sale::from_cart_with_user(&cart, None).unwrap();
    s.create_sale(&sale).unwrap();
    let kds_order = s
        .create_kds_order(crate::CreateKdsOrderInput {
            sale_id: sale.id.clone(),
            store_id: None,
            items_summary: "Item".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    let first = s.ack_kds_order(&kds_order.id, "device-a").unwrap();
    assert!(first);

    let second = s.ack_kds_order(&kds_order.id, "device-b").unwrap();
    assert!(!second, "second ack should return false");
}

#[test]
fn ack_order_already_ready_is_noop() {
    let conn = fresh();
    let s = store(&conn);
    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("SKU-1"), 1, price(100)))
        .unwrap();
    let sale = Sale::from_cart_with_user(&cart, None).unwrap();
    s.create_sale(&sale).unwrap();
    let kds_order = s
        .create_kds_order(crate::CreateKdsOrderInput {
            sale_id: sale.id.clone(),
            store_id: None,
            items_summary: "Item".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    s.update_kds_status(&kds_order.id, "preparing").unwrap();

    let result = s.ack_kds_order(&kds_order.id, "device-a").unwrap();
    assert!(!result, "ack on non-pending order should return false");
}
