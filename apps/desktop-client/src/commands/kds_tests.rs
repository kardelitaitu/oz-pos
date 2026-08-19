use super::*;

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

#[test]
fn kds_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}
