//! Unit tests for the oz-core semantic topology validation engine:
//! the vendored contract parity check, semantic ownership gates,
//! typed-connection gates, and cycle detection.
//!
//! Loaded as the `tests` module of `topology.rs` via `#[path]`; the
//! crate namespace resolves through `use super::*`.

use super::*;
use serde_json::json;

/// The semantic contract is vendored into oz-core so server builds never
/// depend on the UI tree (the `include_str!` above resolves to the local
/// copy). The UI file remains the TypeScript side's source; this test
/// keeps the two byte-identical whenever the full repo is checked out,
/// and skips gracefully in a server-only build context where `ui/` is
/// not part of the source tree (e.g. the Docker builder stage).
#[test]
fn vendored_contract_matches_ui_canonical() {
    let ui_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/src/features/stores/topologySemantics.json");
    if !ui_path.exists() {
        eprintln!("topology parity: ui/ absent (server-only build context) — skipping");
        return;
    }
    let ui_bytes =
        std::fs::read(&ui_path).unwrap_or_else(|e| panic!("read {}: {e}", ui_path.display()));
    assert_eq!(
        SHARED_TOPOLOGY_SEMANTICS_JSON.as_bytes(),
        ui_bytes.as_slice(),
        "vendored crates/oz-core/src/topologySemantics.json drifted from \
         ui/src/features/stores/topologySemantics.json — copy the file \
         across (scripts/verify-topology-parity.py enforces this too)"
    );
}

fn semantic_node(id: &str, node_type: &str, store_profile_id: Option<&str>) -> Value {
    let mut node = json!({
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

fn semantic_location_wire(id: &str, to_node_id: &str) -> Value {
    json!({
        "id": id,
        "from_node_id": "branch",
        "to_node_id": to_node_id,
        "direction": "one-way",
        "from_port_id": "location-out",
        "to_port_id": "location-in",
        "relationship_type": "location",
    })
}

fn validation_code(err: &CoreError) -> &str {
    match err {
        CoreError::TopologyValidation { code, .. } => code,
        other => panic!("expected TopologyValidation, got {other:?}"),
    }
}

#[test]
fn missing_branch_location_is_reported() {
    // A semantic marker is required to reach the branch gate: a pure
    // legacy geometric graph is intentionally accepted (readable during
    // migration), so the workspace carries a store_profile_id.
    let nodes = vec![semantic_node("ws-1", "workspace", Some("default"))];
    let wires = vec![];
    let err = validate_semantic_json(&nodes, &wires).expect_err("must fail without a branch");
    assert_eq!(validation_code(&err), "missing-branch-location");
}

#[test]
fn valid_semantic_graph_passes() {
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
    ];
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    validate_semantic_json(&nodes, &wires).expect("valid graph must pass");
}

#[test]
fn invalid_purpose_key_is_reported() {
    let mut ws = semantic_node("ws-1", "workspace", None);
    ws["metadata"] = json!({ "purposeKey": "dining-room", "typeKey": "store-pos" });
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        ws,
    ];
    let wires = vec![semantic_location_wire("wire-1", "ws-1")];
    let err = validate_semantic_json(&nodes, &wires).expect_err("dining-room needs restaurant-pos");
    assert_eq!(validation_code(&err), "invalid-purpose");
}

#[test]
fn directed_cycle_is_detected() {
    let nodes = vec![
        semantic_node("branch", "branch-location", Some("default")),
        semantic_node("ws-1", "workspace", None),
        semantic_node("ws-2", "workspace", None),
    ];
    let mut wires = vec![
        semantic_location_wire("loc-1", "ws-1"),
        semantic_location_wire("loc-2", "ws-2"),
    ];
    let route = |id: &str, from: &str, to: &str| {
        json!({
            "id": id,
            "from_node_id": from,
            "to_node_id": to,
            "direction": "one-way",
            "from_port_id": "stock-out",
            "to_port_id": "stock-in",
            "relationship_type": "stock-routing",
        })
    };
    wires.push(route("r-1", "ws-1", "ws-2"));
    wires.push(route("r-2", "ws-2", "ws-1"));
    let err = validate_semantic_json(&nodes, &wires).expect_err("cycle must be rejected");
    assert_eq!(validation_code(&err), "cycle-detected");
}

#[test]
fn shared_semantics_contract_parses() {
    let contract = shared_topology_semantics();
    assert!(
        contract
            .pointer("/warehouse/primaryInputs")
            .and_then(Value::as_array)
            .is_some(),
        "contract must expose /warehouse/primaryInputs"
    );
}

#[test]
fn ambiguous_legacy_wire_is_detected() {
    let nodes = [
        semantic_node("ws-1", "workspace", None),
        semantic_node("ws-2", "workspace", None),
    ];
    let wire = json!({
        "id": "g-1",
        "from_node_id": "ws-1",
        "to_node_id": "ws-2",
        "direction": "one-way",
    });
    let node_by_id: std::collections::HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| value_string(node, "id").map(|id| (id, node)))
        .collect();
    assert!(
        ambiguous_legacy_wire(&node_by_id, &wire),
        "workspace-to-workspace geometric wire has no deterministic semantic migration"
    );
}

