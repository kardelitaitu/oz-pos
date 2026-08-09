import { describe, expect, it } from 'vitest';
import { computeAutoLayout } from '@/features/stores/nodeTopologyLayout';

// The topology editor's one-click Auto-layout: a layered wire-direction
// engine. Sources rank 0, each wire target ranks one deeper (BFS), every
// rank becomes a column with rows stacked in prior-y order, and the whole
// result is translated so the placed bounding-box midpoint matches the
// original diagram's — reorganize in place, never jump.
describe('computeAutoLayout (layered wire-direction layout engine)', () => {
  it('ranks a multi-source DAG into direction columns and stacks rows in prior-y order', () => {
    // A deliberately tangled diagram: store at the bottom, workspaces
    // straddling each other, warehouse fed by the second workspace.
    const placed = computeAutoLayout(
      [
        { id: 'store', x: 0, y: 400 },
        { id: 'ws-a', x: 300, y: 100 },
        { id: 'ws-b', x: 700, y: 300 },
        { id: 'wh', x: 200, y: 500 },
      ],
      [
        { id: 'w1', fromNodeId: 'store', toNodeId: 'ws-a' },
        { id: 'w2', fromNodeId: 'store', toNodeId: 'ws-b' },
        { id: 'w3', fromNodeId: 'ws-b', toNodeId: 'wh' },
      ],
    );
    const byId = new Map(placed.map((p) => [p.id, p]));
    // store (rank 0) left of both workspaces (rank 1)…
    expect(byId.get('store')!.x).toBeLessThan(byId.get('ws-a')!.x);
    // …the same-rank workspaces share a column, stacked in prior-y order…
    expect(byId.get('ws-a')!.x).toBe(byId.get('ws-b')!.x);
    expect(byId.get('ws-a')!.y).toBeLessThan(byId.get('ws-b')!.y);
    // …and the rank-2 warehouse (fed by ws-b) is rightmost.
    expect(byId.get('wh')!.x).toBeGreaterThan(byId.get('ws-a')!.x);
    // Column tops align across ranks.
    expect(byId.get('store')!.y).toBe(byId.get('ws-a')!.y);
  });

  it('keeps the diagram centered: the placed bounding-box midpoint equals the original', () => {
    const nodes = [
      { id: 'store', x: 0, y: 400 },
      { id: 'ws-a', x: 300, y: 100 },
      { id: 'ws-b', x: 700, y: 300 },
      { id: 'wh', x: 200, y: 500 },
    ];
    const placed = computeAutoLayout(nodes, [
      { id: 'w1', fromNodeId: 'store', toNodeId: 'ws-a' },
      { id: 'w2', fromNodeId: 'store', toNodeId: 'ws-b' },
      { id: 'w3', fromNodeId: 'ws-b', toNodeId: 'wh' },
    ]);
    const midOf = (values: number[]) => (Math.min(...values) + Math.max(...values)) / 2;
    expect(midOf(placed.map((p) => p.x))).toBe(midOf(nodes.map((n) => n.x)));
    expect(midOf(placed.map((p) => p.y))).toBe(midOf(nodes.map((n) => n.y)));
  });

  it('does not move a single unwired node (anchor uses consistent geometry)', () => {
    // The old inline anchor compared the ORIGINAL origin-midpoint against
    // the PLACED box-midpoint (which adds NODE_WIDTH/2), so a lone node
    // jumped half a node-width on every Auto-layout click.
    expect(computeAutoLayout([{ id: 'a', x: 100, y: 100 }], []))
      .toEqual([{ id: 'a', x: 100, y: 100 }]);
  });

  it('collapses pure cycles into the source column instead of looping forever', () => {
    const placed = computeAutoLayout(
      [
        { id: 'a', x: 0, y: 0 },
        { id: 'b', x: 300, y: 0 },
      ],
      [
        { id: 'w1', fromNodeId: 'a', toNodeId: 'b' },
        { id: 'w2', fromNodeId: 'b', toNodeId: 'a' },
      ],
    );
    const byId = new Map(placed.map((p) => [p.id, p]));
    // Neither node is a source (each is pointed to), so both fall back to
    // rank 0 and land in the same column.
    expect(byId.get('a')!.x).toBe(byId.get('b')!.x);
  });

  it('returns an empty placement list for an empty diagram', () => {
    expect(computeAutoLayout([], [])).toEqual([]);
  });
});
