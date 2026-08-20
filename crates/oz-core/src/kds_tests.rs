use super::*;

// ── KdsStatus as_str ───────────────────────────────────────────

#[test]
fn status_as_str_all_variants() {
    assert_eq!(KdsStatus::Pending.as_str(), "pending");
    assert_eq!(KdsStatus::Preparing.as_str(), "preparing");
    assert_eq!(KdsStatus::Ready.as_str(), "ready");
    assert_eq!(KdsStatus::Served.as_str(), "served");
    assert_eq!(KdsStatus::Cancelled.as_str(), "cancelled");
}

// ── KdsStatus from_str ─────────────────────────────────────────

#[test]
fn status_from_str_all_variants() {
    assert_eq!(KdsStatus::from_str("pending"), Some(KdsStatus::Pending));
    assert_eq!(KdsStatus::from_str("preparing"), Some(KdsStatus::Preparing));
    assert_eq!(KdsStatus::from_str("ready"), Some(KdsStatus::Ready));
    assert_eq!(KdsStatus::from_str("served"), Some(KdsStatus::Served));
    assert_eq!(KdsStatus::from_str("cancelled"), Some(KdsStatus::Cancelled));
}

#[test]
fn status_from_str_invalid() {
    assert_eq!(KdsStatus::from_str("bogus"), None);
    assert_eq!(KdsStatus::from_str(""), None);
    assert_eq!(KdsStatus::from_str("PENDING"), None);
}

#[test]
fn status_from_str_roundtrip() {
    for s in &[
        KdsStatus::Pending,
        KdsStatus::Preparing,
        KdsStatus::Ready,
        KdsStatus::Served,
        KdsStatus::Cancelled,
    ] {
        assert_eq!(KdsStatus::from_str(s.as_str()), Some(s.clone()));
    }
}

// ── Serde roundtrips ───────────────────────────────────────────

#[test]
fn kds_status_serde_roundtrip() {
    let status = KdsStatus::Ready;
    let json = serde_json::to_string(&status).unwrap();
    let back: KdsStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(back, KdsStatus::Ready);
}

#[test]
fn kds_order_serde_roundtrip() {
    let order = KdsOrder {
        id: "o-1".into(),
        sale_id: "s-1".into(),
        store_id: Some("store-default".into()),
        target_instance_id: Some("kds-main".into()),
        status: "pending".into(),
        items_summary: "Coffee x2, Bagel".into(),
        item_count: 3,
        display_number: Some(1),
        received_at: "2025-01-01T12:00:00.000Z".into(),
        started_at: None,
        ready_at: None,
        served_at: None,
        prep_time_seconds: 300,
        kitchen_zone: Some("front".into()),
        notes: "No onions".into(),
        table_number: None,
        priority: true,
    };
    let json = serde_json::to_string(&order).unwrap();
    let back: KdsOrder = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, order.id);
    assert_eq!(back.sale_id, order.sale_id);
    assert_eq!(back.status, order.status);
    assert_eq!(back.items_summary, order.items_summary);
    assert_eq!(back.item_count, order.item_count);
    assert_eq!(back.prep_time_seconds, order.prep_time_seconds);
    assert_eq!(back.kitchen_zone, Some("front".into()));
    assert_eq!(back.notes, order.notes);
}

#[test]
fn create_kds_order_input_serde_roundtrip() {
    let input = CreateKdsOrderInput {
        sale_id: "s-1".into(),
        store_id: None,
        items_summary: "Tea".into(),
        item_count: 1,
        kitchen_zone: None,
        notes: String::new(),
        table_number: None,
        priority: true,
    };
    let json = serde_json::to_string(&input).unwrap();
    let back: CreateKdsOrderInput = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sale_id, "s-1");
    assert_eq!(back.items_summary, "Tea");
    assert_eq!(back.item_count, 1);
    assert_eq!(back.notes, "");
    assert!(back.priority);
}

#[test]
fn kds_order_optional_timestamps() {
    let order = KdsOrder {
        id: "o-2".into(),
        sale_id: "s-2".into(),
        store_id: None,
        target_instance_id: None,
        status: "served".into(),
        items_summary: "Done".into(),
        item_count: 1,
        display_number: None,
        received_at: "2025-01-01T12:00:00.000Z".into(),
        started_at: Some("2025-01-01T12:05:00.000Z".into()),
        ready_at: Some("2025-01-01T12:10:00.000Z".into()),
        served_at: Some("2025-01-01T12:12:00.000Z".into()),
        prep_time_seconds: 720,
        kitchen_zone: None,
        notes: String::new(),
        table_number: None,
        priority: false,
    };
    assert_eq!(
        order.started_at.as_deref(),
        Some("2025-01-01T12:05:00.000Z")
    );
    assert_eq!(order.ready_at.as_deref(), Some("2025-01-01T12:10:00.000Z"));
    assert_eq!(order.served_at.as_deref(), Some("2025-01-01T12:12:00.000Z"));
    assert!(order.display_number.is_none());
}

// ── KdsConnectionStatus ────────────────────────────────────────

#[test]
fn connection_status_as_str_all_variants() {
    assert_eq!(KdsConnectionStatus::Connected.as_str(), "connected");
    assert_eq!(KdsConnectionStatus::Disconnected.as_str(), "disconnected");
    assert_eq!(KdsConnectionStatus::Stale.as_str(), "stale");
}

