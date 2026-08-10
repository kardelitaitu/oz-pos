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

function setsEqual(a: Set<string> | undefined, b: Set<string> | undefined): boolean {
  if (!a || !b) return (a?.size ?? 0) === (b?.size ?? 0);
  if (a.size !== b.size) return false;
  for (const key of a) {
    if (!b.has(key)) return false;
  }
  return true;
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

  const onlyInCurrent: BranchWorkspaceRef[] = [];
  const onlyInOther: BranchWorkspaceRef[] = [];
  const differing: DifferingWorkspace[] = [];

  let shared = 0;
  for (const [id, node] of currentNodes) {
    if (!otherNodes.has(id)) {
      onlyInCurrent.push({ id, name: node.name });
    }
  }
  for (const [id, node] of otherNodes) {
    if (!currentNodes.has(id)) {
      onlyInOther.push({ id, name: node.name });
    } else {
      shared += 1;
    }
  }

  const currentWiring = wiringByNode(current);
  const otherWiring = wiringByNode(other);

  for (const [id, node] of currentNodes) {
    const otherNode = otherNodes.get(id);
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