// ── shared_topology_semantics ─────────────────────────────────────

#[test]
fn shared_topology_semantics_loads_and_has_expected_keys() {
    let sem = shared_topology_semantics();
    assert!(sem.get("warehouse").is_some(), "must have warehouse key");
    assert!(
        sem.get("semanticPairings").is_some(),
        "must have semanticPairings key"
    );
    assert!(
        sem.get("schemaVersion").is_some(),
        "must have schemaVersion key"
    );
}

// ── value_string ──────────────────────────────────────────────────

#[test]
fn value_string_present_string() {
    let val = serde_json::json!({"key": "hello"});
    assert_eq!(value_string(&val, "key"), Some("hello"));
}

#[test]
fn value_string_missing_key() {
    let val = serde_json::json!({"key": "hello"});
    assert_eq!(value_string(&val, "missing"), None);
}

#[test]
fn value_string_non_string_value() {
    let val = serde_json::json!({"key": 42});
    assert_eq!(value_string(&val, "key"), None);
}

#[test]
fn value_string_null_value() {
    let val = serde_json::json!({"key": null});
    assert_eq!(value_string(&val, "key"), None);
}

// ── is_warehouse_primary_input_port ───────────────────────────────

#[test]
fn warehouse_primary_inputs_match_contract() {
    assert!(is_warehouse_primary_input_port(Some("location-in")));
    assert!(is_warehouse_primary_input_port(Some("operation-in")));
}

#[test]
fn warehouse_primary_inputs_reject_operational() {
    assert!(!is_warehouse_primary_input_port(Some("stock-in")));
    assert!(!is_warehouse_primary_input_port(Some("transfer-in")));
}

#[test]
fn warehouse_primary_inputs_reject_none() {
    assert!(!is_warehouse_primary_input_port(None));
}

#[test]
fn warehouse_primary_inputs_reject_unknown() {
    assert!(!is_warehouse_primary_input_port(Some("unknown-port")));
}

// ── is_warehouse_operational_input_port ───────────────────────────

#[test]
fn warehouse_operational_inputs_match_contract() {
    assert!(is_warehouse_operational_input_port(Some("stock-in")));
    assert!(is_warehouse_operational_input_port(Some("transfer-in")));
}

#[test]
fn warehouse_operational_inputs_reject_primary() {
    assert!(!is_warehouse_operational_input_port(Some("location-in")));
    assert!(!is_warehouse_operational_input_port(Some("operation-in")));
}

#[test]
fn warehouse_operational_inputs_reject_none() {
    assert!(!is_warehouse_operational_input_port(None));
}

// ── shared_semantic_pairing_contains ───────────────────────────────

#[test]
fn semantic_pairing_location_match() {
    assert!(shared_semantic_pairing_contains(
        Some("location-out"),
        Some("location-in"),
        Some("location")
    ));
}

#[test]
fn semantic_pairing_stock_routing_match() {
    assert!(shared_semantic_pairing_contains(
        Some("stock-out"),
        Some("stock-in"),
        Some("stock-routing")
    ));
}

#[test]
fn semantic_pairing_ticket_routing_match() {
    assert!(shared_semantic_pairing_contains(
        Some("ticket-out"),
        Some("ticket-in"),
        Some("ticket-routing")
    ));
}

#[test]
fn semantic_pairing_wrong_port_rejected() {
    assert!(!shared_semantic_pairing_contains(
        Some("stock-out"),
        Some("location-in"),
        Some("location")
    ));
}

#[test]
fn semantic_pairing_none_args() {
    assert!(!shared_semantic_pairing_contains(None, None, None));
}

#[test]
fn semantic_pairing_unknown_triple() {
    assert!(!shared_semantic_pairing_contains(
        Some("unknown-out"),
        Some("unknown-in"),
        Some("unknown-rel")
    ));
}

// ── has_semantic_fields ───────────────────────────────────────────