#[test]
fn connection_status_from_str_all_variants() {
    assert_eq!(
        KdsConnectionStatus::from_str("connected"),
        Some(KdsConnectionStatus::Connected)
    );
    assert_eq!(
        KdsConnectionStatus::from_str("disconnected"),
        Some(KdsConnectionStatus::Disconnected)
    );
    assert_eq!(
        KdsConnectionStatus::from_str("stale"),
        Some(KdsConnectionStatus::Stale)
    );
}

#[test]
fn connection_status_from_str_invalid() {
    assert_eq!(KdsConnectionStatus::from_str("bogus"), None);
    assert_eq!(KdsConnectionStatus::from_str(""), None);
}

// ── KdsDevice ──────────────────────────────────────────────────

fn make_device(id: &str, station_ids: Vec<&str>) -> KdsDevice {
    KdsDevice {
        id: id.into(),
        name: format!("Device {id}"),
        restaurant_pos_id: "resto-1".into(),
        station_ids: station_ids.into_iter().map(String::from).collect(),
        is_active: true,
        last_seen_at: None,
        connection_status: KdsConnectionStatus::Disconnected,
        created_at: "2025-01-01T00:00:00.000Z".into(),
        updated_at: "2025-01-01T00:00:00.000Z".into(),
    }
}

fn make_line_item(sku: &str) -> KdsLineItem {
    KdsLineItem {
        id: "li-1".into(),
        kds_order_id: "order-1".into(),
        sku: sku.into(),
        display_name: format!("Product {sku}"),
        qty: 1,
        course: None,
        modifiers: vec![],
        line_position: 0,
        item_status: "pending".into(),
        started_at: None,
        ready_at: None,
        served_at: None,
        created_at: "2025-01-01T00:00:00.000Z".into(),
    }
}

// ── resolve_kds_targets ────────────────────────────────────────

#[test]
fn routing_single_device_receives_all_orders() {
    let devices = vec![make_device("d-1", vec![])]; // empty = broadcast
    let items = vec![make_line_item("SKU-1")];
    let targets = resolve_kds_targets(&items, &devices, |_| None);
    assert_eq!(targets, vec!["d-1"]);
}

#[test]
fn routing_station_targeted_device_gets_matching_orders() {
    let devices = vec![
        make_device("d-grill", vec!["station-grill"]),
        make_device("d-bar", vec!["station-bar"]),
    ];
    let items = vec![make_line_item("STEAK")];
    let targets = resolve_kds_targets(&items, &devices, |sku| {
        if sku == "STEAK" {
            Some("station-grill".into())
        } else {
            None
        }
    });
    assert!(targets.contains(&"d-grill".to_string()));
    assert!(!targets.contains(&"d-bar".to_string()));
}

#[test]
fn routing_untargeted_station_broadcasts_to_all() {
    let devices = vec![
        make_device("d-1", vec!["station-grill"]),
        make_device("d-2", vec!["station-bar"]),
    ];
    let items = vec![make_line_item("UNKNOWN-SKU")];
    // No device claims "unknown-station"
    let targets = resolve_kds_targets(&items, &devices, |_| Some("unknown-station".into()));
    // Both devices should receive it (broadcast fallback)
    assert!(targets.contains(&"d-1".to_string()));
    assert!(targets.contains(&"d-2".to_string()));
}

#[test]
fn routing_inactive_device_excluded() {
    let mut device = make_device("d-1", vec![]);
    device.is_active = false;
    let devices = vec![device];
    let items = vec![make_line_item("SKU-1")];
    let targets = resolve_kds_targets(&items, &devices, |_| None);
    assert!(targets.is_empty());
}

#[test]
fn routing_empty_station_ids_means_broadcast() {
    let devices = vec![make_device("d-broadcast", vec![])];
    let items = vec![make_line_item("SKU-1")];
    let targets = resolve_kds_targets(&items, &devices, |_| None);
    assert_eq!(targets, vec!["d-broadcast"]);
}

#[test]
fn routing_deduplication_across_overlapping_stations() {
    let devices = vec![make_device("d-both", vec!["s1", "s2"])];
    let items = vec![make_line_item("A"), make_line_item("B")];
    let targets = resolve_kds_targets(&items, &devices, |sku| {
        if sku == "A" {
            Some("s1".into())
        } else {
            Some("s2".into())
        }
    });
    // d-both should appear only once
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0], "d-both");
}

#[test]
fn routing_empty_line_items_no_targets() {
    let devices = vec![make_device("d-1", vec!["station-grill"])];
    let items: Vec<KdsLineItem> = vec![];
    let targets = resolve_kds_targets(&items, &devices, |_| Some("station-grill".into()));
    // No line items → no station lookups → no targets from phase 1
    // No broadcast devices → no targets from phase 2
    assert!(targets.is_empty());
}

#[test]
fn routing_mixed_station_and_broadcast() {
    let devices = vec![
        make_device("d-grill", vec!["station-grill"]),
        make_device("d-all", vec![]), // broadcast
    ];
    let items = vec![make_line_item("STEAK")];
    let targets = resolve_kds_targets(&items, &devices, |sku| {
        if sku == "STEAK" {
            Some("station-grill".into())
        } else {
            None
        }
    });
    // Both should receive: d-grill via station, d-all via broadcast
    assert!(targets.contains(&"d-grill".to_string()));
    assert!(targets.contains(&"d-all".to_string()));
}

#[test]
fn routing_no_devices_returns_empty() {
    let items = vec![make_line_item("SKU-1")];
    let targets = resolve_kds_targets(&items, &[], |_| None);
    assert!(targets.is_empty());
}
