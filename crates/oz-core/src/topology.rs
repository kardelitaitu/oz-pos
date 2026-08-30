//! Semantic validation for the topology graph.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice C2: topology deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: exemplary pure validation engine — vendored contract with fail-closed init (documented INVARIANT), every frontend-parity gate carries a rationale comment (direction deliberately ungated, zero-vs-multiple branch codes split), O(N+W) single indexing pass, closed semantic pairing matrix mirrored at the IPC boundary, purpose/type matrix, Kahn cycle detection, structured TopologyValidation error codes; zero findings
next: none | perf: single index pass per validation
*/
//!
//! The topology graph (nodes + wires as serde_json values) is validated
//! against the shared semantic contract (`topologySemantics.json`) and the
//! ADR #34 typed-connection gates. The contract is VENDORED here in
//! `crates/oz-core/src/topologySemantics.json` (embedded via `include_str!`)
//! so server builds never depend on the UI tree; the UI copy in
//! `ui/src/features/stores/` is kept byte-identical by a parity test and
//! `scripts/verify-topology-parity.py`. This module is the domain-level core
//! of the validation engine: it is Tauri-free and value-level, so any client
//! (desktop Apply, tablet preview, tooling) can run the same gates.
//!
//! The desktop command layer (`apps/desktop-client/.../topology/semantics.rs`)
//! delegates here and maps [`CoreError::TopologyValidation`] onto its own
//! `AppError::TopologyValidation` wire shape.

use serde_json::Value;
use std::sync::OnceLock;

use crate::error::CoreError;

/// Shared semantic pairing contract consumed by the validation engine.
///
/// Vendored into oz-core (see the module doc) so compiling the server never
/// touches the UI tree; `topology.rs` sits next to the file it embeds. The
/// UI copy stays canonical for the TypeScript side, and
/// `tests::vendored_contract_matches_ui_canonical` plus
/// `scripts/verify-topology-parity.py` keep the two byte-identical.
const SHARED_TOPOLOGY_SEMANTICS_JSON: &str = include_str!("topologySemantics.json");

/// Load the shared topology semantics contract JSON as a parsed value.
///
/// The contract is checked-in compile-time JSON; malformed JSON is a
/// developer/build error, so parsing fails closed at first use.
pub fn shared_topology_semantics() -> &'static Value {
    static CONTRACT: OnceLock<Value> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        serde_json::from_str(SHARED_TOPOLOGY_SEMANTICS_JSON)
            // INVARIANT: the vendored topologySemantics.json is a checked-in
            // compile-time contract; malformed JSON is a developer/build
            // error, not runtime user data, so initialization must fail
            // closed. Its parity with the UI copy is enforced by the
            // `vendored_contract_matches_ui_canonical` test — see the
            // INVARIANT rationale directly above.
            .expect("shared topology semantics JSON must be valid")
    })
}

fn shared_port_set_contains(path: &str, port_id: Option<&str>) -> bool {
    let Some(port_id) = port_id else {
        return false;
    };
    shared_topology_semantics()
        .pointer(path)
        .and_then(Value::as_array)
        .is_some_and(|ports| ports.iter().any(|port| port.as_str() == Some(port_id)))
}

/// True when the port id is a warehouse primary input (location-in / operation-in).
pub fn is_warehouse_primary_input_port(port_id: Option<&str>) -> bool {
    shared_port_set_contains("/warehouse/primaryInputs", port_id)
}

/// True when the port id is a warehouse operational input port.
pub fn is_warehouse_operational_input_port(port_id: Option<&str>) -> bool {
    shared_port_set_contains("/warehouse/operationalInputs", port_id)
}

/// True when the (from_port, to_port, relationship_type) triple is a declared
/// semantic pairing in the shared contract.
pub fn shared_semantic_pairing_contains(
    from_port_id: Option<&str>,
    to_port_id: Option<&str>,
    relationship_type: Option<&str>,
) -> bool {
    let Some(pairings) = shared_topology_semantics()
        .get("semanticPairings")
        .and_then(Value::as_array)
    else {
        return false;
    };
    pairings.iter().any(|pairing| {
        value_string(pairing, "source") == from_port_id
            && value_string(pairing, "target") == to_port_id
            && value_string(pairing, "relationshipType") == relationship_type
    })
}

