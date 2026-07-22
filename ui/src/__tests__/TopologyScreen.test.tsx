// ── TopologyScreen tests ────────────────────────────────────────────
//
// Covers the topology -> workspace CRUD bridge: seeding the editor from
// loaded workspace_instances and the create / update / archive diff on save.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import TopologyScreen from '@/features/stores/TopologyScreen';

// ── Mocks ──────────────────────────────────────────────────────────

vi.mock('@/api/license', () => ({
  checkLicenseStatus: () => Promise.resolve({ tier: 'standard' }),
}));

const mockListStores = vi.fn();
vi.mock('@/api/stores', () => ({
  listStores: () => mockListStores(),
}));

const mockListWorkspacesScoped = vi.fn();
const mockCreateWorkspace = vi.fn();
const mockUpdateWorkspace = vi.fn();
const mockArchiveWorkspace = vi.fn();

vi.mock('@/api/workspaces', () => ({
  listWorkspacesScoped: (...args: unknown[]) => mockListWorkspacesScoped(...args),
  createWorkspaceInstanceScoped: (...args: unknown[]) => mockCreateWorkspace(...args),
  updateWorkspaceInstanceScoped: (...args: unknown[]) => mockUpdateWorkspace(...args),
  archiveWorkspaceInstanceScoped: (...args: unknown[]) => mockArchiveWorkspace(...args),
}));

const mockSaveTopology = vi.fn();
vi.mock('@/api/topology', () => ({
  saveTopology: (...args: unknown[]) => mockSaveTopology(...args),
  loadTopology: () => Promise.resolve(null),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'test-session-token' }),
}));

const mockAddToast = vi.fn();
vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: mockAddToast }),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({ l10n: { getString: (id: string) => id } }),
  Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
}));

// Capture the props the screen passes to the editor so tests can drive
// the save-diff logic without the real canvas.
let capturedEditorProps: {
  onSave?: (nodes: unknown[], wires: unknown[]) => void;
  workspaceInstances?: unknown[];
} = {};
vi.mock('@/features/stores/NodeTopologyEditor', () => ({
  default: (props: { onSave?: (n: unknown[], w: unknown[]) => void; workspaceInstances?: unknown[] }) => {
    capturedEditorProps = props;
    return null;
  },
}));

// ── Test data ──────────────────────────────────────────────────────

const loadedInstances = [
  {
    instance_id: 'ws-existing',
    type_key: 'store-pos',
    store_id: 'store-1',
    store_name: 'Main Street',
    name: 'Front Register',
    description: 'Old desc',
    icon: 'pos',
    layout_mode: 'sidebar',
    colour: null,
    is_default: false,
  },
];

const sampleStores = [
  { id: 'store-1', name: 'Main Street', is_primary: true, address: '', tax_id: '', currency: 'USD', timezone: 'UTC', created_at: '', updated_at: '' },
];

// ── Tests ──────────────────────────────────────────────────────────

describe('TopologyScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedEditorProps = {};
    mockListStores.mockResolvedValue(sampleStores);
    mockListWorkspacesScoped.mockResolvedValue(loadedInstances);
    mockCreateWorkspace.mockResolvedValue(undefined);
    mockUpdateWorkspace.mockResolvedValue(undefined);
    mockArchiveWorkspace.mockResolvedValue(undefined);
    mockSaveTopology.mockResolvedValue(undefined);
  });

  const renderReady = async () => {
    render(<TopologyScreen />);
    await waitFor(() => expect(capturedEditorProps.onSave).toBeDefined());
    await waitFor(() => expect(capturedEditorProps.workspaceInstances).toHaveLength(1));
  };

  it('seeds the editor with loaded workspace instances', async () => {
    await renderReady();
    expect(capturedEditorProps.workspaceInstances).toEqual([
      { instanceId: 'ws-existing', typeKey: 'store-pos', name: 'Front Register', subtitle: 'Old desc' },
    ]);
  });

  it('creates a new workspace instance for a new canvas node', async () => {
    await renderReady();
    const nodes = [
      { id: 'ws-existing', type: 'workspace', name: 'Front Register', subtitle: 'Old desc', x: 0, y: 0, metadata: { typeKey: 'store-pos', persisted: true } },
      { id: 'ws-new', type: 'workspace', name: 'New Register', subtitle: 'Register', x: 10, y: 10, metadata: { typeKey: 'kds', persisted: false } },
    ];
    await capturedEditorProps.onSave!(nodes, []);

    expect(mockCreateWorkspace).toHaveBeenCalledTimes(1);
    expect(mockCreateWorkspace).toHaveBeenCalledWith('test-session-token', expect.objectContaining({
      id: 'ws-new',
      type_key: 'kds',
      name: 'New Register',
      store_id: 'store-1',
    }));
    expect(mockUpdateWorkspace).not.toHaveBeenCalled();
    expect(mockArchiveWorkspace).not.toHaveBeenCalled();
    expect(mockSaveTopology).toHaveBeenCalledTimes(1);
  });

  it('updates a renamed existing workspace node', async () => {
    await renderReady();
    const nodes = [
      { id: 'ws-existing', type: 'workspace', name: 'Renamed Register', subtitle: 'Old desc', x: 0, y: 0, metadata: { typeKey: 'store-pos', persisted: true } },
    ];
    await capturedEditorProps.onSave!(nodes, []);

    expect(mockUpdateWorkspace).toHaveBeenCalledTimes(1);
    expect(mockUpdateWorkspace).toHaveBeenCalledWith('test-session-token', 'ws-existing', {
      name: 'Renamed Register',
    });
    expect(mockCreateWorkspace).not.toHaveBeenCalled();
    expect(mockArchiveWorkspace).not.toHaveBeenCalled();
  });

  it('archives a workspace instance removed from the canvas', async () => {
    await renderReady();
    await capturedEditorProps.onSave!([], []);

    expect(mockArchiveWorkspace).toHaveBeenCalledTimes(1);
    expect(mockArchiveWorkspace).toHaveBeenCalledWith('test-session-token', 'ws-existing');
    expect(mockCreateWorkspace).not.toHaveBeenCalled();
    expect(mockUpdateWorkspace).not.toHaveBeenCalled();
  });

  it('does not update an unchanged existing workspace node', async () => {
    await renderReady();
    const nodes = [
      { id: 'ws-existing', type: 'workspace', name: 'Front Register', subtitle: 'Old desc', x: 0, y: 0, metadata: { typeKey: 'store-pos', persisted: true } },
    ];
    await capturedEditorProps.onSave!(nodes, []);

    expect(mockCreateWorkspace).not.toHaveBeenCalled();
    expect(mockUpdateWorkspace).not.toHaveBeenCalled();
    expect(mockArchiveWorkspace).not.toHaveBeenCalled();
    // Diagram is still persisted even when no CRUD changes occurred.
    expect(mockSaveTopology).toHaveBeenCalledTimes(1);
  });
});
