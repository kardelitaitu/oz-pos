/**
 * Pure topology export/import + diagram templates.
 *
 * The canvas (nodes + wires + authored bends) serializes to a versioned JSON
 * envelope so a layout can be copied to the clipboard, pasted back, or saved
 * under a name as a reusable template. `deserializeTopology` is STRICT: any
 * malformed entry rejects the whole payload, so a drifted or hand-edited
 * document can never half-load a broken diagram.
 */

import type { TopologyNodeData, TopologyWireData } from './NodeTopologyEditor';

export const TOPOLOGY_EXPORT_FORMAT = 'oz-topology';
export const TOPOLOGY_EXPORT_VERSION = 1;
/** localStorage prefix for saved diagram templates. */
export const TOPOLOGY_TEMPLATE_PREFIX = 'oz-topology-template:';

export interface TopologyExportPayload {
  format: typeof TOPOLOGY_EXPORT_FORMAT;
  version: typeof TOPOLOGY_EXPORT_VERSION;
  nodes: TopologyNodeData[];
  wires: TopologyWireData[];
}

const NODE_TYPES = new Set(['store', 'workspace', 'warehouse', 'hardware']);
const WIRE_DIRECTIONS = new Set(['one-way', 'reverse', 'two-way']);

const isFiniteNumber = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

function isValidNode(n: unknown): n is TopologyNodeData {
  if (!n || typeof n !== 'object') return false;
  const node = n as Record<string, unknown>;
  return (
    typeof node['id'] === 'string' && node['id'].length > 0
    && typeof node['type'] === 'string' && NODE_TYPES.has(node['type'])
    && typeof node['name'] === 'string'
    && isFiniteNumber(node['x']) && isFiniteNumber(node['y'])
  );
}

function isValidWire(w: unknown): w is TopologyWireData {
  if (!w || typeof w !== 'object') return false;
  const wire = w as Record<string, unknown>;
  return (
    typeof wire['id'] === 'string' && wire['id'].length > 0
    && typeof wire['fromNodeId'] === 'string'
    && typeof wire['toNodeId'] === 'string'
    && typeof wire['direction'] === 'string' && WIRE_DIRECTIONS.has(wire['direction'])
  );
}

/** Serialize the canvas into the versioned JSON envelope (pretty-printed for
 *  human inspection / diffing). */
export function serializeTopology(nodes: TopologyNodeData[], wires: TopologyWireData[]): string {
  return JSON.stringify(
    {
      format: TOPOLOGY_EXPORT_FORMAT,
      version: TOPOLOGY_EXPORT_VERSION,
      nodes,
      wires,
    } satisfies TopologyExportPayload,
    null,
    2,
  );
}

/** Parse + validate an export envelope. Strict: a malformed node/wire,
 *  duplicate id, or a version/format mismatch rejects the whole payload
 *  (returns null) rather than half-loading a broken diagram. */
export function deserializeTopology(json: string): TopologyExportPayload | null {
  let raw: unknown;
  try {
    raw = JSON.parse(json);
  } catch {
    return null;
  }
  if (!raw || typeof raw !== 'object') return null;
  const payload = raw as Record<string, unknown>;
  if (payload['format'] !== TOPOLOGY_EXPORT_FORMAT || payload['version'] !== TOPOLOGY_EXPORT_VERSION) return null;
  if (!Array.isArray(payload['nodes']) || !Array.isArray(payload['wires'])) return null;
  if (!payload['nodes'].every(isValidNode)) return null;
  if (!payload['wires'].every(isValidWire)) return null;
  const ids = new Set<string>();
  for (const n of payload['nodes'] as TopologyNodeData[]) {
    if (ids.has(n.id)) return null;
    ids.add(n.id);
  }
  return {
    format: TOPOLOGY_EXPORT_FORMAT,
    version: TOPOLOGY_EXPORT_VERSION,
    nodes: payload['nodes'] as TopologyNodeData[],
    wires: payload['wires'] as TopologyWireData[],
  };
}

/** Save the current canvas under `name` as a reusable template. Returns the
 *  storage key on success, null when storage is unavailable or the name is
 *  empty (caller toasts). */
export function saveTemplate(name: string, nodes: TopologyNodeData[], wires: TopologyWireData[]): string | null {
  const trimmed = name.trim();
  if (!trimmed) return null;
  try {
    const key = `${TOPOLOGY_TEMPLATE_PREFIX}${trimmed}`;
    localStorage.setItem(key, serializeTopology(nodes, wires));
    return key;
  } catch {
    return null;
  }
}

/** Load a saved template by name (null if missing, corrupt, or unavailable). */
export function loadTemplate(name: string): TopologyExportPayload | null {
  try {
    const json = localStorage.getItem(`${TOPOLOGY_TEMPLATE_PREFIX}${name}`);
    return json === null ? null : deserializeTopology(json);
  } catch {
    return null;
  }
}

/** Names of all saved templates, sorted for a stable list. */
export function listTemplates(): string[] {
  const names: string[] = [];
  try {
    for (let i = 0; i < localStorage.length; i += 1) {
      const key = localStorage.key(i);
      if (key && key.startsWith(TOPOLOGY_TEMPLATE_PREFIX)) {
        names.push(key.slice(TOPOLOGY_TEMPLATE_PREFIX.length));
      }
    }
  } catch {
    return [];
  }
  return names.sort((a, b) => a.localeCompare(b));
}

/** Delete a saved template by name. */
export function deleteTemplate(name: string): void {
  try {
    localStorage.removeItem(`${TOPOLOGY_TEMPLATE_PREFIX}${name}`);
  } catch {
    /* storage unavailable — nothing to do */
  }
}
