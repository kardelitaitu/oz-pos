import { screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';
import { loadTopology, saveTopology } from '@/api/topology';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/topology', () => ({
  loadTopology: vi.fn(),
  saveTopology: vi.fn(),
}));

// Passthrough mock: keep real LocalizationProvider/ReactLocalization so
// withFluent (used by renderWithProvidersSync) still works, but replace
// <Localized> with a simple children-rendering passthrough and stub
// useLocalization().getString with a lookup that returns the English
// fallback for known topology keys (tests assert on English text).
//
// <Localized> passthrough handles all UI label text; this map covers the
// ~20 keys used via l10n.getString() for node names, subtitles, toasts,
// dialogs, workspace type labels, and aria attributes.
const TOPOLOGY_EN: Record<string, string> = {
  'topology-new-store': 'New Store',
  'topology-new-store-subtitle': 'Branch',
  'topology-new-workspace': 'New Workspace',
  'topology-new-workspace-subtitle': 'Register',
  'topology-new-warehouse': 'New Warehouse',
  'topology-new-warehouse-subtitle': 'Storage',
  'topology-new-hardware': 'New Hardware',
  'topology-new-hardware-subtitle': 'Peripheral',
  'topology-new-ready': 'Ready',
  'topology-toast-multi-warehouse': 'Multi-Warehouse storage locations require a Pro Tier license.',
  'topology-toast-wire-duplicate': 'A wire already connects these ports.',
  'topology-toast-fallback-warehouse': 'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
  'topology-toast-load-error': 'Failed to load topology',
  'topology-confirm-delete-node-title': 'Delete Node',
  'topology-confirm-delete-wire-title': 'Delete Wire',
  'topology-confirm-delete-node-msg':
    'This node has connected wires. Deleting it will remove all its wires too. This action cannot be undone.',
  'topology-confirm-delete-wire-msg': 'Delete this wire connection? This action cannot be undone.',
  'topology-confirm-delete-label': 'Delete',
  'topology-confirm-preset-title': 'Load Preset',
  'topology-confirm-preset-msg':
    'Loading a preset will replace your current topology. Any unsaved changes will be lost. You can undo this action after loading.',
  'topology-confirm-preset-label': 'Load Preset',
  'topology-canvas-aria-label': 'Topology editor canvas. Use arrow keys to nudge selected nodes, Ctrl+Z to undo.',
  'topology-ws-type-store-pos': 'Retail POS',
  'topology-ws-type-restaurant-pos': 'Restaurant POS',
  'topology-ws-type-kds': 'Kitchen Display (KDS)',
  'topology-ws-type-warehouse': 'Warehouse',
};

vi.mock('@fluent/react', async () => {
  const actual = await vi.importActual('@fluent/react');
  return {
    ...actual,
    Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
    useLocalization: () => ({
      l10n: { getString: (id: string) => TOPOLOGY_EN[id] ?? id },
    }),
  };
});

vi.mock('@/contexts/SettingsContext', () => ({
  useSettings: () => ({
    settings: {
      receipt: {
        showCurrency: false,
        decimalSeparator: 'dot',
        showTax: true,
        footer: '',
        paperWidth: 'standard',
        showTableNumber: false,
        marginTop: 0,
        marginBottom: 0,
        marginLeft: 0,
        marginRight: 0,
      },
      store: { name: 'Test Store', address: '', taxId: '', currency: 'IDR', branch: '' },
      sync: { serverUrl: null, hasApiKey: false, enabled: false },
      brand: { colour: '#10b981', storeName: 'Test Store' },
      preferences: { cardSize: 0, fontSize: 0, fontSmoothing: 'antialiased' },
      currencies: [],
      appVersion: '0.0.19',
    },
    loading: false,
    error: null,
    hasPartialError: false,
    refetch: vi.fn(),
    lastChangedKeys: [],
    markSettingsUpdated: vi.fn(),
  }),
}));

const mockLoadTopology = vi.mocked(loadTopology);
const mockSaveTopology = vi.mocked(saveTopology);

const renderEditor = (props?: { onSave?: (nodes: unknown, wires: unknown) => Promise<Record<string, string> | void> }) =>
  renderWithProvidersSync(<NodeTopologyEditor currentTier="standard" {...props} />, multiStoreFtl, sharedFtl);