#[test]
fn has_semantic_fields_with_store_profile_id() {
    let nodes = vec![serde_json::json!({"store_profile_id": "sp-1"})];
    assert!(has_semantic_fields(&nodes, &[]));
}

#[test]
fn has_semantic_fields_with_metadata_store_profile_id() {
    let nodes = vec![serde_json::json!({"metadata": {"storeProfileId": "sp-1"}})];
    assert!(has_semantic_fields(&nodes, &[]));
}

#[test]
fn has_semantic_fields_with_wire_ports() {
    let wires = vec![serde_json::json!({"from_port_id": "location-out"})];
    assert!(has_semantic_fields(&[], &wires));
}

#[test]
fn has_semantic_fields_with_wire_relationship_type() {
    let wires = vec![serde_json::json!({"relationship_type": "location"})];
    assert!(has_semantic_fields(&[], &wires));
}

#[test]
fn has_semantic_fields_empty_graph() {
    assert!(!has_semantic_fields(&[], &[]));
}

#[test]
fn has_semantic_fields_legacy_geometric_only() {
    let nodes = vec![serde_json::json!({"id": "n1", "x": 100, "y": 200, "type": "workspace"})];
    let wires = vec![serde_json::json!({"from_node_id": "n1", "to_node_id": "n2"})];
    assert!(!has_semantic_fields(&nodes, &wires));
}

// ── semantic_branch_profile_id ────────────────────────────────────

#[test]
fn semantic_branch_profile_id_from_store_type() {
    let nodes = vec![serde_json::json!({
        "id": "b1", "type": "store", "store_profile_id": "sp-42"
    })];
    assert_eq!(semantic_branch_profile_id(&nodes, &[]), Some("sp-42"));
}

#[test]
fn semantic_branch_profile_id_from_branch_location_type() {
    let nodes = vec![serde_json::json!({
        "id": "b1", "type": "branch-location", "store_profile_id": "sp-99"
    })];
    assert_eq!(semantic_branch_profile_id(&nodes, &[]), Some("sp-99"));
}

#[test]
fn semantic_branch_profile_id_from_metadata() {
    let nodes = vec![serde_json::json!({
        "id": "b1", "type": "store",
        "metadata": {"storeProfileId": "sp-meta"}
    })];
    assert_eq!(semantic_branch_profile_id(&nodes, &[]), Some("sp-meta"));
}

#[test]
fn semantic_branch_profile_id_none_for_workspace() {
    let nodes = vec![serde_json::json!({"id": "w1", "type": "workspace"})];
    assert_eq!(semantic_branch_profile_id(&nodes, &[]), None);
}

#[test]
fn semantic_branch_profile_id_none_for_legacy_graph() {
    // Legacy graph has no semantic fields → has_semantic_fields returns false
    let nodes = vec![serde_json::json!({"id": "n1", "type": "workspace", "x": 0, "y": 0})];
    assert_eq!(semantic_branch_profile_id(&nodes, &[]), None);
}

// ── semantic_node_type ────────────────────────────────────────────

#[test]
fn semantic_node_type_present() {
    let node = serde_json::json!({"type": "warehouse"});
    assert_eq!(semantic_node_type(&node), Some("warehouse"));
}

#[test]
fn semantic_node_type_missing() {
    let node = serde_json::json!({"id": "n1"});
    assert_eq!(semantic_node_type(&node), None);
}

#[test]
fn semantic_node_type_non_string() {
    let node = serde_json::json!({"type": 42});
    assert_eq!(semantic_node_type(&node), None);
}

// ── ADR #45: endpoint predicates come from the contract ───────────
//
// Before ADR #45 the pairing table was shared but the endpoint rules — which
// node kinds may sit on each end of a row — were re-written by hand in four
// places: a Rust `match`, a Rust target-only pre-filter, and two TypeScript
// copies. These tests pin the evaluator that replaced them, which is now the
// only home for those rules.

fn typed_node(id: &str, node_type: &str, type_key: Option<&str>) -> Value {
    let mut node = json!({ "id": id, "type": node_type, "name": id });
    if let Some(type_key) = type_key {
        node["metadata"] = json!({ "typeKey": type_key });
    }
    node
}

fn wire_between(
    from: &Value,
    to: &Value,
    from_port: &str,
    to_port: &str,
    relationship: &str,
) -> Value {
    json!({
        "id": "w1",
        "from_node_id": from["id"],
        "to_node_id": to["id"],
        "from_port_id": from_port,
        "to_port_id": to_port,
        "relationship_type": relationship,
        "direction": "one-way",
    })
}

