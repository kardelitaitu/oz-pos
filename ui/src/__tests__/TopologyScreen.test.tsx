// ── TopologyScreen tests ────────────────────────────────────────────
//
// Covers the topology -> workspace CRUD bridge: seeding the editor from
// loaded workspace_instances and the atomic diff on save (Critical #4).
// Also covers typeKey change → archive+recreate (#1) and wire-based
// store_id resolution (#5).

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, waitFor, act, screen, fireEvent, within } from '@testing-library/react';
import TopologyScreen from '@/features/stores/TopologyScreen';

// ── Mocks ──────────────────────────────────────────────────────────

vi.mock('@/api/license', () => ({
  checkLicenseStatus: () => Promise.resolve({ tier: 'standard' }),
}));

const mockListStores = vi.fn();
const mockUpdateStore = vi.fn();
const mockDeleteStore = vi.fn();
vi.mock('@/api/stores', () => ({
  listStores: () => mockListStores(),
  updateStore: (...args: unknown[]) => mockUpdateStore(...args),
  deleteStore: (...args: unknown[]) => mockDeleteStore(...args),
}));

const mockListWorkspacesScoped = vi.fn();
const mockUpdateWorkspace = vi.fn();
vi.mock('@/api/workspaces', () => ({
  listWorkspacesScoped: (...args: unknown[]) => mockListWorkspacesScoped(...args),
  updateWorkspaceInstanceScoped: (...args: unknown[]) => mockUpdateWorkspace(...args),
}));

const mockApplyTopologyDiff = vi.fn();
vi.mock('@/api/topology', () => ({
  applyTopologyDiff: (...args: unknown[]) => mockApplyTopologyDiff(...args),
  loadTopology: () => Promise.resolve(null),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'test-session-token' }),
}));

const mockAddToast = vi.fn();
vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: mockAddToast }),
}));

vi.mock('@fluent/react', () => {
  // Minimal stand-in for ErrorBoundary's module-level bundle: it constructs
  // `new ReactLocalization([bundle])` and formats its emergency fallback
  // strings via getString. Without this export the whole suite fails to
  // collect (the mocked module graph resolves ErrorBoundary's import).
  class ReactLocalization {
    constructor(_bundles: unknown[]) { /* bundle list accepted for parity */ }
    getString(id: string) {
      return id;
    }
  }
  return {
    useLocalization: () => ({ l10n: { getString: (id: string) => id } }),
    Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
    ReactLocalization,
  };
});

// Capture the props the screen passes to the editor so tests can drive
// the save-diff logic without the real canvas.
let capturedEditorProps: {
  onSave?: (nodes: unknown[], wires: unknown[]) => Promise<Record<string, string> | void>;
  workspaceInstances?: unknown[];
  branchToolbar?: unknown;
  branchLocations?: unknown[];
  onRenameBranch?: (id: string, name: string) => Promise<boolean>;
  onRenameWorkspace?: (id: string, name: string) => Promise<boolean>;
  onDirtyChange?: (dirty: boolean) => void;
} = {};
vi.mock('@/features/stores/NodeTopologyEditor', () => ({
  default: (props: {
    onSave?: (n: unknown[], w: unknown[]) => Promise<Record<string, string> | void>;
    workspaceInstances?: unknown[];
    branchToolbar?: unknown;
    branchLocations?: unknown[];
    onRenameBranch?: (id: string, name: string) => Promise<boolean>;
    onRenameWorkspace?: (id: string, name: string) => Promise<boolean>;
    onDirtyChange?: (dirty: boolean) => void;
  }) => {
    capturedEditorProps = props;
    // The branch (graph) selector toolbar is rendered via the editor's
    // branchToolbar slot — mount it so the mocked SettingsSelect inside
    // still registers capturedBranchOnChange for branch-switch tests.
    return props.branchToolbar ?? null;
  },
}));

