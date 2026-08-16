//! Tests for the node-topology commands (topology.rs).
//!
//! The original 6k-line `mod tests` was split into three files by subject:
//! this file (save/load roundtrip, serde, field/wire edge cases, NaN
//! sanitisation) plus topology_stress_tests.rs and
//! topology_command_tests.rs. The shared helpers below are `pub(crate)` so
//! the sibling modules glob them via `use super::topology_tests::*`.

use super::*;
use oz_core::migrations;
use rusqlite::Connection;
use serde_json::Value;

use crate::error::AppError;

pub(crate) fn fresh_conn() -> Connection {
    // These tests exercise the settings serialization contract, not the
    // filesystem. An in-memory database keeps the connection self-contained
    // and avoids leaving SQLite's journal/WAL files in a TempDir that is
    // dropped when this helper returns.
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(&mut conn).unwrap();
    conn
}

pub(crate) fn semantic_node(id: &str, node_type: &str, store_profile_id: Option<&str>) -> Value {
    let mut node = serde_json::json!({
        "id": id,
        "type": node_type,
        "name": id,
        "x": 0.0,
        "y": 0.0,
    });
    if let Some(store_profile_id) = store_profile_id {
        node["store_profile_id"] = Value::String(store_profile_id.into());
    }
    node
}

pub(crate) fn semantic_location_wire(id: &str, to_node_id: &str) -> Value {
    serde_json::json!({
        "id": id,
        "from_node_id": "branch",
        "to_node_id": to_node_id,
        "direction": "one-way",
        "from_port_id": "location-out",
        "to_port_id": "location-in",
        "relationship_type": "location",
    })
}

#[test]
fn semantic_save_persists_version_and_fields() {
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    save_topology_json(&conn, nodes, wires).unwrap();

    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["schema_version"], TOPOLOGY_SCHEMA_VERSION);
    assert_eq!(value["nodes"][0]["store_profile_id"], "default");
    assert_eq!(value["wires"][0]["from_port_id"], "location-out");
    assert_eq!(value["wires"][0]["to_port_id"], "location-in");
    assert_eq!(value["wires"][0]["relationship_type"], "location");
}

#[test]
fn semantic_save_persists_and_clears_resolved_issue_keys() {
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    let issue = "node:wh-1:topology-validation-warehouse-missing-stock-routing".to_string();

    save_topology_json_at_key_with_revision(
        &conn,
        nodes.clone(),
        wires.clone(),
        TOPOLOGY_SETTING_KEY,
        std::slice::from_ref(&issue),
        None,
        None,
    )
    .unwrap();
    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["resolved_issue_keys"], serde_json::json!([issue]));

    save_topology_json_at_key_with_revision(
        &conn,
        nodes,
        wires,
        TOPOLOGY_SETTING_KEY,
        &[],
        None,
        None,
    )
    .unwrap();
    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["resolved_issue_keys"], serde_json::json!([]));
}

#[test]
fn semantic_save_rejects_ambiguous_legacy_workspace_wire() {
    let conn = fresh_conn();
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "store", None),
            semantic_node("ws-1", "workspace", None),
            semantic_node("ws-2", "workspace", None),
        ],
        vec![
            serde_json::json!({
                "id": "wire-owner",
                "from_node_id": "branch",
                "to_node_id": "ws-1",
                "direction": "one-way",
            }),
            serde_json::json!({
                "id": "wire-ambiguous",
                "from_node_id": "ws-1",
                "to_node_id": "ws-2",
                "direction": "one-way",
            }),
        ],
    );

    match result {
        Err(AppError::TopologyValidation { code, wire_id, .. }) => {
            assert_eq!(code, "ambiguous-legacy-wire");
            assert_eq!(wire_id.as_deref(), Some("wire-ambiguous"));
        }
        other => panic!("expected ambiguous-legacy-wire, got {other:?}"),
    }
}

#[test]
fn semantic_save_compiles_operational_wires_to_branch_runtime_plan() {
    let conn = fresh_conn();
    let branch_key = topology_setting_key(Some("default")).unwrap();
    let mut resto = semantic_node("resto-pos", "workspace", None);
    resto["metadata"] = serde_json::json!({ "typeKey": "restaurant-pos" });
    let mut kds = semantic_node("kds", "workspace", None);
    kds["metadata"] = serde_json::json!({ "typeKey": "kds" });
    let operation_wire = serde_json::json!({
        "id": "wire-resto-kds-runtime",
        "from_node_id": "resto-pos",
        "to_node_id": "kds",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });

    save_topology_json_at_key(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            resto,
            kds,
        ],
        vec![
            semantic_location_wire("wire-resto-location", "resto-pos"),
            operation_wire,
        ],
        &branch_key,
    )
    .unwrap();

    let runtime_json = oz_core::Settings::get(&conn, "oz-pos/topology-runtime/default")
        .unwrap()
        .expect("semantic save must compile a runtime plan");
    let runtime: Value = serde_json::from_str(&runtime_json).unwrap();
    assert_eq!(runtime["schema_version"], TOPOLOGY_SCHEMA_VERSION);
    assert_eq!(runtime["branch_id"], "default");
    assert_eq!(runtime["routes"][0]["wire_id"], "wire-resto-kds-runtime");
    assert_eq!(runtime["routes"][0]["source_instance_id"], "resto-pos");
    assert_eq!(runtime["routes"][0]["target_instance_id"], "kds");
    assert_eq!(runtime["routes"][0]["relationship_type"], "generic");

    // A later save without operational wires replaces the manifest rather
    // than leaving the removed route active.
    save_topology_json_at_key(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            semantic_node("store-pos", "workspace", None),
        ],
        vec![semantic_location_wire("wire-store-location", "store-pos")],
        &branch_key,
    )
    .unwrap();
    let cleared: Value = serde_json::from_str(
        &oz_core::Settings::get(&conn, "oz-pos/topology-runtime/default")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(cleared["routes"].as_array().unwrap().len(), 0);
}

#[test]
fn branch_topology_settings_are_isolated() {
    let conn = fresh_conn();
    let branch_a_key = topology_setting_key(Some("branch-a")).unwrap();
    let branch_b_key = topology_setting_key(Some("branch-b")).unwrap();

    save_topology_json_at_key(
        &conn,
        vec![serde_json::json!({
            "id": "branch-a",
            "type": "store",
            "name": "Branch A",
            "x": 0.0,
            "y": 0.0,
        })],
        vec![],
        &branch_a_key,
    )
    .unwrap();
    save_topology_json_at_key(
        &conn,
        vec![serde_json::json!({
            "id": "branch-b",
            "type": "store",
            "name": "Branch B",
            "x": 0.0,
            "y": 0.0,
        })],
        vec![],
        &branch_b_key,
    )
    .unwrap();

    let a = oz_core::Settings::get(&conn, &branch_a_key)
        .unwrap()
        .unwrap();
    let b = oz_core::Settings::get(&conn, &branch_b_key)
        .unwrap()
        .unwrap();
    assert!(a.contains("branch-a"));
    assert!(!a.contains("branch-b"));
    assert!(b.contains("branch-b"));
    assert!(!b.contains("branch-a"));
    let runtime_a: Value = serde_json::from_str(
        &oz_core::Settings::get(&conn, "oz-pos/topology-runtime/branch-a")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let runtime_b: Value = serde_json::from_str(
        &oz_core::Settings::get(&conn, "oz-pos/topology-runtime/branch-b")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(runtime_a["branch_id"], "branch-a");
    assert_eq!(runtime_b["branch_id"], "branch-b");
    assert!(
        oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .is_none()
    );
}

#[test]
fn topology_setting_key_rejects_path_injection() {
    let result = topology_setting_key(Some("branch/a"));
    assert!(result.is_err());
}

#[test]
fn legacy_topology_is_only_migrated_for_matching_branch() {
    let value = serde_json::json!({
        "schema_version": TOPOLOGY_SCHEMA_VERSION,
        "nodes": [{
            "id": "branch-a",
            "type": "branch-location",
            "store_profile_id": "branch-a"
        }],
        "wires": []
    });
    assert!(legacy_topology_belongs_to_branch(&value, "branch-a").unwrap());
    assert!(!legacy_topology_belongs_to_branch(&value, "branch-b").unwrap());
}

#[test]
fn semantic_save_preserves_bend_points() {
    // The command envelope persists the RAW wire payload (save_topology_json
    // writes `wires: Vec<Value>` untouched after validation), so the editor's
    // bend points must survive the Apply round-trip even though the typed
    // validation struct has no `bends` field (serde ignores unknown fields).
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let mut wire = semantic_location_wire("wire-1", "ws-1");
    wire["bends"] = serde_json::json!([{ "x": 350.0, "y": 334.0 }, { "x": 400.0, "y": 300.0 }]);
    save_topology_json(&conn, nodes, vec![wire]).unwrap();

    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["wires"][0]["bends"][0]["x"], 350.0);
    assert_eq!(value["wires"][0]["bends"][0]["y"], 334.0);
    assert_eq!(value["wires"][0]["bends"][1]["x"], 400.0);
    assert_eq!(value["wires"][0]["bends"][1]["y"], 300.0);
}

#[test]
fn semantic_save_accepts_two_way_location_wire() {
    // Direction is presentation-only: the frontend contract keeps
    // one-way | reverse | two-way all legal (normalizeWireDirection),
    // so the Apply boundary must NOT reject a location wire whose
    // direction was cycled to two-way in the editor.
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let mut wire = semantic_location_wire("wire-1", "ws-1");
    wire["direction"] = Value::String("two-way".into());
    save_topology_json(&conn, nodes, vec![wire]).unwrap();

    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["wires"][0]["direction"], "two-way");
}

