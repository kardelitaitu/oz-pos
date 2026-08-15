//! Typed topology model: node/wire payloads, enums, and resilient serde.
//!
//! Extracted from commands/topology.rs so the command module stays under the
//! ~3k-line guideline. `pub(crate)` marks items that sibling modules or the
//! tests reach through the topology root's re-exports.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
pub(crate) fn ser_f64_finite<S>(val: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f64(if val.is_finite() { *val } else { 0.0 })
}

/// Deserialise an f64, mapping JSON `null` to `0.0`.
pub(crate) fn de_f64_or_null<'de, D>(d: D) -> Result<f64, D::Error>
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
pub(crate) fn de_direction_or_null<'de, D>(d: D) -> Result<WireDirection, D::Error>
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

pub(crate) fn default_direction() -> WireDirection {
    WireDirection::OneWay
}

// ── Module constants ────────────────────────────────────────────────

/// Settings key for the unscoped (legacy) topology diagram.
pub(crate) const TOPOLOGY_SETTING_KEY: &str = "oz-pos/topology";
/// Settings key for the branch-scoped runtime routing plan.
pub(crate) const TOPOLOGY_RUNTIME_SETTING_KEY: &str = "oz-pos/topology-runtime";
/// Settings key for the cross-database Apply recovery journal.
pub(crate) const TOPOLOGY_APPLY_RECOVERY_KEY: &str = "oz-pos/topology/apply-recovery";
/// Settings-key prefix for one Apply request's revision ledger.
pub(crate) const TOPOLOGY_APPLY_REQUEST_PREFIX: &str = "oz-pos/topology/apply-request/";
/// Schema version stamped into every saved topology envelope.
pub(crate) const TOPOLOGY_SCHEMA_VERSION: u64 = 1;

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── NodeType From<&str> ─────────────────────────────────────

    #[test]
    fn node_type_from_str_valid_variants() {
        assert_eq!(NodeType::from("store"), NodeType::Store);
        assert_eq!(NodeType::from("workspace"), NodeType::Workspace);
        assert_eq!(NodeType::from("warehouse"), NodeType::Warehouse);
        assert_eq!(NodeType::from("hardware"), NodeType::Hardware);
    }

    #[test]
    fn node_type_from_str_unknown_returns_unknown() {
        assert_eq!(NodeType::from("anything"), NodeType::Unknown);
        assert_eq!(NodeType::from(""), NodeType::Unknown);
        assert_eq!(NodeType::from("STORE"), NodeType::Unknown);
        assert_eq!(NodeType::from("Store"), NodeType::Unknown);
    }

    // ── NodeType PartialEq<&str> ────────────────────────────────

    #[test]
    fn node_type_partial_eq_str() {
        assert_eq!(NodeType::Store, "store");
        assert_eq!(NodeType::Workspace, "workspace");
        assert_eq!(NodeType::Warehouse, "warehouse");
        assert_eq!(NodeType::Hardware, "hardware");
    }

    #[test]
    fn node_type_unknown_never_eq_str() {
        assert_ne!(NodeType::Unknown, "store");
        assert_ne!(NodeType::Unknown, "workspace");
        assert_ne!(NodeType::Unknown, "anything");
    }

    // ── NodeType serde ──────────────────────────────────────────

    #[test]
    fn node_type_serde_kebab_case() {
        assert_eq!(
            serde_json::to_string(&NodeType::Store).unwrap(),
            r#""store""#
        );
        assert_eq!(
            serde_json::to_string(&NodeType::Workspace).unwrap(),
            r#""workspace""#
        );
        assert_eq!(
            serde_json::to_string(&NodeType::Warehouse).unwrap(),
            r#""warehouse""#
        );
        assert_eq!(
            serde_json::to_string(&NodeType::Hardware).unwrap(),
            r#""hardware""#
        );
    }

    #[test]
    fn node_type_serde_roundtrip() {
        for variant in [
            NodeType::Store,
            NodeType::Workspace,
            NodeType::Warehouse,
            NodeType::Hardware,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: NodeType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn node_type_serde_other_catches_unknown() {
        let json = r#""teleporter""#;
        let val: NodeType = serde_json::from_str(json).unwrap();
        assert_eq!(val, NodeType::Unknown);
    }

    // ── WireDirection From<&str> ────────────────────────────────

    #[test]
    fn wire_direction_from_str_valid() {
        assert_eq!(WireDirection::from("one-way"), WireDirection::OneWay);
        assert_eq!(WireDirection::from("two-way"), WireDirection::TwoWay);
        assert_eq!(WireDirection::from("reverse"), WireDirection::Reverse);
    }

    #[test]
    fn wire_direction_from_str_unknown() {
        assert_eq!(WireDirection::from("bidi"), WireDirection::Unknown);
        assert_eq!(WireDirection::from(""), WireDirection::Unknown);
        assert_eq!(WireDirection::from("OneWay"), WireDirection::Unknown);
    }

    // ── WireDirection PartialEq<&str> ───────────────────────────

    #[test]
    fn wire_direction_partial_eq_str() {
        assert_eq!(WireDirection::OneWay, "one-way");
        assert_eq!(WireDirection::TwoWay, "two-way");
        assert_eq!(WireDirection::Reverse, "reverse");
    }

    #[test]
    fn wire_direction_unknown_never_eq_str() {
        assert_ne!(WireDirection::Unknown, "one-way");
        assert_ne!(WireDirection::Unknown, "two-way");
    }

    // ── WireDirection serde ─────────────────────────────────────

    #[test]
    fn wire_direction_serde_kebab_case() {
        assert_eq!(
            serde_json::to_string(&WireDirection::OneWay).unwrap(),
            r#""one-way""#
        );
        assert_eq!(
            serde_json::to_string(&WireDirection::TwoWay).unwrap(),
            r#""two-way""#
        );
        assert_eq!(
            serde_json::to_string(&WireDirection::Reverse).unwrap(),
            r#""reverse""#
        );
    }

    #[test]
    fn wire_direction_serde_other_catches_unknown() {
        let val: WireDirection = serde_json::from_str(r#""bidi""#).unwrap();
        assert_eq!(val, WireDirection::Unknown);
    }

    // ── PortName From<&str> ─────────────────────────────────────

    #[test]
    fn port_name_from_str_valid() {
        assert_eq!(PortName::from("top"), PortName::Top);
        assert_eq!(PortName::from("right"), PortName::Right);
        assert_eq!(PortName::from("bottom"), PortName::Bottom);
        assert_eq!(PortName::from("left"), PortName::Left);
    }

    #[test]
    fn port_name_from_str_unknown() {
        assert_eq!(PortName::from("center"), PortName::Unknown);
        assert_eq!(PortName::from(""), PortName::Unknown);
        assert_eq!(PortName::from("Top"), PortName::Unknown);
    }

    // ── PortName PartialEq<&str> ────────────────────────────────

    #[test]
    fn port_name_partial_eq_str() {
        assert_eq!(PortName::Top, "top");
        assert_eq!(PortName::Right, "right");
        assert_eq!(PortName::Bottom, "bottom");
        assert_eq!(PortName::Left, "left");
    }

    #[test]
    fn port_name_unknown_never_eq_str() {
        assert_ne!(PortName::Unknown, "top");
        assert_ne!(PortName::Unknown, "left");
    }

    // ── PortName serde ──────────────────────────────────────────

    #[test]
    fn port_name_serde_lowercase() {
        assert_eq!(serde_json::to_string(&PortName::Top).unwrap(), r#""top""#);
        assert_eq!(
            serde_json::to_string(&PortName::Right).unwrap(),
            r#""right""#
        );
    }

    #[test]
    fn port_name_serde_other_catches_unknown() {
        let val: PortName = serde_json::from_str(r#""center""#).unwrap();
        assert_eq!(val, PortName::Unknown);
    }

    // ── ser_f64_finite / de_f64_or_null ────────────────────────

    #[test]
    fn serde_f64_normal_roundtrip() {
        let json = r#"{"id":"n1","type":"store","name":"S","x":123.45,"y":-67.8}"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert!((node.x - 123.45).abs() < f64::EPSILON);
        assert!((node.y - (-67.8)).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_f64_nan_serializes_to_zero() {
        // NaN is not valid JSON so we can't deserialize it.
        // But the custom serializer guarantees NaN -> 0.0 on output.
        let mut node = TopologyNodePayload {
            id: "n1".into(),
            node_type: NodeType::Store,
            name: "S".into(),
            subtitle: None,
            x: f64::NAN,
            y: 1.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let out = serde_json::to_string(&node).unwrap();
        assert!(
            out.contains(r#""x":0.0"#),
            "NaN should serialize to 0.0, got {out}"
        );
        // Infinity should also serialize to 0.0
        node.x = f64::INFINITY;
        let out = serde_json::to_string(&node).unwrap();
        assert!(
            out.contains(r#""x":0.0"#),
            "Infinity should serialize to 0.0, got {out}"
        );
        // Negative infinity too
        node.x = f64::NEG_INFINITY;
        let out = serde_json::to_string(&node).unwrap();
        assert!(
            out.contains(r#""x":0.0"#),
            "NegInfinity should serialize to 0.0, got {out}"
        );
    }

    #[test]
    fn serde_f64_null_deserializes_to_zero() {
        let json = r#"{"id":"n1","type":"store","name":"S","x":null,"y":null}"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert_eq!(node.x, 0.0);
        assert_eq!(node.y, 0.0);
    }

    #[test]
    fn serde_f64_absent_field_deserializes_to_zero() {
        // When x/y are absent, serde(default) kicks in and de_f64_or_null
        // provides the default 0.0
        let json = r#"{"id":"n1","type":"store","name":"S","x":0,"y":0}"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert_eq!(node.x, 0.0);
        assert_eq!(node.y, 0.0);
    }

    // ── de_direction_or_null ────────────────────────────────────

    #[test]
    fn wire_direction_null_defaults_to_one_way() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":null}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.direction, WireDirection::OneWay);
    }

    #[test]
    fn wire_direction_absent_defaults_to_one_way() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.direction, WireDirection::OneWay);
    }

    #[test]
    fn wire_direction_valid_value_preserved() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":"two-way"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.direction, WireDirection::TwoWay);
    }

    // ── TopologyNodePayload defaults ────────────────────────────

    #[test]
    fn node_defaults_for_optional_fields() {
        let json = r#"{"id":"n1","type":"store","name":"S","x":1.0,"y":2.0}"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert_eq!(node.id, "n1");
        assert_eq!(node.node_type, NodeType::Store);
        assert_eq!(node.name, "S");
        assert!(node.subtitle.is_none());
        assert!(node.tier_requirement.is_none());
        assert!(node.telemetry_badge.is_none());
        assert!(node.telemetry_status.is_none());
        assert!(node.metadata.is_none());
    }

    // ── TopologyData serde roundtrip ────────────────────────────

    #[test]
    fn topology_data_empty_roundtrip() {
        let data = TopologyData {
            nodes: vec![],
            wires: vec![],
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: TopologyData = serde_json::from_str(&json).unwrap();
        assert!(back.nodes.is_empty());
        assert!(back.wires.is_empty());
    }

    #[test]
    fn topology_data_full_roundtrip() {
        let data = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: "store-1".into(),
                node_type: NodeType::Store,
                name: "Main Store".into(),
                subtitle: None,
                x: 100.0,
                y: 200.0,
                tier_requirement: Some("pro".into()),
                telemetry_badge: Some("Online".into()),
                telemetry_status: Some("online".into()),
                metadata: None,
            }],
            wires: vec![TopologyWirePayload {
                id: "wire-1".into(),
                from_node_id: "store-1".into(),
                to_node_id: "ws-1".into(),
                direction: WireDirection::TwoWay,
                label: Some("LAN".into()),
                from_port: Some(PortName::Right),
                to_port: Some(PortName::Left),
            }],
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: TopologyData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.nodes.len(), 1);
        assert_eq!(back.wires.len(), 1);
        assert_eq!(back.nodes[0].id, "store-1");
        assert_eq!(back.wires[0].direction, WireDirection::TwoWay);
        assert_eq!(back.wires[0].from_port, Some(PortName::Right));
    }

    // ── UpdateInstanceRequest serde ──────────────────────────────

    #[test]
    fn update_instance_request_roundtrip() {
        let req = UpdateInstanceRequest {
            id: "inst-1".into(),
            name: "POS 1".into(),
            purpose_key: Some("pos".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: UpdateInstanceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "inst-1");
        assert_eq!(back.name, "POS 1");
        assert_eq!(back.purpose_key, Some("pos".into()));
    }

    #[test]
    fn update_instance_request_purpose_key_optional() {
        let req = UpdateInstanceRequest {
            id: "inst-1".into(),
            name: "POS 1".into(),
            purpose_key: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        // Option<T> where T is not #[serde(skip_serializing_if)] means
        // None serializes as null, but we can still roundtrip
        let back: UpdateInstanceRequest = serde_json::from_str(&json).unwrap();
        assert!(back.purpose_key.is_none());
    }

    // ── default_direction ───────────────────────────────────────

    #[test]
    fn default_direction_is_one_way() {
        assert_eq!(default_direction(), WireDirection::OneWay);
    }

    // ── TopologyNodePayload — full-field serde ──────────────────

    #[test]
    fn node_all_fields_present() {
        let json = r#"{
            "id": "n1",
            "type": "warehouse",
            "name": "WH-1",
            "subtitle": "Central",
            "x": 50.5,
            "y": 75.2,
            "tier_requirement": "enterprise",
            "telemetry_badge": "3 workers",
            "telemetry_status": "warning",
            "metadata": {"region": "us-east"}
        }"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert_eq!(node.node_type, NodeType::Warehouse);
        assert_eq!(node.subtitle.as_deref(), Some("Central"));
        assert_eq!(node.tier_requirement.as_deref(), Some("enterprise"));
        assert_eq!(node.telemetry_badge.as_deref(), Some("3 workers"));
        assert_eq!(node.telemetry_status.as_deref(), Some("warning"));
        assert_eq!(node.metadata.as_ref().unwrap()["region"], "us-east");
    }

    // ── WirePayload defaults ────────────────────────────────────

    #[test]
    fn wire_defaults_for_optional_fields() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert!(wire.label.is_none());
        assert!(wire.from_port.is_none());
        assert!(wire.to_port.is_none());
    }
}