const getNodeCount = () => document.querySelectorAll('.topology-node').length;
const getWireCount = () => document.querySelectorAll('.wire-group').length;

const selectFirstNode = () => {
  const firstNode = document.querySelector('.topology-node');
  if (firstNode) fireEvent.mouseDown(firstNode as Element, { button: 0 });
};

describe('NodeTopologyEditor Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLoadTopology.mockResolvedValue(null);
    mockSaveTopology.mockResolvedValue(undefined);
  });

  it('renders title and default retail preset nodes', () => {
    renderEditor();

    expect(screen.getByText('Visual Store & Workspace Topology Builder')).toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    expect(screen.getByText('Retail POS #1')).toBeInTheDocument();
    expect(screen.getByText('Main Warehouse')).toBeInTheDocument();
  });

  it('renders tool rack sidebar and preset buttons', () => {
    renderEditor();

    expect(screen.getByText('+ Store Node')).toBeInTheDocument();
    expect(screen.getByText('+ Workspace Node')).toBeInTheDocument();
    expect(screen.getByText('+ Warehouse Node')).toBeInTheDocument();
    expect(screen.getByText('+ Hardware Node')).toBeInTheDocument();
    expect(screen.getByText('Test Order Simulation')).toBeInTheDocument();
  });

  it('switches to restaurant & KDS preset when clicked', () => {
    renderEditor();

    const restoBtn = screen.getByText('Resto & KDS Preset');
    fireEvent.click(restoBtn);

    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(screen.getByText('Kitchen KDS')).toBeInTheDocument();
    expect(screen.getByText('Kitchen Thermal Printer')).toBeInTheDocument();
  });

  it('toggles simulation mode on button click', () => {
    renderEditor();

    const simBtn = screen.getByText('Test Order Simulation');
    fireEvent.click(simBtn);

    expect(screen.getByText('Stop Simulation')).toBeInTheDocument();
  });

  // ── Load persisted topology on mount ──────────────────────────

  it('loads persisted topology on mount when data exists', async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Loaded Store', x: 100, y: 200 },
        { id: 'ws-1', type: 'workspace', name: 'Loaded POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-1', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way' }],
    });

    renderEditor();

    await waitFor(() => {
      expect(screen.getByText('Loaded Store')).toBeInTheDocument();
      expect(screen.getByText('Loaded POS')).toBeInTheDocument();
    });
  });

  it('falls back to retail preset when loadTopology returns null', async () => {
    mockLoadTopology.mockResolvedValue(null);

    renderEditor();

    await waitFor(() => {
      expect(mockLoadTopology).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
  });

  it('falls back to retail preset when loadTopology returns empty nodes', async () => {
    mockLoadTopology.mockResolvedValue({ nodes: [], wires: [] });

    renderEditor();

    await waitFor(() => {
      expect(mockLoadTopology).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
  });

  // ── Save topology ─────────────────────────────────────────────

  it('calls saveTopology with correct payload when Apply Topology Changes clicked', async () => {
    const onSave = vi.fn();
    renderEditor({ onSave });

    const applyBtn = screen.getByText('Apply Topology Changes');
    fireEvent.click(applyBtn);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    const [nodes, wires] = onSave.mock.calls[0]!;
    expect(nodes).toHaveLength(3);
    expect(wires).toHaveLength(2);
    expect(nodes[0].id).toBe('store-1');
    expect(nodes[0].name).toBe('Downtown Branch');
  });

  it('calls saveTopology via onSave with all node fields mapped', async () => {
    const onSave = vi.fn();
    renderEditor({ onSave });

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    const [nodes] = onSave.mock.calls[0]!;
    const storeNode = nodes.find((n: { id: string }) => n.id === 'store-1');
    expect(storeNode).toBeDefined();
    expect(storeNode.name).toBe('Downtown Branch');
    expect(storeNode.subtitle).toBe('Primary Store');
    expect(storeNode.telemetryBadge).toBe('Online (2 POS)');
    expect(storeNode.telemetryStatus).toBe('online');
    expect(storeNode.x).toBe(80);
    expect(storeNode.y).toBe(140);
  });

  // ── Add node ────────────────────────────────────────────────────

  it('adds a new store node when tool rack button clicked', () => {
    renderEditor();

    const initialCount = getNodeCount();

    fireEvent.click(screen.getByText('+ Store Node'));

    expect(getNodeCount()).toBe(initialCount + 1);
    expect(screen.getByText('New Store')).toBeInTheDocument();
  });

  it('adds a new hardware node when tool rack button clicked', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Hardware Node'));

    expect(screen.getByText('New Hardware')).toBeInTheDocument();
  });

  it('prevents adding second warehouse on standard tier', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Warehouse Node'));
    fireEvent.click(screen.getByText('+ Warehouse Node'));

    const warningToasts = screen.queryAllByText(
      'Multi-Warehouse storage locations require a Pro Tier license.',
    );
    expect(warningToasts.length).toBeGreaterThanOrEqual(1);
  });

  // ── Delete node ─────────────────────────────────────────────────

  it('deletes a node without wires immediately', async () => {
    renderEditor();

    // Add a new node (no wires connected) then delete it
    fireEvent.click(screen.getByText('+ Store Node'));
    await waitFor(() => {
      expect(screen.getByText('New Store')).toBeInTheDocument();
    });

    // Select the new node (last one in the DOM)
    const nodes = document.querySelectorAll('.topology-node');
    const newNode = nodes[nodes.length - 1];
    fireEvent.mouseDown(newNode as Element, { button: 0 });

    const deleteBtn = screen.getByText('Delete Selected Element');
    fireEvent.click(deleteBtn);

    await waitFor(() => {
      expect(screen.queryByText('New Store')).not.toBeInTheDocument();
    });
  });

  it('shows confirmation dialog when deleting node with wires', () => {
    renderEditor();

    selectFirstNode();

    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();

    const deleteBtn = screen.getByText('Delete Selected Element');
    fireEvent.click(deleteBtn);

    expect(screen.getByText('Delete Node')).toBeInTheDocument();
    expect(
      screen.getByText(/This node has connected wires/),
    ).toBeInTheDocument();
  });

  // ── Undo ────────────────────────────────────────────────────────

  it('shows Undo button after making changes', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Store Node'));

    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();
  });

  it('restores previous state on undo', () => {
    renderEditor();

    const initialCount = getNodeCount();

    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(initialCount + 1);

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(getNodeCount()).toBe(initialCount);
  });

  // ── Wire deletion undo (#2) ─────────────────────────────────────

  it('restores deleted wire on undo', () => {
    renderEditor();

    // Retail preset has 2 wires
    const initialWireCount = getWireCount();
    expect(initialWireCount).toBe(2);

    // Click a wire hitbox to select the wire (hitting the label text
    // only toggles direction — it doesn't set selectedWireId)
    const hitbox = document.querySelector('.wire-hitbox');
    expect(hitbox).not.toBeNull();
    fireEvent.click(hitbox!);

    const deleteBtn = screen.getByText('Delete Selected Element');
    fireEvent.click(deleteBtn);

    // Confirm the wire deletion dialog
    const confirmDeleteBtn = screen.getByText('Delete');
    fireEvent.click(confirmDeleteBtn);

    expect(getWireCount()).toBe(initialWireCount - 1);

    // Undo should restore the wire
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(getWireCount()).toBe(initialWireCount);
  });

  // ── Wire direction toggle ───────────────────────────────────────

  it('toggles wire direction on label click', () => {
    renderEditor();

    const wireLabels = screen.getAllByText(/→|↔/);
    const firstLabel = wireLabels[0]!;
    expect(firstLabel.textContent).toContain('→');

    fireEvent.click(firstLabel);

    expect(firstLabel.textContent).toContain('↔');
  });

  // ── Zoom controls ───────────────────────────────────────────────

  it('shows zoom percentage and canvas controls', () => {
    renderEditor();

    expect(screen.getByText('Zoom: 100%')).toBeInTheDocument();
    expect(screen.getByText('Fit All')).toBeInTheDocument();
    expect(screen.getByText('Reset View')).toBeInTheDocument();
  });

  // ── Keyboard shortcut guard (#3) ────────────────────────────────

  it('does not delete node when Backspace is pressed in a text field', () => {
    renderEditor();

    // Add a node and select it to open the inspector
    fireEvent.click(screen.getByText('+ Store Node'));
    const nodeCountAfterAdd = getNodeCount();

    // Find the Node Name input in the inspector
    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput).not.toBeNull();

    // Focus the input and fire Backspace
    nameInput.focus();
    fireEvent.keyDown(nameInput, { key: 'Backspace' });

    // Node count should be unchanged — Backspace was handled by the input
    expect(getNodeCount()).toBe(nodeCountAfterAdd);
  });

  // ── Apply button — idMap remapping (#1) ───────────────────────

  it('remaps node and wire IDs when onSave returns idMap', async () => {
    // Load a custom topology with known, stable node IDs so this test
    // does NOT depend on the retail preset internals.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-test', type: 'store', name: 'Remap Store', x: 100, y: 100 },
        { id: 'ws-test', type: 'workspace', name: 'Remap POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-test', from_node_id: 'store-test', to_node_id: 'ws-test', direction: 'one-way' }],
    });

    const onSave = vi.fn().mockResolvedValue({ 'ws-test': 'ws-remapped-id' });
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Remap POS')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // onSave receives the original IDs (remapping happens client-side AFTER return)
    const [nodes] = onSave.mock.calls[0]!;
    const wsTestNode = nodes.find((n: { id: string }) => n.id === 'ws-test');
    expect(wsTestNode).toBeDefined();
    expect(wsTestNode.name).toBe('Remap POS');

    // After remapping, no nodes are lost and component is stable
    expect(getNodeCount()).toBe(2);
    expect(screen.getByText('Remap POS')).toBeInTheDocument();
    expect(screen.getByText('Remap Store')).toBeInTheDocument();
  });

  it('clears selection after idMap remapping', async () => {
    const onSave = vi.fn().mockResolvedValue({ 'ws-1': 'ws-new-id' });
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    // Select the workspace node first
    const wsNode = document.querySelector('.node-type-workspace');
    expect(wsNode).not.toBeNull();
    fireEvent.mouseDown(wsNode as Element, { button: 0 });

    // Inspector should be visible (Delete button appears when something is selected)
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Click Apply — the idMap remapping should clear selection
    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // After remapping, the delete button should disappear (selection cleared)
    await waitFor(() => {
      expect(screen.queryByText('Delete Selected Element')).not.toBeInTheDocument();
    });
  });

  it('handles empty idMap gracefully (no remapping)', async () => {
    const onSave = vi.fn().mockResolvedValue({});
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    const initialNodeCount = getNodeCount();

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // Node count unchanged — no remapping occurred
    expect(getNodeCount()).toBe(initialNodeCount);
    // All original nodes should still be present
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    expect(screen.getByText('Retail POS #1')).toBeInTheDocument();
  });

  it('handles onSave returning undefined (backward compat)', async () => {
    // vi.fn() returns undefined by default, which is the legacy behavior
    const onSave = vi.fn();
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    const initialNodeCount = getNodeCount();

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // No crash, no remapping
    expect(getNodeCount()).toBe(initialNodeCount);
  });

  it('remaps wire endpoints when returning idMap', async () => {
    // Load a custom topology with explicit wire endpoints so we can
    // verify the endpoint IDs onSave receives AND that wires survive
    // client-side remapping.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-wr', type: 'store', name: 'Wire Store', x: 100, y: 100 },
        { id: 'ws-wr', type: 'workspace', name: 'Wire POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-wr', from_node_id: 'store-wr', to_node_id: 'ws-wr', direction: 'one-way' }],
    });

    const onSave = vi.fn().mockResolvedValue({ 'ws-wr': 'ws-remapped' });
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Wire POS')).toBeInTheDocument();
    });

    const initialWireCount = getWireCount();
    expect(initialWireCount).toBe(1);

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // onSave received the wire with original endpoint IDs
    const [, wires] = onSave.mock.calls[0]!;
    expect(wires).toHaveLength(1);
    expect(wires[0].fromNodeId).toBe('store-wr'); // unchanged
    expect(wires[0].toNodeId).toBe('ws-wr'); // old ID, client remaps after return

    // After remapping, wires should still be present (no loss)
    expect(getWireCount()).toBe(1);
  });

  // ── Delete via keyboard shortcut also uses input guard (#3) ─────

  it('does not delete node when Delete is pressed in a text field', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Store Node'));
    const nodeCountAfterAdd = getNodeCount();

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput).not.toBeNull();

    nameInput.focus();
    fireEvent.keyDown(nameInput, { key: 'Delete' });

    // Node count should be unchanged
    expect(getNodeCount()).toBe(nodeCountAfterAdd);
  });

  it('does not intercept Ctrl+Z when typing in a text field', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Store Node'));
    const nodeCountAfterAdd = getNodeCount();

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput).not.toBeNull();

    nameInput.focus();
    fireEvent.keyDown(nameInput, { key: 'z', ctrlKey: true });

    // Ctrl+Z should be handled by the input field, not the canvas handler
    expect(getNodeCount()).toBe(nodeCountAfterAdd);
  });

  // ── Delegation regression: no direct saveTopology when onSave is provided ──

  it('does not call saveTopology directly when onSave is provided (delegation)', async () => {
    const onSave = vi.fn();
    renderEditor({ onSave });

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // The editor must delegate entirely to onSave — never invoke the
    // old saveTopology directly. This verifies the boundary between
    // the editor and the TopologyScreen parent.
    expect(mockSaveTopology).not.toHaveBeenCalled();
  });

  // ── Undo sequence resilience (#6) ───────────────────────────────

  it('undoes multiple sequential additions back to initial state', () => {
    renderEditor();

    const initialCount = getNodeCount();

    // Add 3 nodes
    fireEvent.click(screen.getByText('+ Store Node'));
    fireEvent.click(screen.getByText('+ Hardware Node'));
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(initialCount + 3);

    // Undo 3 times
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(getNodeCount()).toBe(initialCount);
  });

  // ── Redo (#7) ───────────────────────────────────────────────────

  it('redos restore undone state', () => {
    renderEditor();

    const initialCount = getNodeCount();

    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(initialCount + 1);
    expect(screen.getByText('New Store')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(getNodeCount()).toBe(initialCount);
    // Redo button appears after undo
    expect(screen.getByText('Redo (Ctrl+Y)')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Redo (Ctrl+Y)'));
    expect(getNodeCount()).toBe(initialCount + 1);
    expect(screen.getByText('New Store')).toBeInTheDocument();
    // Redo stack consumed, button gone
    expect(screen.queryByText('Redo (Ctrl+Y)')).not.toBeInTheDocument();
  });

  it('clears redo stack on new edit after undo', () => {
    renderEditor();

    fireEvent.click(screen.getByText('+ Store Node'));
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    // Redo should be available
    expect(screen.getByText('Redo (Ctrl+Y)')).toBeInTheDocument();

    // New edit after undo — clears redo branch
    fireEvent.click(screen.getByText('+ Hardware Node'));
    expect(screen.queryByText('Redo (Ctrl+Y)')).not.toBeInTheDocument();
  });

  it('Ctrl+Y keyboard shortcut triggers redo', () => {
    renderEditor();

    const initialCount = getNodeCount();
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(initialCount + 1);

    // Ctrl+Z to undo
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(canvas).not.toBeNull();
    fireEvent.keyDown(canvas!, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(initialCount);

    // Ctrl+Y to redo
    fireEvent.keyDown(canvas!, { key: 'y', ctrlKey: true });
    expect(getNodeCount()).toBe(initialCount + 1);
  });

  it('Ctrl+Shift+Z also triggers redo', () => {
    renderEditor();

    const initialCount = getNodeCount();
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(initialCount + 1);

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(canvas).not.toBeNull();

    // Ctrl+Z to undo
    fireEvent.keyDown(canvas!, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(initialCount);

    // Ctrl+Shift+Z to redo (via the undo handler's shiftKey check)
    fireEvent.keyDown(canvas!, { key: 'z', ctrlKey: true, shiftKey: true });
    expect(getNodeCount()).toBe(initialCount + 1);
  });

  // ── Corrupt wire direction resilience (#10) ─────────────────────

  it('renders without crash when loaded topology has corrupt wire direction', async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Store', x: 100, y: 100 },
        { id: 'ws-1', type: 'workspace', name: 'POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-bad', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'bidirectional' }],
    });

    renderEditor();

    // Should render without crashing — corrupt direction falls back to one-way
    await waitFor(() => {
      expect(screen.getByText('Store')).toBeInTheDocument();
      expect(screen.getByText('POS')).toBeInTheDocument();
    });

    // Wire should still render (just without the two-way marker)
    expect(getWireCount()).toBe(1);
  });
});