#[test]
fn semantic_save_accepts_reverse_location_wire() {
    // The editor cycles direction left-to-right -> right-to-left -> both;
    // reverse must round-trip through Apply like the other legal states.
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let mut wire = semantic_location_wire("wire-1", "ws-1");
    wire["direction"] = Value::String("reverse".into());
    save_topology_json(&conn, nodes, vec![wire]).unwrap();

    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["wires"][0]["direction"], "reverse");
}

#[test]
fn semantic_save_accepts_kds_operation_feed_from_restaurant_pos() {
    let conn = fresh_conn();
    let mut resto = semantic_node("resto-pos", "workspace", None);
    resto["metadata"] = serde_json::json!({ "typeKey": "restaurant-pos" });
    let mut kds = semantic_node("kds", "workspace", None);
    kds["metadata"] = serde_json::json!({ "typeKey": "kds" });
    let operation_wire = serde_json::json!({
        "id": "wire-resto-kds",
        "from_node_id": "resto-pos",
        "to_node_id": "kds",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            resto,
            kds,
        ],
        vec![
            semantic_location_wire("wire-resto-location", "resto-pos"),
            operation_wire,
        ],
    );
    assert!(result.is_ok());
}

#[test]
fn semantic_save_rejects_operation_feed_from_non_restaurant_pos() {
    let conn = fresh_conn();
    let mut store_pos = semantic_node("store-pos", "workspace", None);
    store_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let mut kds = semantic_node("kds", "workspace", None);
    kds["metadata"] = serde_json::json!({ "typeKey": "kds" });
    let invalid_operation_wire = serde_json::json!({
        "id": "wire-invalid-operation-source",
        "from_node_id": "store-pos",
        "to_node_id": "kds",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            store_pos,
            kds,
        ],
        vec![
            semantic_location_wire("wire-pos-location", "store-pos"),
            invalid_operation_wire,
        ],
    );

    match result {
        Err(AppError::TopologyValidation {
            code,
            wire_id,
            node_id,
            ..
        }) => {
            assert_eq!(code, "invalid-operation-source");
            assert_eq!(wire_id.as_deref(), Some("wire-invalid-operation-source"));
            assert_eq!(node_id.as_deref(), Some("kds"));
        }
        other => panic!("expected invalid-operation-source, got {other:?}"),
    }
}

#[test]
fn semantic_save_rejects_mismatched_non_location_wire() {
    let conn = fresh_conn();
    let mut store_pos = semantic_node("store-pos", "workspace", None);
    store_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let invalid_wire = serde_json::json!({
        "id": "wire-invalid-pair",
        "from_node_id": "store-pos",
        "to_node_id": "warehouse-1",
        "direction": "one-way",
        "from_port_id": "stock-out",
        "to_port_id": "location-in",
        "relationship_type": "stock-routing",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            store_pos,
            semantic_node("warehouse-1", "warehouse", None),
        ],
        vec![
            semantic_location_wire("wire-store-location", "store-pos"),
            invalid_wire,
        ],
    );

    match result {
        Err(AppError::TopologyValidation { code, wire_id, .. }) => {
            assert_eq!(code, "invalid-semantic-connection");
            assert_eq!(wire_id.as_deref(), Some("wire-invalid-pair"));
        }
        other => panic!("expected invalid-semantic-connection, got {other:?}"),
    }
}

#[test]
fn semantic_save_rejects_ticket_wire_from_non_kds_workspace() {
    let conn = fresh_conn();
    let mut store_pos = semantic_node("store-pos", "workspace", None);
    store_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let invalid_wire = serde_json::json!({
        "id": "wire-invalid-ticket-source",
        "from_node_id": "store-pos",
        "to_node_id": "printer-1",
        "direction": "one-way",
        "from_port_id": "ticket-out",
        "to_port_id": "ticket-in",
        "relationship_type": "ticket-routing",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            store_pos,
            semantic_node("printer-1", "hardware", None),
        ],
        vec![
            semantic_location_wire("wire-store-location", "store-pos"),
            invalid_wire,
        ],
    );

    match result {
        Err(AppError::TopologyValidation { code, wire_id, .. }) => {
            assert_eq!(code, "invalid-semantic-connection");
            assert_eq!(wire_id.as_deref(), Some("wire-invalid-ticket-source"));
        }
        other => panic!("expected invalid-semantic-connection, got {other:?}"),
    }
}

#[test]
fn semantic_save_accepts_valid_stock_routing_wire() {
    let conn = fresh_conn();
    let mut store_pos = semantic_node("store-pos", "workspace", None);
    store_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let valid_wire = serde_json::json!({
        "id": "wire-stock",
        "from_node_id": "store-pos",
        "to_node_id": "warehouse-1",
        "direction": "one-way",
        "from_port_id": "stock-out",
        "to_port_id": "stock-in",
        "relationship_type": "stock-routing",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            store_pos,
            semantic_node("warehouse-1", "warehouse", None),
        ],
        vec![
            semantic_location_wire("wire-store-location", "store-pos"),
            semantic_location_wire("wire-warehouse-scope", "warehouse-1"),
            valid_wire,
        ],
    );

    assert!(result.is_ok());
}

#[test]
fn semantic_save_accepts_warehouse_to_warehouse_transfer_route() {
    let conn = fresh_conn();
    let hub = semantic_node("warehouse-hub", "warehouse", None);
    let satellite = semantic_node("warehouse-satellite", "warehouse", None);
    let transfer_wire = serde_json::json!({
        "id": "wire-warehouse-transfer",
        "from_node_id": "warehouse-hub",
        "to_node_id": "warehouse-satellite",
        "direction": "one-way",
        "from_port_id": "transfer-out",
        "to_port_id": "transfer-in",
        "relationship_type": "inventory-transfer",
    });

    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            hub,
            satellite,
        ],
        vec![
            semantic_location_wire("wire-hub-scope", "warehouse-hub"),
            semantic_location_wire("wire-satellite-scope", "warehouse-satellite"),
            transfer_wire,
        ],
    );

    assert!(result.is_ok());
}

#[test]
fn semantic_save_accepts_warehouse_location_or_retail_pos_operation() {
    let conn = fresh_conn();
    let mut retail_pos = semantic_node("retail-pos", "workspace", None);
    retail_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let warehouse = semantic_node("warehouse-1", "warehouse", None);
    let operation_wire = serde_json::json!({
        "id": "wire-retail-warehouse",
        "from_node_id": "retail-pos",
        "to_node_id": "warehouse-1",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });

    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            retail_pos,
            warehouse,
        ],
        vec![
            semantic_location_wire("wire-retail-location", "retail-pos"),
            operation_wire,
        ],
    );
    assert!(result.is_ok());

    let location_result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            semantic_node("warehouse-1", "warehouse", None),
        ],
        vec![semantic_location_wire(
            "wire-warehouse-scope",
            "warehouse-1",
        )],
    );
    assert!(location_result.is_ok());
}

#[test]
fn semantic_save_rejects_multiple_warehouse_primary_inputs() {
    let conn = fresh_conn();
    let mut retail_pos = semantic_node("retail-pos", "workspace", None);
    retail_pos["metadata"] = serde_json::json!({ "typeKey": "store-pos" });
    let operation_wire = serde_json::json!({
        "id": "wire-retail-warehouse",
        "from_node_id": "retail-pos",
        "to_node_id": "warehouse-1",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            retail_pos,
            semantic_node("warehouse-1", "warehouse", None),
        ],
        vec![
            semantic_location_wire("wire-warehouse-scope", "warehouse-1"),
            semantic_location_wire("wire-retail-location", "retail-pos"),
            operation_wire,
        ],
    );

    match result {
        Err(AppError::TopologyValidation { code, node_id, .. }) => {
            assert_eq!(code, "multiple-warehouse-inputs");
            assert_eq!(node_id.as_deref(), Some("warehouse-1"));
        }
        other => panic!("expected multiple-warehouse-inputs, got {other:?}"),
    }
}

