// ── Branch-to-branch topology comparison ──────────────────────────
//
// Pure engine behind the screen's Compare panel (round 154): given two
// saved diagrams (the current branch's and another branch's), classify
// the workspace nodes as only-in-current, only-in-other, or shared —
// and for shared ids, flag name / type / wiring differences. An
// operator with several locations can see how two branches' topologies
// differ before editing either one.
//
// Deliberately display-only: it never resolves store ownership or
// builds apply payloads — `planTopologyDiff` / `computeTopologyDiff`
// own the commit side. Wiring is compared as undirected connections
// (direction is presentation, not topology).

import type { TopologyNodePayload, TopologyWirePayload } from '@/api/topology';

/** The subset of a saved diagram the comparison reasons about. */
export interface TopologyDiagram {
  nodes: TopologyNodePayload[];
  wires: TopologyWirePayload[];
}

export interface BranchWorkspaceRef {
  id: string;
  name: string;
}

export interface DifferingWorkspace {
  id: string;
  name: string;
  reasons: Array<'name' | 'type' | 'wiring'>;
}

export interface BranchTopologyComparison {
  /** Workspace ids present in the current diagram only. */
  onlyInCurrent: BranchWorkspaceRef[];
  /** Workspace ids present in the other diagram only. */
  onlyInOther: BranchWorkspaceRef[];
  /** Number of workspace ids present in both diagrams. */
  shared: number;
  /** Shared workspaces whose name, type, or wiring differs. */
  differing: DifferingWorkspace[];
}

/** Undirected connection key — a wire's endpoints, order-normalised. */
function connectionKey(a: string, b: string): string {
  return a < b ? `${a}\u0000${b}` : `${b}\u0000${a}`;
}

/** Incident connection keys per node id (any wire touching the node). */
function wiringByNode(diagram: TopologyDiagram | null): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  if (!diagram) return map;
  for (const wire of diagram.wires) {
    const key = connectionKey(wire.from_node_id, wire.to_node_id);
    for (const endpoint of [wire.from_node_id, wire.to_node_id]) {
      let set = map.get(endpoint);
      if (!set) {
        set = new Set();
        map.set(endpoint, set);
      }
      set.add(key);
    }
  }
  return map;
}

/**
 * Incident connection keys per node id, with wire endpoints remapped
 * through the drift pairing (other-side id → current-side id). A wire
 * whose endpoint is a drifted workspace is compared against the
 * current side's id so wiring can be judged on equal ground.
 */
function wiringByNodeRemapped(
  diagram: TopologyDiagram | null,
  drift: ReadonlyMap<string, string>,
): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  if (!diagram) return map;
  for (const wire of diagram.wires) {
    const a = drift.get(wire.from_node_id) ?? wire.from_node_id;
    const b = drift.get(wire.to_node_id) ?? wire.to_node_id;
    const key = connectionKey(a, b);
    for (const endpoint of [a, b]) {
      let set = map.get(endpoint);
      if (!set) {
        set = new Set();
        map.set(endpoint, set);
      }
      set.add(key);
    }
  }
  return map;
}

function setsEqual(a: Set<string> | undefined, b: Set<string> | undefined): boolean {
  if (!a || !b) return (a?.size ?? 0) === (b?.size ?? 0);
  if (a.size !== b.size) return false;
  for (const key of a) {
    if (!b.has(key)) return false;
  }
  return true;
}

/**
 * Pair drifted ids by semantic identity: same name AND same typeKey.
 * A drifted pair is one other-side workspace whose id is absent on the
 * current side but whose name + typeKey match a current-side workspace
 * exactly. The match must be unambiguous — if two candidates share the
 * same semantic identity, neither is paired (no guessing). Type is part
 * of the key: a type change is a destructive recreate (round 152), i.e.
 * a different instance, not the same workspace with a drifted id.
 */
function findDriftPairs(
  currentNodes: Map<string, TopologyNodePayload>,
  otherNodes: Map<string, TopologyNodePayload>,
): Map<string, string> {
  const drift = new Map<string, string>(); // other id → current id
  for (const [currentId, currentNode] of currentNodes) {
    if (otherNodes.has(currentId)) continue; // exact id match already covers it
    const candidates = [...otherNodes].filter(([otherId, otherNode]) => {
      if (currentNodes.has(otherId)) return false; // exact-matched on the other side
      if (drift.has(otherId)) return false; // already claimed by a previous pair
      return (
        otherNode.name === currentNode.name &&
        otherNode.metadata?.['typeKey'] === currentNode.metadata?.['typeKey']
      );
    });
    if (candidates.length === 1) {
      drift.set(candidates[0]![0], currentId);
    }
  }
  return drift;
}

