// ── IPC contract tests for workspaces.ts ───────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  resolveBootStore,
  listWorkspacesScoped,
  getWorkspaceInstanceScoped,
  createWorkspaceInstanceScoped,
  updateWorkspaceInstanceScoped,
  archiveWorkspaceInstanceScoped,
  listWorkspaceScreensScoped,
  setUserWorkspaceInstancesScoped,
  getUserWorkspaceInstancesScoped,
} from '@/api/workspaces';

describe('workspaces.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('resolveBootStore → resolve_boot_store with deviceId', async () => {
    mockInvoke.mockResolvedValue({ storeId: 's1', instanceId: 'i1' });
    await resolveBootStore('device-1');
    expect(mockInvoke).toHaveBeenCalledWith('resolve_boot_store', { deviceId: 'device-1' });
  });

  it('listWorkspacesScoped → list_workspaces_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspacesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces_scoped', { sessionToken: 'tok' });
  });

  it('getWorkspaceInstanceScoped → get_workspace_instance_scoped', async () => {
    mockInvoke.mockResolvedValue(null);
    await getWorkspaceInstanceScoped('tok', 'i1');
    expect(mockInvoke).toHaveBeenCalledWith('get_workspace_instance_scoped', { sessionToken: 'tok', instanceId: 'i1' });
  });

  it('createWorkspaceInstanceScoped → create_workspace_instance_scoped with req', async () => {
    mockInvoke.mockResolvedValue({ id: 'i2' });
    await createWorkspaceInstanceScoped('tok', { name: 'POS 1', workspaceType: 'pos', storeId: 's1' });
    expect(mockInvoke).toHaveBeenCalledWith('create_workspace_instance_scoped', { sessionToken: 'tok', req: expect.objectContaining({ name: 'POS 1' }) });
  });

  it('updateWorkspaceInstanceScoped → update_workspace_instance_scoped with flat fields', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateWorkspaceInstanceScoped('tok', 'i1', { name: 'POS Updated' });
    expect(mockInvoke).toHaveBeenCalledWith('update_workspace_instance_scoped', {
      sessionToken: 'tok',
      instanceId: 'i1',
      name: 'POS Updated',
      description: null,
      colour: null,
    });
  });

  it('archiveWorkspaceInstanceScoped → archive_workspace_instance_scoped', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await archiveWorkspaceInstanceScoped('tok', 'i1');
    expect(mockInvoke).toHaveBeenCalledWith('archive_workspace_instance_scoped', { sessionToken: 'tok', instanceId: 'i1' });
  });

  it('listWorkspaceScreensScoped → list_workspace_screens_scoped with typeKey', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspaceScreensScoped('tok', 'pos');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspace_screens_scoped', { sessionToken: 'tok', typeKey: 'pos' });
  });

  it('setUserWorkspaceInstancesScoped → set_user_workspace_instances_scoped with flat params', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setUserWorkspaceInstancesScoped('tok', 'u1', ['i1', 'i2']);
    expect(mockInvoke).toHaveBeenCalledWith('set_user_workspace_instances_scoped', {
      sessionToken: 'tok',
      userId: 'u1',
      instanceIds: ['i1', 'i2'],
      defaultInstanceId: null,
    });
  });

  it('getUserWorkspaceInstancesScoped → get_user_workspace_instances_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await getUserWorkspaceInstancesScoped('tok', 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('get_user_workspace_instances_scoped', { sessionToken: 'tok', userId: 'u1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('workspace not found'));
    await expect(listWorkspacesScoped('tok')).rejects.toThrow('workspace not found');
  });
});