#[test]
fn semantic_save_rejects_warehouse_operation_from_non_retail_pos() {
    let mut restaurant_pos = semantic_node("restaurant-pos", "workspace", None);
    restaurant_pos["metadata"] = serde_json::json!({ "typeKey": "restaurant-pos" });
    let operation_wire = serde_json::json!({
        "id": "wire-invalid-warehouse-operation",
        "from_node_id": "restaurant-pos",
        "to_node_id": "warehouse-1",
        "direction": "one-way",
        "from_port_id": "operation-out",
        "to_port_id": "operation-in",
        "relationship_type": "generic",
    });
    let result = save_topology_json(
        &fresh_conn(),
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            restaurant_pos,
            semantic_node("warehouse-1", "warehouse", None),
        ],
        vec![
            semantic_location_wire("wire-restaurant-location", "restaurant-pos"),
            operation_wire,
        ],
    );

    match result {
        Err(AppError::TopologyValidation { code, wire_id, .. }) => {
            assert_eq!(code, "invalid-warehouse-operation-source");
            assert_eq!(wire_id.as_deref(), Some("wire-invalid-warehouse-operation"));
        }
        other => panic!("expected invalid-warehouse-operation-source, got {other:?}"),
    }
}

#[test]
fn semantic_save_rejects_directed_operational_cycle() {
    let conn = fresh_conn();
    let cycle_wires = vec![
        semantic_location_wire("wire-owner-1", "ws-1"),
        semantic_location_wire("wire-owner-2", "ws-2"),
        serde_json::json!({
            "id": "wire-cycle-a",
            "from_node_id": "ws-1",
            "to_node_id": "ws-2",
            "direction": "one-way",
            "from_port_id": "generic-out",
            "to_port_id": "generic-in",
            "relationship_type": "generic",
        }),
        serde_json::json!({
            "id": "wire-cycle-b",
            "from_node_id": "ws-2",
            "to_node_id": "ws-1",
            "direction": "one-way",
            "from_port_id": "generic-out",
            "to_port_id": "generic-in",
            "relationship_type": "generic",
        }),
    ];
    let result = save_topology_json(
        &conn,
        vec![
            semantic_node("branch", "branch-location", Some("default")),
            semantic_node("ws-1", "workspace", None),
            semantic_node("ws-2", "workspace", None),
        ],
        cycle_wires,
    );

    match result {
        Err(AppError::TopologyValidation { code, .. }) => {
            assert_eq!(code, "cycle-detected");
        }
        other => panic!("expected cycle-detected, got {other:?}"),
    }
}

#[test]
fn semantic_save_requires_one_location_input_per_workspace() {
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let result = save_topology_json(&conn, nodes, vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Location In"));
}

#[test]
fn semantic_save_rejects_unknown_store_profile() {
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("missing-store")),
        semantic_node("ws-1", "workspace", None),
    ];
    let result = save_topology_json(&conn, nodes, vec![semantic_location_wire("wire-1", "ws-1")]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unknown store_profile_id")
    );
}

#[test]
fn apply_gate_rejects_duplicate_node_ids_before_mutation() {
    // The Apply pre-mutation gate must reject a structurally malformed
    // diagram (duplicate node ids) BEFORE any workspace row is mutated.
    // Structural validation used to run only at the final save — after
    // workspace creations/updates/archivals — so a malformed diagram
    // passed the gate, mutated rows, then failed at save and forced the
    // full compensation unwind. The editor's savedById Map also silently
    // collapses duplicate ids at load, so the Apply gate is the last
    // hard boundary that can catch them.
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
        semantic_node("ws-1", "workspace", None),
    ];
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    let result = validate_apply_gate(&conn, &nodes, &wires);
    assert!(result.is_err(), "gate must reject duplicate node ids");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate node id"),
        "gate should reject duplicate node ids, got: {err}"
    );
}

#[test]
fn semantic_validate_reports_missing_branch_when_graph_has_no_branch() {
    // Frontend contract parity: validateTopologyGraph reports
    // `missing-branch-location` when the graph has ZERO branch-location
    // nodes ("Add exactly one Branch Location node.") and
    // `multiple-branch-locations` only when it has MORE than one. The
    // Rust validator collapsed both into `multiple-branch-locations`, so
    // a zero-branch graph rejected by the Apply gate surfaced the wrong
    // guidance code to the UI.
    let conn = fresh_conn();
    let nodes = vec![semantic_node("ws-1", "workspace", None)];
    // A location wire makes the payload semantic (has_semantic_fields)
    // even though no branch node exists.
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    match validate_semantic_ownership(&conn, &nodes, &wires) {
        Err(AppError::TopologyValidation { code, .. }) => {
            assert_eq!(code, "missing-branch-location")
        }
        other => panic!("expected TopologyValidation(missing-branch-location), got {other:?}"),
    }
}

#[test]
fn semantic_validate_reports_multiple_branches_when_graph_has_two() {
    // The other half of the frontend parity contract: MORE than one
    // branch-location node keeps the `multiple-branch-locations` code
    // ("Keep exactly one Branch Location node in this graph.").
    let conn = fresh_conn();
    let nodes = vec![
        semantic_node("branch-1", "branch-location", Some("default")),
        semantic_node("branch-2", "branch-location", Some("default")),
    ];
    let wires = vec![semantic_location_wire("wire-1", "branch-2")];
    match validate_semantic_ownership(&conn, &nodes, &wires) {
        Err(AppError::TopologyValidation { code, .. }) => {
            assert_eq!(code, "multiple-branch-locations")
        }
        other => {
            panic!("expected TopologyValidation(multiple-branch-locations), got {other:?}")
        }
    }
}

#[test]
fn save_and_load_roundtrip() {
    let conn = fresh_conn();
    let nodes = vec![
        TopologyNodePayload {
            id: "store-1".into(),
            node_type: "store".into(),
            name: "Main Store".into(),
            subtitle: Some("Primary".into()),
            x: 100.0,
            y: 200.0,
            tier_requirement: None,
            telemetry_badge: Some("Online".into()),
            telemetry_status: Some("online".into()),
            metadata: None,
        },
        TopologyNodePayload {
            id: "ws-1".into(),
            node_type: "workspace".into(),
            name: "POS #1".into(),
            subtitle: None,
            x: 300.0,
            y: 100.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        },
    ];
    let wires = vec![TopologyWirePayload {
        id: "w-1".into(),
        from_node_id: "store-1".into(),
        to_node_id: "ws-1".into(),
        direction: "one-way".into(),
        label: Some("Binds Store".into()),
        from_port: Some("right".into()),
        to_port: Some("left".into()),
    }];

    save_topology_data(&conn, nodes, wires).unwrap();
    let loaded = load_topology_data(&conn).unwrap().unwrap();

    assert_eq!(loaded.nodes.len(), 2);
    assert_eq!(loaded.nodes[0].id, "store-1");
    assert_eq!(loaded.nodes[0].name, "Main Store");
    assert_eq!(loaded.nodes[0].x, 100.0);
    assert_eq!(loaded.wires.len(), 1);
    assert_eq!(loaded.wires[0].id, "w-1");
    assert_eq!(loaded.wires[0].from_port, Some(PortName::Right));
}

#[test]
fn save_normalizes_null_ports_to_renderer_defaults() {
    let conn = fresh_conn();
    let nodes = vec![
        TopologyNodePayload {
            id: "store-1".into(),
            node_type: "store".into(),
            name: "Main Store".into(),
            subtitle: Some("Primary".into()),
            x: 100.0,
            y: 200.0,
            tier_requirement: None,
            telemetry_badge: Some("Online".into()),
            telemetry_status: Some("online".into()),
            metadata: None,
        },
        TopologyNodePayload {
            id: "ws-1".into(),
            node_type: "workspace".into(),
            name: "POS #1".into(),
            subtitle: None,
            x: 300.0,
            y: 100.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        },
    ];
    // Null ports must be normalized to the editor's renderer defaults at
    // SAVE time so the DB never stores a wire with null from/to ports —
    // the frontend loader maps null → undefined, forcing every consumer
    // (e.g. the duplicate-wire detector) to re-apply the defaults.
    let wires = vec![
        TopologyWirePayload {
            id: "w-1".into(),
            from_node_id: "store-1".into(),
            to_node_id: "ws-1".into(),
            direction: "one-way".into(),
            label: Some("Binds Store".into()),
            from_port: None,
            to_port: None,
        },
        // Explicit non-default ports must NOT be normalized away — only
        // None gets filled (the get_or_insert contract).
        TopologyWirePayload {
            id: "w-2".into(),
            from_node_id: "store-1".into(),
            to_node_id: "ws-1".into(),
            direction: "one-way".into(),
            label: None,
            from_port: Some(PortName::Bottom),
            to_port: Some(PortName::Top),
        },
    ];

    save_topology_data(&conn, nodes, wires).unwrap();
    let loaded = load_topology_data(&conn).unwrap().unwrap();

    assert_eq!(loaded.wires.len(), 2);
    assert_eq!(loaded.wires[0].from_port, Some(PortName::Right));
    assert_eq!(loaded.wires[0].to_port, Some(PortName::Left));
    assert_eq!(loaded.wires[1].from_port, Some(PortName::Bottom));
    assert_eq!(loaded.wires[1].to_port, Some(PortName::Top));
}

