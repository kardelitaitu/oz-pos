/** Pure canvas-diff computation for the topology editor's Apply summary
 *  (round 148). The dirty chip now shows what an Apply would commit —
 *  the canvas diff against the last committed snapshot plus the revision
 *  bump — instead of a bare "Unsaved changes". Kept OUT of the component
 *  file so it is unit-testable in isolation and the editor stays lean.
 *
 *  Semantics mirror the editor's dirty check: identity by node/wire id,
 *  position changes (x/y) count as MOVED, everything else is a count.
 *  `added` = nodesAdded + wiresAdded, `removed` = nodesRemoved +
 *  wiresRemoved — the summary the user reads at a glance. */

import type { TopologyNodeData, TopologyWireData } from './NodeTopologyEditor';

export interface CanvasDiffSummary {
  nodesAdded: number;
  nodesRemoved: number;
  nodesMoved: number;
  wiresAdded: number;
  wiresRemoved: number;
  /** Total lines the user should scan: added + removed + moved. */
  total: number;
}

export function computeCanvasDiff(
  prevNodes: TopologyNodeData[],
  prevWires: TopologyWireData[],
  nextNodes: TopologyNodeData[],
  nextWires: TopologyWireData[],
): CanvasDiffSummary {
  const prevNodeIds = new Set(prevNodes.map((n) => n.id));
  const nextNodeIds = new Set(nextNodes.map((n) => n.id));
  const prevById = new Map(prevNodes.map((n) => [n.id, n]));
  const nextById = new Map(nextNodes.map((n) => [n.id, n]));

  let nodesAdded = 0;
  let nodesRemoved = 0;
  let nodesMoved = 0;
  for (const id of nextNodeIds) {
    if (!prevNodeIds.has(id)) {
      nodesAdded += 1;
    } else {
      const before = prevById.get(id)!;
      const after = nextById.get(id)!;
      if (before.x !== after.x || before.y !== after.y) nodesMoved += 1;
    }
  }
  for (const id of prevNodeIds) {
    if (!nextNodeIds.has(id)) nodesRemoved += 1;
  }

  const prevWireIds = new Set(prevWires.map((w) => w.id));
  const nextWireIds = new Set(nextWires.map((w) => w.id));
  let wiresAdded = 0;
  let wiresRemoved = 0;
  for (const id of nextWireIds) {
    if (!prevWireIds.has(id)) wiresAdded += 1;
  }
  for (const id of prevWireIds) {
    if (!nextWireIds.has(id)) wiresRemoved += 1;
  }

  return {
    nodesAdded,
    nodesRemoved,
    nodesMoved,
    wiresAdded,
    wiresRemoved,
    total: nodesAdded + nodesRemoved + nodesMoved + wiresAdded + wiresRemoved,
  };
}
