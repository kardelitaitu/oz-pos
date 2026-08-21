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

  it('resolveBootStore calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 'ws1', name: 'Default' });
    await resolveBootStore('device-1');
    expect(mockInvoke).toHaveBeenCalledWith('resolve_boot_store', { deviceId: 'device-1' });
  });

  it('resolveBootStore with no args defaults to null', async () => {
    mockInvoke.mockResolvedValue({ id: 'ws1' });
    await resolveBootStore();
    expect(mockInvoke).toHaveBeenCalledWith('resolve_boot_store', { deviceId: null });
  });

  it('listWorkspacesScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspacesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces_scoped', { sessionToken: 'tok' });
  });

  it('listWorkspaceScreensScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspaceScreensScoped('tok', 'pos');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspace_screens_scoped', { sessionToken: 'tok', typeKey: 'pos' });
  });

  it('listWorkspaces calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspaces('ticket-1', 'store-1');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces', { ticket: 'ticket-1', storeId: 'store-1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('workspace not found'));
    await expect(listWorkspacesScoped('bad')).rejects.toThrow('workspace not found');
  });
});