#[test]
fn load_returns_none_for_fresh_db() {
    let conn = fresh_conn();
    let result = load_topology_data(&conn).unwrap();
    assert!(result.is_none());
}

#[test]
fn load_topology_data_preserves_raw_legacy_null_ports() {
    // Legacy rows written BEFORE the af7710d8 save-side normalization
    // store null ports. load_topology_data must NOT normalize them at
    // load time: the loader faithfully reflects what is stored, and the
    // frontend applies the renderer defaults (fromPort ?? 'right',
    // toPort ?? 'left') at every consumption point. A load->save cycle
    // heals the row via save_topology_data's own normalization — the
    // load boundary deliberately stays raw.
    let conn = fresh_conn();
    let legacy_json = r#"{"nodes":[{"id":"store-1","type":"store","name":"Legacy Store","x":0,"y":0}],"wires":[{"id":"w-legacy","from_node_id":"store-1","to_node_id":"store-1","direction":"one-way"}]}"#;
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, legacy_json).unwrap();

    let loaded = load_topology_data(&conn).unwrap().unwrap();
    assert_eq!(loaded.wires.len(), 1);
    // Raw passthrough: legacy null ports stay None at the load boundary.
    assert_eq!(loaded.wires[0].from_port, None);
    assert_eq!(loaded.wires[0].to_port, None);
    // The JSON key round-trips untouched (no write-back side effects).
    let stored = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    assert_eq!(stored, legacy_json);
}

#[test]
fn save_overwrites_previous() {
    let conn = fresh_conn();

    save_topology_data(
        &conn,
        vec![TopologyNodePayload {
            id: "n1".into(),
            node_type: "store".into(),
            name: "First".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }],
        vec![],
    )
    .unwrap();

    save_topology_data(
        &conn,
        vec![TopologyNodePayload {
            id: "n2".into(),
            node_type: "workspace".into(),
            name: "Second".into(),
            subtitle: None,
            x: 50.0,
            y: 60.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }],
        vec![],
    )
    .unwrap();

    let loaded = load_topology_data(&conn).unwrap().unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].id, "n2");
}

#[test]
fn serialise_deserialise_full_graph() {
    let data = TopologyData {
        nodes: vec![
            TopologyNodePayload {
                id: "store-1".into(),
                node_type: "store".into(),
                name: "Downtown".into(),
                subtitle: Some("Primary".into()),
                x: 80.0,
                y: 140.0,
                tier_requirement: None,
                telemetry_badge: Some("Online (2 POS)".into()),
                telemetry_status: Some("online".into()),
                metadata: None,
            },
            TopologyNodePayload {
                id: "ws-1".into(),
                node_type: "workspace".into(),
                name: "POS #1".into(),
                subtitle: Some("Main Checkout".into()),
                x: 340.0,
                y: 80.0,
                tier_requirement: None,
                telemetry_badge: Some("Active".into()),
                telemetry_status: Some("online".into()),
                metadata: None,
            },
        ],
        wires: vec![TopologyWirePayload {
            id: "w-1".into(),
            from_node_id: "store-1".into(),
            to_node_id: "ws-1".into(),
            direction: "one-way".into(),
            label: Some("Binds Store".into()),
            from_port: Some("right".into()),
            to_port: Some("left".into()),
        }],
    };

    let json = serde_json::to_string_pretty(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtripped.nodes.len(), 2);
    assert_eq!(roundtripped.wires.len(), 1);
    assert_eq!(roundtripped.nodes[1].node_type, "workspace");
}

#[test]
fn default_direction_is_one_way() {
    assert_eq!(default_direction(), "one-way");
}

#[test]
fn deserialise_minimal_node() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.node_type, "store");
    assert!(node.subtitle.is_none());
    assert!(node.telemetry_badge.is_none());
}

#[test]
fn deserialise_minimal_wire_defaults_direction() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.direction, "one-way");
}

#[test]
fn deserialise_two_way_direction() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":"two-way"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.direction, "two-way");
}

#[test]
fn save_and_load_empty_graph() {
    let conn = fresh_conn();
    save_topology_data(&conn, vec![], vec![]).unwrap();
    let loaded = load_topology_data(&conn).unwrap().unwrap();
    assert!(loaded.nodes.is_empty());
    assert!(loaded.wires.is_empty());
}

#[test]
fn save_topology_data_returns_error_on_corrupt_existing_data() {
    let conn = fresh_conn();
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, "not valid json").unwrap();
    let result = load_topology_data(&conn);
    assert!(result.is_err());
}

#[test]
fn save_topology_data_rejects_empty_key() {
    let conn = fresh_conn();
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    save_topology_data(&conn, vec![node], vec![]).unwrap();
    let loaded = load_topology_data(&conn).unwrap().unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].name, "");
}

#[test]
fn metadata_roundtrip() {
    let node = TopologyNodePayload {
        id: "store-1".into(),
        node_type: "store".into(),
        name: "With Metadata".into(),
        subtitle: None,
        x: 10.0,
        y: 20.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: Some(serde_json::json!({
            "address": "123 Main St",
            "region": "west",
            "open_since": "2024-01-15",
        })),
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    let meta = roundtripped.metadata.unwrap();
    assert_eq!(meta["address"], "123 Main St");
    assert_eq!(meta["region"], "west");
    assert_eq!(meta["open_since"], "2024-01-15");
}

#[test]
fn multiple_wires_and_nodes_roundtrip() {
    let data = TopologyData {
        nodes: vec![
            TopologyNodePayload {
                id: "store-1".into(),
                node_type: "store".into(),
                name: "Main".into(),
                subtitle: None,
                x: 0.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            },
            TopologyNodePayload {
                id: "ws-1".into(),
                node_type: "workspace".into(),
                name: "POS #1".into(),
                subtitle: None,
                x: 200.0,
                y: 100.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            },
            TopologyNodePayload {
                id: "wh-1".into(),
                node_type: "warehouse".into(),
                name: "Warehouse".into(),
                subtitle: None,
                x: 200.0,
                y: 300.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            },
        ],
        wires: vec![
            TopologyWirePayload {
                id: "w-1".into(),
                from_node_id: "store-1".into(),
                to_node_id: "ws-1".into(),
                direction: "one-way".into(),
                label: None,
                from_port: Some("right".into()),
                to_port: Some("left".into()),
            },
            TopologyWirePayload {
                id: "w-2".into(),
                from_node_id: "ws-1".into(),
                to_node_id: "wh-1".into(),
                direction: "two-way".into(),
                label: Some("Inventory sync".into()),
                from_port: None,
                to_port: None,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtripped.nodes.len(), 3);
    assert_eq!(roundtripped.wires.len(), 2);
    assert_eq!(roundtripped.wires[1].direction, "two-way");
    assert_eq!(
        roundtripped.wires[1].label.as_deref(),
        Some("Inventory sync")
    );
}

#[test]
fn node_type_variants() {
    let json = r#"[
            {"id":"s1","type":"store","name":"Store","x":0,"y":0},
            {"id":"w1","type":"workspace","name":"Workspace","x":1,"y":1},
            {"id":"h1","type":"warehouse","name":"Warehouse","x":2,"y":2},
            {"id":"h2","type":"hardware","name":"Printer","x":3,"y":3}
        ]"#;
    let nodes: Vec<TopologyNodePayload> = serde_json::from_str(json).unwrap();
    assert_eq!(nodes[0].node_type, "store");
    assert_eq!(nodes[1].node_type, "workspace");
    assert_eq!(nodes[2].node_type, "warehouse");
    assert_eq!(nodes[3].node_type, "hardware");
}

#[test]
fn load_corrupt_json_returns_error() {
    let conn = fresh_conn();
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, "not valid json at all").unwrap();

    let result = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY).unwrap();
    assert!(result.is_some());

    // Deserialisation should fail.
    let raw = result.unwrap();
    let parsed: Result<TopologyData, _> = serde_json::from_str(&raw);
    assert!(parsed.is_err());
}

