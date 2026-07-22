// ── MultiStoreDashboardScreen tests ─────────────────────────────────
//
// Covers: loading state, error state with retry, stat cards,
// store cards with primary badge, and data rendering.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import MultiStoreDashboardScreen from '@/features/stores/MultiStoreDashboardScreen';
import type { StoreProfile } from '@/api/stores';
import type { TerminalDto } from '@/api/terminals';

// ── Mocks ──────────────────────────────────────────────────────────

const mockListStores = vi.fn();
const mockListTerminals = vi.fn();

vi.mock('@/api/stores', () => ({
  listStores: () => mockListStores(),
  setPrimaryStore: vi.fn(),
  deleteStore: vi.fn(),
}));

vi.mock('@/api/terminals', () => ({
  listTerminals: () => mockListTerminals(),
}));

// Topology / workspace integration mocks (topology view is not exercised
// by these tests, but the component imports these on mount).
vi.mock('@/api/license', () => ({
  checkLicenseStatus: () => Promise.resolve({ tier: 'standard' }),
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

// Capture the props (onSave + seed) the dashboard passes to the editor so
// tests can drive the save-diff logic without the real canvas.
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

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: { getString: (id: string) => id },
  }),
  Localized: ({ children }: { id: string; children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

// TerminalStatusPanel renders nothing in tests.
vi.mock('@/features/terminals/TerminalStatusPanel', () => ({
  default: () => null,
}));

// ── Test data ──────────────────────────────────────────────────────

const sampleStores: StoreProfile[] = [
  {
    id: 'store-1',
    name: 'Main Street',
    is_primary: true,
    address: '123 Main St',
    tax_id: 'TAX-001',
    currency: 'USD',
    timezone: 'America/New_York',
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'store-2',
    name: 'Downtown',
    is_primary: false,
    address: '',
    tax_id: '',
    currency: 'USD',
    timezone: 'America/Chicago',
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
];

const sampleTerminals: TerminalDto[] = [
  {
    id: 'term-1',
    name: 'Register 1',
    deviceId: 'dev-term-1',
    isActive: true,
    lastSeenAt: new Date().toISOString(),
    metadata: null,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  },
  {
    id: 'term-2',
    name: 'Register 2',
    deviceId: 'dev-term-2',
    isActive: false,
    lastSeenAt: null,
    metadata: null,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  },
];

// ── Tests ──────────────────────────────────────────────────────────

describe('MultiStoreDashboardScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedEditorProps = {};
    mockListStores.mockResolvedValue(sampleStores);
    mockListTerminals.mockResolvedValue(sampleTerminals);
    mockListWorkspacesScoped.mockResolvedValue([]);
    mockCreateWorkspace.mockResolvedValue(undefined);
    mockUpdateWorkspace.mockResolvedValue(undefined);
    mockArchiveWorkspace.mockResolvedValue(undefined);
    mockSaveTopology.mockResolvedValue(undefined);
  });

  // ── Loading state ─────────────────────────────────────────────

  it('shows loading skeleton while data is being fetched', () => {
    // Never resolve — keeps loading state.
    mockListStores.mockReturnValue(new Promise(() => {}));
    mockListTerminals.mockReturnValue(new Promise(() => {}));

    render(<MultiStoreDashboardScreen />);

    expect(document.querySelector('.multi-store-dashboard-loading-skeleton')).toBeInTheDocument();
  });

  // ── Error state ──────────────────────────────────────────────

  it('shows error message and retry button on fetch failure', async () => {
    mockListStores.mockRejectedValue(new Error('Network error'));

    render(<MultiStoreDashboardScreen />);

    await waitFor(() => {
      expect(screen.getByText('multi-store-error-load')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('retries when retry button is clicked', async () => {
    mockListStores.mockRejectedValueOnce(new Error('Network error'));

    render(<MultiStoreDashboardScreen />);

    await waitFor(() => {
      expect(screen.getByText('multi-store-error-load')).toBeInTheDocument();
    });

    // On retry, resolve successfully.
    mockListStores.mockResolvedValueOnce(sampleStores);
    mockListTerminals.mockResolvedValueOnce(sampleTerminals);

    await userEvent.click(screen.getByRole('button', { name: /retry/i }));

    await waitFor(() => {
      expect(screen.getByText('Main Street')).toBeInTheDocument();
    });
  });

  // ── Data state ───────────────────────────────────────────────

  it('renders stat cards with correct counts', async () => {
    render(<MultiStoreDashboardScreen />);

    await waitFor(() => {
      expect(screen.getByText('Main Street')).toBeInTheDocument();
    });

    // "2" appears in Total Stores, Total Terminals, and each store's terminal count.
    const twos = screen.getAllByText('2');
    expect(twos.length).toBe(4);
  });

  it('renders store cards with primary badge', async () => {
    render(<MultiStoreDashboardScreen />);

    await waitFor(() => {
      expect(screen.getByText('Main Street')).toBeInTheDocument();
    });

    // Primary store badge
    expect(screen.getByText('Primary')).toBeInTheDocument();

    // Both store names visible
    expect(screen.getByText('Downtown')).toBeInTheDocument();
  });

  // ── Topology → workspace CRUD bridge ──────────────────────────
  //
  // The dashboard passes an onSave handler to NodeTopologyEditor that
  // diffs canvas workspace nodes against the loaded workspace_instances
  // and calls create / update / archive accordingly.

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

  const renderTopologyReady = async () => {
    mockListWorkspacesScoped.mockResolvedValue(loadedInstances);
    render(<MultiStoreDashboardScreen />);
    // Switch to topology view so NodeTopologyEditor (mock) mounts and
    // captures the onSave prop.
    await waitFor(() => expect(screen.getByText('Main Street')).toBeInTheDocument());
    await userEvent.click(screen.getByText('🗺️ Node Topology Builder'));
    await waitFor(() => expect(capturedEditorProps.onSave).toBeDefined());
  };

  it('seeds the editor with loaded workspace instances', async () => {
    await renderTopologyReady();
    expect(capturedEditorProps.workspaceInstances).toEqual([
      { instanceId: 'ws-existing', typeKey: 'store-pos', name: 'Front Register', subtitle: 'Old desc' },
    ]);
  });

  it('creates a new workspace instance for a new canvas node', async () => {
    await renderTopologyReady();
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
    await renderTopologyReady();
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
    await renderTopologyReady();
    // Canvas has no workspace nodes → the loaded instance was removed.
    await capturedEditorProps.onSave!([], []);

    expect(mockArchiveWorkspace).toHaveBeenCalledTimes(1);
    expect(mockArchiveWorkspace).toHaveBeenCalledWith('test-session-token', 'ws-existing');
    expect(mockCreateWorkspace).not.toHaveBeenCalled();
    expect(mockUpdateWorkspace).not.toHaveBeenCalled();
  });

  it('does not update an unchanged existing workspace node', async () => {
    await renderTopologyReady();
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
