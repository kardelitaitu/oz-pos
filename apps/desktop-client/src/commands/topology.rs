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
}