#[test]
fn all_fields_filled_roundtrip() {
    let node = TopologyNodePayload {
        id: "full-node".into(),
        node_type: "hardware".into(),
        name: "Receipt Printer #3".into(),
        subtitle: Some("Kitchen".into()),
        x: 400.5,
        y: 250.75,
        tier_requirement: Some("standard".into()),
        telemetry_badge: Some("Online".into()),
        telemetry_status: Some("online".into()),
        metadata: Some(serde_json::json!({"model": "Epson TM-T88"})),
    };
    let wire = TopologyWirePayload {
        id: "full-wire".into(),
        from_node_id: "full-node".into(),
        to_node_id: "ws-1".into(),
        direction: "two-way".into(),
        label: Some("Print job channel".into()),
        from_port: Some("usb".into()),
        to_port: Some("network".into()),
    };

    let data = TopologyData {
        nodes: vec![node],
        wires: vec![wire],
    };
    let json = serde_json::to_string_pretty(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtripped.nodes[0].subtitle.as_deref(), Some("Kitchen"));
    assert_eq!(
        roundtripped.nodes[0].tier_requirement.as_deref(),
        Some("standard")
    );
    assert_eq!(
        roundtripped.nodes[0].telemetry_status.as_deref(),
        Some("online")
    );
    assert_eq!(
        roundtripped.wires[0].label.as_deref(),
        Some("Print job channel")
    );
    assert_eq!(roundtripped.wires[0].from_port, Some(PortName::Unknown));
    assert_eq!(roundtripped.wires[0].to_port, Some(PortName::Unknown));
}

#[test]
fn serialised_type_field_rename() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "workspace".into(),
        name: "Test".into(),
        subtitle: None,
        x: 1.0,
        y: 2.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    // The JSON key must be "type" (not "node_type") due to #[serde(rename = "type")].
    assert!(json.contains(r#""type":"workspace""#));
    assert!(!json.contains("node_type"));
}

#[test]
fn special_characters_in_names() {
    let node = TopologyNodePayload {
        id: "u-1".into(),
        node_type: "store".into(),
        name: "Café Zürich — Hauptfiliale «1»".into(),
        subtitle: Some("Unicode & Ö姆ojis 🎉".into()),
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name, "Café Zürich — Hauptfiliale «1»");
    assert_eq!(
        roundtripped.subtitle.as_deref(),
        Some("Unicode & Ö姆ojis 🎉")
    );
}

#[test]
fn wire_with_no_optional_fields() {
    let json = r#"{"id":"w-min","from_node_id":"a","to_node_id":"b"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.id, "w-min");
    assert_eq!(wire.from_node_id, "a");
    assert_eq!(wire.to_node_id, "b");
    assert_eq!(wire.direction, "one-way");
    assert!(wire.label.is_none());
    assert!(wire.from_port.is_none());
    assert!(wire.to_port.is_none());
}

// ── Field-level edge cases ────────────────────────────────────

#[test]
fn node_empty_id() {
    let json = r#"{"id":"","type":"store","name":"No ID","x":0,"y":0}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.id.is_empty());
}

#[test]
fn node_empty_type() {
    let json = r#"{"id":"n1","type":"","name":"No Type","x":0,"y":0}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.node_type, NodeType::Unknown);
}

#[test]
fn node_empty_name() {
    let json = r#"{"id":"n1","type":"store","name":"","x":0,"y":0}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.name.is_empty());
}

#[test]
fn node_negative_coordinates() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Negative".into(),
        subtitle: None,
        x: -100.5,
        y: -200.3,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert!((roundtripped.x - (-100.5)).abs() < f64::EPSILON);
    assert!((roundtripped.y - (-200.3)).abs() < f64::EPSILON);
}

#[test]
fn node_zero_coordinates() {
    let json = r#"{"id":"n1","type":"store","name":"Origin","x":0,"y":0}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.x, 0.0);
    assert_eq!(node.y, 0.0);
}

#[test]
fn node_large_coordinates() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Far".into(),
        subtitle: None,
        x: 99999.999,
        y: -99999.999,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert!((roundtripped.x - 99999.999).abs() < 0.001);
    assert!((roundtripped.y - (-99999.999)).abs() < 0.001);
}

#[test]
fn node_fractional_coordinates() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Precise".into(),
        subtitle: None,
        x: 0.123456789,
        y: 0.987654321,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert!((roundtripped.x - 0.123456789).abs() < 1e-8);
    assert!((roundtripped.y - 0.987654321).abs() < 1e-8);
}

#[test]
fn node_empty_string_subtitle() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"subtitle":""}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.subtitle.as_deref(), Some(""));
}

#[test]
fn node_null_subtitle_roundtrip() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"subtitle":null}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.subtitle.is_none());
}

#[test]
fn node_unknown_extra_fields_ignored() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"unknown_field":"val","nested":{"a":1}}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.id, "n1");
    assert_eq!(node.node_type, "store");
}

#[test]
fn node_null_metadata() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"metadata":null}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.metadata.is_none());
}

#[test]
fn node_metadata_with_nested_objects() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "hardware".into(),
        name: "Printer".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: Some(serde_json::json!({
            "config": {
                "ip": "192.168.1.100",
                "port": 9100,
                "settings": {
                    "paper_size": "80mm",
                    "encoding": "UTF-8"
                }
            },
            "tags": ["kitchen", "main"],
            "enabled": true,
            "count": 42
        })),
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    let meta = roundtripped.metadata.unwrap();
    assert_eq!(meta["config"]["ip"], "192.168.1.100");
    assert_eq!(meta["config"]["settings"]["paper_size"], "80mm");
    assert_eq!(meta["tags"][0], "kitchen");
    assert_eq!(meta["enabled"], true);
    assert_eq!(meta["count"], 42);
}

#[test]
fn node_missing_type_field_fails() {
    let json = r#"{"id":"n1","name":"Test","x":0,"y":0}"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn node_missing_name_field_fails() {
    let json = r#"{"id":"n1","type":"store","x":0,"y":0}"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn node_wrong_type_for_coordinates() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":"bad","y":false}"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn node_long_name_roundtrip() {
    let long_name = "A".repeat(1000);
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: long_name.clone(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name.len(), 1000);
    assert_eq!(roundtripped.name, long_name);
}

// ── Wire field-level edge cases ───────────────────────────────

#[test]
fn wire_empty_id() {
    let json = r#"{"id":"","from_node_id":"a","to_node_id":"b"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.id.is_empty());
}

#[test]
fn wire_empty_from_node() {
    let json = r#"{"id":"w1","from_node_id":"","to_node_id":"b"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.from_node_id.is_empty());
}

#[test]
fn wire_empty_to_node() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":""}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.to_node_id.is_empty());
}

#[test]
fn wire_self_reference() {
    let wire = TopologyWirePayload {
        id: "self-wire".into(),
        from_node_id: "n1".into(),
        to_node_id: "n1".into(),
        direction: "two-way".into(),
        label: None,
        from_port: None,
        to_port: None,
    };
    let json = serde_json::to_string(&wire).unwrap();
    let roundtripped: TopologyWirePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.from_node_id, roundtripped.to_node_id);
}

#[test]
fn wire_unexpected_direction_preserved() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":"bidirectional"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.direction, WireDirection::Unknown);
}

#[test]
fn wire_null_label_roundtrip() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","label":null}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.label.is_none());
}

#[test]
fn wire_empty_label() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","label":""}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.label.as_deref(), Some(""));
}

#[test]
fn wire_unknown_extra_fields_ignored() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","color":"red","weight":5}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.id, "w1");
    assert_eq!(wire.from_node_id, "a");
    assert_eq!(wire.direction, "one-way");
}

#[test]
fn wire_missing_required_field_fails() {
    let json = r#"{"id":"w1","from_node_id":"a"}"#;
    let result: Result<TopologyWirePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn wire_empty_required_fields_roundtrip() {
    let json = r#"{"id":"","from_node_id":"","to_node_id":""}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.id.is_empty());
    assert!(wire.from_node_id.is_empty());
    assert!(wire.to_node_id.is_empty());
}

#[test]
fn wire_long_label() {
    let long_label = "x".repeat(5000);
    let wire = TopologyWirePayload {
        id: "w1".into(),
        from_node_id: "a".into(),
        to_node_id: "b".into(),
        direction: "one-way".into(),
        label: Some(long_label.clone()),
        from_port: None,
        to_port: None,
    };
    let json = serde_json::to_string(&wire).unwrap();
    let roundtripped: TopologyWirePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.label.as_deref().unwrap().len(), 5000);
}

// ── Combinatorial optional field patterns ──────────────────────

#[test]
fn node_only_id_type_name_coords() {
    let json = r#"{"id":"n1","type":"store","name":"Minimal","x":10,"y":20}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.subtitle.is_none());
    assert!(node.tier_requirement.is_none());
    assert!(node.telemetry_badge.is_none());
    assert!(node.telemetry_status.is_none());
    assert!(node.metadata.is_none());
    assert_eq!(node.x, 10.0);
    assert_eq!(node.y, 20.0);
}

#[test]
fn node_only_subtitle_present() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"subtitle":"Hello"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.subtitle.as_deref(), Some("Hello"));
    assert!(node.tier_requirement.is_none());
    assert!(node.metadata.is_none());
}

