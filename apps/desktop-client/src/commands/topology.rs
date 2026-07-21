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
}