// ── Canvas overlay descriptors (round 158) ───────────────────────
//
// The Compare panel's spatial diff: the other branch's topology rendered
// over the canvas. Other-only workspaces become ghost cards at their
// SAVED positions in the other diagram; current-only workspaces get a red
// marker on their existing card; shared-but-differing workspaces an amber
// one. Pure and display-only — the editor consumes the descriptor list
// and never writes anything back.

export interface TopologyOverlay {
  /** Other-only workspaces: render as ghost cards at the other diagram's
   *  saved positions (its coordinates, not the current side's). */
  ghosts: Array<{ id: string; name: string; x: number; y: number }>;
  /** Current-only workspace ids: a red marker on the existing card. */
  onlyHere: string[];
  /** Shared-but-differing workspace ids: an amber marker on the existing
   *  card. A drifted-id pair (round 155) is shared — it lands here only
   *  when its wiring actually differs. */
  differing: string[];
}

export function buildTopologyOverlay(
  current: TopologyDiagram | null,
  other: TopologyDiagram | null,
): TopologyOverlay {
  const comparison = compareBranchTopologies(current, other);

  const otherPos = new Map(
    (other?.nodes ?? [])
      .filter((n) => n.type === 'workspace')
      .map((n) => [n.id, n] as const),
  );
  const currentPos = new Map(
    (current?.nodes ?? [])
      .filter((n) => n.type === 'workspace')
      .map((n) => [n.id, n] as const),
  );

  const ghosts: TopologyOverlay['ghosts'] = [];
  for (const ref of comparison.onlyInOther) {
    const node = otherPos.get(ref.id);
    if (!node) continue;
    ghosts.push({ id: ref.id, name: ref.name, x: node.x, y: node.y });
  }

  const onlyHere = comparison.onlyInCurrent
    .map((ref) => ref.id)
    .filter((id) => currentPos.has(id));
  const differing = comparison.differing
    .map((ref) => ref.id)
    .filter((id) => currentPos.has(id));

  return { ghosts, onlyHere, differing };
}

export function compareBranchTopologies(
  current: TopologyDiagram | null,
  other: TopologyDiagram | null,
): BranchTopologyComparison {
  const currentNodes = new Map(
    (current?.nodes ?? [])
      .filter((n) => n.type === 'workspace')
      .map((n) => [n.id, n] as const),
  );
  const otherNodes = new Map(
    (other?.nodes ?? [])
      .filter((n) => n.type === 'workspace')
      .map((n) => [n.id, n] as const),
  );

  // Pair drifted ids first so shared counting and wiring comparison see
  // semantic identity, not raw ids.
  const drift = findDriftPairs(currentNodes, otherNodes);

  const onlyInCurrent: BranchWorkspaceRef[] = [];
  const onlyInOther: BranchWorkspaceRef[] = [];
  const differing: DifferingWorkspace[] = [];

  const matchedCurrent = new Set<string>(drift.values());
  const matchedOther = new Set<string>(drift.keys());

  let shared = 0;
  for (const [id, node] of currentNodes) {
    if (!otherNodes.has(id) && !matchedCurrent.has(id)) {
      onlyInCurrent.push({ id, name: node.name });
    }
  }
  for (const [id, node] of otherNodes) {
    if (!currentNodes.has(id) && !matchedOther.has(id)) {
      onlyInOther.push({ id, name: node.name });
    } else if (currentNodes.has(id)) {
      shared += 1;
    }
  }
  shared += drift.size;

  const currentWiring = wiringByNode(current);
  const otherWiring = wiringByNodeRemapped(other, drift);

  const otherIdByCurrentId = new Map<string, string>();
  for (const [otherId, currentId] of drift) {
    otherIdByCurrentId.set(currentId, otherId);
  }

  for (const [id, node] of currentNodes) {
    const otherNode = otherNodes.get(id) ?? otherNodes.get(otherIdByCurrentId.get(id) ?? '');
    if (!otherNode) continue;
    const reasons: DifferingWorkspace['reasons'] = [];
    if (node.name !== otherNode.name) reasons.push('name');
    if (node.metadata?.['typeKey'] !== otherNode.metadata?.['typeKey']) reasons.push('type');
    if (!setsEqual(currentWiring.get(id), otherWiring.get(id))) reasons.push('wiring');
    if (reasons.length > 0) {
      differing.push({ id, name: node.name, reasons });
    }
  }

  return { onlyInCurrent, onlyInOther, shared, differing };
}