/// Read a string field from a JSON value, returning `None` when absent or non-string.
pub fn value_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// True when the graph carries semantic ownership fields (profile ids or
/// typed wire ports/relationships) rather than legacy geometric payloads.
pub fn has_semantic_fields(nodes: &[Value], wires: &[Value]) -> bool {
    nodes.iter().any(|node| {
        node.get("store_profile_id").is_some()
            || node
                .get("metadata")
                .and_then(|metadata| metadata.get("storeProfileId"))
                .is_some()
    }) || wires.iter().any(|wire| {
        ["from_port_id", "to_port_id", "relationship_type"]
            .iter()
            .any(|key| wire.get(*key).is_some())
    })
}

/// Resolve the single canonical branch profile id when the graph is semantic.
pub fn semantic_branch_profile_id<'a>(nodes: &'a [Value], wires: &[Value]) -> Option<&'a str> {
    if !has_semantic_fields(nodes, wires) {
        return None;
    }
    nodes
        .iter()
        .find(|node| {
            matches!(
                value_string(node, "type"),
                Some("store" | "branch-location")
            )
        })
        .and_then(|node| {
            value_string(node, "store_profile_id").or_else(|| {
                node.get("metadata")
                    .and_then(|metadata| value_string(metadata, "storeProfileId"))
            })
        })
}

fn semantic_type_key(node: &Value) -> &str {
    node.get("metadata")
        .and_then(|metadata| value_string(metadata, "typeKey"))
        .unwrap_or("store-pos")
}

/// The node's semantic `type` field.
pub fn semantic_node_type(node: &Value) -> Option<&str> {
    value_string(node, "type")
}

/// Return true when a geometric wire has no deterministic semantic migration.
/// Known legacy identities remain readable; ambiguous workspace relationships
/// must be repaired in the editor before Apply can persist or compile them.
fn ambiguous_legacy_wire(
    node_by_id: &std::collections::HashMap<&str, &Value>,
    wire: &Value,
) -> bool {
    if ["from_port_id", "to_port_id", "relationship_type"]
        .iter()
        .any(|key| wire.get(*key).is_some())
    {
        return false;
    }
    let Some(from_node) =
        value_string(wire, "from_node_id").and_then(|id| node_by_id.get(id).copied())
    else {
        return false;
    };
    let Some(to_node) = value_string(wire, "to_node_id").and_then(|id| node_by_id.get(id).copied())
    else {
        return false;
    };
    let from_type = semantic_node_type(from_node);
    let to_type = semantic_node_type(to_node);
    let from_type_key = semantic_type_key(from_node);
    let to_type_key = semantic_type_key(to_node);

    !matches!(
        (from_type, from_type_key, to_type, to_type_key),
        (
            Some("store" | "branch-location"),
            _,
            Some("workspace" | "warehouse"),
            _
        ) | (Some("workspace"), _, Some("warehouse"), _)
            | (
                Some("workspace"),
                "restaurant-pos",
                Some("workspace"),
                "kds"
            )
            | (Some("workspace"), "kds", Some("hardware"), _)
    )
}

/// Mirror the frontend's closed semantic pairing matrix at the IPC boundary.
/// Node kinds are checked as well as port ids because callers can invoke the
/// command without going through the canvas drag gate.
fn find_directed_cycle_node(nodes: &[Value], wires: &[Value]) -> Option<String> {
    let mut adjacency: std::collections::HashMap<String, Vec<String>> = nodes
        .iter()
        .filter_map(|node| value_string(node, "id").map(|id| (id.to_owned(), Vec::new())))
        .collect();
    let mut indegree: std::collections::HashMap<String, usize> =
        adjacency.keys().cloned().map(|id| (id, 0)).collect();

    for wire in wires {
        let Some(from_id) = value_string(wire, "from_node_id") else {
            continue;
        };
        let Some(to_id) = value_string(wire, "to_node_id") else {
            continue;
        };
        if !adjacency.contains_key(from_id) || !adjacency.contains_key(to_id) {
            continue;
        }
        let Some(targets) = adjacency.get_mut(from_id) else {
            continue;
        };
        let Some(degree) = indegree.get_mut(to_id) else {
            continue;
        };
        targets.push(to_id.to_owned());
        *degree += 1;
    }

    let mut queue: std::collections::VecDeque<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(node_id) = queue.pop_front() {
        visited += 1;
        for target_id in adjacency.get(&node_id).into_iter().flatten() {
            let Some(degree) = indegree.get_mut(target_id) else {
                continue;
            };
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(target_id.clone());
            }
        }
    }

    if visited == indegree.len() {
        None
    } else {
        indegree
            .into_iter()
            .find(|(_, degree)| *degree > 0)
            .map(|(id, _)| id)
    }
}

