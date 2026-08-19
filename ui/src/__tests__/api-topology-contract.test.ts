// ── IPC contract tests for topology.ts ─────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  canSaveTopology,
  loadTopology,
} from '@/api/topology';

describe('topology.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('canSaveTopology → can_save_topology with sessionToken', async () => {
    mockInvoke.mockResolvedValue(true);
    await canSaveTopology('tok');
    expect(mockInvoke).toHaveBeenCalledWith('can_save_topology', { sessionToken: 'tok' });
  });

  it('loadTopology without branchId → load_topology (no args)', async () => {
    mockInvoke.mockResolvedValue(null);
    await loadTopology();
    // branchId is undefined → loggedInvoke gets undefined as second arg
    expect(mockInvoke).toHaveBeenCalledWith('load_topology', undefined);
  });

  it('loadTopology with branchId → load_topology with branchId', async () => {
    mockInvoke.mockResolvedValue(null);
    await loadTopology('branch-1');
    expect(mockInvoke).toHaveBeenCalledWith('load_topology', { branchId: 'branch-1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('invalid topology'));
    await expect(canSaveTopology('tok')).rejects.toThrow('invalid topology');
  });
});
