
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
