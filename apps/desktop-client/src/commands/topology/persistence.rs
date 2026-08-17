//! Topology persistence: branch-scoped settings keys, save/load, and the
//! cross-database Apply recovery journal.
//!
//! Extracted from commands/topology.rs. Depends on the semantic engine
//! (`super::semantics`) for the save/Apply-time gates.

use rusqlite::{Connection, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::workspaces::CreateInstanceRequest;
use crate::error::AppError;
use crate::state::AppState;

use super::model::*;
use super::semantics::*;

/// Resolve the branch-scoped runtime plan key paired with a topology key.
pub(crate) fn topology_runtime_setting_key(topology_key: &str) -> Result<String, AppError> {
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
pub(crate) fn compile_topology_runtime_plan(
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
pub(crate) fn topology_setting_key(branch_id: Option<&str>) -> Result<String, AppError> {
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

#[cfg(test)]
pub(crate) fn save_topology_json_at_key(
    conn: &Connection,
    nodes: Vec<Value>,
    wires: Vec<Value>,
    setting_key: &str,
) -> Result<u64, AppError> {
    save_topology_json_at_key_with_revision(conn, nodes, wires, setting_key, &[], None, None)
}

pub(crate) fn save_topology_json_at_key_with_revision(
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
pub(crate) fn save_topology_json(
    conn: &Connection,
    nodes: Vec<Value>,
    wires: Vec<Value>,
) -> Result<(), AppError> {
    save_topology_json_at_key(conn, nodes, wires, TOPOLOGY_SETTING_KEY).map(|_| ())
}

/// Snapshot of a workspace row touched by a topology Apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkspaceApplySnapshot {
    id: String,
    name: String,
    description: String,
    colour: Option<String>,
    purpose_key: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TopologyApplyRecovery {
    /// Store the Apply is scoped to (cross-database compensation identity).
    pub(crate) store_id: String,
    /// Branch the topology diff belongs to, when scoped.
    #[serde(default)]
    pub(crate) topology_branch_id: Option<String>,
    /// Workspace instance creations to replay on compensation.
    pub(crate) creations: Vec<CreateInstanceRequest>,
    /// Pre-mutation workspace row snapshots for restore-on-failure.
    pub(crate) snapshots: Vec<WorkspaceApplySnapshot>,
    /// Exact previous topology setting JSON to restore on compensation.
    pub(crate) previous_topology: Option<String>,
    /// Exact canonical diagram JSON expected after the Apply. Recovery uses
    /// it to distinguish a crash before the global write from a crash after
    /// it, because the workspace and global databases cannot share a SQLite
    /// transaction.
    #[serde(default)]
    pub(crate) desired_topology: Option<String>,
}

/// Restore the topology setting after a compensating Apply failure.
///
/// Diagram settings and workspace instances live in separate SQLite
/// databases, so Apply uses a forward-write plus compensation boundary. The
/// restore itself is transactional and preserves the exact prior raw setting,
/// including legacy envelopes.
pub(crate) fn restore_topology_setting(
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

pub(crate) fn persist_topology_recovery(
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

pub(crate) fn clear_topology_recovery(conn: &Connection) -> Result<(), AppError> {
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

pub(crate) async fn recover_pending_topology_apply(
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
pub(crate) async fn snapshot_workspace_rows(
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
pub(crate) async fn compensate_workspace_diff(
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
pub(crate) fn validate_semantic_ownership(
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
pub(crate) fn validate_apply_gate(
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

pub(crate) fn validate_warehouse_quota(
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
pub(crate) fn validate_warehouse_capacity(
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
pub(crate) fn validate_diagram_payloads(nodes: &[Value], wires: &[Value]) -> Result<(), AppError> {
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
pub(crate) fn validate_topology_structure(
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

// ── Unit tests for pure validation functions ─────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::topology::model::TOPOLOGY_RUNTIME_SETTING_KEY;
    use crate::commands::topology::model::TOPOLOGY_SETTING_KEY;

    // ── topology_setting_key ────────────────────────────────────

    #[test]
    fn topology_setting_key_none_returns_base() {
        let key = topology_setting_key(None).unwrap();
        assert_eq!(key, TOPOLOGY_SETTING_KEY);
    }

    #[test]
    fn topology_setting_key_with_branch() {
        let key = topology_setting_key(Some("main")).unwrap();
        assert_eq!(key, format!("{TOPOLOGY_SETTING_KEY}/main"));
    }

    #[test]
    fn topology_setting_key_empty_branch_rejected() {
        assert!(topology_setting_key(Some("")).is_err());
        assert!(topology_setting_key(Some("  ")).is_err());
    }

    #[test]
    fn topology_setting_key_slash_rejected() {
        assert!(topology_setting_key(Some("a/b")).is_err());
    }

    #[test]
    fn topology_setting_key_control_chars_rejected() {
        assert!(topology_setting_key(Some("branch test")).is_err());
        assert!(topology_setting_key(Some("branchtest")).is_err());
    }

    #[test]
    fn topology_setting_key_too_long_rejected() {
        let long = "a".repeat(201);
        assert!(topology_setting_key(Some(&long)).is_err());
    }

    #[test]
    fn topology_setting_key_max_length_ok() {
        let ok = "a".repeat(200);
        assert!(topology_setting_key(Some(&ok)).is_ok());
    }

    // ── topology_runtime_setting_key ────────────────────────────

    #[test]
    fn topology_runtime_setting_key_base_returns_runtime_base() {
        let key = topology_runtime_setting_key(TOPOLOGY_SETTING_KEY).unwrap();
        assert_eq!(key, TOPOLOGY_RUNTIME_SETTING_KEY);
    }

    #[test]
    fn topology_runtime_setting_key_branch_returns_runtime_branch() {
        let branch_key = format!("{TOPOLOGY_SETTING_KEY}/west");
        let key = topology_runtime_setting_key(&branch_key).unwrap();
        assert_eq!(key, format!("{TOPOLOGY_RUNTIME_SETTING_KEY}/west"));
    }

    #[test]
    fn topology_runtime_setting_key_invalid_prefix_rejected() {
        assert!(topology_runtime_setting_key("wrong-prefix").is_err());
    }

    // ── validate_topology_structure ─────────────────────────────

    fn make_node(id: &str, node_type: &str) -> TopologyNodePayload {
        TopologyNodePayload {
            id: id.into(),
            node_type: node_type.into(),
            name: format!("Name {id}"),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        }
    }

    fn make_wire(id: &str, from: &str, to: &str) -> TopologyWirePayload {
        TopologyWirePayload {
            id: id.into(),
            from_node_id: from.into(),
            to_node_id: to.into(),
            direction: WireDirection::OneWay,
            label: None,
            from_port: None,
            to_port: None,
        }
    }

    #[test]
    fn validate_topology_empty_is_ok() {
        assert!(validate_topology_structure(&[], &[]).is_ok());
    }

    #[test]
    fn validate_topology_single_node_no_wires() {
        let nodes = vec![make_node("n1", "store")];
        assert!(validate_topology_structure(&nodes, &[]).is_ok());
    }

    #[test]
    fn validate_topology_valid_wire() {
        let nodes = vec![make_node("n1", "store"), make_node("n2", "workspace")];
        let wires = vec![make_wire("w1", "n1", "n2")];
        assert!(validate_topology_structure(&nodes, &wires).is_ok());
    }

    #[test]
    fn validate_topology_duplicate_node_id_rejected() {
        let nodes = vec![make_node("n1", "store"), make_node("n1", "workspace")];
        let err = validate_topology_structure(&nodes, &[]).unwrap_err();
        assert!(format!("{err}").contains("duplicate node id"));
    }

    #[test]
    fn validate_topology_unknown_node_type_rejected() {
        let nodes = vec![make_node("n1", "teleporter")];
        let err = validate_topology_structure(&nodes, &[]).unwrap_err();
        assert!(format!("{err}").contains("unknown type"));
    }

    #[test]
    fn validate_topology_duplicate_wire_id_rejected() {
        let nodes = vec![make_node("n1", "store"), make_node("n2", "workspace")];
        let wires = vec![make_wire("w1", "n1", "n2"), make_wire("w1", "n2", "n1")];
        let err = validate_topology_structure(&nodes, &wires).unwrap_err();
        assert!(format!("{err}").contains("duplicate wire id"));
    }

    #[test]
    fn validate_topology_unknown_wire_direction_rejected() {
        let nodes = vec![make_node("n1", "store"), make_node("n2", "workspace")];
        let mut wire = make_wire("w1", "n1", "n2");
        wire.direction = WireDirection::Unknown;
        let err = validate_topology_structure(&nodes, &[wire]).unwrap_err();
        assert!(format!("{err}").contains("unknown direction"));
    }

    #[test]
    fn validate_topology_unknown_port_rejected() {
        let nodes = vec![make_node("n1", "store"), make_node("n2", "workspace")];
        let mut wire = make_wire("w1", "n1", "n2");
        wire.from_port = Some(PortName::Unknown);
        let err = validate_topology_structure(&nodes, &[wire]).unwrap_err();
        assert!(format!("{err}").contains("unknown port"));
    }

    #[test]
    fn validate_topology_wire_references_unknown_node() {
        let nodes = vec![make_node("n1", "store")];
        let wires = vec![make_wire("w1", "n1", "nonexistent")];
        let err = validate_topology_structure(&nodes, &wires).unwrap_err();
        assert!(
            format!("{err}").contains("unknown from_node_id")
                || format!("{err}").contains("unknown to_node_id")
        );
    }

    #[test]
    fn validate_topology_valid_wire_with_ports() {
        let nodes = vec![make_node("n1", "store"), make_node("n2", "workspace")];
        let mut wire = make_wire("w1", "n1", "n2");
        wire.from_port = Some(PortName::Right);
        wire.to_port = Some(PortName::Left);
        wire.direction = WireDirection::TwoWay;
        assert!(validate_topology_structure(&nodes, &[wire]).is_ok());
    }

    // ── validate_warehouse_quota ────────────────────────────────

    fn wh_node(id: &str) -> Value {
        serde_json::json!({"id": id, "type": "warehouse", "name": "WH", "x": 0, "y": 0})
    }

    fn store_node_val(id: &str) -> Value {
        serde_json::json!({"id": id, "type": "store", "name": "Store", "x": 0, "y": 0})
    }

    #[test]
    fn validate_warehouse_quota_no_warehouses_always_ok() {
        use oz_core::subscription::SubscriptionTier;
        let nodes = vec![store_node_val("n1")];
        assert!(validate_warehouse_quota(&nodes, &SubscriptionTier::Free).is_ok());
        assert!(validate_warehouse_quota(&nodes, &SubscriptionTier::Pro).is_ok());
    }

    #[test]
    fn validate_warehouse_quota_free_tier_one_warehouse() {
        use oz_core::subscription::SubscriptionTier;
        let nodes = vec![wh_node("n1")];
        assert!(validate_warehouse_quota(&nodes, &SubscriptionTier::Free).is_ok());
    }

    #[test]
    fn validate_warehouse_quota_free_tier_two_warehouses_rejected() {
        use oz_core::subscription::SubscriptionTier;
        let nodes = vec![wh_node("n1"), wh_node("n2")];
        let err = validate_warehouse_quota(&nodes, &SubscriptionTier::Free).unwrap_err();
        assert!(format!("{err}").contains("quota exceeded"));
    }

    #[test]
    fn validate_warehouse_quota_no_limit_for_tier_without_cap() {
        use oz_core::subscription::SubscriptionTier;
        let nodes: Vec<Value> = (0..100).map(|i| wh_node(&format!("w{i}"))).collect();
        // Premium/Enterprise have no warehouse cap (§3).
        assert!(validate_warehouse_quota(&nodes, &SubscriptionTier::Premium).is_ok());
        assert!(validate_warehouse_quota(&nodes, &SubscriptionTier::Enterprise).is_ok());
    }

    // ── validate_diagram_payloads ───────────────────────────────

    #[test]
    fn validate_diagram_payloads_empty_is_ok() {
        assert!(validate_diagram_payloads(&[], &[]).is_ok());
    }

    #[test]
    fn validate_diagram_payloads_valid_node_wire() {
        let nodes = vec![serde_json::json!({"id":"n1","type":"store","name":"S","x":0,"y":0})];
        let wires = vec![];
        assert!(validate_diagram_payloads(&nodes, &wires).is_ok());
    }

    #[test]
    fn validate_diagram_payloads_invalid_node_type_rejected() {
        let nodes = vec![serde_json::json!({"id":"n1","type":"teleporter","name":"S","x":0,"y":0})];
        let err = validate_diagram_payloads(&nodes, &[]).unwrap_err();
        assert!(
            format!("{err}").contains("unknown type")
                || format!("{err}").contains("invalid topology")
        );
    }

    #[test]
    fn validate_diagram_payloads_branch_location_mapped_to_store() {
        // "branch-location" should be silently mapped to "store"
        let nodes =
            vec![serde_json::json!({"id":"n1","type":"branch-location","name":"S","x":0,"y":0})];
        assert!(validate_diagram_payloads(&nodes, &[]).is_ok());
    }

    #[test]
    fn validate_diagram_payloads_invalid_json_rejected() {
        let nodes = vec![serde_json::json!({"missing_fields": true})];
        let err = validate_diagram_payloads(&nodes, &[]).unwrap_err();
        assert!(format!("{err}").contains("invalid topology nodes"));
    }
}
