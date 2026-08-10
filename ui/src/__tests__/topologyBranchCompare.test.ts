// ── compareBranchTopologies unit tests ────────────────────────────
//
// Pins the branch-to-branch topology comparison (round 154): given two
// saved diagrams, classify workspace nodes as only-in-current,
// only-in-other, shared-and-identical, or shared-but-differing (name,
// type, or wiring). The screen's Compare panel renders this summary so
// an operator can see how two locations' topologies differ before
// editing either one.

import { describe, expect, it } from 'vitest';
import { compareBranchTopologies } from '@/features/stores/topologyBranchCompare';
import type { TopologyDiagram } from '@/features/stores/topologyBranchCompare';
import type { TopologyNodePayload, TopologyWirePayload } from '@/api/topology';

// ── Fixtures ──────────────────────────────────────────────────────

function wsNode(id: string, name: string, typeKey = 'store-pos'): TopologyNodePayload {
  return { id, type: 'workspace', name, x: 0, y: 0, metadata: { typeKey } };
}

function wire(from: string, to: string, rel: string, id = `w-${from}-${to}`): TopologyWirePayload {
  return {
    id,
    from_node_id: from,
    to_node_id: to,
    direction: 'one-way',
    relationship_type: rel,
  };
}

const current: TopologyDiagram = {
  nodes: [
    wsNode('ws-pos', 'Front Register'),
    wsNode('ws-kds', 'Kitchen Display', 'kds'),
  ],
  wires: [wire('ws-pos', 'ws-kds', 'generic')],
};

const other: TopologyDiagram = {
  nodes: [
    wsNode('ws-pos', 'Front Register'),
    wsNode('ws-wh', 'Stock Room'),
  ],
  wires: [wire('ws-pos', 'ws-wh', 'stock-routing')],
};

// ── Tests ─────────────────────────────────────────────────────────

describe('compareBranchTopologies', () => {
  it('reports workspaces present in only one of the two diagrams', () => {
    const result = compareBranchTopologies(current, other);

    expect(result.onlyInCurrent).toEqual([{ id: 'ws-kds', name: 'Kitchen Display' }]);
    expect(result.onlyInOther).toEqual([{ id: 'ws-wh', name: 'Stock Room' }]);
  });

  it('counts shared workspaces and flags wiring differences on the shared id', () => {
    const result = compareBranchTopologies(current, other);

    expect(result.shared).toBe(1);
    expect(result.differing).toHaveLength(1);
    expect(result.differing[0]).toMatchObject({
      id: 'ws-pos',
      name: 'Front Register',
      reasons: ['wiring'],
    });
  });

  it('flags a name difference and a type difference on a shared workspace', () => {
    const renamed: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Back Register')],
      wires: [],
    };
    const retyped: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register', 'restaurant-pos')],
      wires: [],
    };
    const base: TopologyDiagram = { nodes: [wsNode('ws-pos', 'Front Register')], wires: [] };

    expect(compareBranchTopologies(base, renamed).differing[0]!.reasons).toEqual(['name']);
    expect(compareBranchTopologies(base, retyped).differing[0]!.reasons).toEqual(['type']);
  });

  it('does not flag a shared workspace that is identical in both diagrams', () => {
    const result = compareBranchTopologies(current, current);

    expect(result.shared).toBe(2);
    expect(result.onlyInCurrent).toEqual([]);
    expect(result.onlyInOther).toEqual([]);
    expect(result.differing).toEqual([]);
  });

  it('treats a missing diagram as an empty one (everything in the other side)', () => {
    const result = compareBranchTopologies(null, other);

    expect(result.onlyInOther).toEqual([
      { id: 'ws-pos', name: 'Front Register' },
      { id: 'ws-wh', name: 'Stock Room' },
    ]);
    expect(result.onlyInCurrent).toEqual([]);
    expect(result.shared).toBe(0);
    expect(result.differing).toEqual([]);
  });

  it('ignores direction flips — direction is presentation, not a wiring difference', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register'), wsNode('ws-kds', 'KDS', 'kds')],
      wires: [wire('ws-pos', 'ws-kds', 'generic')],
    };
    const b: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register'), wsNode('ws-kds', 'KDS', 'kds')],
      wires: [{ ...wire('ws-pos', 'ws-kds', 'generic'), direction: 'reverse' }],
    };

    expect(compareBranchTopologies(a, b).differing).toEqual([]);
  });

  it('returns empty for two null diagrams', () => {
    const result = compareBranchTopologies(null, null);

    expect(result.onlyInCurrent).toEqual([]);
    expect(result.onlyInOther).toEqual([]);
    expect(result.shared).toBe(0);
    expect(result.differing).toEqual([]);
  });
});
