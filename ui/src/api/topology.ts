// ── Topology Persistence ───────────────────────────────────────────
// Save / load the node topology graph via Tauri IPC.  The backend
// serialises nodes + wires as JSON and stores them in the settings
// table under the key `oz-pos/topology`.

import { loggedInvoke } from '@/utils/logged-invoke';

/** A single node in the topology graph. */
export interface TopologyNodePayload {
  id: string;
  type: string;
  name: string;
  subtitle?: string;
  x: number;
  y: number;
  tier_requirement?: string;
  telemetry_badge?: string;
  telemetry_status?: string;
  metadata?: Record<string, string>;
}

/** A wire connecting two port sockets. */
export interface TopologyWirePayload {
  id: string;
  from_node_id: string;
  to_node_id: string;
  direction: string;
  label?: string;
  from_port?: string;
  to_port?: string;
}

/** Complete topology graph persisted to the backend. */
export interface TopologyData {
  nodes: TopologyNodePayload[];
  wires: TopologyWirePayload[];
}

/** Persist the topology graph. Overwrites any previous save. */
export const saveTopology = (
  nodes: TopologyNodePayload[],
  wires: TopologyWirePayload[],
): Promise<void> => loggedInvoke('save_topology', { nodes, wires });

/** Load the persisted topology graph, or `null` if none saved yet. */
export const loadTopology = (): Promise<TopologyData | null> =>
  loggedInvoke<TopologyData | null>('load_topology');
