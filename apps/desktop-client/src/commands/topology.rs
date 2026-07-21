//! Tauri commands for persisting the node topology graph.
//!
//! Topology data (nodes + wires) is serialised as JSON and stored in the
//! `settings` table under the key `oz-pos/topology`. On first load, the
//! command returns `None` so the front-end falls back to the built-in
//! retail preset.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use crate::error::AppError;
use crate::state::AppState;

/// Serialised topology persisted in the settings table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyData {
    pub nodes: Vec<TopologyNodePayload>,
    pub wires: Vec<TopologyWirePayload>,
}

/// One node in the topology graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNodePayload {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub name: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub tier_requirement: Option<String>,
    #[serde(default)]
    pub telemetry_badge: Option<String>,
    #[serde(default)]
    pub telemetry_status: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// One wire connecting two ports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyWirePayload {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub from_port: Option<String>,
    #[serde(default)]
    pub to_port: Option<String>,
}

fn default_direction() -> String {
    "one-way".into()
}

// ── Commands ───────────────────────────────────────────────────────

const TOPOLOGY_SETTING_KEY: &str = "oz-pos/topology";

/// Save the topology graph to the settings store.
///
/// Serialises the nodes + wires payloads as JSON and writes them to
/// the `oz-pos/topology` setting key. Any previous topology is
/// overwritten.
#[command]
pub async fn save_topology(
    nodes: Vec<TopologyNodePayload>,
    wires: Vec<TopologyWirePayload>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let data = TopologyData { nodes, wires };
    let json = serde_json::to_string(&data).map_err(|e| AppError::Internal(e.to_string()))?;

    let conn = state.db.lock().await;
    oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json)?;
    Ok(())
}