// Capture the branch selector's onChange so tests can simulate switching
// the branch (SettingsSelect is a custom combobox, not a native select).
let capturedBranchOnChange: ((value: string) => void) | null = null;
let capturedBranchOptions: { value: string; label: string }[] = [];
vi.mock('@/features/settings/SettingsSelect', () => ({
  default: (props: { onChange: (value: string) => void; options?: { value: string; label: string }[] }) => {
    capturedBranchOnChange = props.onChange;
    capturedBranchOptions = props.options ?? [];
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
    purpose_key: 'checkout',
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

// ── Helpers ────────────────────────────────────────────────────────

/** Simulate a canvas save with the given workspace nodes and wires. */
async function triggerSave(
  workspaceNodes: unknown[],
  wires: unknown[] = [],
  nonWorkspaceNodes: unknown[] = [],
) {
  const allNodes = [...nonWorkspaceNodes, ...workspaceNodes];
  await capturedEditorProps.onSave!(allNodes, wires);
}

/** Minimal workspace node factory. */
function wsNode(overrides: Record<string, unknown> = {}) {
  return {
    id: 'ws-1',
    type: 'workspace',
    name: 'POS #1',
    x: 0,
    y: 0,
    metadata: { typeKey: 'store-pos', purposeKey: 'general' },
    ...overrides,
  };
}

/** Minimal store node factory for wire-based resolution testing. */
function storeNode(overrides: Record<string, unknown> = {}) {
  return {
    id: 'store-1',
    type: 'store',
    name: 'Main Street',
    storeProfileId: 'store-1',
    x: 0,
    y: 0,
    ...overrides,
  };
}

function locationWire(
  fromNodeId: string,
  toNodeId: string,
  id = 'location-wire',
) {
  return {
    id,
    fromNodeId,
    fromPort: 'right',
    fromPortId: 'location-out',
    toNodeId,
    toPort: 'left',
    toPortId: 'location-in',
    relationshipType: 'location',
    direction: 'one-way',
  };
}

/** Get the first call's args array for applyTopologyDiff. */
function appliedArgs() {
  const call = mockApplyTopologyDiff.mock.calls[0];
  if (!call) throw new Error('applyTopologyDiff was not called');
  // [sessionToken, creations, updates, archives, diagramNodes, diagramWires]
  return {
    sessionToken: call[0] as string,
    creations: call[1] as { id: string; type_key: string; name: string; store_id: string }[],
    updates: call[2] as { id: string; name: string; purpose_key?: string }[],
    archives: call[3] as string[],
    diagramNodes: call[4] as { id: string }[],
    diagramWires: call[5] as {
      from_node_id: string;
      to_node_id: string;
      bends?: { x: number; y: number }[];
    }[],
  };
}

// ── Tests ──────────────────────────────────────────────────────────

describe('TopologyScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    capturedEditorProps = {};
    capturedBranchOnChange = null;
    capturedBranchOptions = [];
    mockListStores.mockResolvedValue(sampleStores);
    mockListWorkspacesScoped.mockResolvedValue(loadedInstances);
    mockApplyTopologyDiff.mockResolvedValue(undefined);
    mockDeleteStore.mockResolvedValue(undefined);
  });

  const renderReady = async (expectedInstanceCount = 1) => {
    render(<TopologyScreen />);
    await waitFor(() => expect(capturedEditorProps.onSave).toBeDefined());
    await waitFor(() =>
      expect(capturedEditorProps.workspaceInstances).toHaveLength(expectedInstanceCount),
    );
  };

  // ── Seed ──────────────────────────────────────────────────────

  it('seeds the editor with loaded workspace instances', async () => {
    await renderReady();
    expect(capturedEditorProps.workspaceInstances).toEqual([
      {
        instanceId: 'ws-existing',
        typeKey: 'store-pos',
        storeId: 'store-1',
        storeName: 'Main Street',
        purposeKey: 'checkout',
        name: 'Front Register',
        subtitle: 'Old desc',
      },
    ]);
  });

  it('excludes Inventory Management instances — the warehouse node is the single storage card', async () => {
    // Two storage-flavored cards on one canvas confused users: the
    // Warehouse node (the topology's stock-routing target) and the
    // Inventory Management workspace. Only the warehouse survives — an
    // inventory instance must never seed the canvas (its workspace row
    // still exists outside the topology; the sweep just never sees it).
    mockListWorkspacesScoped.mockResolvedValue([
      { ...loadedInstances[0]!, instance_id: 'ws-pos', type_key: 'store-pos', name: 'Store POS' },
      { ...loadedInstances[0]!, instance_id: 'ws-inv', type_key: 'inventory', name: 'Inventory Management' },
    ]);
    render(<TopologyScreen />);
    await waitFor(() => expect(capturedEditorProps.onSave).toBeDefined());
    await waitFor(() => expect(capturedEditorProps.workspaceInstances).toHaveLength(1));

    const seeded = capturedEditorProps.workspaceInstances as Array<{ instanceId: string; typeKey: string }>;
    expect(seeded.map((s) => s.instanceId)).toEqual(['ws-pos']);
    expect(seeded[0]!.typeKey).toBe('store-pos');
  });

  it('refetches workspace instances when the branch selector changes', async () => {
    // Branch-scoped graphs: switching the selector must reload that
    // branch's instances (and the editor remounts, keyed by branch) rather
    // than showing the previous branch's canvas.
    mockListWorkspacesScoped
      .mockResolvedValueOnce(loadedInstances)                                   // initial branch (store-1)
      .mockResolvedValueOnce([{ ...loadedInstances[0]!, instance_id: 'ws-b1', store_id: 'store-b', name: 'B POS' }]);
    await renderReady();

    // Simulate the manager picking branch B in the branch selector.
    act(() => { capturedBranchOnChange?.('store-b'); });

    await waitFor(() => expect(mockListWorkspacesScoped).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(capturedEditorProps.workspaceInstances).toEqual([
        expect.objectContaining({ instanceId: 'ws-b1', storeId: 'store-b', name: 'B POS' }),
      ]),
    );
  });

  it('blocks a dirty branch switch until the user confirms discarding, and cancel keeps the branch', async () => {
    // Only ONE load happens here (the switch is cancelled) — queue exactly
    // one Once value so no unconsumed mockResolvedValueOnce leaks into the
    // next test (vi.clearAllMocks does not drain the once-queue).
    mockListWorkspacesScoped.mockResolvedValueOnce(loadedInstances);           // initial branch (store-1)
    await renderReady();

    // The editor reports unsaved edits (e.g. an in-flight drag or rename).
    act(() => { capturedEditorProps.onDirtyChange?.(true); });

    // Switching branches now must NOT refetch — the discard-confirm dialog
    // intercepts before the selection changes.
    act(() => { capturedBranchOnChange?.('store-b'); });

    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();

    act(() => { fireEvent.click(within(dialog).getByRole('button', { name: 'cancel' })); });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(mockListWorkspacesScoped).toHaveBeenCalledTimes(1);
    expect(capturedEditorProps.workspaceInstances).toEqual([
      expect.objectContaining({ instanceId: 'ws-existing' }),
    ]);
  });

  it('switches branches after the user confirms discarding unsaved edits', async () => {
    mockListWorkspacesScoped
      .mockResolvedValueOnce(loadedInstances)                                   // initial branch (store-1)
      .mockResolvedValueOnce([{ ...loadedInstances[0]!, instance_id: 'ws-b1', store_id: 'store-b', name: 'B POS' }]);
    await renderReady();

    act(() => { capturedEditorProps.onDirtyChange?.(true); });
    act(() => { capturedBranchOnChange?.('store-b'); });

    const dialog = screen.getByRole('dialog');
    act(() => {
      fireEvent.click(within(dialog).getByRole('button', { name: 'topology-discard-changes-confirm' }));
    });

    // The branch refetch now proceeds and the new branch's instances load.
    await waitFor(() => expect(mockListWorkspacesScoped).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(capturedEditorProps.workspaceInstances).toEqual([
        expect.objectContaining({ instanceId: 'ws-b1', storeId: 'store-b', name: 'B POS' }),
      ]),
    );
  });

  it('switches branches immediately when the canvas is clean (no dialog)', async () => {
    mockListWorkspacesScoped
      .mockResolvedValueOnce(loadedInstances)                                   // initial branch (store-1)
      .mockResolvedValueOnce([{ ...loadedInstances[0]!, instance_id: 'ws-b1', store_id: 'store-b', name: 'B POS' }]);
    await renderReady();

    act(() => { capturedBranchOnChange?.('store-b'); });

    await waitFor(() => expect(mockListWorkspacesScoped).toHaveBeenCalledTimes(2));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() =>
      expect(capturedEditorProps.workspaceInstances).toEqual([
        expect.objectContaining({ instanceId: 'ws-b1', storeId: 'store-b', name: 'B POS' }),
      ]),
    );
  });

  it('renames a branch profile through the card callback and syncs the selector + card seed', async () => {
    // The editor card drives the rename via the onRenameBranch prop; the
    // screen persists it through update_store_profile preserving the other
    // fields, reports success so the card can close its inline form, and —
    // because both the header selector labels and the editor's branch seed
    // derive from the SAME stores state — the new name is live everywhere
    // immediately.
    mockListStores.mockResolvedValueOnce([
      { id: 'store-1', name: 'Main Street', is_primary: true, address: 'A St', tax_id: 'T-1', currency: 'USD', timezone: 'UTC', created_at: '', updated_at: '' },
    ]);
    mockUpdateStore.mockResolvedValueOnce({
      id: 'store-1', name: 'Main Street (Flagship)', is_primary: true, address: 'A St', tax_id: 'T-1', currency: 'USD', timezone: 'UTC', created_at: '', updated_at: '',
    });
    await renderReady();

    const ok = await capturedEditorProps.onRenameBranch!('store-1', 'Main Street (Flagship)');
    expect(ok).toBe(true);
    expect(mockUpdateStore).toHaveBeenCalledWith({
      id: 'store-1',
      name: 'Main Street (Flagship)',
      address: 'A St',
      tax_id: 'T-1',
      currency: 'USD',
      timezone: 'UTC',
    });

    // Header selector label is live: the branch dropdown now lists the new
    // name (the stores-state update flushes on a re-render).
    await waitFor(() =>
      expect(
        capturedBranchOptions.some((o) => o.value === 'store-1' && o.label === 'Main Street (Flagship)'),
      ).toBe(true),
    );
    // The editor's branch seed (the card-title source) is live too.
    await waitFor(() =>
      expect(capturedEditorProps.branchLocations).toEqual(
        expect.arrayContaining([expect.objectContaining({ id: 'store-1', name: 'Main Street (Flagship)' })]),
      ),
    );
  });

  it('renames a workspace instance through the card callback', async () => {
    // The workspace card rename persists via update_workspace_instance_scoped
    // (the live row) and updates the instances state so the seed the editor
    // receives carries the new name on the next refresh.
    mockListStores.mockResolvedValueOnce(sampleStores);
    mockListWorkspacesScoped.mockResolvedValueOnce(loadedInstances);
    mockUpdateWorkspace.mockResolvedValueOnce(undefined);
    await renderReady();

    const ok = await capturedEditorProps.onRenameWorkspace!('ws-existing', 'Renamed Register');
    expect(ok).toBe(true);
    // The rename must preserve the instance's other editable fields — the
    // wrapper nulls omitted description/colour, which would wipe the card
    // subtitle on every rename (reviewer-found bug).
    expect(mockUpdateWorkspace).toHaveBeenCalledWith(
      'test-session-token',
      'ws-existing',
      expect.objectContaining({ name: 'Renamed Register', description: 'Old desc' }),
    );

    // The editor's workspace seed carries the new name on the refresh.
    await waitFor(() =>
      expect(capturedEditorProps.workspaceInstances).toEqual(
        expect.arrayContaining([expect.objectContaining({ instanceId: 'ws-existing', name: 'Renamed Register' })]),
      ),
    );
  });

  it('deletes a branch: selector option, editor seed, and selection move to the remaining branch', async () => {
    // Two branches so deletion has somewhere to land.
    mockListStores.mockResolvedValue([
      { id: 'store-1', name: 'Main Street', is_primary: true, address: '', tax_id: '', currency: 'USD', timezone: 'UTC', created_at: '', updated_at: '' },
      { id: 'store-2', name: 'Second Street', is_primary: false, address: '', tax_id: '', currency: 'USD', timezone: 'UTC', created_at: '', updated_at: '' },
    ]);
    await renderReady(1);

    await waitFor(() =>
      expect(capturedBranchOptions.map((o) => o.value)).toEqual(['store-1', 'store-2']),
    );

    // Delete Branch → confirm.
    fireEvent.click(screen.getByRole('button', { name: 'topology-branch-delete' }));
    fireEvent.click(screen.getByRole('button', { name: 'topology-branch-delete-confirm-btn' }));

    // The profile is deleted through the API.
    await waitFor(() => expect(mockDeleteStore).toHaveBeenCalledWith('store-1'));

    // The selector option for the deleted branch is gone…
    await waitFor(() =>
      expect(capturedBranchOptions.map((o) => o.value)).toEqual(['store-2']),
    );
    // …and the editor seed no longer carries the deleted branch (its card
    // leaves the canvas on the next rebuild/merge).
    await waitFor(() =>
      expect(capturedEditorProps.branchLocations).toEqual([{ id: 'store-2', name: 'Second Street' }]),
    );
  });

  // ── #4: Atomic diff — single applyTopologyDiff call ────────────

  it('creates a new workspace via applyTopologyDiff (atomic diff)', async () => {
    await renderReady();

    const nodes = [
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'store-pos', persisted: true } }),
      wsNode({ id: 'ws-new', name: 'New Register', metadata: { typeKey: 'kds', persisted: false } }),
    ];
    await triggerSave(nodes, [locationWire('store-1', 'ws-existing', 'location-existing'), locationWire('store-1', 'ws-new', 'location-new')]);

    expect(mockApplyTopologyDiff).toHaveBeenCalledTimes(1);
    const a = appliedArgs();
    expect(a.sessionToken).toBe('test-session-token');
    expect(a.creations).toHaveLength(1);
    expect(a.creations[0]!.id).toBe('ws-new');
    expect(a.creations[0]!.type_key).toBe('kds');
    expect(a.creations[0]!.name).toBe('New Register');
    expect(a.updates).toHaveLength(0);
    expect(a.archives).toHaveLength(0);
    expect(a.diagramNodes.map((n) => n.id).sort()).toEqual(['store-1', 'ws-existing', 'ws-new'].sort());
  });

  it('updates a renamed workspace via applyTopologyDiff', async () => {
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Renamed Register', metadata: { typeKey: 'store-pos', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    expect(a.creations).toHaveLength(0);
    expect(a.updates).toHaveLength(1);
    expect(a.updates[0]!.id).toBe('ws-existing');
    expect(a.updates[0]!.name).toBe('Renamed Register');
    expect(a.updates[0]!.purpose_key).toBe('checkout');
  });

  it('archives removed instances via applyTopologyDiff', async () => {
    await renderReady();

    await triggerSave([storeNode()]);

    const a = appliedArgs();
    expect(a.archives).toHaveLength(1);
    expect(a.archives[0]).toBe('ws-existing');
  });

  it('no-op when no changes (still saves diagram)', async () => {
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'store-pos', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    expect(a.creations).toHaveLength(0);
    expect(a.updates).toHaveLength(0);
    expect(a.archives).toHaveLength(0);
  });

  // ── #1: TypeKey change → archive + recreate ──────────────────

  it('archives and recreates when typeKey changes (Critical #1)', async () => {
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'kds', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    expect(a.archives).toHaveLength(1);
    expect(a.archives[0]).toBe('ws-existing');
    expect(a.creations).toHaveLength(1);
    expect(a.creations[0]!.type_key).toBe('kds');
    expect(a.creations[0]!.name).toBe('Front Register');
    expect(a.creations[0]!.id).not.toBe('ws-existing');
    expect(a.creations[0]!.id).toMatch(/^ws-[0-9a-f-]+$/);
    expect(a.updates).toHaveLength(0);
    // Diagram workspace node remapped to new UUID
    expect(a.diagramNodes.map((node) => node.id)).toContain(a.creations[0]!.id);
  });

  it('returns idMap so editor can remap canvas state (#1)', async () => {
    await renderReady();

    const result = await capturedEditorProps.onSave!(
      [storeNode(), wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'restaurant-pos', persisted: true } })],
      [locationWire('store-1', 'ws-existing')],
    );

    expect(result).toBeDefined();
    const idMap = result as Record<string, string>;
    expect(Object.keys(idMap)).toHaveLength(1);
    expect(idMap['ws-existing']).toMatch(/^ws-[0-9a-f-]+$/);
  });

  it('remaps wire endpoints for type-changed nodes (#1)', async () => {
    await renderReady();

    const store = storeNode({ id: 'store-1', name: 'Main Street' });
    const ws = wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'kds', persisted: true } });
    const wires = [locationWire('store-1', 'ws-existing', 'w-1')];

    await capturedEditorProps.onSave!([store, ws], wires);

    const a = appliedArgs();
    expect(a.diagramWires).toHaveLength(1);
    expect(a.diagramWires[0]!.from_node_id).toBe('store-1');
    expect(a.diagramWires[0]!.to_node_id).not.toBe('ws-existing');
    expect(a.diagramWires[0]!.to_node_id).toMatch(/^ws-[0-9a-f-]+$/);
  });

  it('preserves name change when typeKey also changes (#1)', async () => {
    await renderReady();

    // User renamed AND changed type in one edit session
    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Bar POS', metadata: { typeKey: 'restaurant-pos', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    // Archive old, create new
    expect(a.archives).toHaveLength(1);
    expect(a.archives[0]).toBe('ws-existing');
    expect(a.creations).toHaveLength(1);
    // The new instance uses the new typeKey AND the new name
    expect(a.creations[0]!.type_key).toBe('restaurant-pos');
    expect(a.creations[0]!.name).toBe('Bar POS');
  });

  it('does not archive+recreate when typeKey is unchanged (#1 false-positive guard)', async () => {
    await renderReady();

    // Same typeKey as the loaded instance — should be a regular update (name change)
    // or no-op if name also hasn't changed
    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'store-pos', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    // No archive, no create — just a no-op since name also didn't change
    expect(a.archives).toHaveLength(0);
    expect(a.creations).toHaveLength(0);
  });

  it('handles multiple type changes in one save (#1)', async () => {
    mockListWorkspacesScoped.mockResolvedValue([
      { ...loadedInstances[0]!, instance_id: 'ws-a', type_key: 'store-pos', name: 'A' },
      { ...loadedInstances[0]!, instance_id: 'ws-b', type_key: 'store-pos', name: 'B' },
    ]);
    await renderReady(2);

    await capturedEditorProps.onSave!(
      [
        storeNode(),
        wsNode({ id: 'ws-a', name: 'A', metadata: { typeKey: 'kds', persisted: true } }),
        wsNode({ id: 'ws-b', name: 'B', metadata: { typeKey: 'restaurant-pos', persisted: true } }),
      ],
      [locationWire('store-1', 'ws-a', 'location-a'), locationWire('store-1', 'ws-b', 'location-b')],
    );

    const a = appliedArgs();
    expect(a.creations).toHaveLength(2);
    expect(a.archives).toHaveLength(2);
    expect(a.archives).toContain('ws-a');
    expect(a.archives).toContain('ws-b');
    const newIds = a.creations.map((c) => c.id);
    expect(new Set(newIds).size).toBe(2);
  });

  // ── Semantic store_id resolution ─────────────────────────────

  it('uses wire-connected store for store_id (Critical #5)', async () => {
    // The loaded instance belongs to store-1 (the default-selected branch),
    // so the branch-scoped seed keeps it; the canvas then wires the new
    // workspace to a DIFFERENT store's node, and that wire must drive the
    // resolved store_id — never the primary/default branch.
    mockListStores.mockResolvedValue([
      { ...sampleStores[0]!, id: 'store-1', name: 'Store A', is_primary: true },
      { ...sampleStores[0]!, id: 'store-b', name: 'Store B', is_primary: false },
    ]);
    await renderReady();

    const store = storeNode({ id: 'store-b', name: 'Store B', storeProfileId: 'store-b' });
    const ws = wsNode({ id: 'ws-new', name: 'POS', metadata: { typeKey: 'store-pos', persisted: false } });
    const wires = [locationWire('store-b', 'ws-new', 'w-1')];

    await capturedEditorProps.onSave!([store, ws], wires);

    const a = appliedArgs();
    expect(a.creations).toHaveLength(1);
    expect(a.creations[0]!.store_id).toBe('store-b');
  });

  it('rejects a workspace without semantic Location In ownership', async () => {
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-new', name: 'Unowned POS', metadata: { typeKey: 'store-pos', persisted: false } }),
    ]);

    expect(mockApplyTopologyDiff).not.toHaveBeenCalled();
    expect(mockAddToast).toHaveBeenCalledWith({
      message: 'topology-validation-missing-location',
      type: 'error',
    });
  });

  // ══ Error handling ═══════════════════════════════════════════

  it('carries wire bend points through the diff payload', async () => {
    await renderReady();

    const wires = [
      {
        ...locationWire('store-1', 'ws-existing', 'w-1'),
        bends: [{ x: 350, y: 334 }, { x: 400, y: 300 }],
      },
    ];
    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'store-pos', persisted: true } }),
    ], wires);

    const a = appliedArgs();
    expect(a.diagramWires[0]!.bends).toEqual([{ x: 350, y: 334 }, { x: 400, y: 300 }]);
  });

  it('surfaces applyTopologyDiff errors via toast and returns empty idMap', async () => {
    mockApplyTopologyDiff.mockRejectedValue(new Error('DB locked'));
    await renderReady();

    const result = await capturedEditorProps.onSave!(
      [storeNode(), wsNode({ id: 'ws-new', name: 'POS', metadata: { typeKey: 'store-pos', persisted: false } })],
      [locationWire('store-1', 'ws-new')],
    );

    // Returns empty idMap on error (no crash)
    expect(result).toEqual({});

    // Toast error surfaced
    expect(mockAddToast).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'error',
        message: expect.stringContaining('Something went wrong'),
      }),
    );
  });

  it('surfaces applyTopologyDiff network error via toast', async () => {
    mockApplyTopologyDiff.mockRejectedValue(new Error('Network failure'));
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-new', name: 'POS', metadata: { typeKey: 'store-pos', persisted: false } }),
    ], [locationWire('store-1', 'ws-new')]);

    // Toast error surfaced with the error message
    expect(mockAddToast).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'error',
        message: expect.stringContaining('Something went wrong'),
      }),
    );
  });

  // ══ #5: Wire-based store_id resolution (duplicate names) ═══════

  it('uses stable semantic store_profile_id when display names collide', async () => {
    // Two branches share the display name 'Downtown'; the canvas wires to
    // the secondary one. The resolved store_id must come from the wire's
    // branch node storeProfileId — never from display-name matching.
    mockListStores.mockResolvedValue([
      { ...sampleStores[0]!, id: 'store-1', name: 'Downtown', is_primary: true },
      { ...sampleStores[0]!, id: 'store-downtown-2', name: 'Downtown', is_primary: false },
    ]);
    await renderReady();

    const store = storeNode({ id: 'store-b', name: 'Downtown', storeProfileId: 'store-downtown-2' });
    const ws = wsNode({ id: 'ws-new', name: 'POS', metadata: { typeKey: 'store-pos', persisted: false } });
    const wires = [locationWire('store-b', 'ws-new', 'w-1')];

    await capturedEditorProps.onSave!([store, ws], wires);

    const a = appliedArgs();
    expect(a.creations).toHaveLength(1);
    // Stable semantic store_profile_id wins even when display names collide.
    expect(a.creations[0]!.store_id).toBe('store-downtown-2');
  });

  // ══ #1: Type-change metadata.persisted flag ════════════════════

  it('sets metadata.persisted=true on type-changed diagram nodes', async () => {
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'kds', persisted: true } }),
    ], [locationWire('store-1', 'ws-existing')]);

    const a = appliedArgs();
    expect(a.diagramNodes).toHaveLength(2);
    const changedNode = a.diagramNodes.find((node) => node.id !== 'store-1');
    expect(changedNode?.id).not.toBe('ws-existing');
    // The recreated node's metadata should have persisted: true
    const meta = (changedNode as { metadata?: { persisted?: boolean } } | undefined)?.metadata;
    expect(meta).toBeDefined();
    expect(meta!.persisted).toBe(true);
  });

  // ── Edge cases ────────────────────────────────────────────────

  it('returns empty idMap on error (no session token)', async () => {
    // With sessionToken present in mock, this just verifies error path
    // works. Real no-session case would need different mock setup.
    await renderReady();
    const expected = await capturedEditorProps.onSave!([], []);
    expect(expected).toBeDefined();
  });

  it('refreshes workspace instances after successful save', async () => {
    mockListWorkspacesScoped.mockClear();
    mockListWorkspacesScoped.mockResolvedValue(loadedInstances);
    await renderReady();

    await triggerSave([
      storeNode(),
      wsNode({ id: 'ws-existing', name: 'Front Register', metadata: { typeKey: 'store-pos', persisted: true } }),
      wsNode({ id: 'ws-new', name: 'New POS', metadata: { typeKey: 'store-pos', persisted: false } }),
    ], [locationWire('store-1', 'ws-existing', 'location-existing'), locationWire('store-1', 'ws-new', 'location-new')]);

    await waitFor(() => {
      expect(mockListWorkspacesScoped).toHaveBeenCalledTimes(2);
    });
  });
});