/// Mirror the frontend's cycle gate at the IPC boundary.
fn semantic_wire_matches_contract(wire: &Value, from_node: &Value, to_node: &Value) -> bool {
    let from_port = value_string(wire, "from_port_id");
    let to_port = value_string(wire, "to_port_id");
    let relationship = value_string(wire, "relationship_type");
    if !shared_semantic_pairing_contains(from_port, to_port, relationship) {
        return false;
    }
    let from_type_key = semantic_type_key(from_node);
    let to_type_key = semantic_type_key(to_node);
    let from_type = semantic_node_type(from_node);
    let to_type = semantic_node_type(to_node);

    match (from_port, to_port, relationship) {
        (Some("stock-out"), Some("stock-in"), Some("stock-routing")) => {
            to_type == Some("warehouse")
                && ((from_type == Some("workspace")
                    && matches!(from_type_key, "store-pos" | "restaurant-pos"))
                    || from_type == Some("warehouse"))
        }
        (Some("transfer-out"), Some("transfer-in"), Some("inventory-transfer")) => {
            to_type == Some("warehouse")
                && ((from_type == Some("workspace")
                    && matches!(from_type_key, "store-pos" | "restaurant-pos"))
                    || from_type == Some("warehouse"))
        }
        (Some("ticket-out"), Some("ticket-in"), Some("ticket-routing")) => {
            from_type == Some("workspace") && from_type_key == "kds" && to_type == Some("hardware")
        }
        (Some("operation-out"), Some("operation-in"), Some("generic")) => {
            from_type == Some("workspace")
                && ((from_type_key == "restaurant-pos"
                    && to_type == Some("workspace")
                    && to_type_key == "kds")
                    || (from_type_key == "store-pos" && to_type == Some("warehouse")))
        }
        (Some("device-out"), Some("generic-in"), Some("hardware-connection")) => {
            from_type == Some("hardware") && to_type == Some("hardware")
        }
        // The generic pair is retained as a future-facing contract member;
        // no current node emits generic-out, but a valid pair must not be
        // rejected merely because its producer is not yet registered.
        (Some("generic-out"), Some("generic-in"), Some("generic")) => true,
        _ => false,
    }
}

/// Build a structured [`CoreError::TopologyValidation`] failure.
fn topology_validation(
    code: &str,
    node_id: Option<&str>,
    wire_id: Option<&str>,
    port_id: Option<&str>,
    message: impl Into<String>,
) -> CoreError {
    CoreError::TopologyValidation {
        code: code.into(),
        node_id: node_id.map(str::to_owned),
        wire_id: wire_id.map(str::to_owned),
        port_id: port_id.map(str::to_owned),
        message: message.into(),
    }
}

