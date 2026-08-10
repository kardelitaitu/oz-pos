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

  // ── Id-drift tolerance (round 155) ─────────────────────────────
  //
  // A saved diagram can predate the instance-id conventions, or a
  // workspace can be archived-and-recreated under a new UUID (the
  // destructive type change from round 152). The same logical
  // workspace then has different ids across the two branches. Today
  // that reports phantom only-in-current + only-in-other entries and
  // hides wiring differences — the comparison must pair by semantic
  // identity (name + typeKey) when ids disagree.

  it('pairs a same-name same-type workspace across branches with a drifted id', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register'), wsNode('ws-kds', 'KDS', 'kds')],
      wires: [wire('ws-pos', 'ws-kds', 'generic')],
    };
    const b: TopologyDiagram = {
      nodes: [wsNode('ws-pos-v2', 'Front Register'), wsNode('ws-kds', 'KDS', 'kds')],
      wires: [wire('ws-pos-v2', 'ws-kds', 'generic')],
    };

    const result = compareBranchTopologies(a, b);
    expect(result.onlyInCurrent).toEqual([]);
    expect(result.onlyInOther).toEqual([]);
    expect(result.shared).toBe(2);
    expect(result.differing).toEqual([]);
  });

  it('flags a wiring difference on a drifted-id workspace instead of phantom entries', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register'), wsNode('ws-kds', 'KDS', 'kds')],
      wires: [wire('ws-pos', 'ws-kds', 'generic')],
    };
    const b: TopologyDiagram = {
      nodes: [wsNode('ws-pos-v2', 'Front Register'), wsNode('ws-wh', 'Stock Room')],
      wires: [wire('ws-pos-v2', 'ws-wh', 'stock-routing')],
    };

    const result = compareBranchTopologies(a, b);
    expect(result.onlyInCurrent).toEqual([{ id: 'ws-kds', name: 'KDS' }]);
    expect(result.onlyInOther).toEqual([{ id: 'ws-wh', name: 'Stock Room' }]);
    expect(result.shared).toBe(1);
    expect(result.differing).toEqual([
      { id: 'ws-pos', name: 'Front Register', reasons: ['wiring'] },
    ]);
  });

  it('does not guess when a drifted id is ambiguous (two same-key candidates)', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register')],
      wires: [],
    };
    const b: TopologyDiagram = {
      nodes: [
        wsNode('ws-pos-a', 'Front Register'),
        wsNode('ws-pos-b', 'Front Register'),
      ],
      wires: [],
    };

    const result = compareBranchTopologies(a, b);
    expect(result.onlyInCurrent).toEqual([{ id: 'ws-pos', name: 'Front Register' }]);
    expect(result.onlyInOther).toEqual([
      { id: 'ws-pos-a', name: 'Front Register' },
      { id: 'ws-pos-b', name: 'Front Register' },
    ]);
    expect(result.shared).toBe(0);
  });

  it('remaps a wire between two drifted workspaces when comparing', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-a', 'A', 't1'), wsNode('ws-b', 'B', 't2')],
      wires: [wire('ws-a', 'ws-b', 'generic')],
    };
    const b: TopologyDiagram = {
      nodes: [wsNode('ws-a2', 'A', 't1'), wsNode('ws-b2', 'B', 't2')],
      wires: [wire('ws-a2', 'ws-b2', 'generic')],
    };

    const result = compareBranchTopologies(a, b);
    expect(result.onlyInCurrent).toEqual([]);
    expect(result.onlyInOther).toEqual([]);
    expect(result.shared).toBe(2);
    expect(result.differing).toEqual([]);
  });

  it('does not pair when the type differs — a type change is a different instance', () => {
    const a: TopologyDiagram = {
      nodes: [wsNode('ws-pos', 'Front Register', 'store-pos')],
      wires: [],
    };
    const b: TopologyDiagram = {
      nodes: [wsNode('ws-pos-v2', 'Front Register', 'restaurant-pos')],
      wires: [],
    };

    const result = compareBranchTopologies(a, b);
    expect(result.onlyInCurrent).toEqual([{ id: 'ws-pos', name: 'Front Register' }]);
    expect(result.onlyInOther).toEqual([{ id: 'ws-pos-v2', name: 'Front Register' }]);
    expect(result.shared).toBe(0);
  });
});