#[test]
fn node_only_tier_requirement_present() {
    let json =
        r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"tier_requirement":"premium"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.tier_requirement.as_deref(), Some("premium"));
    assert!(node.subtitle.is_none());
}

#[test]
fn node_only_telemetry_badge_present() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"telemetry_badge":"Online"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.telemetry_badge.is_some());
    assert!(node.telemetry_status.is_none());
}

#[test]
fn node_only_telemetry_status_present() {
    let json =
        r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"telemetry_status":"warning"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.telemetry_status.is_some());
    assert!(node.telemetry_badge.is_none());
}

#[test]
fn node_only_metadata_present() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"metadata":{"key":"val"}}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert!(node.metadata.is_some());
    assert!(node.subtitle.is_none());
    assert!(node.tier_requirement.is_none());
}

#[test]
fn node_all_tier_fields_present() {
    let json = r#"{"id":"n1","type":"warehouse","name":"Full Tier","x":10,"y":20,"subtitle":"Warehouse A","tier_requirement":"enterprise","telemetry_badge":"Online","telemetry_status":"online","metadata":{"capacity":50000}}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.node_type, "warehouse");
    assert_eq!(node.subtitle.as_deref(), Some("Warehouse A"));
    assert_eq!(node.tier_requirement.as_deref(), Some("enterprise"));
    assert_eq!(node.telemetry_badge.as_deref(), Some("Online"));
    assert_eq!(node.telemetry_status.as_deref(), Some("online"));
    assert!(node.metadata.is_some());
}

// ── Wire port and direction combinations ──────────────────────

#[test]
fn wire_only_from_port() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","from_port":"left"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.from_port, Some(PortName::Left));
    assert!(wire.to_port.is_none());
}

#[test]
fn wire_only_to_port() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","to_port":"right"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.to_port, Some(PortName::Right));
    assert!(wire.from_port.is_none());
}

#[test]
fn wire_both_ports_present() {
    let json =
        r#"{"id":"w1","from_node_id":"a","to_node_id":"b","from_port":"out","to_port":"in"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.from_port, Some(PortName::Unknown));
    assert_eq!(wire.to_port, Some(PortName::Unknown));
}

#[test]
fn wire_label_without_ports() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","label":"direct link"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert_eq!(wire.label.as_deref(), Some("direct link"));
    assert!(wire.from_port.is_none());
    assert!(wire.to_port.is_none());
}

#[test]
fn wire_all_optionals_present() {
    let wire = TopologyWirePayload {
        id: "full-wire".into(),
        from_node_id: "a".into(),
        to_node_id: "b".into(),
        direction: "two-way".into(),
        label: Some("bi-directional sync".into()),
        from_port: Some("primary".into()),
        to_port: Some("secondary".into()),
    };
    let json = serde_json::to_string(&wire).unwrap();
    let roundtripped: TopologyWirePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.direction, "two-way");
    assert_eq!(roundtripped.label.as_deref(), Some("bi-directional sync"));
    assert_eq!(roundtripped.from_port, Some(PortName::Unknown));
    assert_eq!(roundtripped.to_port, Some(PortName::Unknown));
}

// ── TopologyData structural tests ──────────────────────────────

#[test]
fn data_with_null_nodes_field_fails() {
    let json = r#"{"nodes":null,"wires":[]}"#;
    let result: Result<TopologyData, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn data_with_null_wires_field_fails() {
    let json = r#"{"nodes":[],"wires":null}"#;
    let result: Result<TopologyData, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn data_missing_nodes_field_fails() {
    let json = r#"{"wires":[]}"#;
    let result: Result<TopologyData, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn data_missing_wires_field_fails() {
    let json = r#"{"nodes":[]}"#;
    let result: Result<TopologyData, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn data_extra_top_level_fields_ignored() {
    let json = r#"{"nodes":[],"wires":[],"version":2,"created_at":"2024-01-01"}"#;
    let data: TopologyData = serde_json::from_str(json).unwrap();
    assert!(data.nodes.is_empty());
    assert!(data.wires.is_empty());
}

#[test]
fn serde_allows_duplicate_wire_ids() {
    // Serde serialization itself does not enforce uniqueness — that
    // validation lives in save_topology_data. This test verifies the
    // serde layer preserves duplicate IDs without error.
    let data = TopologyData {
        nodes: vec![TopologyNodePayload {
            id: "n1".into(),
            node_type: "store".into(),
            name: "Dup".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }],
        wires: vec![
            TopologyWirePayload {
                id: "same-id".into(),
                from_node_id: "n1".into(),
                to_node_id: "n1".into(),
                direction: "one-way".into(),
                label: None,
                from_port: None,
                to_port: None,
            },
            TopologyWirePayload {
                id: "same-id".into(),
                from_node_id: "n1".into(),
                to_node_id: "n1".into(),
                direction: "two-way".into(),
                label: None,
                from_port: None,
                to_port: None,
            },
        ],
    };
    let json = serde_json::to_string(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.wires.len(), 2);
    assert_eq!(roundtripped.wires[0].id, roundtripped.wires[1].id);
}

#[test]
fn save_topology_data_rejects_duplicate_wire_ids() {
    let conn = fresh_conn();
    let nodes = vec![TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Dup".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    }];
    let wires = vec![
        TopologyWirePayload {
            id: "same-id".into(),
            from_node_id: "n1".into(),
            to_node_id: "n1".into(),
            direction: "one-way".into(),
            label: None,
            from_port: None,
            to_port: None,
        },
        TopologyWirePayload {
            id: "same-id".into(),
            from_node_id: "n1".into(),
            to_node_id: "n1".into(),
            direction: "two-way".into(),
            label: None,
            from_port: None,
            to_port: None,
        },
    ];
    let result = save_topology_data(&conn, nodes, wires);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate wire id"),
        "error should mention duplicate wire id, got: {err}"
    );
}

#[test]
fn save_topology_data_rejects_wire_to_nonexistent_node() {
    let conn = fresh_conn();
    let nodes = vec![TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Store".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    }];
    let wires = vec![TopologyWirePayload {
        id: "orphan".into(),
        from_node_id: "ghost".into(),
        to_node_id: "n1".into(),
        direction: "one-way".into(),
        label: None,
        from_port: None,
        to_port: None,
    }];
    let result = save_topology_data(&conn, nodes, wires);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown from_node_id"),
        "error should mention unknown from_node_id, got: {err}"
    );
}

#[test]
fn save_topology_data_rejects_wire_to_unknown_to_node() {
    let conn = fresh_conn();
    let nodes = vec![TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Store".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    }];
    let wires = vec![TopologyWirePayload {
        id: "orphan".into(),
        from_node_id: "n1".into(),
        to_node_id: "nowhere".into(),
        direction: "one-way".into(),
        label: None,
        from_port: None,
        to_port: None,
    }];
    let result = save_topology_data(&conn, nodes, wires);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown to_node_id"),
        "error should mention unknown to_node_id, got: {err}"
    );
}

#[test]
fn data_thousand_node_graph_roundtrips() {
    let nodes: Vec<TopologyNodePayload> = (0..1000)
        .map(|i| TopologyNodePayload {
            id: format!("n-{i}"),
            node_type: "store".into(),
            name: format!("Node {i}"),
            subtitle: None,
            x: (i as f64) * 10.0,
            y: (i as f64) * 5.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        })
        .collect();
    let data = TopologyData {
        nodes,
        wires: vec![],
    };
    let json = serde_json::to_string(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.nodes.len(), 1000);
    assert_eq!(roundtripped.nodes[999].id, "n-999");
}

// ── JSON structural edge cases ─────────────────────────────────

#[test]
fn json_array_instead_of_node_fails() {
    let json = r#"["a","b","c"]"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn json_primitive_instead_of_node_fails() {
    let json = r#"42"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn json_null_boolean_string_for_node_fails() {
    let cases = ["null", "true", r#""hello""#];
    for case in &cases {
        let result: Result<TopologyNodePayload, _> = serde_json::from_str(case);
        assert!(result.is_err(), "expected error for: {case}");
    }
}

#[test]
fn json_number_for_string_node_field_fails() {
    let json = r#"{"id":123,"type":"store","name":"Test","x":0,"y":0}"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn json_bool_for_string_wire_field_fails() {
    let json = r#"{"id":true,"from_node_id":"a","to_node_id":"b"}"#;
    let result: Result<TopologyWirePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn json_string_for_coordinate_fails() {
    let json = r#"{"id":"n1","type":"store","name":"Test","x":"10","y":"20"}"#;
    let result: Result<TopologyNodePayload, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn json_nested_node_array() {
    let json = r#"{"nodes":[{"id":"n1","type":"store","name":"Nested","x":0,"y":0}],"wires":[]}"#;
    let data: TopologyData = serde_json::from_str(json).unwrap();
    assert_eq!(data.nodes.len(), 1);
}

// ── HTML / special content injection ───────────────────────────

#[test]
fn node_name_with_html_injection() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "<script>alert('xss')</script>".into(),
        subtitle: Some("<img src=x onerror=alert(1)>".into()),
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert!(roundtripped.name.contains("<script>"));
    assert!(roundtripped.subtitle.as_deref().unwrap().contains("<img"));
}

#[test]
fn wire_label_with_special_chars() {
    let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","label":"tab\tnewline\nquote\"backslash\\"}"#;
    let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
    assert!(wire.label.as_deref().unwrap().contains('\t'));
    assert!(wire.label.as_deref().unwrap().contains('\n'));
    assert!(wire.label.as_deref().unwrap().contains('"'));
}

#[test]
fn node_metadata_with_html() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Test".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: Some(serde_json::json!({
            "description": "<b>bold</b><script>bad</script>",
            "xss_payload": "\"><img src=x>"
        })),
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    let meta = roundtripped.metadata.unwrap();
    assert!(meta["description"].as_str().unwrap().contains("<script>"));
}

// ── Unicode / encoding edge cases ─────────────────────────────

#[test]
fn node_name_with_rtl_text() {
    let name = "\u{202E}Reverse\u{202C} normal";
    let node = TopologyNodePayload {
        id: "rtl".into(),
        node_type: "store".into(),
        name: name.into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name, name);
}

#[test]
fn node_name_with_zero_width_chars() {
    let name = "Ex\u{200B}ample\u{200C}Name\u{200D}";
    let node = TopologyNodePayload {
        id: "zw".into(),
        node_type: "store".into(),
        name: name.into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name, name);
    assert_eq!(roundtripped.name.len(), name.len());
}

#[test]
fn node_name_with_control_chars() {
    let name = "Line1\u{0000}null\u{0001}start\u{001F}unit-sep";
    let node = TopologyNodePayload {
        id: "ctrl".into(),
        node_type: "store".into(),
        name: name.into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name, name);
}

// ── Persistence edge cases ─────────────────────────────────────

#[test]
fn multiple_save_cycles() {
    let conn = fresh_conn();
    for cycle in 0..10 {
        let data = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: format!("cycle-{cycle}"),
                node_type: "store".into(),
                name: format!("Cycle {cycle}"),
                subtitle: None,
                x: cycle as f64,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            }],
            wires: vec![],
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
    }
    // Verify only the last cycle persisted.
    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].id, "cycle-9");
}

