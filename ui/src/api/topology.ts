// ── Topology Persistence ───────────────────────────────────────────
// Save / load the node topology graph via Tauri IPC. The backend
// serialises nodes + wires as JSON and stores each branch under a
// branch-specific settings key derived from `oz-pos/topology`.

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
  metadata?: Record<string, unknown>;
  /** Canonical store_profiles.id for a Branch Location node. */
  store_profile_id?: string;
}

/** A wire connecting two port sockets. */
export interface TopologyWirePayload {
  id: string;
  from_node_id: string;
  to_node_id: string;
  direction: string;
  label?: string;
  /** Orthogonal bend points the wire routes through (canvas coords). */
  bends?: Array<{ x: number; y: number }>;
  from_port?: string;
  to_port?: string;
  /** Semantic source port ID; geometric anchors remain presentation data. */
  from_port_id?: string;
  /** Semantic target port ID; geometric anchors remain presentation data. */
  to_port_id?: string;
  /** Closed semantic relationship type. */
  relationship_type?: string;
}

/** Complete topology graph persisted to the backend. */
export interface TopologyData {
  /** Version of the semantic graph envelope. Legacy payloads omit this. */
  schema_version?: number;
  nodes: TopologyNodePayload[];
  wires: TopologyWirePayload[];
}

/** Persist the topology graph for one branch. Overwrites that branch's save. */
export const saveTopology = (
  nodes: TopologyNodePayload[],
  wires: TopologyWirePayload[],
  branchId?: string,
): Promise<void> =>
  loggedInvoke('save_topology', {
    nodes,
    wires,
    ...(branchId !== undefined ? { branchId } : {}),
  });

/** Load the persisted topology graph for a branch, or `null` if none saved yet. */
export const loadTopology = (branchId?: string): Promise<TopologyData | null> =>
  loggedInvoke<TopologyData | null>(
    'load_topology',
    branchId !== undefined ? { branchId } : undefined,
  );

// ── Atomic topology diff (Critical #4) ───────────────────────────

/**
 * Request body for creating a workspace instance in a topology diff.
 *
 * Mirrors `CreateInstanceRequest` from `@/api/workspaces` — kept here
 * because the topology module is the canonical owner of the diff
 * contract. Both types must stay in sync.
 */
export interface CreateInstanceRequest {
  id: string;
  type_key: string;
  store_id: string;
  name: string;
  /** Controlled business purpose; independent from type and display label. */
  purpose_key?: string;
  description?: string;
  colour?: string;
}

/** Request body for updating a workspace instance in a topology diff. */
export interface UpdateInstanceRequest {
  id: string;
  name: string;
  purpose_key?: string;
}

/**
 * Apply a full topology diff atomically.
 *
 * Creates, updates, and archives workspace instances within a single
 * SQLite transaction on the store database, then saves the topology
 * diagram (nodes + wires) on the global database under the selected branch
 * identity.
 *
 * Replaces the previous pattern of 4+ sequential `await` calls
 * (createWorkspaceInstanceScoped, updateWorkspaceInstanceScoped,
 * archiveWorkspaceInstanceScoped, saveTopology) with a single atomic
 * round-trip. If any workspace mutation fails, all are rolled back.
 */
export const applyTopologyDiff = (
  sessionToken: string,
  workspaceCreations: CreateInstanceRequest[],
  workspaceUpdates: UpdateInstanceRequest[],
  workspaceArchives: string[],
  diagramNodes: TopologyNodePayload[],
  diagramWires: TopologyWirePayload[],
  branchId?: string,
): Promise<void> =>
  loggedInvoke('apply_topology_diff', {
    sessionToken,
    workspaceCreations,
    workspaceUpdates,
    workspaceArchives,
    diagramNodes,
    diagramWires,
    ...(branchId !== undefined ? { branchId } : {}),
  });