#[test]
fn node_kind_token_canonicalizes_the_branch_alias_and_workspace_family() {
    assert_eq!(
        node_kind_token(&typed_node("b", "store", None)),
        "branch-location",
        "`store` is the serialized alias; the contract speaks the canonical name"
    );
    assert_eq!(
        node_kind_token(&typed_node("w", "warehouse", None)),
        "warehouse"
    );
    assert_eq!(
        node_kind_token(&typed_node("h", "hardware", None)),
        "hardware"
    );
    assert_eq!(
        node_kind_token(&typed_node("k", "workspace", Some("kds"))),
        "workspace:kds"
    );
    // A workspace with no recorded type key is the Store POS baseline — the
    // same default `semantic_type_key` applies, so the two can never disagree.
    assert_eq!(
        node_kind_token(&typed_node("p", "workspace", None)),
        "workspace:store-pos"
    );
}

#[test]
fn endpoints_admit_declared_pairs_and_the_family_form() {
    // An exact declared pair.
    assert!(pairing_admits_kinds(
        "operation-out",
        "operation-in",
        "workspace:restaurant-pos",
        "workspace:kds"
    ));
    // A token written without a `:` suffix covers the family: the Location row
    // means "any workspace", not an enumeration of every type key.
    assert!(pairing_admits_kinds(
        "location-out",
        "location-in",
        "@branch-root",
        "workspace:store-pos"
    ));
    assert!(pairing_admits_kinds(
        "location-out",
        "location-in",
        "@branch-root",
        "warehouse"
    ));
    // The future-facing generic row carries the wildcard.
    assert!(pairing_admits_kinds(
        "generic-out",
        "generic-in",
        "hardware",
        "warehouse"
    ));
}

#[test]
fn endpoints_refuse_undeclared_pairs_and_unknown_kinds() {
    // The operation row's two admitted pairs are deliberately NOT the cross
    // product of its endpoints: a Store POS operational feed into a KDS is
    // undeclared, and this is why the contract lists pairs instead of sets.
    assert!(!pairing_admits_kinds(
        "operation-out",
        "operation-in",
        "workspace:store-pos",
        "workspace:kds"
    ));
    // A workspace may not originate a ticket feed; only a KDS may.
    assert!(!pairing_admits_kinds(
        "ticket-out",
        "ticket-in",
        "workspace:store-pos",
        "hardware"
    ));
    // Only the Branch Location owns location-out.
    assert!(!pairing_admits_kinds(
        "location-out",
        "location-in",
        "workspace:store-pos",
        "warehouse"
    ));
    // An unregistered workspace type is not authorable until the contract
    // declares it. That is the discipline ADR #45 buys: adding a POS type is a
    // contract edit, and both gates pick it up at once.
    assert!(!pairing_admits_kinds(
        "stock-out",
        "stock-in",
        "workspace:pharmacy-pos",
        "warehouse"
    ));
    // Unknown semantics fail closed, exactly as unknown port ids always have.
    assert!(!pairing_admits_kinds(
        "made-up-out",
        "stock-in",
        "warehouse",
        "warehouse"
    ));
}

#[test]
fn contract_gate_refuses_an_operation_feed_from_a_non_workspace_source() {
    // Regression for ADR #45 §1. The wire loop used to skip the contract check
    // for any operation-in wire whose TARGET was a KDS or a warehouse, without
    // ever inspecting the source — so this wire bypassed semantic validation
    // completely, while the frontend gate refused to offer it. The pre-filter
    // now requires a workspace source, so the gate runs.
    let hardware = typed_node("hw-1", "hardware", None);
    let kds = typed_node("kds-1", "workspace", Some("kds"));
    let wire = wire_between(&hardware, &kds, "operation-out", "operation-in", "generic");
    assert!(
        !semantic_wire_matches_contract(&wire, &hardware, &kds),
        "a hardware-sourced operational feed must not pass the contract gate"
    );
}

#[test]
fn contract_gate_still_admits_both_declared_operation_feeds() {
    let resto = typed_node("resto", "workspace", Some("restaurant-pos"));
    let kds = typed_node("kds", "workspace", Some("kds"));
    let store = typed_node("pos", "workspace", Some("store-pos"));
    let warehouse = typed_node("wh", "warehouse", None);
    assert!(semantic_wire_matches_contract(
        &wire_between(&resto, &kds, "operation-out", "operation-in", "generic"),
        &resto,
        &kds
    ));
    assert!(semantic_wire_matches_contract(
        &wire_between(
            &store,
            &warehouse,
            "operation-out",
            "operation-in",
            "generic"
        ),
        &store,
        &warehouse
    ));
}