/// Load the persisted topology graph.
///
/// Returns `None` when no topology has been saved yet (the front-end
/// should fall back to the built-in retail preset).
#[command]
pub async fn load_topology(state: State<'_, AppState>) -> Result<Option<TopologyData>, AppError> {
    let conn = state.db.lock().await;
    let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)?;
    match raw {
        Some(json) => {
            let data: TopologyData =
                serde_json::from_str(&json).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn fresh_conn() -> Connection {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut conn = Connection::open(&path).unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    #[test]
    fn save_and_load_roundtrip() {
        let conn = fresh_conn();
        let nodes = vec![TopologyNodePayload {
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
        }];
        let wires = vec![TopologyWirePayload {
            id: "w-1".into(),
            from_node_id: "store-1".into(),
            to_node_id: "ws-1".into(),
            direction: "one-way".into(),
            label: Some("Binds Store".into()),
            from_port: Some("right".into()),
            to_port: Some("left".into()),
        }];

        // Save via settings directly (mirrors the Tauri command).
        let data = TopologyData {
            nodes: nodes.clone(),
            wires: wires.clone(),
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();

        // Load.
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();

        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.nodes[0].id, "store-1");
        assert_eq!(loaded.nodes[0].name, "Main Store");
        assert_eq!(loaded.nodes[0].x, 100.0);
        assert_eq!(loaded.wires.len(), 1);
        assert_eq!(loaded.wires[0].id, "w-1");
        assert_eq!(loaded.wires[0].from_port.as_deref(), Some("right"));
    }

    #[test]
    fn load_returns_none_for_fresh_db() {
        let conn = fresh_conn();
        let result = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn save_overwrites_previous() {
        let conn = fresh_conn();

        // First save.
        let data1 = TopologyData {
            nodes: vec![TopologyNodePayload {
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
            wires: vec![],
        };
        let json1 = serde_json::to_string(&data1).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json1).unwrap();

        // Second save overwrites.
        let data2 = TopologyData {
            nodes: vec![TopologyNodePayload {
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
            wires: vec![],
        };
        let json2 = serde_json::to_string(&data2).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json2).unwrap();

        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
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
        let data = TopologyData {
            nodes: vec![],
            wires: vec![],
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();

        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert!(loaded.nodes.is_empty());
        assert!(loaded.wires.is_empty());
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
        assert_eq!(roundtripped.wires[0].from_port.as_deref(), Some("usb"));
        assert_eq!(roundtripped.wires[0].to_port.as_deref(), Some("network"));
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
        assert!(node.node_type.is_empty());
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
        assert_eq!(wire.direction, "bidirectional");
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
        let json =
            r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"telemetry_badge":"Online"}"#;
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
        let json =
            r#"{"id":"n1","type":"store","name":"Test","x":0,"y":0,"metadata":{"key":"val"}}"#;
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
        assert_eq!(wire.from_port.as_deref(), Some("left"));
        assert!(wire.to_port.is_none());
    }

    #[test]
    fn wire_only_to_port() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","to_port":"right"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.to_port.as_deref(), Some("right"));
        assert!(wire.from_port.is_none());
    }

    #[test]
    fn wire_both_ports_present() {
        let json =
            r#"{"id":"w1","from_node_id":"a","to_node_id":"b","from_port":"out","to_port":"in"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.from_port.as_deref(), Some("out"));
        assert_eq!(wire.to_port.as_deref(), Some("in"));
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
        assert_eq!(roundtripped.from_port.as_deref(), Some("primary"));
        assert_eq!(roundtripped.to_port.as_deref(), Some("secondary"));
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
    fn data_with_duplicate_wire_ids_roundtrips() {
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
        let json =
            r#"{"nodes":[{"id":"n1","type":"store","name":"Nested","x":0,"y":0}],"wires":[]}"#;
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

        let loaded_types: Vec<&str> = loaded.nodes.iter().map(|n| n.node_type.as_str()).collect();
        assert_eq!(loaded_types, types);
    }

    #[test]
    fn node_with_telemetry_status_but_no_badge() {
        let json = r#"{"id":"n1","type":"hardware","name":"Sensor","x":0,"y":0,"telemetry_status":"offline"}"#;
        let node: TopologyNodePayload = serde_json::from_str(json).unwrap();
        assert_eq!(node.telemetry_status.as_deref(), Some("offline"));
        assert!(node.telemetry_badge.is_none());
    }

    #[test]
    fn node_with_telemetry_badge_but_no_status() {
        let json = r#"{"id":"n1","type":"hardware","name":"Sensor","x":0,"y":0,"telemetry_badge":"Online"}"#;
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
        assert!(debug.contains("store"));
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

    // ── NaN / Infinity coordinate bugs ─────────────────────────────
    //
    // BUG: serde_json (default) serialises f64::NAN / INFINITY as JSON
    // `null`, which then FAILS to deserialise back to f64.  Any node
    // whose coordinates become non-finite poisons the entire topology
    // until manually deleted from the settings table.

    #[test]
    fn nan_x_coordinate_roundtrip_fails() {
        let node = TopologyNodePayload {
            id: "nan-x".into(),
            node_type: "store".into(),
            name: "NaN X".into(),
            subtitle: None,
            x: f64::NAN,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        // NaN is serialised as JSON `null` — which f64 cannot parse.
        let result: Result<TopologyNodePayload, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "NaN x should fail roundtrip");
    }

    #[test]
    fn nan_y_coordinate_roundtrip_fails() {
        let node = TopologyNodePayload {
            id: "nan-y".into(),
            node_type: "store".into(),
            name: "NaN Y".into(),
            subtitle: None,
            x: 0.0,
            y: f64::NAN,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        let result: Result<TopologyNodePayload, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "NaN y should fail roundtrip");
    }

    #[test]
    fn infinity_x_coordinate_roundtrip_fails() {
        let node = TopologyNodePayload {
            id: "inf-x".into(),
            node_type: "store".into(),
            name: "Inf X".into(),
            subtitle: None,
            x: f64::INFINITY,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        let result: Result<TopologyNodePayload, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "Infinity x should fail roundtrip");
    }

    #[test]
    fn neg_infinity_y_coordinate_roundtrip_fails() {
        let node = TopologyNodePayload {
            id: "neg-inf-y".into(),
            node_type: "store".into(),
            name: "Neg Inf Y".into(),
            subtitle: None,
            x: 0.0,
            y: f64::NEG_INFINITY,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        let result: Result<TopologyNodePayload, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "-Infinity y should fail roundtrip");
    }

    #[test]
    fn mixed_nan_poisons_entire_topology() {
        let good = TopologyNodePayload {
            id: "good".into(),
            node_type: "store".into(),
            name: "Good".into(),
            subtitle: None,
            x: 1.0,
            y: 2.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let bad = TopologyNodePayload {
            id: "bad".into(),
            node_type: "store".into(),
            name: "Bad".into(),
            subtitle: None,
            x: f64::NAN,
            y: 3.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        };
        let data = TopologyData {
            nodes: vec![good, bad],
            wires: vec![],
        };
        let json = serde_json::to_string(&data).unwrap();
        // Even one NaN in a multi-node topology makes the entire thing unloadable.
        let result: Result<TopologyData, _> = serde_json::from_str(&json);
        assert!(
            result.is_err(),
            "single NaN should poison entire TopologyData"
        );
    }

    // ── Graph integrity — wiring bugs ──────────────────────────────

    #[test]
    fn wire_to_nonexistent_node_persists() {
        let conn = fresh_conn();
        let data = TopologyData {
            nodes: vec![],
            wires: vec![TopologyWirePayload {
                id: "orphan".into(),
                from_node_id: "ghost".into(),
                to_node_id: "nowhere".into(),
                direction: "one-way".into(),
                label: None,
                from_port: None,
                to_port: None,
            }],
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.wires.len(), 1);
        assert_eq!(loaded.wires[0].from_node_id, "ghost");
        // System does NOT validate node references — this is a design gap.
        let node_ids: Vec<&str> = loaded.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(!node_ids.contains(&"ghost"));
    }

    #[test]
    fn duplicate_node_ids_are_not_rejected() {
        let data = TopologyData {
            nodes: vec![
                TopologyNodePayload {
                    id: "dup-id".into(),
                    node_type: "store".into(),
                    name: "First".into(),
                    subtitle: None,
                    x: 0.0,
                    y: 0.0,
                    tier_requirement: None,
                    telemetry_badge: None,
                    telemetry_status: None,
                    metadata: None,
                },
                TopologyNodePayload {
                    id: "dup-id".into(),
                    node_type: "workspace".into(),
                    name: "Second".into(),
                    subtitle: None,
                    x: 100.0,
                    y: 0.0,
                    tier_requirement: None,
                    telemetry_badge: None,
                    telemetry_status: None,
                    metadata: None,
                },
            ],
            wires: vec![],
        };
        let json = serde_json::to_string(&data).unwrap();
        let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.nodes.len(), 2);
        assert_eq!(roundtripped.nodes[0].id, roundtripped.nodes[1].id);
    }

    #[test]
    fn thousand_wires_between_two_nodes_roundtrip() {
        let wires: Vec<TopologyWirePayload> = (0..1000)
            .map(|i| TopologyWirePayload {
                id: format!("w-{i}"),
                from_node_id: "a".into(),
                to_node_id: "b".into(),
                direction: "one-way".into(),
                label: None,
                from_port: None,
                to_port: None,
            })
            .collect();
        let data = TopologyData {
            nodes: vec![],
            wires,
        };
        let json = serde_json::to_string(&data).unwrap();
        let roundtripped: TopologyData = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.wires.len(), 1000);
        assert_eq!(roundtripped.wires[0].id, "w-0");
        assert_eq!(roundtripped.wires[999].id, "w-999");
    }

    #[test]
    fn star_topology_preserved_through_db() {
        let conn = fresh_conn();
        let mut nodes: Vec<TopologyNodePayload> = (0..50)
            .map(|i| TopologyNodePayload {
                id: format!("leaf-{i}"),
                node_type: "workspace".into(),
                name: format!("Leaf {i}"),
                subtitle: None,
                x: (i as f64) * 20.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            })
            .collect();
        nodes.push(TopologyNodePayload {
            id: "hub".into(),
            node_type: "store".into(),
            name: "Hub".into(),
            subtitle: None,
            x: 500.0,
            y: 500.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: None,
        });
        let wires: Vec<TopologyWirePayload> = (0..50)
            .map(|i| TopologyWirePayload {
                id: format!("w-{i}"),
                from_node_id: "hub".into(),
                to_node_id: format!("leaf-{i}"),
                direction: "two-way".into(),
                label: None,
                from_port: None,
                to_port: None,
            })
            .collect();
        let data = TopologyData { nodes, wires };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.nodes.len(), 51);
        assert_eq!(loaded.wires.len(), 50);
        let hub_wires: Vec<&TopologyWirePayload> = loaded
            .wires
            .iter()
            .filter(|w| w.from_node_id == "hub")
            .collect();
        assert_eq!(hub_wires.len(), 50);
    }

    // ── Large-scale persistence stress ─────────────────────────────

    #[test]
    fn five_thousand_node_graph_db_roundtrip() {
        let conn = fresh_conn();
        let nodes: Vec<TopologyNodePayload> = (0..5000)
            .map(|i| TopologyNodePayload {
                id: format!("node-{i:05}"),
                node_type: if i % 4 == 0 {
                    "store"
                } else if i % 4 == 1 {
                    "workspace"
                } else if i % 4 == 2 {
                    "warehouse"
                } else {
                    "hardware"
                }
                .into(),
                name: format!("Node {i}"),
                subtitle: if i % 10 == 0 {
                    Some(format!("tenth-{i}"))
                } else {
                    None
                },
                x: (i as f64) * 1.5,
                y: (i as f64) * 0.5,
                tier_requirement: if i % 3 == 0 {
                    Some("premium".into())
                } else {
                    None
                },
                telemetry_badge: if i % 5 == 0 {
                    Some("Online".into())
                } else {
                    None
                },
                telemetry_status: if i % 7 == 0 {
                    Some("online".into())
                } else {
                    None
                },
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
        assert_eq!(loaded.nodes.len(), 5000);
        // Verify a few sample nodes for data integrity.
        assert_eq!(loaded.nodes[0].id, "node-00000");
        assert_eq!(loaded.nodes[999].node_type, "hardware");
        assert_eq!(loaded.nodes[1000].name, "Node 1000");
        // Nodes where i % 10 == 0 get a subtitle ("tenth-0", "tenth-10", …).
        assert_eq!(loaded.nodes[0].subtitle.as_deref(), Some("tenth-0"));
        assert_eq!(loaded.nodes[10].subtitle.as_deref(), Some("tenth-10"));
        assert!(
            loaded.nodes[9].subtitle.is_none(),
            "i=9 → no subtitle (9 % 10 = 9)"
        );
    }

    #[test]
    fn three_thousand_wires_db_roundtrip() {
        let conn = fresh_conn();
        let nodes: Vec<TopologyNodePayload> = (0..100)
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
            .collect();
        let wires: Vec<TopologyWirePayload> = (0..3000)
            .map(|i| TopologyWirePayload {
                id: format!("w-{i}"),
                from_node_id: format!("n-{}", i % 100),
                to_node_id: format!("n-{}", (i + 1) % 100),
                direction: if i % 2 == 0 { "one-way" } else { "two-way" }.into(),
                label: if i % 10 == 0 {
                    Some(format!("Label {i}"))
                } else {
                    None
                },
                from_port: None,
                to_port: None,
            })
            .collect();
        let data = TopologyData { nodes, wires };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.nodes.len(), 100);
        assert_eq!(loaded.wires.len(), 3000);
        assert_eq!(loaded.wires[0].direction, "one-way");
        assert_eq!(loaded.wires[1].direction, "two-way");
        assert_eq!(loaded.wires[0].label.as_deref(), Some("Label 0"));
        assert!(loaded.wires[1].label.is_none());
    }

    // ── Extreme string values ──────────────────────────────────────

    #[test]
    fn node_id_with_special_url_chars() {
        let special_id = "store/1?region=west#section@host:port/path";
        let node = TopologyNodePayload {
            id: special_id.into(),
            node_type: "store".into(),
            name: "URL Chars".into(),
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
        assert_eq!(roundtripped.id, special_id);
    }

    #[test]
    fn hundred_kb_node_name_roundtrip() {
        let long_name = "X".repeat(100_000);
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
        assert_eq!(roundtripped.name.len(), 100_000);
        assert_eq!(roundtripped.name, long_name);
    }

    #[test]
    fn hundred_kb_wire_label_roundtrip() {
        let long_label = "Y".repeat(100_000);
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
        assert_eq!(roundtripped.label.as_deref().unwrap().len(), 100_000);
    }

    // ── Direction edge cases ───────────────────────────────────────

    #[test]
    fn empty_direction_preserved() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":""}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.direction, "");
    }

    #[test]
    fn direction_with_unicode() {
        let wire = TopologyWirePayload {
            id: "w1".into(),
            from_node_id: "a".into(),
            to_node_id: "b".into(),
            direction: "↔ bidirectional ←→".into(),
            label: None,
            from_port: None,
            to_port: None,
        };
        let json = serde_json::to_string(&wire).unwrap();
        let roundtripped: TopologyWirePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.direction, "↔ bidirectional ←→");
    }

    #[test]
    fn direction_preserved_through_db() {
        let conn = fresh_conn();
        let data = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: "n1".into(),
                node_type: "store".into(),
                name: "Dir Test".into(),
                subtitle: None,
                x: 0.0,
                y: 0.0,
                tier_requirement: None,
                telemetry_badge: None,
                telemetry_status: None,
                metadata: None,
            }],
            wires: vec![TopologyWirePayload {
                id: "w1".into(),
                from_node_id: "n1".into(),
                to_node_id: "n2".into(),
                direction: "custom-hybrid".into(),
                label: None,
                from_port: None,
                to_port: None,
            }],
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.wires[0].direction, "custom-hybrid");
    }

    // ── Metadata extreme values ────────────────────────────────────

    #[test]
    fn deeply_nested_metadata_roundtrip() {
        let mut nested = serde_json::json!("deep");
        for _ in 0..64 {
            nested = serde_json::json!({"level": nested});
        }
        let node = TopologyNodePayload {
            id: "deep-meta".into(),
            node_type: "store".into(),
            name: "Deep".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: Some(nested),
        };
        let json = serde_json::to_string(&node).unwrap();
        let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
        let meta = roundtripped.metadata.unwrap();
        // Navigate 64 levels deep.
        let mut current = &meta;
        for _ in 0..64 {
            current = &current["level"];
        }
        assert_eq!(current.as_str(), Some("deep"));
    }

    #[test]
    fn metadata_with_large_array() {
        let large_array: Vec<i64> = (0..10_000).collect();
        let node = TopologyNodePayload {
            id: "array-meta".into(),
            node_type: "store".into(),
            name: "Large Array".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: Some(serde_json::json!({"values": large_array})),
        };
        let json = serde_json::to_string(&node).unwrap();
        let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
        let meta = roundtripped.metadata.unwrap();
        assert_eq!(meta["values"].as_array().unwrap().len(), 10_000);
        assert_eq!(meta["values"][9999], 9999);
    }

    #[test]
    fn metadata_with_hundred_kb_string() {
        let big_string = "Z".repeat(100_000);
        let node = TopologyNodePayload {
            id: "big-str-meta".into(),
            node_type: "store".into(),
            name: "Big String".into(),
            subtitle: None,
            x: 0.0,
            y: 0.0,
            tier_requirement: None,
            telemetry_badge: None,
            telemetry_status: None,
            metadata: Some(serde_json::json!({"big_field": big_string})),
        };
        let json = serde_json::to_string(&node).unwrap();
        let roundtripped: TopologyNodePayload = serde_json::from_str(&json).unwrap();
        let meta = roundtripped.metadata.unwrap();
        assert_eq!(meta["big_field"].as_str().unwrap().len(), 100_000);
    }

    // ── Ordering stability through DB ──────────────────────────────

    #[test]
    fn wire_order_preserved_through_db() {
        let conn = fresh_conn();
        let wires: Vec<TopologyWirePayload> = (0..100)
            .map(|i| TopologyWirePayload {
                id: format!("ordered-wire-{i:03}"),
                from_node_id: "a".into(),
                to_node_id: "b".into(),
                direction: "one-way".into(),
                label: Some(format!("label-{i}")),
                from_port: None,
                to_port: None,
            })
            .collect();
        let data = TopologyData {
            nodes: vec![],
            wires,
        };
        let json = serde_json::to_string(&data).unwrap();
        oz_core::Settings::set(&conn, TOPOLOGY_SETTING_KEY, &json).unwrap();
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        for i in 0..100 {
            assert_eq!(
                loaded.wires[i].id,
                format!("ordered-wire-{i:03}"),
                "wire order broken at index {i}"
            );
            assert_eq!(
                loaded.wires[i].label.as_deref(),
                Some(format!("label-{i}").as_str()),
            );
        }
    }

    #[test]
    fn node_order_preserved_through_db() {
        let conn = fresh_conn();
        let nodes: Vec<TopologyNodePayload> = (0..100)
            .map(|i| TopologyNodePayload {
                id: format!("ordered-node-{i:03}"),
                node_type: "store".into(),
                name: format!("Node {i}"),
                subtitle: None,
                x: i as f64,
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
        for i in 0..100 {
            assert_eq!(
                loaded.nodes[i].id,
                format!("ordered-node-{i:03}"),
                "node order broken at index {i}"
            );
            assert_eq!(loaded.nodes[i].name, format!("Node {i}"),);
        }
    }

    // ── Concurrent modification simulation ─────────────────────────

    #[test]
    fn sequential_save_a_then_b_then_verify_b() {
        let conn = fresh_conn();
        // First, save A.
        let data_a = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: "a".into(),
                node_type: "store".into(),
                name: "Data A".into(),
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
            &serde_json::to_string(&data_a).unwrap(),
        )
        .unwrap();

        // Then, save B.
        let data_b = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: "b".into(),
                node_type: "workspace".into(),
                name: "Data B".into(),
                subtitle: None,
                x: 100.0,
                y: 100.0,
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
            &serde_json::to_string(&data_b).unwrap(),
        )
        .unwrap();

        // Verify B is loaded.
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.nodes[0].id, "b");
        assert_eq!(loaded.nodes[0].name, "Data B");
    }

    #[test]
    fn hundred_save_cycles_data_integrity() {
        let conn = fresh_conn();
        for i in 0..100 {
            let data = TopologyData {
                nodes: vec![TopologyNodePayload {
                    id: format!("cycle-{i}"),
                    node_type: "store".into(),
                    name: format!("Cycle {i} with some data"),
                    subtitle: if i % 2 == 0 {
                        Some(format!("even-{i}"))
                    } else {
                        None
                    },
                    x: i as f64,
                    y: (i * 2) as f64,
                    tier_requirement: if i % 3 == 0 {
                        Some("premium".into())
                    } else {
                        None
                    },
                    telemetry_badge: if i % 5 == 0 {
                        Some("Online".into())
                    } else {
                        None
                    },
                    telemetry_status: if i % 7 == 0 {
                        Some("online".into())
                    } else {
                        None
                    },
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
        }
        // Verify last cycle.
        let loaded_raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        let loaded: TopologyData = serde_json::from_str(&loaded_raw).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.nodes[0].id, "cycle-99");
        assert_eq!(loaded.nodes[0].name, "Cycle 99 with some data");
        assert_eq!(loaded.nodes[0].x, 99.0);
        assert_eq!(loaded.nodes[0].y, 198.0);
        assert!(loaded.nodes[0].subtitle.is_none()); // 99 is odd
        assert_eq!(loaded.nodes[0].tier_requirement.as_deref(), Some("premium")); // 99 % 3 == 0
    }

    // ── Settings raw-value inspection ──────────────────────────────

    #[test]
    fn raw_stored_json_is_valid_topology() {
        let conn = fresh_conn();
        let data = TopologyData {
            nodes: vec![TopologyNodePayload {
                id: "raw-test".into(),
                node_type: "hardware".into(),
                name: "Raw Check".into(),
                subtitle: Some("verify".into()),
                x: 10.0,
                y: 20.0,
                tier_requirement: Some("standard".into()),
                telemetry_badge: Some("Online".into()),
                telemetry_status: Some("online".into()),
                metadata: Some(serde_json::json!({"checked": true})),
            }],
            wires: vec![TopologyWirePayload {
                id: "raw-wire".into(),
                from_node_id: "raw-test".into(),
                to_node_id: "other".into(),
                direction: "two-way".into(),
                label: Some("raw-label".into()),
                from_port: Some("out".into()),
                to_port: Some("in".into()),
            }],
        };
        oz_core::Settings::set(
            &conn,
            TOPOLOGY_SETTING_KEY,
            &serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        let raw = oz_core::Settings::get(&conn, TOPOLOGY_SETTING_KEY)
            .unwrap()
            .unwrap();
        // Raw JSON must parse as TopologyData.
        let reparsed: TopologyData = serde_json::from_str(&raw).unwrap();
        assert_eq!(reparsed.nodes.len(), 1);
        assert_eq!(reparsed.wires.len(), 1);
        // JSON must contain expected field markers.
        assert!(raw.contains(r#""type":"hardware""#));
        assert!(raw.contains(r#""direction":"two-way""#));
        assert!(raw.contains(r#""metadata""#));
    }

    // ── Empty / missing / null direction edge cases ────────────────

    #[test]
    fn direction_null_fails() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b","direction":null}"#;
        let result: Result<TopologyWirePayload, _> = serde_json::from_str(json);
        // serde's `#[serde(default)]` only applies to *absent* fields, not
        // explicitly-null ones — null for a plain String is a type error.
        assert!(result.is_err());
    }

    #[test]
    fn direction_missing_defaults_to_one_way() {
        let json = r#"{"id":"w1","from_node_id":"a","to_node_id":"b"}"#;
        let wire: TopologyWirePayload = serde_json::from_str(json).unwrap();
        assert_eq!(wire.direction, "one-way");
    }
}
