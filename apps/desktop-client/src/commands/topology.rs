//! Tauri commands for persisting the node topology graph.
//!
//! Topology data (nodes + wires) is serialised as JSON and stored in the
//! `settings` table under the key `oz-pos/topology`. On first load, the
//! command returns `None` so the front-end falls back to the built-in
//! retail preset.

use rusqlite::{Connection, Transaction, TransactionBehavior};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use tauri::State;

use oz_core::db::Store;
use oz_core::permissions;
use oz_core::subscription::TenantSubscription;

use crate::commands::authz::require_permission_for_session;
use crate::commands::workspaces::CreateInstanceRequest;
use crate::error::AppError;
use crate::state::AppState;

/// Serialised topology persisted in the settings table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyData {
    /// Nodes in the topology graph (stores, workspaces, warehouses, hardware).
    pub nodes: Vec<TopologyNodePayload>,
    /// Wires (edges) connecting nodes in the topology graph.
    pub wires: Vec<TopologyWirePayload>,
}

// ── Serde helpers for resilience ─────────────────────────────────

/// Serialise an f64, replacing NaN/Infinity with `0.0`.
///
/// serde_json (default) serialises non-finite floats as JSON `null`,
/// which cannot roundtrip back to `f64`.  This guard prevents the
/// entire topology from being poisoned by a single bad coordinate.
fn ser_f64_finite<S>(val: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f64(if val.is_finite() { *val } else { 0.0 })
}

/// Deserialise an f64, mapping JSON `null` to `0.0`.
fn de_f64_or_null<'de, D>(d: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum F64ish {
        Num(f64),
        Null,
    }
    match F64ish::deserialize(d)? {
        F64ish::Num(n) => Ok(n),
        F64ish::Null => Ok(0.0),
    }
}

/// Deserialise a `String`, mapping JSON `null` to the default direction.
///
/// `#[serde(default)]` only kicks in when the field is *absent*, not
/// when it is explicitly `null`.  This helper covers the `null` case.
fn de_direction_or_null<'de, D>(d: D) -> Result<WireDirection, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Dir {
        Some(WireDirection),
        Null,
    }
    match Dir::deserialize(d)? {
        Dir::Some(s) => Ok(s),
        Dir::Null => Ok(default_direction()),
    }
}

// ── Data types ───────────────────────────────────────────────────

/// Valid node types in the topology graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeType {
    /// Retail store branch.
    Store,
    /// POS / register workspace.
    Workspace,
    /// Warehouse / storage location.
    Warehouse,
    /// Printer or peripheral hardware.
    Hardware,
    /// Catch-all for unknown/corrupt node types — rejected on save.
    #[serde(other)]
    Unknown,
}

impl PartialEq<&str> for NodeType {
    fn eq(&self, other: &&str) -> bool {
        match self {
            NodeType::Store => *other == "store",
            NodeType::Workspace => *other == "workspace",
            NodeType::Warehouse => *other == "warehouse",
            NodeType::Hardware => *other == "hardware",
            NodeType::Unknown => false,
        }
    }
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s {
            "store" => NodeType::Store,
            "workspace" => NodeType::Workspace,
            "warehouse" => NodeType::Warehouse,
            "hardware" => NodeType::Hardware,
            _ => NodeType::Unknown,
        }
    }
}

/// Valid wire directions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WireDirection {
    /// One-directional flow (single arrow).
    OneWay,
    /// Bidirectional flow (arrows on both ends).
    TwoWay,
    /// Right-to-left flow (arrow on the from end).
    Reverse,
    /// Catch-all for unknown/corrupt directions — rejected on save.
    #[serde(other)]
    Unknown,
}

impl PartialEq<&str> for WireDirection {
    fn eq(&self, other: &&str) -> bool {
        match self {
            WireDirection::OneWay => *other == "one-way",
            WireDirection::TwoWay => *other == "two-way",
            WireDirection::Reverse => *other == "reverse",
            WireDirection::Unknown => false,
        }
    }
}

impl From<&str> for WireDirection {
    fn from(s: &str) -> Self {
        match s {
            "one-way" => WireDirection::OneWay,
            "two-way" => WireDirection::TwoWay,
            "reverse" => WireDirection::Reverse,
            _ => WireDirection::Unknown,
        }
    }
}

/// Valid port names on a topology node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortName {
    /// Top edge of the node card.
    Top,
    /// Right edge of the node card.
    Right,
    /// Bottom edge of the node card.
    Bottom,
    /// Left edge of the node card.
    Left,
    /// Catch-all for unknown/corrupt port names — rejected on save.
    #[serde(other)]
    Unknown,
}

impl PartialEq<&str> for PortName {
    fn eq(&self, other: &&str) -> bool {
        match self {
            PortName::Top => *other == "top",
            PortName::Right => *other == "right",
            PortName::Bottom => *other == "bottom",
            PortName::Left => *other == "left",
            PortName::Unknown => false,
        }
    }
}

impl From<&str> for PortName {
    fn from(s: &str) -> Self {
        match s {
            "top" => PortName::Top,
            "right" => PortName::Right,
            "bottom" => PortName::Bottom,
            "left" => PortName::Left,
            _ => PortName::Unknown,
        }
    }
}

/// One node in the topology graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNodePayload {
    /// Unique identifier for the node (e.g. "store-1", "ws-main").
    pub id: String,
    /// Node kind: store, workspace, warehouse, or hardware.
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Display name shown on the topology card.
    pub name: String,
    /// Optional subtitle shown below the name.
    #[serde(default)]
    pub subtitle: Option<String>,
    /// X-coordinate of the node on the canvas.
    #[serde(serialize_with = "ser_f64_finite", deserialize_with = "de_f64_or_null")]
    pub x: f64,
    /// Y-coordinate of the node on the canvas.
    #[serde(serialize_with = "ser_f64_finite", deserialize_with = "de_f64_or_null")]
    pub y: f64,
    /// Minimum license tier required to use this node (e.g. "pro").
    #[serde(default)]
    pub tier_requirement: Option<String>,
    /// Badge text shown on the node card (e.g. "Online", "2 POS").
    #[serde(default)]
    pub telemetry_badge: Option<String>,
    /// Status indicator: "online", "offline", or "warning".
    #[serde(default)]
    pub telemetry_status: Option<String>,
    /// Arbitrary JSON metadata (address, region, model, capacity, etc.).
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// One wire connecting two ports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyWirePayload {
    /// Unique identifier for this wire.
    pub id: String,
    /// Node ID that the wire originates from.
    pub from_node_id: String,
    /// Node ID that the wire connects to.
    pub to_node_id: String,
    /// Direction: one-way (default), two-way, or reverse.
    #[serde(default = "default_direction")]
    #[serde(deserialize_with = "de_direction_or_null")]
    pub direction: WireDirection,
    /// Optional label displayed along the wire.
    #[serde(default)]
    pub label: Option<String>,
    /// Source port anchor point (e.g. left, right, top, bottom).
    #[serde(default)]
    pub from_port: Option<PortName>,
    /// Target port anchor point (e.g. left, right, top, bottom).
    #[serde(default)]
    pub to_port: Option<PortName>,
}

fn default_direction() -> WireDirection {
    WireDirection::OneWay
}

// ── Free functions (testable without Tauri runtime) ────────────────

const TOPOLOGY_SETTING_KEY: &str = "oz-pos/topology";
const TOPOLOGY_RUNTIME_SETTING_KEY: &str = "oz-pos/topology-runtime";
const TOPOLOGY_APPLY_RECOVERY_KEY: &str = "oz-pos/topology/apply-recovery";
const TOPOLOGY_APPLY_REQUEST_PREFIX: &str = "oz-pos/topology/apply-request/";
const TOPOLOGY_SCHEMA_VERSION: u64 = 1;
const SHARED_TOPOLOGY_SEMANTICS_JSON: &str =
    include_str!("../../../../ui/src/features/stores/topologySemantics.json");

/// Resolve the branch-scoped runtime plan key paired with a topology key.
fn topology_runtime_setting_key(topology_key: &str) -> Result<String, AppError> {
    if topology_key == TOPOLOGY_SETTING_KEY {
        return Ok(TOPOLOGY_RUNTIME_SETTING_KEY.to_owned());
    }
    let prefix = format!("{TOPOLOGY_SETTING_KEY}/");
    let branch_id = topology_key
        .strip_prefix(&prefix)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::Internal("invalid topology setting key".into()))?;
    Ok(format!("{TOPOLOGY_RUNTIME_SETTING_KEY}/{branch_id}"))
}