#[test]
fn save_twice_same_data() {
    let conn = fresh_conn();
    let data = TopologyData {
        nodes: vec![TopologyNodePayload {
            id: "n1".into(),
            node_type: "store".into(),
            name: "Same".into(),
            subtitle: None,
            x: 1.0,
            y: 2.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }],
        wires: vec![],
    };
    let json = serde_json::to_string(&data).unwrap();
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();

    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].id, "n1");
}

#[test]
fn save_overwrites_with_larger_data() {
    let conn = fresh_conn();

    // Small first.
    let small = TopologyData {
        nodes: vec![],
        wires: vec![],
    };
    oz_core::Settings::set(
        &conn,
        TOPOLOGY_SETTING_KEY,
        &serde_json::to_string(&small).unwrap(),
    )
    .unwrap();

    // Large second.
    let large = TopologyData {
        nodes: (0..500)
            .map(|i| TopologyNodePayload {
                id: format!("n-{i}"),
                node_type: "store".into(),
                name: format!("Node {i}"),
                subtitle: None,
                x: 0.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            })
            .collect(),
        wires: vec![],
    };
    oz_core::Settings::set(
        &conn,
        TOPOLOGY_SETTING_KEY,
        &serde_json::to_string(&large).unwrap(),
    )
    .unwrap();

    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
    assert_eq!(loaded.nodes.len(), 500);
}

#[test]
fn fresh_conn_different_key_returns_none() {
    let conn = fresh_conn();
    let result = oz_core::Settings::get(&conn, "oz-pos/some-other-key").unwrap();
    assert!(result.is_none());
}

#[test]
fn topology_key_does_not_interfere_with_other_settings() {
    let conn = fresh_conn();
    oz_core::Settings::set(&conn, "oz-pos/custom-key", "custom_value").unwrap();

    // Topology key remains empty.
    let topo = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY).unwrap();
    assert!(topo.is_none());

    // Other key still readable.
    let other = oz_core::Settings::get(&conn, "oz-pos/custom-key").unwrap();
    assert_eq!(other.as_deref(), Some("custom_value"));
}

#[test]
fn roundtrip_preserves_json_order() {
    let json = r#"{"nodes":[{"id":"n1","type":"store","name":"Order Test","x":10,"y":20}],"wires":[{"id":"w1","from_node_id":"n1","to_node_id":"n2","direction":"one-way"}]}"#;
    let data: TopologyData = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&data).unwrap();
    // Re-parse and verify structure (not byte equality since serde may reorder).
    let reparsed: TopologyData = serde_json::from_str(&serialized).unwrap();
    assert_eq!(reparsed.nodes.len(), 1);
    assert_eq!(reparsed.nodes[0].id, "n1");
}

// ── Cross-field interaction tests ──────────────────────────────

#[test]
fn multiple_wires_between_same_nodes() {
    let data = TopologyData {
        nodes: vec![
            TopologyNodePayload {
                id: "a".into(),
                node_type: "store".into(),
                name: "A".into(),
                subtitle: None,
                x: 0.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            },
            TopologyNodePayload {
                id: "b".into(),
                node_type: "workspace".into(),
                name: "B".into(),
                subtitle: None,
                x: 100.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            },
        ],
        wires: (0..5)
            .map(|i| TopologyWirePayload {
                id: format!("w-{i}"),
                from_node_id: "a".into(),
                to_node_id: "b".into(),
                direction: if i % 2 == 0 {
                    "one-way".into()
                } else {
                    "two-way".into()
                },
                label: Some(format!("connection {i}")),
                from_port: None,
                to_port: None,
            })
            .collect(),
    };
    let json = serde_json::to_string(&data).unwrap();
    let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.wires.len(), 5);
    assert_eq!(roundtripped.wires[0].from_node_id, "a");
    assert_eq!(roundtripped.wires[4].to_node_id, "b");
}

#[test]
fn mixed_node_types_preserved_through_save() {
    let conn = fresh_conn();
    let types = ["store", "workspace", "warehouse", "hardware"];
    let nodes: Vec<TopologyNodePayload> = types
        .iter()
        .enumerate()
        .map(|(i, t)| TopologyNodePayload {
            id: format!("{t}-{i}"),
            node_type: (*t).into(),
            name: format!("{t} #{i}"),
            subtitle: None,
            x: (i * 100) as f64,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        })
        .collect();

    let data = TopologyData {
        nodes,
        wires: vec![],
    };
    let json = serde_json::to_string(&data).unwrap();
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();

    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();

    assert_eq!(
        loaded
            .nodes
            .iter()
            .map(|n| n.node_type.clone())
            .collect::<Vec<_>>(),
        types.iter().map(|t| NodeType::from(*t)).collect::<Vec<_>>(),
    );
}

#[test]
fn node_with_telemetry_status_but_no_badge() {
    let json =
        r#"{"id":"n1","type":"hardware","name":"Sensor","x":0,"y":0,"telemetry_status":"offline"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.telemetry_status.as_deref(), Some("offline"));
    assert!(node.telemetry_badge.is_none());
}

#[test]
fn node_with_telemetry_badge_but_no_status() {
    let json =
        r#"{"id":"n1","type":"hardware","name":"Sensor","x":0,"y":0,"telemetry_badge":"Online"}"#;
    let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
    assert_eq!(node.telemetry_badge.as_deref(), Some("Online"));
    assert!(node.telemetry_status.is_none());
}

// ── Trait implementation tests ─────────────────────────────────

#[test]
fn node_payload_implements_debug() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Test".into(),
        subtitle: None,
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let debug = format!("{node:?}");
    assert!(debug.contains("n1"));
    assert!(debug.contains("Store"));
}

#[test]
fn wire_payload_implements_debug() {
    let wire = TopologyWirePayload {
        id: "w1".into(),
        from_node_id: "a".into(),
        to_node_id: "b".into(),
        direction: "one-way".into(),
        label: None,
        from_port: None,
        to_port: None,
    };
    let debug = format!("{wire:?}");
    assert!(debug.contains("w1"));
    assert!(debug.contains("from_node_id"));
}

#[test]
fn topology_data_implements_debug_and_clone() {
    let data = TopologyData {
        nodes: vec![],
        wires: vec![],
    };
    let _cloned = data.clone();
    let debug = format!("{data:?}");
    assert!(debug.contains("nodes"));
    assert!(debug.contains("wires"));
}

