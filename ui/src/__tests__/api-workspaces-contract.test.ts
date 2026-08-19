import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  resolveBootStore,
  listWorkspacesScoped,
  listWorkspaceScreensScoped,
  listWorkspaces,
} from '@/api/workspaces';

describe('workspaces.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('resolveBootStore calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ id: 'ws1', name: 'Default' });
    await resolveBootStore();
    expect(mockInvoke).toHaveBeenCalledWith('resolve_boot_store');
  });

  it('listWorkspacesScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspacesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces_scoped', { sessionToken: 'tok' });
  });

  it('listWorkspaceScreensScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspaceScreensScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspace_screens_scoped', { sessionToken: 'tok' });
  });

  it('listWorkspaces calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspaces();
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces');
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('workspace not found'));
    await expect(listWorkspacesScoped('bad')).rejects.toThrow('workspace not found');
  });
});
