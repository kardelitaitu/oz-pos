import { afterEach, describe, expect, it, vi } from 'vitest';
import { historyEntry, validWiresForNodes } from '../features/stores/topologyHistoryIntegrity';

/**
 * The history-integrity guards drop a wire when its endpoints are missing
 * from the same node set (push-time in historyEntry, restore-time in
 * popUndo/popRedo). Legitimate snapshots never drop anything, so a drop is
 * a corruption signal — these pin that the diagnostic fires loudly and
 * names the offending wire, and that clean snapshots stay silent.
 */

const node = (id: string) => ({ id, type: 'workspace', name: id });
const wire = (id: string, fromNodeId: string, toNodeId: string) => ({ id, fromNodeId, toNodeId });

describe('topology history-integrity guards', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('drops a dangling wire at the restore boundary and warns with its id and endpoints', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const wires = [wire('w-1', 'a', 'b'), wire('w-2', 'a', 'ghost')];

    const kept = validWiresForNodes([node('a'), node('b')], wires, 'restore');

    expect(kept).toEqual([wires[0]]);
    expect(warn).toHaveBeenCalledTimes(1);
    const message = String(warn.mock.calls[0]![0]);
    expect(message).toContain('[topology]');
    expect(message).toContain('restore-time guard dropped 1 dangling wire(s)');
    expect(message).toContain('w-2 (a -> ghost)');
  });

  it('warns at the push boundary when entry creation absorbs the drop', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    // Both wires dangle: one endpoint is absent, the other wire is fully ghost.
    const entry = historyEntry([node('a')], [wire('w-1', 'a', 'b'), wire('w-2', 'b', 'a')]);

    expect(entry.wires).toEqual([]);
    expect(warn).toHaveBeenCalledTimes(1);
    const message = String(warn.mock.calls[0]![0]);
    expect(message).toContain('push-time guard dropped 2 dangling wire(s)');
    expect(message).toContain('w-1 (a -> b)');
    expect(message).toContain('w-2 (b -> a)');
  });

  it('is silent and identity-preserving for a consistent snapshot', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const nodes = [node('a'), node('b')];
    const wires = [wire('w-1', 'a', 'b')];

    expect(validWiresForNodes(nodes, wires, 'restore')).toEqual(wires);
    expect(historyEntry(nodes, wires).wires).toEqual(wires);
    expect(warn).not.toHaveBeenCalled();
  });
});