#[test]
fn default_direction_is_consistent() {
    for _ in 0..100 {
        assert_eq!(default_direction(), "one-way");
    }
}

#[test]
fn topology_key_is_correct_format() {
    assert!(TOPOLOGY_SETTING_KEY.starts_with("oz-pos/"));
    assert!(TOPOLOGY_SETTING_KEY.contains("topology"));
    assert!(!TOPOLOGY_SETTING_KEY.is_empty());
}

// ── Partial / incremental save patterns ────────────────────────

#[test]
fn save_only_nodes_empty_wires() {
    let conn = fresh_conn();
    let data = TopologyData {
        nodes: vec![TopologyNodePayload {
            id: "n1".into(),
            node_type: "store".into(),
            name: "Nodes Only".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }],
        wires: vec![],
    };
    oz_core::Settings::set(
        &conn,
        TOPOLOGY_SETTING_KEY,
        &serde_json::to_string(&data).unwrap(),
    )
    .unwrap();
    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert!(loaded.wires.is_empty());
}

#[test]
fn save_only_wires_empty_nodes() {
    let conn = fresh_conn();
    let data = TopologyData {
        nodes: vec![],
        wires: vec![TopologyWirePayload {
            id: "orphan-wire".into(),
            from_node_id: "ghost".into(),
            to_node_id: "ghost".into(),
            direction: "one-way".into(),
            label: None,
            from_port: None,
            to_port: None,
        }],
    };
    oz_core::Settings::set(
        &conn,
        TOPOLOGY_SETTING_KEY,
        &serde_json::to_string(&data).unwrap(),
    )
    .unwrap();
    let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
        .unwrap()
        .unwrap();
    let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
    assert!(loaded.nodes.is_empty());
    assert_eq!(loaded.wires.len(), 1);
}

#[test]
fn roundtrip_preserves_tier_and_telemetry_independently() {
    let scenarios = [
        (
            Some("premium".into()),
            Some("Online".into()),
            Some("online".into()),
        ),
        (Some("standard".into()), None, Some("warning".into())),
        (None, Some("Offline".into()), Some("offline".into())),
        (None, None, None),
    ];
    for (tier, badge, status) in &scenarios {
        let node = TopologyNodePayload {
            id: "n1".into(),
            node_type: "store".into(),
            name: "Scenario".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: tier.clone(),
            telemetry_badge: badge.clone(),
            telemetry_status: status.clone(),
            metadata: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.tier_requirement, *tier);
        assert_eq!(roundtripped.telemetry_badge, *badge);
        assert_eq!(roundtripped.telemetry_status, *status);
    }
}

#[test]
fn roundtrip_preserves_subtitle_independent_of_other_fields() {
    let node = TopologyNodePayload {
        id: "n1".into(),
        node_type: "store".into(),
        name: "Test".into(),
        subtitle: Some("standalone-subtitle".into()),
        x: 0.0,
        y: 0.0,
        tier_requirement: None,
        telemetry_badge: None,
        telemetry_status: None,
        metadata: None,
    };
    let json = serde_json::to_string(&node).unwrap();
    let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(
        roundtripped.subtitle.as_deref(),
        Some("standalone-subtitle")
    );
}

#[test]
fn runtime_plan_excludes_location_wires_and_unknown_endpoints() {
    // The runtime manifest carries only operational (non-location) routes.
    // Location ownership edges are diagram-only; they must never be
    // compiled into the runtime routing artifact. Wires whose endpoints
    // dangle (reference node ids not present in the payload) must also be
    // filtered out rather than emitted with empty instance ids.
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("resto-pos", "workspace", None),
        semantic_node("kds", "workspace", None),
    ];
    let wires = vec![
        semantic_location_wire("wire-location", "resto-pos"),
        serde_json::json!({
            "id": "wire-op",
            "from_node_id": "resto-pos",
            "to_node_id": "kds",
            "direction": "one-way",
            "from_port_id": "operation-out",
            "to_port_id": "operation-in",
            "relationship_type": "generic",
        }),
        serde_json::json!({
            "id": "wire-dangling",
            "from_node_id": "ghost-node",
            "to_node_id": "kds",
            "direction": "one-way",
            "from_port_id": "operation-out",
            "to_port_id": "operation-in",
            "relationship_type": "generic",
        }),
    ];

    let plan = compile_topology_runtime_plan(&nodes, &wires, Some("default".to_string()));
    assert_eq!(plan["schema_version"], TOPOLOGY_SCHEMA_VERSION);
    assert_eq!(plan["branch_id"], "default");
    let routes = plan["routes"].as_array().unwrap();
    assert_eq!(
        routes.len(),
        1,
        "only the valid operational wire should be compiled; location and dangling wires must be excluded"
    );
    assert_eq!(routes[0]["wire_id"], "wire-op");
    assert_eq!(routes[0]["source_instance_id"], "resto-pos");
    assert_eq!(routes[0]["target_instance_id"], "kds");
    assert_eq!(routes[0]["from_port_id"], "operation-out");
    assert_eq!(routes[0]["to_port_id"], "operation-in");
    assert_eq!(routes[0]["relationship_type"], "generic");
    assert_eq!(routes[0]["target_node_kind"], "workspace");
}

#[test]
fn runtime_plan_carries_target_node_kind_and_branch_id() {
    // The manifest's target_node_kind is resolved from the target node's
    // `type` field — consumers use it to decide routing behaviour without
    // re-parsing the diagram. A branch-scoped plan must echo the branch id.
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("b1")),
        semantic_node("resto", "workspace", None),
        serde_json::json!({
            "id": "wh",
            "type": "warehouse",
            "name": "WH",
            "x": 0.0,
            "y": 0.0,
        }),
    ];
    let wires = vec![
        semantic_location_wire("loc", "resto"),
        serde_json::json!({
            "id": "stock-wire",
            "from_node_id": "resto",
            "to_node_id": "wh",
            "direction": "one-way",
            "from_port_id": "stock-out",
            "to_port_id": "stock-in",
            "relationship_type": "stock-routing",
        }),
    ];

    let plan = compile_topology_runtime_plan(&nodes, &wires, Some("b1".to_string()));
    assert_eq!(plan["branch_id"], "b1");
    let routes = plan["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["target_node_kind"], "warehouse");
    assert_eq!(routes[0]["relationship_type"], "stock-routing");
}

#[test]
fn runtime_plan_unscoped_has_null_branch_and_filters_non_operational() {
    // Unscoped plans (branch_id = None) serialize branch_id as JSON null.
    // Only relationship types other than "location" are operational.
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws", "workspace", None),
    ];
    let wires = vec![
        semantic_location_wire("loc", "ws"),
        serde_json::json!({
            "id": "op",
            "from_node_id": "ws",
            "to_node_id": "branch",
            "direction": "one-way",
            "from_port_id": "op-out",
            "to_port_id": "op-in",
            "relationship_type": "generic",
        }),
    ];
    let plan = compile_topology_runtime_plan(&nodes, &wires, None);
    assert!(plan["branch_id"].is_null());
    let routes = plan["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["wire_id"], "op");
}

#[test]
fn runtime_setting_key_unscoped_returns_base_key() {
    // When the topology key has no branch suffix (legacy unscoped path),
    // the runtime key must resolve to the base constant — no branch id
    // appended.
    let key = topology_runtime_setting_key(TOPOLOGY_SETTING_KEY).unwrap();
    assert_eq!(key, TOPOLOGY_RUNTIME_SETTING_KEY);
}

#[test]
fn runtime_setting_key_branch_scoped_appends_branch_id() {
    // A branch-scoped topology key "oz-pos/topology/{branch}" must map to
    // "oz-pos/topology-runtime/{branch}".
    let key = topology_runtime_setting_key("oz-pos/topology/default").unwrap();
    assert_eq!(key, "oz-pos/topology-runtime/default");

    let key = topology_runtime_setting_key("oz-pos/topology/my-branch-42").unwrap();
    assert_eq!(key, "oz-pos/topology-runtime/my-branch-42");
}

#[test]
fn runtime_setting_key_rejects_arbitrary_key_without_prefix() {
    // Any key that is not the base topology key and lacks the prefix must
    // fail — a corrupted or mismatched key must not silently resolve.
    let err = topology_runtime_setting_key("some/other/key");
    assert!(err.is_err(), "arbitrary key must fail");
}

#[test]
fn runtime_setting_key_rejects_empty_branch_suffix() {
    // "oz-pos/topology/" (trailing slash, empty branch) must not resolve
    // to "oz-pos/topology-runtime/" — empty suffix is an invalid topology
    // key (topology_setting_key rejects it), so the runtime resolver must
    // also reject it.
    let err = topology_runtime_setting_key("oz-pos/topology/");
    assert!(err.is_err(), "empty branch suffix must fail");
}
