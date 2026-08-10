import { describe, expect, it } from 'vitest';
import type { TopologyNodeData, TopologyWireData } from '../features/stores/NodeTopologyEditor';
import { computeCanvasDiff } from '../features/stores/topologyCanvasDiff';

const node = (id: string, x: number, y: number): TopologyNodeData =>
  ({ id, type: 'store', name: id, x, y } as TopologyNodeData);

const wire = (id: string, from: string, to: string): TopologyWireData =>
  ({ id, fromNodeId: from, toNodeId: to, direction: 'one-way' } as TopologyWireData);

describe('computeCanvasDiff', () => {
  it('returns all-zero counts for identical states', () => {
    const nodes = [node('a', 0, 0), node('b', 100, 100)];
    const wires = [wire('w1', 'a', 'b')];
    const diff = computeCanvasDiff(nodes, wires, nodes, wires);
    expect(diff).toEqual({
      nodesAdded: 0, nodesRemoved: 0, nodesMoved: 0,
      wiresAdded: 0, wiresRemoved: 0, total: 0,
    });
  });

  it('counts added, removed, and moved nodes separately', () => {
    const prev = [node('a', 0, 0), node('b', 100, 100), node('gone', 50, 50)];
    const next = [node('a', 0, 0), node('b', 124, 100), node('fresh', 200, 200)];
    const diff = computeCanvasDiff(prev, [], next, []);
    expect(diff.nodesAdded).toBe(1);   // fresh
    expect(diff.nodesRemoved).toBe(1); // gone
    expect(diff.nodesMoved).toBe(1);   // b: 100→124
    expect(diff.total).toBe(3);
  });

  it('counts wire adds and removes by id', () => {
    const prev = [wire('w1', 'a', 'b'), wire('w-old', 'b', 'c')];
    const next = [wire('w1', 'a', 'b'), wire('w-new', 'a', 'c')];
    const diff = computeCanvasDiff([], prev, [], next);
    expect(diff.wiresAdded).toBe(1);
    expect(diff.wiresRemoved).toBe(1);
    expect(diff.total).toBe(2);
  });

  it('does not count a wire endpoint rewrite as a change (id is identity)', () => {
    const prev = [wire('w1', 'a', 'b')];
    const next = [wire('w1', 'b', 'a')];
    const diff = computeCanvasDiff([], prev, [], next);
    expect(diff.wiresAdded).toBe(0);
    expect(diff.wiresRemoved).toBe(0);
    expect(diff.total).toBe(0);
  });

  it('treats a never-committed canvas as everything added', () => {
    const diff = computeCanvasDiff([], [], [node('a', 0, 0)], [wire('w1', 'a', 'a')]);
    expect(diff.nodesAdded).toBe(1);
    expect(diff.wiresAdded).toBe(1);
    expect(diff.total).toBe(2);
  });
});