/// Compile operational semantic wires into the runtime routing artifact.
///
/// Location ownership edges stay in the diagram contract; operational edges
/// are copied into a branch-scoped manifest consumed by runtime adapters. The
/// manifest deliberately keeps stable instance IDs and semantic port fields,
/// never display names or canvas coordinates.
fn compile_topology_runtime_plan(
    nodes: &[Value],
    wires: &[Value],
    branch_id: Option<String>,
) -> Value {
    let node_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .filter_map(|node| value_string(node, "id"))
        .collect();
    let node_by_id: std::collections::HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| value_string(node, "id").map(|id| (id, node)))
        .collect();
    let routes: Vec<Value> = wires
        .iter()
        .filter(|wire| value_string(wire, "relationship_type") != Some("location"))
        .filter(|wire| {
            node_ids.contains(value_string(wire, "from_node_id").unwrap_or_default())
                && node_ids.contains(value_string(wire, "to_node_id").unwrap_or_default())
        })
        .map(|wire| {
            serde_json::json!({
                "wire_id": value_string(wire, "id").unwrap_or_default(),
                "source_instance_id": value_string(wire, "from_node_id").unwrap_or_default(),
                "target_instance_id": value_string(wire, "to_node_id").unwrap_or_default(),
                "from_port_id": value_string(wire, "from_port_id").unwrap_or_default(),
                "to_port_id": value_string(wire, "to_port_id").unwrap_or_default(),
                "relationship_type": value_string(wire, "relationship_type").unwrap_or_default(),
                "target_node_kind": value_string(
                    node_by_id
                        .get(value_string(wire, "to_node_id").unwrap_or_default())
                        .copied()
                        .unwrap_or(&Value::Null),
                    "type",
                ).unwrap_or_default(),
            })
        })
        .collect();
    serde_json::json!({
        "schema_version": TOPOLOGY_SCHEMA_VERSION,
        "branch_id": branch_id,
        "routes": routes,
    })
}

/// Resolve the settings key for one branch topology.
///
/// The unscoped key remains the compatibility path for legacy callers. New
/// branch-aware callers always use a separate key, so one branch can never
/// overwrite another branch's diagram.
fn topology_setting_key(branch_id: Option<&str>) -> Result<String, AppError> {
    let Some(branch_id) = branch_id else {
        return Ok(TOPOLOGY_SETTING_KEY.to_owned());
    };
    if branch_id.trim().is_empty()
        || branch_id.len() > 200
        || branch_id.chars().any(|ch| ch.is_control() || ch == '/')
    {
        return Err(AppError::Invalid(
            "topology branch id contains invalid characters".into(),
        ));
    }
    Ok(format!("{TOPOLOGY_SETTING_KEY}/{branch_id}"))
}

fn shared_topology_semantics() -> &'static Value {
    static CONTRACT: OnceLock<Value> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        serde_json::from_str(SHARED_TOPOLOGY_SEMANTICS_JSON)
            // INVARIANT: topologySemantics.json is a checked-in compile-time
            // contract; malformed JSON is a developer/build error, not runtime
            // user data, so initialization must fail closed.
            // INVARIANT: checked-in contract JSON is validated at build time.
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

fn is_warehouse_primary_input_port(port_id: Option<&str>) -> bool {
    shared_port_set_contains("/warehouse/primaryInputs", port_id)
}

fn is_warehouse_operational_input_port(port_id: Option<&str>) -> bool {
    shared_port_set_contains("/warehouse/operationalInputs", port_id)
}