/// Validate the semantic ownership contract for a topology graph.
///
/// Legacy geometric payloads remain readable during migration. A payload that
/// contains semantic ownership fields is validated strictly: it must contain
/// one identified Branch Location, every non-KDS workspace must have exactly
/// one `location-out` to `location-in` edge, and every KDS must have exactly
/// one Restaurant POS operation feed. Geometry and display names are never
/// used to infer ownership here.
pub fn validate_semantic_json(nodes: &[Value], wires: &[Value]) -> Result<(), CoreError> {
    // Index the graph once so the per-wire and per-node gates below are
    // O(N + W) instead of O(N × W). `node_by_id` keeps the FIRST node for
    // each id (matching the previous linear `find` scans); the id sets and
    // the incoming-wire index preserve wire order and first-match
    // semantics, so no gate changes its result or its report order.
    let mut node_by_id: std::collections::HashMap<&str, &Value> = std::collections::HashMap::new();
    for node in nodes {
        if let Some(id) = value_string(node, "id") {
            node_by_id.entry(id).or_insert(node);
        }
    }
    let warehouse_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .filter(|node| semantic_node_type(node) == Some("warehouse"))
        .filter_map(|node| value_string(node, "id"))
        .collect();
    let restaurant_pos_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .filter(|node| semantic_type_key(node) == "restaurant-pos")
        .filter_map(|node| value_string(node, "id"))
        .collect();
    let retail_pos_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .filter(|node| {
            semantic_node_type(node) == Some("workspace") && semantic_type_key(node) == "store-pos"
        })
        .filter_map(|node| value_string(node, "id"))
        .collect();
    let mut incoming_by_target: std::collections::HashMap<&str, Vec<&Value>> =
        std::collections::HashMap::new();
    for wire in wires {
        if let Some(target) = value_string(wire, "to_node_id") {
            incoming_by_target.entry(target).or_default().push(wire);
        }
    }
    if let Some(wire) = wires
        .iter()
        .find(|wire| ambiguous_legacy_wire(&node_by_id, wire))
    {
        return Err(topology_validation(
            "ambiguous-legacy-wire",
            None,
            value_string(wire, "id"),
            None,
            format!(
                "legacy wire {} has no deterministic semantic relationship; repair it in the topology editor",
                value_string(wire, "id").unwrap_or("<unknown>")
            ),
        ));
    }
    if !has_semantic_fields(nodes, wires) {
        return Ok(());
    }

    let branches: Vec<&Value> = nodes
        .iter()
        .filter(|node| {
            matches!(
                value_string(node, "type"),
                Some("store" | "branch-location")
            )
        })
        .collect();
    // Frontend parity: validateTopologyGraph reports `missing-branch-location`
    // for ZERO branches and `multiple-branch-locations` only for MORE than
    // one — collapsing them made a zero-branch graph surface the wrong
    // guidance code to the UI.
    if branches.is_empty() {
        return Err(topology_validation(
            "missing-branch-location",
            None,
            None,
            None,
            "semantic topology requires a Branch Location node".to_string(),
        ));
    }
    if branches.len() > 1 {
        return Err(topology_validation(
            "multiple-branch-locations",
            None,
            None,
            None,
            format!(
                "semantic topology requires exactly one Branch Location, found {}",
                branches.len()
            ),
        ));
    }
    let branch = branches[0];
    let branch_id = value_string(branch, "id").unwrap_or_default();
    let profile_id = value_string(branch, "store_profile_id")
        .or_else(|| {
            branch
                .get("metadata")
                .and_then(|metadata| value_string(metadata, "storeProfileId"))
        })
        .unwrap_or_default();
    if branch_id.is_empty() || profile_id.is_empty() {
        return Err(topology_validation(
            "branch-location-missing-identity",
            Some(branch_id),
            None,
            None,
            "Branch Location requires a canonical store_profile_id",
        ));
    }

    let workspace_ids: Vec<&str> = nodes
        .iter()
        .filter(|node| value_string(node, "type") == Some("workspace"))
        .filter_map(|node| value_string(node, "id"))
        .collect();
    let workspace_id_set: std::collections::HashSet<&str> = workspace_ids.iter().copied().collect();
    let mut seen_location_wires = std::collections::HashSet::new();
    for wire in wires {
        if value_string(wire, "relationship_type") != Some("location") {
            continue;
        }
        let key = (
            value_string(wire, "from_node_id"),
            value_string(wire, "from_port_id"),
            value_string(wire, "to_node_id"),
            value_string(wire, "to_port_id"),
        );
        if !seen_location_wires.insert(key) {
            return Err(topology_validation(
                "duplicate-wire",
                None,
                value_string(wire, "id"),
                None,
                format!(
                    "duplicate semantic location wire: {}",
                    value_string(wire, "id").unwrap_or("<unknown>")
                ),
            ));
        }
        if value_string(wire, "from_node_id") != Some(branch_id)
            || value_string(wire, "from_port_id") != Some("location-out")
            || value_string(wire, "to_port_id") != Some("location-in")
            // Direction is deliberately NOT part of this gate: the frontend
            // contract treats it as presentation-only (one-way | reverse |
            // two-way are all legal — normalizeWireDirection). Rejecting a
            // location wire whose direction was cycled in the editor would
            // be a frontend/backend contract drift.
            || (!workspace_id_set.contains(value_string(wire, "to_node_id").unwrap_or_default())
                && !warehouse_ids.contains(value_string(wire, "to_node_id").unwrap_or_default()))
        {
            return Err(topology_validation(
                "invalid-location-connection",
                None,
                value_string(wire, "id"),
                None,
                format!(
                    "invalid semantic location wire: {}",
                    value_string(wire, "id").unwrap_or("<unknown>")
                ),
            ));
        }
    }

    if let Some(cycle_node) = find_directed_cycle_node(nodes, wires) {
        return Err(topology_validation(
            "cycle-detected",
            Some(&cycle_node),
            None,
            None,
            format!("topology contains a directed cycle involving node {cycle_node}"),
        ));
    }

    for wire in wires {
        if value_string(wire, "relationship_type") == Some("location") {
            continue;
        }
        let Some(from_node) =
            value_string(wire, "from_node_id").and_then(|id| node_by_id.get(id).copied())
        else {
            continue;
        };
        let Some(to_node) =
            value_string(wire, "to_node_id").and_then(|id| node_by_id.get(id).copied())
        else {
            continue;
        };
        let is_kds_operation = value_string(wire, "from_port_id") == Some("operation-out")
            && value_string(wire, "to_port_id") == Some("operation-in")
            && value_string(wire, "relationship_type") == Some("generic")
            && semantic_node_type(to_node) == Some("workspace")
            && semantic_type_key(to_node) == "kds";
        let is_warehouse_operation = value_string(wire, "from_port_id") == Some("operation-out")
            && value_string(wire, "to_port_id") == Some("operation-in")
            && value_string(wire, "relationship_type") == Some("generic")
            && semantic_node_type(to_node) == Some("warehouse");
        if is_kds_operation || is_warehouse_operation {
            continue;
        }
        if !semantic_wire_matches_contract(wire, from_node, to_node) {
            return Err(topology_validation(
                "invalid-semantic-connection",
                None,
                value_string(wire, "id"),
                value_string(wire, "to_port_id"),
                format!(
                    "wire {} has an incompatible semantic connection",
                    value_string(wire, "id").unwrap_or("<unknown>")
                ),
            ));
        }
    }

    for &workspace_id in &workspace_ids {
        let workspace_node = node_by_id.get(workspace_id).copied();
        let purpose_key = workspace_node
            .and_then(|node| node.get("metadata"))
            .and_then(|metadata| value_string(metadata, "purposeKey"))
            .unwrap_or("general");
        let type_key = workspace_node
            .and_then(|node| node.get("metadata"))
            .and_then(|metadata| value_string(metadata, "typeKey"))
            .unwrap_or("store-pos");
        let purpose_valid = matches!(
            (purpose_key, type_key),
            (
                "general",
                "store-pos" | "restaurant-pos" | "kds" | "warehouse"
            ) | ("checkout" | "returns", "store-pos")
                | ("dining-room", "restaurant-pos")
                | ("kitchen-hot-line", "kds")
                | ("stock-control" | "receiving", "warehouse")
        );
        if !purpose_valid {
            return Err(topology_validation(
                "invalid-purpose",
                Some(workspace_id),
                None,
                None,
                format!(
                    "workspace {workspace_id} has unsupported purpose_key {purpose_key} for type_key {type_key}"
                ),
            ));
        }
        let is_kds = type_key == "kds";
        let incoming_slice: &[&Value] = incoming_by_target
            .get(workspace_id)
            .map_or(&[], |wires| wires.as_slice());
        let operation_inputs: Vec<&Value> = incoming_slice
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("generic")
                    && value_string(wire, "to_port_id") == Some("operation-in")
            })
            .copied()
            .collect();
        let incoming = if is_kds {
            operation_inputs.len()
        } else {
            incoming_slice
                .iter()
                .filter(|wire| {
                    value_string(wire, "relationship_type") == Some("location")
                        && value_string(wire, "to_port_id") == Some("location-in")
                })
                .count()
        };
        if incoming != 1 {
            return Err(topology_validation(
                if incoming == 0 {
                    if is_kds {
                        "missing-operation-input"
                    } else {
                        "missing-location-input"
                    }
                } else if is_kds {
                    "multiple-operation-inputs"
                } else {
                    "multiple-location-inputs"
                },
                Some(workspace_id),
                None,
                Some(if is_kds {
                    "operation-in"
                } else {
                    "location-in"
                }),
                format!(
                    "workspace {workspace_id} requires exactly one {} connection, found {incoming}",
                    if is_kds {
                        "Operation In"
                    } else {
                        "Location In"
                    }
                ),
            ));
        }
        if is_kds {
            let operation_wire = operation_inputs[0];
            let source_is_restaurant_pos =
                operation_wire.get("from_port_id").and_then(Value::as_str) == Some("operation-out")
                    && restaurant_pos_ids
                        .contains(value_string(operation_wire, "from_node_id").unwrap_or_default());
            if !source_is_restaurant_pos {
                return Err(topology_validation(
                    "invalid-operation-source",
                    Some(workspace_id),
                    value_string(operation_wire, "id"),
                    Some("operation-in"),
                    format!(
                        "workspace {workspace_id} Operation In must receive operation-out from Restaurant POS"
                    ),
                ));
            }
        }
    }

    // A Stock Room has one primary inbound scope: Branch Location or Retail
    // POS Operation. Stock/transfer routes remain separate operational edges.
    for warehouse in nodes
        .iter()
        .filter(|node| semantic_node_type(node) == Some("warehouse"))
    {
        let warehouse_id = value_string(warehouse, "id").unwrap_or_default();
        let incoming_slice: &[&Value] = incoming_by_target
            .get(warehouse_id)
            .map_or(&[], |wires| wires.as_slice());
        let location_inputs: Vec<&Value> = incoming_slice
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("location")
                    && is_warehouse_primary_input_port(value_string(wire, "to_port_id"))
                    && value_string(wire, "to_port_id") == Some("location-in")
            })
            .copied()
            .collect();
        let operation_inputs: Vec<&Value> = incoming_slice
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("generic")
                    && is_warehouse_primary_input_port(value_string(wire, "to_port_id"))
                    && value_string(wire, "to_port_id") == Some("operation-in")
            })
            .copied()
            .collect();
        let primary_count = location_inputs.len() + operation_inputs.len();
        if primary_count == 0 {
            return Err(topology_validation(
                "missing-warehouse-input",
                Some(warehouse_id),
                None,
                Some("location-in"),
                format!(
                    "warehouse {warehouse_id} requires one Location or Retail POS Operation connection"
                ),
            ));
        }
        if primary_count > 1 {
            let duplicate = operation_inputs
                .first()
                .or_else(|| location_inputs.get(1))
                .copied();
            return Err(topology_validation(
                "multiple-warehouse-inputs",
                Some(warehouse_id),
                duplicate.and_then(|wire| value_string(wire, "id")),
                duplicate.and_then(|wire| value_string(wire, "to_port_id")),
                format!(
                    "warehouse {warehouse_id} accepts only one primary Location or Retail POS Operation connection"
                ),
            ));
        }
        for operation_wire in operation_inputs {
            let source_is_retail_pos = retail_pos_ids
                .contains(value_string(operation_wire, "from_node_id").unwrap_or_default())
                && value_string(operation_wire, "from_port_id") == Some("operation-out");
            if !source_is_retail_pos {
                return Err(topology_validation(
                    "invalid-warehouse-operation-source",
                    Some(warehouse_id),
                    value_string(operation_wire, "id"),
                    Some("operation-in"),
                    format!(
                        "warehouse {warehouse_id} Operation In must receive operation-out from Retail POS"
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "topology_tests.rs"]
mod tests;