fn shared_semantic_pairing_contains(
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

fn topology_apply_request_key(request_id: &str) -> Result<String, AppError> {
    if request_id.trim().is_empty()
        || request_id.len() > 200
        || request_id.chars().any(|ch| ch.is_control() || ch == '/')
    {
        return Err(AppError::Invalid(
            "topology request id contains invalid characters".into(),
        ));
    }
    Ok(format!("{TOPOLOGY_APPLY_REQUEST_PREFIX}{request_id}"))
}

fn topology_revision_from_json(value: &Value) -> u64 {
    value.get("revision").and_then(Value::as_u64).unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn topology_apply_fingerprint(
    store_id: &str,
    branch_id: Option<&str>,
    base_revision: u64,
    workspace_creations: &[CreateInstanceRequest],
    workspace_updates: &[UpdateInstanceRequest],
    workspace_archives: &[String],
    diagram_nodes: &[Value],
    diagram_wires: &[Value],
    resolved_issue_keys: &[String],
) -> Result<String, AppError> {
    let payload = serde_json::json!({
        "store_id": store_id,
        "branch_id": branch_id,
        "base_revision": base_revision,
        "workspace_creations": workspace_creations,
        "workspace_updates": workspace_updates,
        "workspace_archives": workspace_archives,
        "diagram_nodes": diagram_nodes,
        "diagram_wires": diagram_wires,
        "resolved_issue_keys": resolved_issue_keys,
    });
    let bytes = serde_json::to_vec(&payload)
        .map_err(|e| AppError::Internal(format!("serialize topology request: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn topology_apply_ledger_json(revision: u64, fingerprint: &str) -> Result<String, AppError> {
    serde_json::to_string(&serde_json::json!({
        "revision": revision,
        "fingerprint": fingerprint,
    }))
    .map_err(|e| AppError::Internal(format!("serialize topology request ledger: {e}")))
}

fn current_topology_revision(conn: &Connection, setting_key: &str) -> Result<u64, AppError> {
    let Some(raw) = oz_core::Settings::get(conn, setting_key)? else {
        return Ok(0);
    };
    let value: Value = serde_json::from_str(&raw)
        .map_err(|e| AppError::Internal(format!("invalid topology JSON: {e}")))?;
    Ok(topology_revision_from_json(&value))
}

fn topology_envelope_json(
    nodes: &[Value],
    wires: &[Value],
    revision: u64,
    resolved_issue_keys: &[String],
) -> Result<String, AppError> {
    let wires: Vec<Value> = wires
        .iter()
        .cloned()
        .map(|mut wire| {
            if let Some(object) = wire.as_object_mut() {
                object
                    .entry("from_port")
                    .or_insert_with(|| Value::String("right".into()));
                object
                    .entry("to_port")
                    .or_insert_with(|| Value::String("left".into()));
            }
            wire
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "schema_version": TOPOLOGY_SCHEMA_VERSION,
        "revision": revision,
        "nodes": nodes,
        "wires": wires,
        "resolved_issue_keys": resolved_issue_keys,
    }))
    .map_err(|e| AppError::Internal(format!("serialize topology: {e}")))
}

fn value_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Allow a canonical legacy diagram to be read once by its matching branch.
///
/// Unscoped diagrams without a stable `store_profile_id` are intentionally
/// not guessed into a branch: doing so would recreate the cross-branch leak
/// this key split is meant to prevent.
fn legacy_topology_belongs_to_branch(value: &Value, branch_id: &str) -> Result<bool, AppError> {
    let (nodes, wires) = if value.get("schema_version").is_some() {
        validate_topology_envelope(value)?
    } else {
        let object = value
            .as_object()
            .ok_or_else(|| AppError::Internal("topology payload must be an object".into()))?;
        let nodes = object
            .get("nodes")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let wires = object
            .get("wires")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        (nodes, wires)
    };
    Ok(semantic_branch_profile_id(nodes, wires) == Some(branch_id))
}

fn topology_validation(
    code: &str,
    node_id: Option<&str>,
    wire_id: Option<&str>,
    port_id: Option<&str>,
    message: impl Into<String>,
) -> AppError {
    AppError::TopologyValidation {
        code: code.into(),
        node_id: node_id.map(str::to_owned),
        wire_id: wire_id.map(str::to_owned),
        port_id: port_id.map(str::to_owned),
        message: message.into(),
    }
}

fn has_semantic_fields(nodes: &[Value], wires: &[Value]) -> bool {
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

fn semantic_branch_profile_id<'a>(nodes: &'a [Value], wires: &[Value]) -> Option<&'a str> {
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

fn semantic_node_type(node: &Value) -> Option<&str> {
    value_string(node, "type")
}

/// Return true when a geometric wire has no deterministic semantic migration.
/// Known legacy identities remain readable; ambiguous workspace relationships
/// must be repaired in the editor before Apply can persist or compile them.
fn ambiguous_legacy_wire(nodes: &[Value], wire: &Value) -> bool {
    if ["from_port_id", "to_port_id", "relationship_type"]
        .iter()
        .any(|key| wire.get(*key).is_some())
    {
        return false;
    }
    let Some(from_node) = value_string(wire, "from_node_id").and_then(|id| {
        nodes
            .iter()
            .find(|node| value_string(node, "id") == Some(id))
    }) else {
        return false;
    };
    let Some(to_node) = value_string(wire, "to_node_id").and_then(|id| {
        nodes
            .iter()
            .find(|node| value_string(node, "id") == Some(id))
    }) else {
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

/// Validate the semantic ownership contract at the IPC boundary.
///
/// Legacy geometric payloads remain readable during migration. A payload that
/// contains semantic ownership fields is validated strictly: it must contain
/// one identified Branch Location, every non-KDS workspace must have exactly
/// one `location-out` to `location-in` edge, and every KDS must have exactly
/// one Restaurant POS operation feed. Geometry and display names are never
/// used to infer ownership here.
fn validate_semantic_json(nodes: &[Value], wires: &[Value]) -> Result<(), AppError> {
    if let Some(wire) = wires.iter().find(|wire| ambiguous_legacy_wire(nodes, wire)) {
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
            || (!workspace_ids.contains(&value_string(wire, "to_node_id").unwrap_or_default())
                && !nodes.iter().any(|node| {
                    value_string(node, "id") == value_string(wire, "to_node_id")
                        && semantic_node_type(node) == Some("warehouse")
                }))
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

    let node_by_id = |node_id: Option<&str>| {
        node_id.and_then(|id| {
            nodes
                .iter()
                .find(|node| value_string(node, "id") == Some(id))
        })
    };
    for wire in wires {
        if value_string(wire, "relationship_type") == Some("location") {
            continue;
        }
        let Some(from_node) = node_by_id(value_string(wire, "from_node_id")) else {
            continue;
        };
        let Some(to_node) = node_by_id(value_string(wire, "to_node_id")) else {
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

    for workspace_id in workspace_ids {
        let purpose_key = nodes
            .iter()
            .find(|node| value_string(node, "id") == Some(workspace_id))
            .and_then(|node| node.get("metadata"))
            .and_then(|metadata| value_string(metadata, "purposeKey"))
            .unwrap_or("general");
        let type_key = nodes
            .iter()
            .find(|node| value_string(node, "id") == Some(workspace_id))
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
        let operation_inputs: Vec<&Value> = wires
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("generic")
                    && value_string(wire, "to_node_id") == Some(workspace_id)
                    && value_string(wire, "to_port_id") == Some("operation-in")
            })
            .collect();
        let incoming = if is_kds {
            operation_inputs.len()
        } else {
            wires
                .iter()
                .filter(|wire| {
                    value_string(wire, "relationship_type") == Some("location")
                        && value_string(wire, "to_node_id") == Some(workspace_id)
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
                    && nodes.iter().any(|node| {
                        value_string(node, "id") == value_string(operation_wire, "from_node_id")
                            && node
                                .get("metadata")
                                .and_then(|metadata| value_string(metadata, "typeKey"))
                                == Some("restaurant-pos")
                    });
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
        let location_inputs: Vec<&Value> = wires
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("location")
                    && value_string(wire, "to_node_id") == Some(warehouse_id)
                    && is_warehouse_primary_input_port(value_string(wire, "to_port_id"))
                    && value_string(wire, "to_port_id") == Some("location-in")
            })
            .collect();
        let operation_inputs: Vec<&Value> = wires
            .iter()
            .filter(|wire| {
                value_string(wire, "relationship_type") == Some("generic")
                    && value_string(wire, "to_node_id") == Some(warehouse_id)
                    && is_warehouse_primary_input_port(value_string(wire, "to_port_id"))
                    && value_string(wire, "to_port_id") == Some("operation-in")
            })
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
            let source_is_retail_pos = nodes.iter().any(|node| {
                value_string(node, "id") == value_string(operation_wire, "from_node_id")
                    && semantic_node_type(node) == Some("workspace")
                    && semantic_type_key(node) == "store-pos"
                    && value_string(operation_wire, "from_port_id") == Some("operation-out")
            });
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

/// Persist the exact command payload in a versioned graph envelope.
fn validate_topology_envelope(value: &Value) -> Result<(&[Value], &[Value]), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Internal("topology payload must be an object".into()))?;
    if let Some(version) = object.get("schema_version")
        && version.as_u64() != Some(TOPOLOGY_SCHEMA_VERSION)
    {
        return Err(topology_validation(
            "unsupported-schema-version",
            None,
            None,
            None,
            format!("unsupported topology schema version: {}", version),
        ));
    }
    let nodes = object
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Internal("topology payload is missing nodes".into()))?;
    let wires = object
        .get("wires")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Internal("topology payload is missing wires".into()))?;
    Ok((nodes.as_slice(), wires.as_slice()))
}

/// Minimal load-time shape gate.
///
/// The load boundary stays raw: stored nodes and wires must carry only the
/// field the editor cannot operate without — a non-empty `id` (the editor
/// keys every node and wire by id). Display/geometry fields (`name`,
/// `subtitle`, `x`, `y`), directions, ports, and unknown node types are all
/// healed or folded by the frontend load path (normalizeWireDirection,
/// nodeKind's unknown-type fold, port defaults, ghost-wire filtering). Wire
/// endpoints are deliberately NOT required here: the editor's ghost-wire
/// filter drops an endpoint-less wire exactly like a wire with unknown
/// endpoints (already served raw), so requiring endpoint presence would
/// brick a whole topology over one legacy wire. Requiring any of these
/// fields would brick a whole topology over a single legacy row — the same
/// failure the raw-load fixes for corrupt directions and semantic
/// violations were about. The strict typed parse (name/x/y required) still
/// runs at the save/Apply boundary, where the healed value must hold.
fn validate_load_shape(nodes: &[Value], wires: &[Value]) -> Result<(), AppError> {
    for node in nodes {
        require_load_id(node, "node")?;
    }
    for wire in wires {
        require_load_id(wire, "wire")?;
    }
    Ok(())
}

/// A loadable node or wire must be an object with a non-empty id.
fn require_load_id(value: &Value, what: &str) -> Result<(), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Internal(format!("topology {what} must be an object")))?;
    let id = object.get("id").and_then(Value::as_str).unwrap_or_default();
    if id.trim().is_empty() {
        return Err(AppError::Internal(format!(
            "topology {what} is missing a valid id"
        )));
    }
    Ok(())
}

#[cfg(test)]
fn save_topology_json_at_key(
    conn: &Connection,
    nodes: Vec<Value>,
    wires: Vec<Value>,
    setting_key: &str,
) -> Result<u64, AppError> {
    save_topology_json_at_key_with_revision(conn, nodes, wires, setting_key, &[], None, None)
}

fn save_topology_json_at_key_with_revision(
    conn: &Connection,
    nodes: Vec<Value>,
    wires: Vec<Value>,
    setting_key: &str,
    resolved_issue_keys: &[String],
    expected_revision: Option<u64>,
    request: Option<(&str, &str)>,
) -> Result<u64, AppError> {
    validate_semantic_ownership(conn, &nodes, &wires)?;
    // The legacy typed structs validate geometry and known serialized node
    // kinds. `branch-location` is a semantic alias, so normalize only the
    // temporary validation copy; the raw command payload is persisted intact.
    validate_diagram_payloads(&nodes, &wires)?;
    // IMMEDIATE transaction: BEGIN takes the reserved write lock up front, so
    // the revision read + conflict check below are atomic against peer
    // writers. Previously the read ran outside any lock (TOCTOU) — a
    // concurrent writer could commit between this read and this save's
    // commit, and both saves would succeed, silently dropping the peer's
    // revision (lost update). Serializing writers at BEGIN means a save that
    // blocks on a peer re-reads the fresh revision after the peer commits and
    // is rejected with a conflict.
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let current_revision = current_topology_revision(&tx, setting_key)?;
    if let Some(expected) = expected_revision
        && expected != current_revision
    {
        return Err(topology_validation(
            "topology-revision-conflict",
            None,
            None,
            None,
            format!("topology revision conflict: expected {expected}, current {current_revision}"),
        ));
    }
    let revision = current_revision.saturating_add(1);
    let runtime_key = topology_runtime_setting_key(setting_key)?;
    let runtime_branch_id = setting_key
        .strip_prefix(&format!("{TOPOLOGY_SETTING_KEY}/"))
        .map(str::to_owned);
    let runtime_plan = compile_topology_runtime_plan(&nodes, &wires, runtime_branch_id);
    let runtime_json = serde_json::to_string(&runtime_plan)
        .map_err(|e| AppError::Internal(format!("serialize topology runtime plan: {e}")))?;
    let json = topology_envelope_json(&nodes, &wires, revision, resolved_issue_keys)?;
    oz_core::Settings::set(&tx, setting_key, &json)?;
    oz_core::Settings::set(&tx, &runtime_key, &runtime_json)?;
    if let Some((request_key, fingerprint)) = request {
        let ledger = topology_apply_ledger_json(revision, fingerprint)?;
        oz_core::Settings::set(&tx, request_key, &ledger)?;
        oz_core::Settings::remove(&tx, TOPOLOGY_APPLY_RECOVERY_KEY)?;
    }
    tx.commit()?;
    Ok(revision)
}

#[cfg(test)]
/// Test convenience wrapper: unscoped save used only by the unit tests.
///
/// Production's unscoped save is the `save_topology` command with
/// `branch_id: None`, which resolves the same key through
/// `topology_setting_key(None)` and calls `save_topology_json_at_key`
/// directly — this wrapper is a byte-equivalent alias of that exact path
/// (same `TOPOLOGY_SETTING_KEY` constant, same keyed function), kept as a
/// concise abbreviation for the test call sites. Do NOT wire it into
/// production: the command's single key-resolution + single save is the
/// cleaner expression of the unscoped case.
fn save_topology_json(
    conn: &Connection,
    nodes: Vec<Value>,
    wires: Vec<Value>,
) -> Result<(), AppError> {
    save_topology_json_at_key(conn, nodes, wires, TOPOLOGY_SETTING_KEY).map(|_| ())
}

/// Snapshot of a workspace row touched by a topology Apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceApplySnapshot {
    id: String,
    name: String,
    description: String,
    colour: Option<String>,
    purpose_key: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopologyApplyRecovery {
    store_id: String,
    #[serde(default)]
    topology_branch_id: Option<String>,
    creations: Vec<CreateInstanceRequest>,
    snapshots: Vec<WorkspaceApplySnapshot>,
    previous_topology: Option<String>,
    /// Exact canonical diagram JSON expected after the Apply. Recovery uses
    /// it to distinguish a crash before the global write from a crash after
    /// it, because the workspace and global databases cannot share a SQLite
    /// transaction.
    #[serde(default)]
    desired_topology: Option<String>,
}

/// Restore the topology setting after a compensating Apply failure.
///
/// Diagram settings and workspace instances live in separate SQLite
/// databases, so Apply uses a forward-write plus compensation boundary. The
/// restore itself is transactional and preserves the exact prior raw setting,
/// including legacy envelopes.
fn restore_topology_setting(
    conn: &Connection,
    setting_key: &str,
    previous: Option<&str>,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    match previous {
        Some(json) => oz_core::Settings::set(&tx, setting_key, json)?,
        None => {
            oz_core::Settings::remove(&tx, setting_key)?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn persist_topology_recovery(
    conn: &Connection,
    recovery: &TopologyApplyRecovery,
) -> Result<(), AppError> {
    let json = serde_json::to_string(recovery)
        .map_err(|e| AppError::Internal(format!("serialize topology recovery: {e}")))?;
    let tx = conn.unchecked_transaction()?;
    oz_core::Settings::set(&tx, TOPOLOGY_APPLY_RECOVERY_KEY, &json)?;
    tx.commit()?;
    Ok(())
}

fn clear_topology_recovery(conn: &Connection) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    oz_core::Settings::remove(&tx, TOPOLOGY_APPLY_RECOVERY_KEY)?;
    tx.commit()?;
    Ok(())
}

/// Complete a previously interrupted cross-database Apply before accepting a
/// new mutation. The journal is intentionally retained until both databases
/// are restored, making compensation retryable after a process crash or
/// transient database lock.
pub async fn recover_pending_topology_apply_at_startup(state: &AppState) -> Result<(), AppError> {
    let expected_store_id = {
        let db = state.db.lock().await;
        let Some(raw) = oz_core::Settings::get(&db, TOPOLOGY_APPLY_RECOVERY_KEY)? else {
            return Ok(());
        };
        serde_json::from_str::<TopologyApplyRecovery>(&raw)
            .map(|recovery| recovery.store_id)
            .map_err(|e| AppError::Internal(format!("invalid topology recovery journal: {e}")))?
    };
    let _apply_guard = state.topology_apply_lock.lock().await;
    recover_pending_topology_apply(state, &expected_store_id).await
}

async fn recover_pending_topology_apply(
    state: &AppState,
    expected_store_id: &str,
) -> Result<(), AppError> {
    let recovery = {
        let db = state.db.lock().await;
        oz_core::Settings::get(&db, TOPOLOGY_APPLY_RECOVERY_KEY)?
            .map(|json| serde_json::from_str::<TopologyApplyRecovery>(&json))
            .transpose()
            .map_err(|e| AppError::Internal(format!("invalid topology recovery journal: {e}")))?
    };
    let Some(recovery) = recovery else {
        return Ok(());
    };
    if recovery.store_id != expected_store_id {
        return Err(AppError::Internal(format!(
            "topology Apply recovery is pending for store {}, not {}",
            recovery.store_id, expected_store_id
        )));
    }
    // If the desired diagram is already present, the process crashed after
    // the global commit but before clearing the journal. Do not compensate a
    // successful Apply; simply finalize the journal.
    if let Some(desired) = recovery.desired_topology.as_deref() {
        let current = {
            let db = state.db.lock().await;
            let key = topology_setting_key(recovery.topology_branch_id.as_deref())?;
            oz_core::Settings::get(&db, &key)?
        };
        if current.as_deref() == Some(desired) {
            let db = state.db.lock().await;
            clear_topology_recovery(&db)?;
            return Ok(());
        }
    }
    compensate_workspace_diff(
        state,
        &recovery.store_id,
        &recovery.creations,
        &recovery.snapshots,
    )
    .await?;
    {
        let db = state.db.lock().await;
        let setting_key = topology_setting_key(recovery.topology_branch_id.as_deref())?;
        restore_topology_setting(&db, &setting_key, recovery.previous_topology.as_deref())?;
        clear_topology_recovery(&db)?;
    }
    Ok(())
}

/// Capture rows that the workspace portion of Apply will update or archive.
async fn snapshot_workspace_rows(
    state: &AppState,
    store_id: &str,
    updates: &[UpdateInstanceRequest],
    archives: &[String],
) -> Result<Vec<WorkspaceApplySnapshot>, AppError> {
    let conn = state
        .db_manager
        .open_store(store_id)
        .map_err(|e| AppError::Internal(format!("opening store db for compensation: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock for compensation: {e}")))?;
    let mut ids = std::collections::HashSet::new();
    ids.extend(updates.iter().map(|item| item.id.as_str()));
    ids.extend(archives.iter().map(String::as_str));
    let mut snapshots = Vec::with_capacity(ids.len());
    for id in ids {
        let row = db
            .query_row(
                "SELECT id, name, description, colour, purpose_key, status FROM workspace_instances WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(WorkspaceApplySnapshot {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        colour: row.get(3)?,
                        purpose_key: row.get(4)?,
                        status: row.get(5)?,
                    })
                },
            )
            .map_err(|e| AppError::Internal(format!("snapshot workspace {id}: {e}")))?;
        snapshots.push(row);
    }
    Ok(snapshots)
}

/// Compensate workspace mutations after a global diagram write fails.
async fn compensate_workspace_diff(
    state: &AppState,
    store_id: &str,
    creations: &[CreateInstanceRequest],
    snapshots: &[WorkspaceApplySnapshot],
) -> Result<(), AppError> {
    let conn = state
        .db_manager
        .open_store(store_id)
        .map_err(|e| AppError::Internal(format!("opening store db for rollback: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock for rollback: {e}")))?;
    let tx = db.unchecked_transaction()?;
    for creation in creations {
        tx.execute(
            "DELETE FROM workspace_instances WHERE id = ?1",
            rusqlite::params![creation.id],
        )?;
    }
    for snapshot in snapshots {
        tx.execute(
            "UPDATE workspace_instances
             SET name = ?2, description = ?3, colour = ?4, purpose_key = ?5,
                 status = ?6, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            rusqlite::params![
                snapshot.id,
                snapshot.name,
                snapshot.description,
                snapshot.colour,
                snapshot.purpose_key,
                snapshot.status,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Verify the canonical Branch Location exists in the current global database.
fn validate_semantic_ownership(
    conn: &Connection,
    nodes: &[Value],
    wires: &[Value],
) -> Result<(), AppError> {
    validate_semantic_json(nodes, wires)?;
    if !has_semantic_fields(nodes, wires) {
        return Ok(());
    }
    let Some(profile_id) = semantic_branch_profile_id(nodes, wires) else {
        return Ok(());
    };
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM store_profiles WHERE id = ?1)",
        rusqlite::params![profile_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(topology_validation(
            "unknown-branch-location",
            None,
            None,
            None,
            format!("Branch Location references unknown store_profile_id: {profile_id}"),
        ));
    }
    Ok(())
}

/// Pre-mutation validation gate for a topology Apply.
///
/// Rejects malformed diagrams BEFORE any workspace creation, update, or
/// archival. The semantic ownership checks are DB-backed (branch identity
/// must exist); the structural checks (duplicate node/wire ids, unknown
/// node types, unknown directions/ports, ghost endpoints) must also run
/// here — running them only at the final save would let a malformed
/// diagram mutate workspace rows and then fail at save, forcing the
/// compensation cycle to unwind a partial apply.
fn validate_apply_gate(
    conn: &Connection,
    nodes: &[Value],
    wires: &[Value],
) -> Result<(), AppError> {
    // Production Apply is the strict semantic boundary. Legacy geometric
    // payloads remain readable by the low-level load/save compatibility
    // helpers, but they must not bypass ownership and entitlement checks on
    // the authenticated mutation command.
    if !has_semantic_fields(nodes, wires) {
        return Err(topology_validation(
            "semantic-contract-required",
            None,
            None,
            None,
            "topology Apply requires canonical semantic node and wire fields",
        ));
    }
    validate_semantic_ownership(conn, nodes, wires)?;
    validate_diagram_payloads(nodes, wires)
}

fn validate_warehouse_quota(
    nodes: &[Value],
    tier: &oz_core::subscription::SubscriptionTier,
) -> Result<(), AppError> {
    if let Some(limit) = tier.max_warehouses()
        && nodes
            .iter()
            .filter(|node| value_string(node, "type") == Some("warehouse"))
            .count() as i64
            > limit
    {
        return Err(AppError::PermissionDenied(format!(
            "topology warehouse quota exceeded: limit {limit}"
        )));
    }
    Ok(())
}

/// Enforce the backend-owned warehouse capacity invariant for tiers that
/// expose capacity-aware routing. UI validation remains useful feedback, but
/// a direct IPC caller must not be able to route stock into a full warehouse.
fn validate_warehouse_capacity(
    nodes: &[Value],
    wires: &[Value],
    tier: &oz_core::subscription::SubscriptionTier,
    resolved_issue_keys: &[String],
) -> Result<(), AppError> {
    if !matches!(
        tier,
        oz_core::subscription::SubscriptionTier::Pro
            | oz_core::subscription::SubscriptionTier::Premium
            | oz_core::subscription::SubscriptionTier::Enterprise
    ) {
        return Ok(());
    }
    for warehouse in nodes
        .iter()
        .filter(|node| semantic_node_type(node) == Some("warehouse"))
    {
        let Some(metadata) = warehouse.get("metadata") else {
            continue;
        };
        let Some(stock) = metadata.get("stock").and_then(Value::as_f64) else {
            continue;
        };
        let Some(capacity) = metadata.get("capacity").and_then(Value::as_f64) else {
            continue;
        };
        let warehouse_id = value_string(warehouse, "id");
        if stock >= capacity
            && let Some(wire) = wires.iter().find(|wire| {
                value_string(wire, "to_node_id") == warehouse_id
                    && is_warehouse_operational_input_port(value_string(wire, "to_port_id"))
                    && matches!(
                        value_string(wire, "relationship_type"),
                        Some("stock-routing" | "inventory-transfer")
                    )
            })
        {
            return Err(topology_validation(
                "warehouse-at-capacity",
                warehouse_id,
                value_string(wire, "id"),
                value_string(wire, "to_port_id"),
                format!(
                    "warehouse {} is at capacity ({stock}/{capacity})",
                    warehouse_id.unwrap_or("<unknown>")
                ),
            ));
        }

        // A capacity-aware warehouse with room must have an operational
        // stock/transfer route unless the user explicitly dismissed this
        // branch-scoped prompt in the topology document. This mirrors the
        // frontend contract but remains authoritative for direct IPC callers.
        if stock < capacity {
            let has_operational_route = wires.iter().any(|wire| {
                value_string(wire, "to_node_id") == warehouse_id
                    && is_warehouse_operational_input_port(value_string(wire, "to_port_id"))
                    && matches!(
                        value_string(wire, "relationship_type"),
                        Some("stock-routing" | "inventory-transfer")
                    )
            });
            let issue_key = format!(
                "node:{}:topology-validation-warehouse-missing-stock-routing",
                warehouse_id.unwrap_or_default()
            );
            if !has_operational_route && !resolved_issue_keys.iter().any(|key| key == &issue_key) {
                return Err(topology_validation(
                    "warehouse-missing-stock-routing",
                    warehouse_id,
                    None,
                    None,
                    format!(
                        "warehouse {} has capacity but no operational stock or transfer route",
                        warehouse_id.unwrap_or("<unknown>")
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Parse raw diagram values into the legacy typed payloads and run the
/// structural validator (duplicate ids, unknown types/directions/ports,
/// ghost endpoints) without persisting them. `branch-location` is a
/// semantic alias, so normalize it only for the temporary validation copy;
/// the raw command payload is persisted intact.
fn validate_diagram_payloads(nodes: &[Value], wires: &[Value]) -> Result<(), AppError> {
    let typed_node_values: Vec<Value> = nodes
        .iter()
        .map(|node| {
            let mut node = node.clone();
            if node.get("type").and_then(Value::as_str) == Some("branch-location") {
                node["type"] = Value::String("store".into());
            }
            node
        })
        .collect();
    let typed_nodes: Vec<TopologyNodePayload> =
        serde_json::from_value(Value::Array(typed_node_values))
            .map_err(|e| AppError::Internal(format!("invalid topology nodes: {e}")))?;
    let typed_wires: Vec<TopologyWirePayload> =
        serde_json::from_value(Value::Array(wires.to_vec()))
            .map_err(|e| AppError::Internal(format!("invalid topology wires: {e}")))?;
    // Reuse the existing structural validator without persisting its legacy
    // representation — the save callers write the raw command payload intact.
    validate_topology_structure(&typed_nodes, &typed_wires)
}

/// Validate typed node and wire structure without persisting it.
fn validate_topology_structure(
    nodes: &[TopologyNodePayload],
    wires: &[TopologyWirePayload],
) -> Result<(), AppError> {
    let mut node_ids = std::collections::HashSet::new();
    for node in nodes {
        if !node_ids.insert(&node.id) {
            return Err(AppError::Internal(format!(
                "duplicate node id: {}",
                node.id
            )));
        }
        if node.node_type == NodeType::Unknown {
            return Err(AppError::Internal(format!(
                "node {} has unknown type",
                node.id
            )));
        }
    }
    let mut wire_ids = std::collections::HashSet::new();
    for wire in wires {
        if !wire_ids.insert(&wire.id) {
            return Err(AppError::Internal(format!(
                "duplicate wire id: {}",
                wire.id
            )));
        }
        if wire.direction == WireDirection::Unknown {
            return Err(AppError::Internal(format!(
                "wire {} has unknown direction",
                wire.id
            )));
        }
        if wire.from_port == Some(PortName::Unknown) || wire.to_port == Some(PortName::Unknown) {
            return Err(AppError::Internal(format!(
                "wire {} has unknown port",
                wire.id
            )));
        }
        if !node_ids.contains(&wire.from_node_id) {
            return Err(AppError::Internal(format!(
                "wire {} references unknown from_node_id: {}",
                wire.id, wire.from_node_id
            )));
        }
        if !node_ids.contains(&wire.to_node_id) {
            return Err(AppError::Internal(format!(
                "wire {} references unknown to_node_id: {}",
                wire.id, wire.to_node_id
            )));
        }
    }
    Ok(())
}

/// Serialise and persist topology data to the settings store.
///
/// Writes the nodes + wires payloads as JSON under the
/// `oz-pos/topology` key. Any previous topology is overwritten.
/// The write is wrapped in a transaction to satisfy the project
/// rule that all database writes must occur inside a transaction.
///
/// # Validation
///
/// - Wire IDs must be unique within the topology.
/// - Wire `from_node_id` and `to_node_id` must reference existing nodes.
pub fn save_topology_data(
    conn: &Connection,
    nodes: Vec<TopologyNodePayload>,
    wires: Vec<TopologyWirePayload>,
) -> Result<(), AppError> {
    // Normalize null ports to the editor's renderer defaults so the DB
    // never stores a wire with null from/to ports — the frontend loader
    // maps null -> undefined, forcing every consumer (e.g. the frontend
    // duplicate-wire detector) to re-apply these same defaults
    // (fromPort ?? 'right', toPort ?? 'left'). Done BEFORE validation so
    // the port checks below see the values that will actually be stored.
    let wires: Vec<TopologyWirePayload> = wires
        .into_iter()
        .map(|mut w| {
            // get_or_insert fills ONLY None — explicitly-set ports (e.g. a
            // bottom/top anchor chosen in the editor) survive untouched.
            w.from_port.get_or_insert(PortName::Right);
            w.to_port.get_or_insert(PortName::Left);
            w
        })
        .collect();

    // Validate wire IDs are unique.
    let mut seen_wire_ids = std::collections::HashSet::new();
    for wire in &wires {
        if !seen_wire_ids.insert(&wire.id) {
            return Err(AppError::Internal(format!(
                "duplicate wire id: {}",
                wire.id
            )));
        }
    }

    // Validate node IDs are unique.
    //
    // Without this, the `node_ids` HashSet built below would silently
    // collapse duplicate node ids, making wire endpoint resolution
    // ambiguous (a wire pointing at "n1" could resolve to either
    // duplicate). This mirrors the wire-id uniqueness check.
    let mut seen_node_ids = std::collections::HashSet::new();
    for node in &nodes {
        if !seen_node_ids.insert(&node.id) {
            return Err(AppError::Internal(format!(
                "duplicate node id: {}",
                node.id
            )));
        }
    }

    // Validate node types are known (reject #[serde(other)]).
    for node in &nodes {
        if node.node_type == NodeType::Unknown {
            return Err(AppError::Internal(format!(
                "node {} has unknown type",
                node.id
            )));
        }
    }

    // Validate wire directions and ports are known.
    for wire in &wires {
        if wire.direction == WireDirection::Unknown {
            return Err(AppError::Internal(format!(
                "wire {} has unknown direction",
                wire.id
            )));
        }
        if wire.from_port == Some(PortName::Unknown) {
            return Err(AppError::Internal(format!(
                "wire {} has unknown from_port",
                wire.id
            )));
        }
        if wire.to_port == Some(PortName::Unknown) {
            return Err(AppError::Internal(format!(
                "wire {} has unknown to_port",
                wire.id
            )));
        }
    }

    // Validate wire endpoints reference existing nodes.
    let node_ids: std::collections::HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    for wire in &wires {
        if !node_ids.contains(wire.from_node_id.as_str()) {
            return Err(AppError::Internal(format!(
                "wire {} references unknown from_node_id: {}",
                wire.id, wire.from_node_id
            )));
        }
        if !node_ids.contains(wire.to_node_id.as_str()) {
            return Err(AppError::Internal(format!(
                "wire {} references unknown to_node_id: {}",
                wire.id, wire.to_node_id
            )));
        }
    }

    let data = TopologyData { nodes, wires };
    let json = serde_json::to_string(&serde_json::json!({
        "schema_version": TOPOLOGY_SCHEMA_VERSION,
        "nodes": data.nodes,
        "wires": data.wires,
    }))
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let tx = conn.unchecked_transaction()?;
    oz_core::Settings::set(&tx, TOPOLOGY_SETTING_KEY, &json)?;
    tx.commit()?;
    Ok(())
}

/// Load and deserialise persisted topology data.
///
/// Returns `None` when no topology has been saved yet.
///
/// Returns `None` when no topology has been saved yet.
///
/// # Why ports stay raw on the load side
///
/// This function deliberately does **not** normalize legacy null wire ports
/// (rows written before `save_topology_data` gained its `get_or_insert`
/// defaults). The loader is a faithful reflection of what is stored —
/// normalizing here would mask rows that still need healing, and the
/// frontend applies the renderer defaults (`fromPort ?? 'right'`, `toPort ??
/// 'left'`) at every consumption point anyway. A load -> save cycle heals a
/// legacy row via the save-side normalization; the load boundary stays raw.
/// Pinned by the `..._preserves_raw_legacy_null_ports` test below.
pub fn load_topology_data(conn: &Connection) -> Result<Option<TopologyData>, AppError> {
    let raw = oz_core::Settings::get(conn, TOPOLOGY_SETTING_KEY)?;
    match raw {
        Some(json) => {
            let value: Value =
                serde_json::from_str(&json).map_err(|e| AppError::Internal(e.to_string()))?;
            let data_value = if value.get("schema_version").is_some() {
                validate_topology_envelope(&value)?;
                serde_json::json!({
                    "nodes": value.get("nodes").cloned().unwrap_or(Value::Array(vec![])),
                    "wires": value.get("wires").cloned().unwrap_or(Value::Array(vec![])),
                })
            } else {
                value
            };
            let data: TopologyData = serde_json::from_value(data_value)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}

// ── Commands ───────────────────────────────────────────────────────

/// Return whether the authenticated session can save topology changes.
///
/// The frontend uses this capability probe for UI gating; the Apply command
/// repeats the permission check server-side and remains authoritative.
#[tauri::command]
pub async fn can_save_topology(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::STAFF_UPDATE).await?;
    Ok(true)
}

/// Test-only compatibility harness for the retired direct topology writer.
///
/// Production topology persistence is exclusively `apply_topology_diff`, which
/// performs authorization, revision checks, workspace diffing, and recovery
/// journaling. Keeping this helper under `cfg(test)` preserves low-level
/// command round-trip coverage without exposing a second production write
/// path through Tauri IPC.
#[cfg(test)]
async fn save_topology(
    nodes: Vec<Value>,
    wires: Vec<Value>,
    branch_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let setting_key = topology_setting_key(branch_id.as_deref())?;
    let conn = state.db.lock().await;
    save_topology_json_at_key(&conn, nodes, wires, &setting_key).map(|_| ())
}

/// Load the persisted topology graph.
///
/// Returns `None` when no topology has been saved yet (the front-end
/// should fall back to the built-in retail preset).
///
/// # Load boundary stays raw
///
/// Stored values are served raw so the frontend's documented load-time
/// healing (normalizeWireDirection, ghost-wire filtering, port defaults)
/// can run — mirroring `load_topology_data`. Structure is enforced at the
/// save boundary (`save_topology_json_at_key`), where the healed value must hold.
/// Do NOT re-add `validate_topology_structure` here: a single stored
/// corrupt value would brick the whole topology instead of letting the
/// editor repair it.
#[tauri::command]
pub async fn load_topology(
    branch_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<Value>, AppError> {
    let setting_key = topology_setting_key(branch_id.as_deref())?;
    let conn = state.db.lock().await;
    let raw = match oz_core::Settings::get(&conn, &setting_key)? {
        Some(json) => Some(json),
        None => {
            // Migrate only an old diagram whose canonical branch identity
            // proves it belongs to this branch. Ambiguous legacy geometry is
            // left unassigned rather than leaked into every branch.
            let Some(branch_id) = branch_id.as_deref() else {
                return Ok(None);
            };
            let Some(legacy_json) = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)? else {
                return Ok(None);
            };
            let value: Value = serde_json::from_str(&legacy_json)
                .map_err(|e| AppError::Internal(format!("invalid topology JSON: {e}")))?;
            if legacy_topology_belongs_to_branch(&value, branch_id)? {
                Some(legacy_json)
            } else {
                None
            }
        }
    };
    let Some(json) = raw else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(&json)
        .map_err(|e| AppError::Internal(format!("invalid topology JSON: {e}")))?;
    let (nodes, wires) = validate_topology_envelope(&value)?;
    // Minimal shape gate only: stored nodes and wires must carry the id the
    // editor keys by (see validate_load_shape for the rationale). Neither
    // the closed-union structural gate (validate_topology_structure) NOR the
    // semantic-ownership gate (validate_semantic_ownership) runs at load:
    // the frontend contract heals healable corruption at the editor load
    // path (normalizeWireDirection, ghost-wire filtering, port defaults)
    // and surfaces contract violations (missing-location-input etc.) as
    // Apply-time toasts the user repairs in the editor — the free function
    // load_topology_data is documented raw-by-design ("the load boundary
    // stays raw"). Rejecting a stored row for display-level gaps would
    // brick the whole topology instead of letting the editor repair it.
    // Both gates run at the save/Apply boundary (save_topology_json_at_key), where
    // the healed value must hold.
    validate_load_shape(nodes, wires)?;
    Ok(Some(value))
}

/// Request body for updating a workspace instance within a topology diff.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateInstanceRequest {
    /// Instance ID to update.
    pub id: String,
    /// New display name.
    pub name: String,
    /// New controlled business purpose, when changed.
    #[serde(default)]
    pub purpose_key: Option<String>,
}

/// Result returned after a topology Apply commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyApplyResult {
    /// Revision assigned to the committed branch topology.
    pub revision: u64,
}

/// Apply a full topology diff atomically (Critical #4).
///
/// Creates, updates, and archives workspace instances within a single
/// SQLite transaction on the store database, then saves the topology
/// diagram (nodes + wires) on the global database.
///
/// # Transaction guarantee
///
/// All workspace instance mutations (create, update, archive) execute
/// inside a single SQLite transaction. If any operation fails, the
/// entire set of workspace changes rolls back. The create step runs its
/// INSERT SQL *directly* on the outer transaction rather than delegating
/// to `Store::create_workspace_instance` — that helper opens its own
/// `unchecked_transaction` (`BEGIN`), which SQLite rejects with "cannot
/// start a transaction within a transaction" when nested (see the
/// `create_workspace_instance_cannot_nest_in_open_transaction` test in
/// oz-core). The update and archive steps delegate to
/// `Store::{update_workspace_instance,archive_instance}`, which use
/// `Connection::execute` directly and therefore compose safely inside
/// the outer transaction.
///
/// The topology diagram save is a separate step on the global DB. The command
/// snapshots the affected workspace rows and previous diagram, then compensates
/// both databases if the second write fails. A compensation failure is returned
/// explicitly so the caller can surface an operator-recovery condition.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn apply_topology_diff(
    session_token: String,
    workspace_creations: Vec<CreateInstanceRequest>,
    workspace_updates: Vec<UpdateInstanceRequest>,
    workspace_archives: Vec<String>,
    diagram_nodes: Vec<Value>,
    diagram_wires: Vec<Value>,
    branch_id: Option<String>,
    base_revision: u64,
    request_id: String,
    resolved_issue_keys: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<TopologyApplyResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let _apply_guard = state.topology_apply_lock.lock().await;
    let topology_key = topology_setting_key(branch_id.as_deref())?;
    let request_key = topology_apply_request_key(&request_id)?;
    let resolved_issue_keys = resolved_issue_keys.unwrap_or_default();
    let request_fingerprint = topology_apply_fingerprint(
        &session.store_id,
        branch_id.as_deref(),
        base_revision,
        &workspace_creations,
        &workspace_updates,
        &workspace_archives,
        &diagram_nodes,
        &diagram_wires,
        &resolved_issue_keys,
    )?;

    // Authorization: workspace topology changes require admin access. The
    // session user's identity + role live in the GLOBAL identity DB — the
    // store-scoped DB below has an empty `users` table by design, so the
    // gate MUST run here against the global DB. (Authorizing against the
    // store connection would deny every caller — owner included — with
    // "user not found".)
    require_permission_for_session(&state, &session, permissions::STAFF_UPDATE).await?;

    // A retried request returns the original result without repeating any
    // workspace mutation. The process-wide Apply lock also makes the
    // revision check and this ledger lookup deterministic.
    {
        let global_db = state.db.lock().await;
        if let Some(raw) = oz_core::Settings::get(&global_db, &request_key)? {
            let value: Value = serde_json::from_str(&raw)
                .map_err(|e| AppError::Internal(format!("invalid topology request ledger: {e}")))?;
            if let Some(stored_fingerprint) = value.get("fingerprint").and_then(Value::as_str) {
                if stored_fingerprint != request_fingerprint {
                    return Err(AppError::Invalid(
                        "topology request id was already used for a different Apply".into(),
                    ));
                }
                let revision = value
                    .get("revision")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        AppError::Internal("topology request ledger has no revision".into())
                    })?;
                return Ok(TopologyApplyResult { revision });
            }
            // A pre-fingerprint ledger entry can only come from an interrupted
            // development build. Remove it rather than treating an unbound
            // request id as an idempotent success for an unrelated payload.
            oz_core::Settings::remove(&global_db, &request_key)?;
        }
    }

    // Finish any prior cross-database Apply before comparing revisions. A
    // prior process may have committed the diagram but not cleared its
    // journal, in which case recovery must finalize it first.
    recover_pending_topology_apply(&state, &session.store_id).await?;
    {
        let global_db = state.db.lock().await;
        let current_revision = current_topology_revision(&global_db, &topology_key)?;
        if current_revision != base_revision {
            return Err(topology_validation(
                "topology-revision-conflict",
                None,
                None,
                None,
                format!(
                    "topology revision conflict: expected {base_revision}, current {current_revision}"
                ),
            ));
        }
    }

    // Reject malformed graphs before any workspace mutation. Legacy
    // geometric payloads remain accepted during the migration window.
    {
        let global_db = state.db.lock().await;
        validate_apply_gate(&global_db, &diagram_nodes, &diagram_wires)?;
    }

    // Capture lengths before the workspace block consumes the vectors
    // (via `into_iter`-style moves). Also used for tracing after the
    // diagram save.
    let created = workspace_creations.len();
    let updated = workspace_updates.len();
    let archived = workspace_archives.len();
    let node_count = diagram_nodes.len();
    let wire_count = diagram_wires.len();

    // Capture the exact diagram state before mutating the store database.
    // If the later global write fails, the workspace transaction is
    // compensated from this snapshot.
    let previous_topology = {
        let global_db = state.db.lock().await;
        oz_core::Settings::get(&global_db, &topology_key)?
    };
    let desired_topology = topology_envelope_json(
        &diagram_nodes,
        &diagram_wires,
        base_revision.saturating_add(1),
        &resolved_issue_keys,
    )?;

    // Snapshot all pre-existing rows that a later compensation may need to restore.
    let workspace_snapshot = snapshot_workspace_rows(
        &state,
        &session.store_id,
        &workspace_updates,
        &workspace_archives,
    )
    .await?;

    // A semantic graph is scoped to one canonical branch. The backend compiler
    // binds creates to that stable identity, rather than trusting a caller's
    // arbitrary store_id or falling back to a primary/default store.
    if let Some(branch_profile_id) = semantic_branch_profile_id(&diagram_nodes, &diagram_wires) {
        if branch_profile_id != session.store_id {
            return Err(AppError::PermissionDenied(format!(
                "topology Branch Location {branch_profile_id} is outside the session store"
            )));
        }
        if let Some(requested_branch_id) = branch_id.as_deref()
            && requested_branch_id != branch_profile_id
        {
            return Err(topology_validation(
                "branch-id-mismatch",
                None,
                None,
                None,
                format!(
                    "topology branch {requested_branch_id} does not match Branch Location {branch_profile_id}"
                ),
            ));
        }
        for creation in &workspace_creations {
            if creation.store_id != branch_profile_id {
                return Err(AppError::TopologyValidation {
                    code: "workspace-store-mismatch".into(),
                    node_id: None,
                    wire_id: None,
                    port_id: None,
                    message: format!(
                        "workspace {} must be compiled to Branch Location {}",
                        creation.id, branch_profile_id
                    ),
                });
            }
        }
    }

    // Load entitlement before acquiring the non-Send store connection guard.
    // Tauri command futures must remain Send across every await boundary.
    let effective_tier = {
        let global_db = state.db.lock().await;
        TenantSubscription::validate_clock_rollback(&global_db)?;
        let subscription = TenantSubscription::load(&global_db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?;
        subscription.verify_signature()?;
        subscription.effective_tier()
    };
    validate_warehouse_quota(&diagram_nodes, &effective_tier)?;
    validate_warehouse_capacity(
        &diagram_nodes,
        &diagram_wires,
        &effective_tier,
        &resolved_issue_keys,
    )?;

    // The journal is written BEFORE any store mutation. If the process
    // crashes after the store commit, startup/next Apply can compare the
    // desired diagram and compensate deterministically.
    let recovery = TopologyApplyRecovery {
        store_id: session.store_id.clone(),
        topology_branch_id: branch_id.clone(),
        creations: workspace_creations.clone(),
        snapshots: workspace_snapshot.clone(),
        previous_topology: previous_topology.clone(),
        desired_topology: Some(desired_topology.clone()),
    };
    {
        let db = state.db.lock().await;
        persist_topology_recovery(&db, &recovery)?;
    }

    // ── Workspace CRUD in a single transaction ────────────────────────
    //
    // Scoped in a block so all non-`Send` types (MutexGuard, Store,
    // Transaction) are dropped before the `state.db.lock().await` call
    // below. Tauri requires command futures to be `Send`.
    {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);

        // Preserve the same subscription and entitlement boundary as the
        // standalone workspace-create command. The topology diff must not
        // become an entitlement bypass just because it batches mutations.
        for creation in &workspace_creations {
            if creation.id.trim().is_empty()
                || creation.type_key.trim().is_empty()
                || creation.store_id.trim().is_empty()
                || creation.name.trim().is_empty()
            {
                return Err(AppError::Invalid(
                    "workspace creation requires non-empty id, type_key, store_id, and name".into(),
                ));
            }
            if creation.store_id != session.store_id {
                return Err(AppError::PermissionDenied(format!(
                    "workspace {} targets a different store",
                    creation.id
                )));
            }
            if !effective_tier.allows_workspace_type(&creation.type_key) {
                return Err(AppError::PermissionDenied(format!(
                    "subscription tier does not allow workspace type {}",
                    creation.type_key
                )));
            }
            if creation
                .purpose_key
                .as_deref()
                .unwrap_or("general")
                .trim()
                .is_empty()
            {
                return Err(AppError::Invalid(
                    "workspace purpose_key must not be empty".into(),
                ));
            }
        }
        for update in &workspace_updates {
            let owner: String = store
                .conn()
                .query_row(
                    "SELECT store_id FROM workspace_instances WHERE id = ?1",
                    rusqlite::params![update.id],
                    |row| row.get(0),
                )
                .map_err(|_| {
                    AppError::PermissionDenied(format!(
                        "workspace {} is not in the session store",
                        update.id
                    ))
                })?;
            if owner != session.store_id {
                return Err(AppError::PermissionDenied(format!(
                    "workspace {} is not in the session store",
                    update.id
                )));
            }
        }
        for archive_id in &workspace_archives {
            let owner: String = store
                .conn()
                .query_row(
                    "SELECT store_id FROM workspace_instances WHERE id = ?1",
                    rusqlite::params![archive_id],
                    |row| row.get(0),
                )
                .map_err(|_| {
                    AppError::PermissionDenied(format!(
                        "workspace {archive_id} is not in the session store"
                    ))
                })?;
            if owner != session.store_id {
                return Err(AppError::PermissionDenied(format!(
                    "workspace {archive_id} is not in the session store"
                )));
            }
        }
        if let Some(limit) = effective_tier.max_pos_instances() {
            let current = store.count_active_instances(&session.store_id)?;
            let archived_ids: std::collections::HashSet<&str> =
                workspace_archives.iter().map(String::as_str).collect();
            let archived_active = archived_ids
                .iter()
                .filter(|id| {
                    store
                        .conn()
                        .query_row(
                            "SELECT status = 'active' FROM workspace_instances WHERE id = ?1",
                            rusqlite::params![id],
                            |row| row.get::<_, bool>(0),
                        )
                        .unwrap_or(false)
                })
                .count() as i64;
            let projected = current - archived_active + workspace_creations.len() as i64;
            if projected > limit {
                return Err(AppError::PermissionDenied(format!(
                    "workspace instance quota exceeded: limit {limit}, current {current}, archived {archived_active}, requested {}, projected {projected}",
                    workspace_creations.len()
                )));
            }
        }

        // Inside this transaction, all create / update / archive SQL runs
        // *directly* on `tx`. We deliberately do NOT delegate to
        // `Store::create_workspace_instance` here: that method opens its
        // own transaction via `unchecked_transaction`, which issues a raw
        // `BEGIN` that SQLite rejects ("cannot start a transaction within
        // a transaction") when an outer transaction is already open. See
        // `create_workspace_instance_cannot_nest_in_open_transaction` in
        // oz-core. Running the INSERT/UPDATE SQL directly preserves the
        // single-transaction atomicity: if any step fails, the whole
        // batch rolls back.
        let tx = db
            .unchecked_transaction()
            .map_err(|e| AppError::Internal(format!("begin transaction: {e}")))?;

        // 1. Create new workspace instances (direct SQL — no nested tx).
        for creation in &workspace_creations {
            // Mirrors Store::create_workspace_instance's existence check
            // + INSERT, minus the nested transaction.
            let exists: bool = tx
                .query_row(
                    "SELECT COUNT(*) > 0 FROM workspace_instances WHERE id = ?1",
                    rusqlite::params![creation.id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if exists {
                return Err(AppError::Internal(format!(
                    "workspace instance already exists: {}",
                    creation.id
                )));
            }
            tx.execute(
                "INSERT INTO workspace_instances \
                 (id, type_key, store_id, name, description, colour, purpose_key, status, last_accessed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', \
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                rusqlite::params![
                    creation.id,
                    creation.type_key,
                    creation.store_id,
                    creation.name,
                    creation.description.as_deref().unwrap_or(""),
                    creation.colour.as_deref(),
                    creation.purpose_key.as_deref().unwrap_or("general"),
                ],
            )
            .map_err(|e| AppError::Internal(format!("create instance {}: {e}", creation.id)))?;
        }

        // 2. Update existing workspace instances (rename only).
        //
        // `update_workspace_instance` uses `self.conn.execute` directly
        // (no nested transaction), so it composes safely inside this tx.
        let tx_store = Store::new(&tx);
        for update in &workspace_updates {
            tx_store.update_workspace_instance(&update.id, &update.name, None, None)?;
            if let Some(purpose_key) = update.purpose_key.as_deref() {
                if purpose_key.trim().is_empty() {
                    return Err(AppError::Invalid(
                        "workspace purpose_key must not be empty".into(),
                    ));
                }
                tx.execute(
                    "UPDATE workspace_instances SET purpose_key = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
                    rusqlite::params![update.id, purpose_key],
                )?;
            }
        }

        // 3. Archive workspace instances removed from the canvas.
        //
        // `archive_instance` also uses `self.conn.execute` directly, so
        // it is safe to call within this transaction. A 0-rows-affected
        // archive surfaces as NotFound, which aborts (and rolls back)
        // the whole batch.
        for archive_id in &workspace_archives {
            tx_store.archive_instance(archive_id)?;
        }

        tx.commit()
            .map_err(|e| AppError::Internal(format!("commit transaction: {e}")))?;
        // db, store, tx, tx_store all drop here when the block ends.
    }

    // ── Save topology diagram on global database ─────────────────────
    //
    // This `.await` is now safe — all non-`Send` types from the store
    // DB block have been dropped.
    let global_db = state.db.lock().await;
    if let Err(save_error) = save_topology_json_at_key_with_revision(
        &global_db,
        diagram_nodes,
        diagram_wires,
        &topology_key,
        &resolved_issue_keys,
        Some(base_revision),
        Some((&request_key, &request_fingerprint)),
    ) {
        drop(global_db);
        // The durable recovery journal was written before the workspace
        // transaction. Keep it until both databases have been compensated.
        if let Err(compensation_error) = compensate_workspace_diff(
            &state,
            &session.store_id,
            &workspace_creations,
            &workspace_snapshot,
        )
        .await
        {
            return Err(AppError::Internal(format!(
                "topology save failed ({save_error}); workspace compensation pending ({compensation_error})"
            )));
        }
        let restore = {
            let db = state.db.lock().await;
            restore_topology_setting(&db, &topology_key, previous_topology.as_deref())
        };
        if let Err(restore_error) = restore {
            return Err(AppError::Internal(format!(
                "topology save failed ({save_error}); diagram compensation pending ({restore_error})"
            )));
        }
        {
            let db = state.db.lock().await;
            clear_topology_recovery(&db)?;
        }
        return Err(save_error);
    }

    // The `global_db` guard from the save is still held on the success path
    // — re-locking `state.db` here would deadlock (tokio::sync::Mutex is not
    // reentrant), so read the committed revision through the guard we
    // already own. (Latent since the success path was first built; no test
    // exercised the real command end-to-end until round 136.)
    let revision = current_topology_revision(&global_db, &topology_key)?;
    drop(global_db);
    let result = TopologyApplyResult { revision };
    tracing::info!(
        created,
        updated,
        archived,
        nodes = node_count,
        wires = wire_count,
        revision = result.revision,
        "topology diff applied"
    );

    Ok(result)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod topology_tests;
