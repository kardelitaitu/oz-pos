import { useState, type ComponentProps } from 'react';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor, { type WorkspaceInstanceSeed } from '../features/stores/NodeTopologyEditor';
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
  'topology-toast-selection-dropped': 'The selected element is not part of this preset and was deselected.',
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

type TopologyTier = Exclude<ComponentProps<typeof NodeTopologyEditor>['currentTier'], undefined>;

const renderEditor = (props?: {
  onSave?: (nodes: unknown, wires: unknown) => Promise<Record<string, string> | void>;
  currentTier?: TopologyTier;
}) =>
  renderWithProvidersSync(<NodeTopologyEditor currentTier="standard" {...props} />, multiStoreFtl, sharedFtl);

/**
 * Harness that re-renders the editor with a NEW workspaceInstances array
 * on demand — simulating the TopologyScreen parent refreshing instances
 * after a save/apply. The load effect depends on [workspaceInstances], so
 * a new identity re-triggers the non-skip reload path.
 */
function ReloadingHarness({ next }: { next: WorkspaceInstanceSeed[] }) {
  const [instances, setInstances] = useState<WorkspaceInstanceSeed[] | undefined>(undefined);
  return (
    <>
      <button type="button" onClick={() => setInstances(next)}>
        reload-instances
      </button>
      {/* exactOptionalPropertyTypes: omit the prop while instances is undefined */}
      <NodeTopologyEditor currentTier="standard" {...(instances ? { workspaceInstances: instances } : {})} />
    </>
  );
}

const getNodeCount = () => document.querySelectorAll('.topology-node').length;
const getWireCount = () => document.querySelectorAll('.wire-group').length;

// Preset renders nodes in array order: [store-1, ws-1, wh-1].
const nodeAt = (idx: number) =>
  document.querySelectorAll('.topology-node')[idx] as HTMLElement;
const portOf = (node: HTMLElement, port: string) =>
  node.querySelector(`.node-port-socket.port-${port}`) as HTMLElement;
const previewLine = () => document.querySelector('path.wire-path[opacity="0.5"]');

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

  // ── Click-to-select must not pollute undo/dirty state (#11) ───────

  it('does not enable Undo when a node is merely clicked (no drag)', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node');
    expect(firstNode).not.toBeNull();

    // Plain click: mousedown + mouseup with zero movement. Selecting a
    // node is not an edit, so it must not push an undo entry.
    fireEvent.mouseDown(firstNode as Element, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseUp(firstNode as Element, { button: 0, clientX: 0, clientY: 0 });

    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();
  });

  it('does not mark the canvas dirty when a node is clicked without editing', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node');
    expect(firstNode).not.toBeNull();

    fireEvent.mouseDown(firstNode as Element, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseUp(firstNode as Element, { button: 0, clientX: 0, clientY: 0 });

    // A plain click is not an edit — the preset must load directly,
    // without the "unsaved changes" confirm dialog.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));

    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
  });

  it('enables Undo only after an actual drag and restores the position on undo', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(firstNode).not.toBeNull();
    expect(canvas).not.toBeNull();

    // Retail preset: store-1 at (80, 140), snapped to the 24px grid.
    expect(firstNode.style.left).toBe('80px');
    expect(firstNode.style.top).toBe('140px');

    // mousedown at clientX/Y 0, then drag 48px right + 48px down.
    // newX = snap(0 - (0 - 80)) = snap(128) = 120; newY = snap(188) = 192.
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 48, clientY: 48 });

    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();
    expect(firstNode.style.left).toBe('120px');
    expect(firstNode.style.top).toBe('192px');

    fireEvent.mouseUp(canvas, { button: 0 });
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(firstNode.style.left).toBe('80px');
    expect(firstNode.style.top).toBe('140px');
  });

  // ── Drag released outside the canvas must cancel (#13) ───────────

  it('cancels the node drag when the pointer is released outside the canvas', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(firstNode).not.toBeNull();
    expect(canvas).not.toBeNull();
    expect(firstNode.style.left).toBe('80px');
    expect(firstNode.style.top).toBe('140px');

    // mousedown then drag 48px right + down, as in the normal drag test.
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 48, clientY: 48 });
    expect(firstNode.style.left).toBe('120px');
    expect(firstNode.style.top).toBe('192px');

    // Release OUTSIDE the canvas (document-level mouseup never reaches
    // the canvas onMouseUp handler). The drag must be cancelled.
    fireEvent.mouseUp(document, { button: 0 });

    // Further mousemoves over the canvas must NOT move the node — no
    // button is held and no ghost drag may follow the cursor.
    fireEvent.mouseMove(canvas, { clientX: 96, clientY: 96 });
    fireEvent.mouseMove(canvas, { clientX: 144, clientY: 144 });

    expect(firstNode.style.left).toBe('120px');
    expect(firstNode.style.top).toBe('192px');
  });

  // ── Arrow-key auto-repeat must not flood the undo stack (#14) ────

  it('ignores auto-repeated arrow nudges so one nudge is one undo step', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(firstNode).not.toBeNull();
    expect(canvas).not.toBeNull();
    expect(firstNode.style.left).toBe('80px');

    selectFirstNode();

    // Shift+ArrowRight moves a full grid step: snap(80 + 24) = 96.
    fireEvent.keyDown(canvas, { key: 'ArrowRight', shiftKey: true });
    expect(firstNode.style.left).toBe('96px');

    // Holding the key fires repeated keydowns (repeat: true). Those are
    // the SAME held nudge — they must not move further nor create extra
    // undo entries.
    fireEvent.keyDown(canvas, { key: 'ArrowRight', shiftKey: true, repeat: true });
    expect(firstNode.style.left).toBe('96px');

    // A single undo must return the node to the ORIGINAL position — the
    // held key produced exactly one history entry.
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(firstNode.style.left).toBe('80px');
  });

  // ── Fresh topology reload clears the undo stack (#15) ────────────

  it('clears the undo stack when a fresh topology loads (non-skip path)', async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [{ id: 'store-x', type: 'store', name: 'Loaded Store', x: 100, y: 100 }],
      wires: [],
    });

    renderWithProvidersSync(
      <ReloadingHarness
        next={[{ instanceId: 'ws-new', typeKey: 'restaurant-pos', name: 'New Instance' }]}
      />,
      multiStoreFtl,
      sharedFtl,
    );

    // Legacy path (no instances yet): saved diagram renders.
    await waitFor(() => {
      expect(screen.getByText('Loaded Store')).toBeInTheDocument();
    });

    // Make an edit so the undo stack has an entry.
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();
    expect(screen.getByText('New Store')).toBeInTheDocument();

    // Parent pushes a fresh workspaceInstances array → non-skip reload.
    fireEvent.click(screen.getByText('reload-instances'));

    // The new authoritative topology replaces the canvas — the manually
    // added node is gone (rebuilt from instances, not the undo stack).
    await waitFor(() => {
      expect(screen.getByText('New Instance')).toBeInTheDocument();
      expect(screen.queryByText('New Store')).not.toBeInTheDocument();
    });

    // The undo stack must be cleared — Undo can never restore a stale
    // canvas that contradicts the loaded workspace instances.
    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();
  });

  it('resets the inspector edit session when a preset loads over a selected node', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const nameInput = () => document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;

    // Select store-1 and edit its name — one session entry.
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.change(nameInput(), { target: { value: 'Renamed Branch' } });
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    // Dirty — the preset load asks for confirmation, then replaces the
    // canvas. store-1 stays selected (both presets have store-1).
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    const confirmBtn = screen.getAllByText('Load Preset').find((el) => el.tagName === 'BUTTON');
    fireEvent.click(confirmBtn as Element);
    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(nameInput().value).toBe('Grand Bistro');

    // Editing the SAME node after the preset load must start a fresh
    // session — one undo returns to the preset name, not the pre-preset
    // renamed state (which would prove the entry was never pushed).
    fireEvent.change(nameInput(), { target: { value: 'Grand Bistro Edited' } });
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(nameInput().value).toBe('Grand Bistro');
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

  it('clears the undo stack when a save remaps node ids', async () => {
    // Custom topology with a stable workspace id so the remap is deterministic.
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

    // Make an edit so the undo stack holds a pre-save entry.
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    // Apply — archive+recreate remaps 'ws-test' → 'ws-remapped-id' after
    // onSave resolves. The undo stack must be cleared: every pre-save entry
    // holds the OLD id, which no longer exists on the canvas or in the DB.
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();
  });

  it('keeps the undo stack when a save does not remap ids', async () => {
    const onSave = vi.fn().mockResolvedValue({});
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // No remap → ids unchanged → the pre-save undo entry stays valid, so
    // undo-after-save keeps working (the entry restores real, existing ids).
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();
  });

  it('does not ask about unsaved changes when a preset loads after a successful Apply', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    // Make an edit so the canvas is dirty.
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    // Apply persists the canvas — after a successful save the canvas matches
    // the backend, so a preset load must NOT ask about unsaved changes.
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    fireEvent.click(screen.getByText('Retail Preset'));

    // No "Load Preset" confirm dialog — the preset loads directly.
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
  });

  it('re-arms the unsaved-changes dialog for a new edit made after Apply', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    // Edit → save (canvas clean) → new edit re-dirties the canvas.
    fireEvent.click(screen.getByText('+ Store Node'));
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });
    fireEvent.click(screen.getByText('+ Hardware Node'));

    // The new unsaved edit must bring the confirm dialog back.
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThanOrEqual(1);
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

  // ── Inspector edits are undoable and mark the canvas dirty (#12) ──

  it('pushes a single undo entry for an inspector rename burst', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput).not.toBeNull();
    expect(nameInput.value).toBe('Downtown Branch');

    // Type a burst: two change events in one focus session. The whole
    // burst must be a SINGLE undo entry, not one per keystroke.
    fireEvent.change(nameInput, { target: { value: 'Renamed' } });
    fireEvent.change(nameInput, { target: { value: 'Renamed Branch' } });

    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    // One undo must restore the ORIGINAL name.
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(
      (document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement).value,
    ).toBe('Downtown Branch');
  });

  it('marks the canvas dirty when the inspector renames a node', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: 'Renamed' } });

    // A rename is a real edit — the preset load must ask first.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));

    // "Load Preset" appears as both the modal title and the confirm
    // button — either is proof the confirm dialog opened.
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('pushes an undo entry when the workspace type select changes', () => {
    renderEditor();

    const wsNode = document.querySelector('.node-type-workspace') as HTMLElement;
    expect(wsNode).not.toBeNull();
    fireEvent.mouseDown(wsNode, { button: 0, clientX: 0, clientY: 0 });

    const select = document.querySelector('.inspector-select') as HTMLSelectElement;
    expect(select).not.toBeNull();
    expect(select.value).toBe('store-pos');

    fireEvent.change(select, { target: { value: 'restaurant-pos' } });
    expect(select.value).toBe('restaurant-pos');
    expect(screen.getByText('Undo (Ctrl+Z)')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect((document.querySelector('.inspector-select') as HTMLSelectElement).value).toBe('store-pos');
  });

  // ── Selection re-validation on undo/redo/preset load (#16) ───────

  it('clears the dangling node selection when undoing a node add', () => {
    renderEditor();

    // Adding a node auto-selects it — the tool-rack Delete button appears.
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('New Store')).toBeInTheDocument();
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Undo removes the node. The selection must NOT stay dangling at the
    // now-gone node — the Delete button (rendered for any selection) is
    // the observable proof the selection was cleared.
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(screen.queryByText('New Store')).not.toBeInTheDocument();
    expect(screen.queryByText('Delete Selected Element')).not.toBeInTheDocument();
  });

  it('preserves a still-valid node selection when undoing a drag', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(firstNode).not.toBeNull();
    expect(firstNode.style.left).toBe('80px');

    // Drag the selected node (history pushed on first movement), then undo.
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 48, clientY: 48 });
    expect(firstNode.style.left).toBe('120px');
    fireEvent.mouseUp(canvas, { button: 0 });

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(firstNode.style.left).toBe('80px');

    // The node still exists — its selection must survive the undo.
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
  });

  it('preserves a still-valid wire selection when undoing a direction toggle', () => {
    renderEditor();

    // Select the first retail wire via its hitbox.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    expect(hitbox).not.toBeNull();
    fireEvent.click(hitbox);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Toggle its direction (pushes an undo entry), then undo.
    const wireLabels = screen.getAllByText(/→|↔/);
    fireEvent.click(wireLabels[0]!);
    expect(wireLabels[0]!.textContent).toContain('↔');

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    // Direction restored and the wire still exists — selection preserved.
    expect(screen.getAllByText(/→|↔/)[0]!.textContent).toContain('→');
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
  });

  it('clears a wire selection when a preset load removes the selected wire', () => {
    renderEditor();

    // Restaurant preset has 4 wires (w-1..w-4); retail has only 2 (w-1, w-2).
    // Select w-3 — it exists only in the restaurant preset.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    const hitboxes = document.querySelectorAll('.wire-hitbox');
    expect(hitboxes.length).toBe(4);
    fireEvent.click(hitboxes[2]!);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Clean canvas (no edits) → Retail Preset loads directly. w-3 no longer
    // exists, so its selection must not dangle at a removed wire.
    fireEvent.click(screen.getByText('Retail Preset'));

    expect(screen.queryByText('Delete Selected Element')).not.toBeInTheDocument();
  });

  // ── Undo-of-delete re-selects the restored node (#17) ─────────────

  it('re-selects a node restored by undoing a dialog (wired) delete', () => {
    renderEditor();

    // store-1 has connected wires → the dialog delete path.
    const storeNode = document.querySelector('.node-type-store') as HTMLElement;
    expect(storeNode).not.toBeNull();
    fireEvent.mouseDown(storeNode, { button: 0 });
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Delete Selected Element'));
    expect(screen.getByText('Delete Node')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Delete')); // confirm
    expect(screen.queryByText('Downtown Branch')).not.toBeInTheDocument();

    // Undo restores store-1 AND must re-select it so the inspector reopens.
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    expect(
      (document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement).value,
    ).toBe('Downtown Branch');
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
  });

  it('re-selects a node restored by undoing an immediate (wireless) delete', () => {
    renderEditor();

    // A freshly added node has no wires → the immediate delete path.
    fireEvent.click(screen.getByText('+ Store Node'));
    expect(screen.getByText('New Store')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Delete Selected Element'));
    expect(screen.queryByText('New Store')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(screen.getByText('New Store')).toBeInTheDocument();
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
  });

  it('does not re-select a node when undoing a wire deletion', () => {
    renderEditor();

    // Select w-1 via its hitbox and delete it through the dialog.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    expect(hitbox).not.toBeNull();
    fireEvent.click(hitbox);
    fireEvent.click(screen.getByText('Delete Selected Element'));
    fireEvent.click(screen.getByText('Delete')); // confirm
    expect(getWireCount()).toBe(1);

    // Undo restores the wire — no node was restored, so nothing may be
    // re-selected (the Delete button must stay hidden).
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(getWireCount()).toBe(2);
    expect(screen.queryByText('Delete Selected Element')).not.toBeInTheDocument();
  });

  // ── Toast when a preset load drops the selection (#18) ────────────

  it('shows a toast when a preset load drops the selected node', () => {
    renderEditor();

    // wh-1 (Main Warehouse) exists only in the retail preset.
    const warehouse = document.querySelector('.node-type-warehouse') as HTMLElement;
    expect(warehouse).not.toBeNull();
    fireEvent.mouseDown(warehouse, { button: 0 });
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // The restaurant preset has no warehouse node — the selection is dropped.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));

    expect(
      screen.getByText('The selected element is not part of this preset and was deselected.'),
    ).toBeInTheDocument();
  });

  it('shows a toast when a preset load drops the selected wire', () => {
    renderEditor();

    // Load the restaurant preset, then select w-3 — it exists only there.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    const hitboxes = document.querySelectorAll('.wire-hitbox');
    expect(hitboxes.length).toBe(4);
    fireEvent.click(hitboxes[2]!);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Retail preset has only w-1/w-2 — the wire selection is dropped.
    fireEvent.click(screen.getByText('Retail Preset'));

    expect(
      screen.getByText('The selected element is not part of this preset and was deselected.'),
    ).toBeInTheDocument();
  });

// ── Wire creation via port connections ──────────────────────────

describe('NodeTopologyEditor — wire creation', () => {
  it('creates a wire when two ports on different nodes are connected', () => {
    renderEditor();
    const baseline = getWireCount();

    // store-1 bottom → ws-1 top (not an existing connection).
    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(1), 'top'));

    expect(getWireCount()).toBe(baseline + 1);
  });

  it('rejects a duplicate connection with a toast and no new wire', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline + 1);

    // Same two ports again — duplicate.
    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(1), 'top'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(screen.getByText('A wire already connects these ports.')).toBeInTheDocument();
  });

  it('cancels the connection when clicking the same node again', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(0), 'right')); // same node → cancel

    expect(getWireCount()).toBe(baseline);
  });

  it('undoes a created wire in a single undo step', () => {
    renderEditor();
    const baseline = getWireCount();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline + 1);

    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getWireCount()).toBe(baseline);
  });

  it('blocks a second workspace→warehouse fallback wire on the standard tier', () => {
    renderEditor();
    const baseline = getWireCount();

    // ws-1 bottom → wh-1 top: workspace→warehouse, but the retail preset
    // already has one warehouse wire (w-2) — standard tier allows one.
    fireEvent.click(portOf(nodeAt(1), 'bottom'));
    fireEvent.click(portOf(nodeAt(2), 'top'));

    expect(getWireCount()).toBe(baseline);
    expect(
      screen.getByText(
        'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
      ),
    ).toBeInTheDocument();
  });
});

// ── Duplicate detection vs defaulted (null) ports ───────────────

describe('NodeTopologyEditor — duplicate detection vs defaulted ports', () => {
  it('rejects a duplicate wire against a loaded wire whose ports are defaulted', async () => {
    // The backend stores from_port/to_port as Option<PortName> and legacy
    // topologies round-trip wires with no ports. The load path maps that to
    // undefined — the wire renders on the DEFAULT ports (source right →
    // target left). Reconnecting the same default ports must be rejected
    // as a duplicate, not silently create a second overlapping wire.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Loaded Store', x: 100, y: 200 },
        { id: 'ws-1', type: 'workspace', name: 'Loaded POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-1', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way' }],
    });

    renderEditor();

    await waitFor(() => expect(screen.getByText('Loaded Store')).toBeInTheDocument());

    const baseline = getWireCount();
    expect(baseline).toBe(1);

    fireEvent.click(portOf(nodeAt(0), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));

    expect(getWireCount()).toBe(baseline);
    expect(screen.getByText('A wire already connects these ports.')).toBeInTheDocument();
  });

  it('rejects a reversed duplicate against a loaded wire with defaulted ports', async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Loaded Store', x: 100, y: 200 },
        { id: 'ws-1', type: 'workspace', name: 'Loaded POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-1', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way' }],
    });

    renderEditor();

    await waitFor(() => expect(screen.getByText('Loaded Store')).toBeInTheDocument());

    const baseline = getWireCount();
    fireEvent.click(portOf(nodeAt(1), 'left')); // start from the target node's side
    fireEvent.click(portOf(nodeAt(0), 'right'));

    expect(getWireCount()).toBe(baseline);
    expect(screen.getByText('A wire already connects these ports.')).toBeInTheDocument();
  });

  it('treats a literal null port payload (serde None) as a defaulted port', async () => {
    // serde serializes Option<PortName>::None as JSON null. The payload
    // interface types it as optional-string, but the runtime wire can carry
    // explicit nulls — the load path must coalesce them to undefined so
    // duplicate detection and rendering agree on the defaults.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Loaded Store', x: 100, y: 200 },
        { id: 'ws-1', type: 'workspace', name: 'Loaded POS', x: 300, y: 100 },
      ],
      wires: [{
        id: 'w-1',
        from_node_id: 'store-1',
        to_node_id: 'ws-1',
        direction: 'one-way',
        from_port: null as unknown as string,
        to_port: null as unknown as string,
      }],
    });

    renderEditor();

    await waitFor(() => expect(screen.getByText('Loaded Store')).toBeInTheDocument());

    const baseline = getWireCount();
    fireEvent.click(portOf(nodeAt(0), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));

    expect(getWireCount()).toBe(baseline);
    expect(screen.getByText('A wire already connects these ports.')).toBeInTheDocument();
  });
});

// ── Delete / Backspace key flow ─────────────────────────────────

describe('NodeTopologyEditor — Delete/Backspace key flow', () => {
  it('Delete key deletes a selected wireless node immediately without a dialog', async () => {
    renderEditor();
    const baseline = getNodeCount();

    fireEvent.click(screen.getByText('+ Store Node'));
    await waitFor(() => expect(screen.getByText('New Store')).toBeInTheDocument());
    const nodes = document.querySelectorAll('.topology-node');
    fireEvent.mouseDown(nodes[nodes.length - 1] as Element, { button: 0 });

    fireEvent.keyDown(window, { key: 'Delete' });

    await waitFor(() => expect(getNodeCount()).toBe(baseline));
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
  });

  it('Backspace key behaves like Delete for a selected wireless node', async () => {
    renderEditor();
    const baseline = getNodeCount();

    fireEvent.click(screen.getByText('+ Store Node'));
    await waitFor(() => expect(screen.getByText('New Store')).toBeInTheDocument());
    const nodes = document.querySelectorAll('.topology-node');
    fireEvent.mouseDown(nodes[nodes.length - 1] as Element, { button: 0 });

    fireEvent.keyDown(window, { key: 'Backspace' });

    await waitFor(() => expect(getNodeCount()).toBe(baseline));
  });

  it('Delete key opens the confirm dialog for a wired node; Cancel leaves everything intact', () => {
    renderEditor();
    const nodeCount = getNodeCount();
    const wireCount = getWireCount();
    selectFirstNode(); // store-1 has connected wires

    fireEvent.keyDown(window, { key: 'Delete' });

    expect(screen.getByText('Delete Node')).toBeInTheDocument();
    expect(screen.getByText(/This node has connected wires/)).toBeInTheDocument();

    fireEvent.click(screen.getByText('Cancel'));
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
    expect(getNodeCount()).toBe(nodeCount);
    expect(getWireCount()).toBe(wireCount);
  });

  it('confirming the Delete-key dialog removes the wired node and its wires', () => {
    renderEditor();
    const nodeCount = getNodeCount();
    const wireCount = getWireCount();
    selectFirstNode();

    fireEvent.keyDown(window, { key: 'Delete' });
    fireEvent.click(screen.getByText('Delete')); // confirm label

    expect(getNodeCount()).toBe(nodeCount - 1);
    expect(getWireCount()).toBe(wireCount - 1); // store-1's only wire (w-1) goes with it
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
  });

  it('Delete key on a selected wire opens the wire dialog; confirm removes the wire', () => {
    renderEditor();
    const wireCount = getWireCount();

    const hitbox = document.querySelector('.wire-hitbox') as HTMLElement;
    fireEvent.click(hitbox);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Delete' });
    expect(screen.getByText('Delete Wire')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Delete')); // confirm label
    expect(getWireCount()).toBe(wireCount - 1);
  });
});

it('does not toast when the selected node survives a preset load', () => {
    renderEditor();

    // store-1 exists in BOTH presets — its selection must survive.
    const store = document.querySelector('.node-type-store') as HTMLElement;
    expect(store).not.toBeNull();
    fireEvent.mouseDown(store, { button: 0 });
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Resto & KDS Preset'));

    expect(
      screen.queryByText('The selected element is not part of this preset and was deselected.'),
    ).not.toBeInTheDocument();
    // The inspector stays open on the surviving store node (restaurant name).
    expect(
      (document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement).value,
    ).toBe('Grand Bistro');
  });
});

// ── Escape connection-cancel flow ───────────────────────────────

describe('NodeTopologyEditor — Escape connection-cancel flow', () => {
  it('Escape cancels an in-flight connection before it completes', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from store-1's bottom port — a ghost preview line appears.
    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    expect(previewLine()).not.toBeNull();

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'Escape' });

    // Preview gone, and clicking a target port on another node does NOT
    // complete a wire — the connection was cancelled, not left dangling.
    expect(previewLine()).toBeNull();
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline);
  });

  it('Escape during a connection also clears the current selection', () => {
    renderEditor();

    selectFirstNode();
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    expect(previewLine()).not.toBeNull();

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'Escape' });

    expect(document.querySelector('.topology-node.node-selected')).toBeNull();
    expect(previewLine()).toBeNull();
  });

  it('Escape typed in a text field does not cancel the connection (input guard)', () => {
    renderEditor();
    const baseline = getWireCount();

    // Select a node so the inspector (with its text input) is open.
    selectFirstNode();
    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    expect(previewLine()).not.toBeNull();

    const nameInput = document.querySelector(
      '.inspector-field input[type="text"]',
    ) as HTMLInputElement;
    expect(nameInput).not.toBeNull();
    nameInput.focus();
    fireEvent.keyDown(nameInput, { key: 'Escape' });

    // The connection is still in flight — completing it creates the wire.
    expect(previewLine()).not.toBeNull();
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline + 1);
  });
});

// ── Pro-tier warehouse fallback wire label ──────────────────────

describe('NodeTopologyEditor — Pro-tier warehouse fallback label', () => {
  it('allows a second workspace→warehouse wire with the fallback label on Pro', () => {
    renderEditor({ currentTier: 'pro' });
    const baseline = getWireCount();

    // ws-1 bottom → wh-1 top: the retail preset already has one warehouse
    // wire, so the new one is the fallback (priority 2) — allowed on Pro.
    fireEvent.click(portOf(nodeAt(1), 'bottom'));
    fireEvent.click(portOf(nodeAt(2), 'top'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(
      screen.queryByText(
        'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
      ),
    ).not.toBeInTheDocument();

    // The new wire carries the fallback label (raw key in the identity-l10n mock).
    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    expect(last.textContent).toContain('topology-wire-label-fallback');
  });
});

// ── First warehouse wire: stock-deduct label ────────────────────

describe('NodeTopologyEditor — first warehouse wire stock-deduct label', () => {
  it('allows the first workspace→warehouse wire on standard tier with the stock-deduct label', async () => {
    // Custom topology with a warehouse but NO warehouse wires, so the first
    // ws→wh connection takes the stock-deduct (priority 1) path.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Store', x: 80, y: 120 },
        { id: 'ws-1', type: 'workspace', name: 'POS', x: 240, y: 80 },
        { id: 'wh-1', type: 'warehouse', name: 'WH', x: 420, y: 160 },
      ],
      wires: [],
    });
    renderEditor();

    await waitFor(() => expect(getNodeCount()).toBe(3));
    const baseline = getWireCount();

    // ws-1 bottom → wh-1 top: workspace→warehouse, first one — allowed on
    // the standard tier, labelled as the primary stock-deduction path.
    fireEvent.click(portOf(nodeAt(1), 'bottom'));
    fireEvent.click(portOf(nodeAt(2), 'top'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(
      screen.queryByText(
        'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
      ),
    ).not.toBeInTheDocument();

    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    expect(last.textContent).toContain('topology-wire-label-stock-deduct');
  });
});

// ── Warehouse tool-card tier lock ───────────────────────────────

describe('NodeTopologyEditor — warehouse tool-card tier lock', () => {
  it('locks the warehouse tool-card on standard tier when a warehouse exists', () => {
    renderEditor(); // retail preset already contains a warehouse

    const locked = document.querySelector('.tool-card.locked');
    expect(locked).not.toBeNull();
    expect(locked!.textContent).toContain('Pro');

    const before = getNodeCount();
    fireEvent.click(screen.getByText('+ Warehouse Node'));
    // handleAddNode guards the tier: toast, no new node.
    expect(getNodeCount()).toBe(before);
    expect(
      screen.getByText('Multi-Warehouse storage locations require a Pro Tier license.'),
    ).toBeInTheDocument();
  });

  it('unlocks the warehouse tool-card on Pro tier and adds a warehouse', () => {
    renderEditor({ currentTier: 'pro' });

    expect(document.querySelector('.tool-card.locked')).toBeNull();

    const before = getNodeCount();
    fireEvent.click(screen.getByText('+ Warehouse Node'));
    expect(getNodeCount()).toBe(before + 1);
  });
});

// ── Zoom controls behavior ──────────────────────────────────────

describe('NodeTopologyEditor — zoom controls behavior', () => {
  it('zooms with the mouse wheel and Reset View returns to 100%', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.wheel(canvas, { deltaY: -100, clientX: 10, clientY: 10 });
    expect(screen.getByText('Zoom: 110%')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Reset View'));
    expect(screen.getByText('Zoom: 100%')).toBeInTheDocument();
  });

  it('Fit All recomputes the zoom from the node bounds', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.wheel(canvas, { deltaY: -100, clientX: 10, clientY: 10 });
    expect(screen.getByText('Zoom: 110%')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Fit All'));
    // Fit-to-bounds replaces the wheel zoom with a computed value (still
    // clamped to the 40%..200% range).
    expect(screen.queryByText('Zoom: 110%')).not.toBeInTheDocument();
    expect(screen.getByText(/^Zoom: (?:[4-9]\d|1\d\d|200)%$/)).toBeInTheDocument();
  });
});

// ── Canvas pan ──────────────────────────────────────────────────

describe('NodeTopologyEditor — canvas pan', () => {
  it('pans the viewport when dragging on empty canvas background', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const viewport = document.querySelector('.node-canvas-viewport') as HTMLElement;
    expect(viewport.style.transform).toContain('translate(0px, 0px)');

    // mousedown at (100,100) on the background, drag to (150,130): the
    // viewport must translate by the pointer delta (50, 30).
    fireEvent.mouseDown(canvas, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(document, { clientX: 150, clientY: 130 });
    fireEvent.mouseUp(document, { button: 0 });

    expect(viewport.style.transform).toContain('translate(50px, 30px)');
  });

  it('dragging a node moves the node and leaves the viewport translation untouched', () => {
    renderEditor();
    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const viewport = document.querySelector('.node-canvas-viewport') as HTMLElement;
    const beforeLeft = firstNode.style.left;

    // Mirrors the established node-drag pattern: node gets the mousedown,
    // the canvas receives the move/up events.
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 48, clientY: 48 });
    fireEvent.mouseUp(canvas, { button: 0 });

    expect(firstNode.style.left).not.toBe(beforeLeft);
    expect(viewport.style.transform).toContain('translate(0px, 0px)');
  });
});

// ── Simulation pulse ────────────────────────────────────────────

describe('NodeTopologyEditor — simulation pulse', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows a pulse dot on wires while simulating and hides it when stopped', () => {
    vi.useFakeTimers();
    renderEditor();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(document.querySelector('.wire-simulation-pulse')).not.toBeNull();

    fireEvent.click(screen.getByText('Stop Simulation'));
    expect(document.querySelector('.wire-simulation-pulse')).toBeNull();
  });

  it('advances the pulse dot along the wire on each 30ms tick', () => {
    vi.useFakeTimers();
    renderEditor();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    const pulse = document.querySelector('.wire-simulation-pulse');
    expect(pulse).not.toBeNull();
    const beforeCx = pulse!.getAttribute('cx');

    // One 30ms interval tick: simPulseStep 0 -> 1, the dot moves along the
    // bezier curve (the wire's x-range guarantees cx changes).
    act(() => {
      vi.advanceTimersByTime(30);
    });

    const after = document.querySelector('.wire-simulation-pulse');
    expect(after).not.toBeNull();
    // The pulse follows a cubic bezier from the wire's start to end port, so
    // cx must change on every tick for any wire whose endpoints differ in x
    // (true of every preset wire today — keep that in mind if the preset's
    // geometry is ever edited).
    expect(after!.getAttribute('cx')).not.toBe(beforeCx);
  });
});

// ── Apply failure resilience ────────────────────────────────────

describe('NodeTopologyEditor — Apply failure resilience', () => {
  it('keeps edits, stays dirty, and preserves undo when Apply fails', async () => {
    renderEditor({
      onSave: async () => {
        throw new Error('boom');
      },
    });

    // Make a dirty edit: add a node.
    fireEvent.click(screen.getByText('+ Store Node'));
    const countAfterEdit = getNodeCount();

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    // The failure surfaces as an error toast (plainErrorMessage sanitizes the
    // thrown Error to the generic fallback, so pin the save-error key itself).
    await waitFor(() =>
      expect(screen.getByText(/topology-toast-save-error/)).toBeInTheDocument(),
    );

    // The in-memory edit survives the failed save.
    expect(getNodeCount()).toBe(countAfterEdit);

    // Undo still works — the pre-save history entry was not cleared.
    // (Asserted BEFORE opening a dialog: an open confirm dialog owns the
    // keyboard, so canvas shortcuts are inert under it.)
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(countAfterEdit - 1);

    // Still dirty: a preset click asks about unsaved changes (confirm dialog
    // title + the unsaved-changes message body are both rendered).
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThan(0);
    expect(
      screen.getByText(/Loading a preset will replace your current topology/),
    ).toBeInTheDocument();
  });

  it('does not clear selection when Apply fails before the idMap branch', async () => {
    renderEditor({ onSave: vi.fn().mockRejectedValue(new Error('late-fail')) });

    fireEvent.click(screen.getByText('+ Store Node'));
    // Select a node so a selection exists before the failed save.
    selectFirstNode();
    const selectedBefore = document.querySelector('.topology-node.node-selected');
    expect(selectedBefore).not.toBeNull();

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() =>
      expect(screen.getByText(/topology-toast-save-error/)).toBeInTheDocument(),
    );

    // Selection survives — the failure returns before the idMap branch clears it.
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();
  });
});

// ── Keyboard wire-direction toggle ──────────────────────────────

describe('NodeTopologyEditor — keyboard wire-direction toggle', () => {
  it('toggles wire direction with Enter and Space (keyboard parity)', () => {
    renderEditor();

    const wireLabels = screen.getAllByText(/→|↔/);
    const firstLabel = wireLabels[0]!;
    expect(firstLabel.textContent).toContain('→');

    fireEvent.keyDown(firstLabel, { key: 'Enter' });
    expect(firstLabel.textContent).toContain('↔');

    fireEvent.keyDown(firstLabel, { key: ' ' });
    expect(firstLabel.textContent).toContain('→');
  });
});

// ── Hover-target preview snap ───────────────────────────────────

describe('NodeTopologyEditor — hover-target preview snap', () => {
  it('snaps the in-flight preview to a port when hovering near it', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    // Start a connection from store-1's bottom port.
    fireEvent.click(portOf(nodeAt(0), 'bottom'));

    // Move to ws-1's top port: canvas coords are node.x + NODE_WIDTH/2,
    // node.y - 6 (pan 0 / zoom 1 / zero rect in jsdom, so clientX/Y ==
    // canvas coords and the hit is exact).
    const wsX = parseFloat(nodeAt(1).style.left);
    const wsY = parseFloat(nodeAt(1).style.top);
    const targetX = wsX + 100;
    const targetY = wsY - 6;
    fireEvent.mouseMove(canvas, { clientX: targetX, clientY: targetY });

    const preview = document.querySelector(
      'path.wire-path[opacity="0.5"]',
    ) as SVGPathElement | null;
    expect(preview).not.toBeNull();
    const d = preview!.getAttribute('d')!;
    const nums = d.match(/-?\d+(\.\d+)?/g)!.map(Number);
    const endX = nums[nums.length - 2]!;
    const endY = nums[nums.length - 1]!;
    // The preview endpoint snapped to the ws-1 top port.
    expect(endX).toBeCloseTo(targetX, 1);
    expect(endY).toBeCloseTo(targetY, 1);
  });
});

// ── Wire arrow markers ──────────────────────────────────────────

describe('NodeTopologyEditor — wire arrow markers', () => {
  it('renders two-way markers on a toggled wire and only the end marker on one-way wires', () => {
    renderEditor();

    // Retail preset wires are one-way: end marker only, no start marker.
    const oneWayPath = document.querySelector('path.wire-path.one-way');
    expect(oneWayPath).not.toBeNull();
    expect(oneWayPath!.getAttribute('marker-start')).toBeNull();
    expect(oneWayPath!.getAttribute('marker-end')).toBe('url(#arrow-end)');

    // Toggle the first wire to two-way via its label.
    const wireLabels = screen.getAllByText(/→|↔/);
    const oneWayCount = document.querySelectorAll('path.wire-path.one-way').length;
    fireEvent.click(wireLabels[0]!);

    // The toggled wire now carries the start marker and the ↔ label, and
    // exactly ONE wire left the one-way set — the others are untouched.
    expect(document.querySelectorAll('path.wire-path.one-way').length).toBe(oneWayCount - 1);
    const twoWayPath = document.querySelector('path.wire-path.two-way');
    expect(twoWayPath).not.toBeNull();
    expect(twoWayPath!.getAttribute('marker-start')).toBe('url(#arrow-start)');
    expect(twoWayPath!.getAttribute('marker-end')).toBe('url(#arrow-end)');
    expect(wireLabels[0]!.textContent).toContain('↔');
  });
});

// ── Fresh-node animation pulse ──────────────────────────────────

describe('NodeTopologyEditor — fresh-node animation pulse', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('marks a newly added node as fresh and clears the pulse after 400ms', () => {
    vi.useFakeTimers();
    renderEditor();

    fireEvent.click(screen.getByText('+ Store Node'));
    expect(document.querySelector('.topology-node.node-fresh')).not.toBeNull();

    // The 400ms fresh-pulse timer expires.
    act(() => {
      vi.advanceTimersByTime(400);
    });
    expect(document.querySelector('.topology-node.node-fresh')).toBeNull();
  });
});

// ── Undo history cap ────────────────────────────────────────────

describe('NodeTopologyEditor — undo history cap', () => {
  it('caps the undo stack at 50 entries, evicting the oldest', () => {
    renderEditor();
    const initial = getNodeCount(); // retail preset: 3

    // 51 node adds — each pushes one history entry; the cap (pushHistory
    // evicts when length > 50, so the stack holds at most 50) drops the
    // oldest, which is the snapshot of the ORIGINAL pre-edit state.
    for (let i = 0; i < 51; i++) {
      fireEvent.click(screen.getByText('+ Store Node'));
    }
    expect(getNodeCount()).toBe(initial + 51);

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    // Exactly 50 undos are available: they walk back to initial + 1 (the
    // first add's snapshot is the evicted one).
    for (let i = 0; i < 50; i++) {
      fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    }
    expect(getNodeCount()).toBe(initial + 1);

    // The evicted oldest entry (the original state) is unreachable — the
    // 51st undo is a no-op on the empty stack.
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(initial + 1);
  });
});

// ── Direction toggle undo/redo ──────────────────────────────────

describe('NodeTopologyEditor — direction toggle undo/redo', () => {
  it('undo restores a toggled wire direction and redo re-applies it', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const firstLabel = screen.getAllByText(/→|↔/)[0]!;
    expect(firstLabel.textContent).toContain('→');

    // Toggle to two-way — handleToggleWireDirection pushes history.
    fireEvent.click(firstLabel);
    expect(firstLabel.textContent).toContain('↔');

    // One undo returns to one-way; one redo re-applies two-way.
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(firstLabel.textContent).toContain('→');
    fireEvent.keyDown(canvas, { key: 'y', ctrlKey: true });
    expect(firstLabel.textContent).toContain('↔');
  });
});

// ── Connected label on regular wires ────────────────────────────

describe('NodeTopologyEditor — connected wire label', () => {
  it('labels a regular store→workspace wire as connected', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline + 1);

    // Non-warehouse wires carry the plain connected label (raw identity key).
    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    expect(last.textContent).toContain('topology-wire-label-connected');
  });
});

// ── Preset load cancels in-flight connection ────────────────────

describe('NodeTopologyEditor — preset load cancels in-flight connection', () => {
  it('cancels an in-flight connection when a preset is loaded', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from store-1's bottom port — ghost preview appears.
    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    expect(previewLine()).not.toBeNull();

    // Load the SAME preset mid-connection (no edits yet, so it loads
    // directly without the unsaved-changes dialog).
    fireEvent.click(screen.getByText('Retail Preset'));

    // The canvas was replaced — the in-flight connection must be cancelled:
    // no ghost preview may survive the replacement...
    expect(previewLine()).toBeNull();
    // ...and a subsequent target-port click must start a NEW connection
    // instead of completing the stale one (no wire may be created).
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(baseline);
  });

  it('cancels an in-flight connection when the canvas reloads from instances', async () => {
    // Saved diagram with a store + workspace; the harness then replaces the
    // canvas via a workspaceInstances reload (post-save/apply path).
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Store', x: 80, y: 120 },
        { id: 'ws-1', type: 'workspace', name: 'POS', x: 240, y: 80 },
      ],
      wires: [],
    });
    renderWithProvidersSync(
      <ReloadingHarness
        next={[{ instanceId: 'ws-1', typeKey: 'store-pos', name: 'POS' }]}
      />,
      multiStoreFtl,
      sharedFtl,
    );

    await waitFor(() => expect(screen.getByText('Store')).toBeInTheDocument());

    fireEvent.click(portOf(nodeAt(0), 'bottom'));
    expect(previewLine()).not.toBeNull();

    // Trigger the workspaceInstances reload — the canvas is replaced.
    fireEvent.click(screen.getByText('reload-instances'));
    await waitFor(() => expect(previewLine()).toBeNull());

    // The stale connection cannot complete: a target click creates no wire.
    fireEvent.click(portOf(nodeAt(1), 'top'));
    expect(getWireCount()).toBe(0);
  });
});

// ── Escape on an open dialog does not touch canvas state ────────

describe('NodeTopologyEditor — dialog Escape isolation', () => {
  it('Escape cancelling the delete dialog keeps the node selected', () => {
    renderEditor();

    // Select a wired node so the delete flow opens the confirm dialog.
    selectFirstNode();
    fireEvent.click(screen.getByText('Delete Selected Element'));
    expect(screen.getByText('Delete Node')).toBeInTheDocument();

    // Escape closes the dialog (the Modal's focus trap owns it)...
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'Escape' });
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();

    // ...without the editor's window-level handler stealing the selection
    // (the dialog must own the keyboard while it is open).
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();
  });

  it('Escape cancelling the unsaved-changes preset dialog leaves the edit intact', () => {
    renderEditor();

    // Make a dirty edit so the preset click opens the confirm dialog. The
    // add also selects the new node, so the guard's effect is observable:
    // Escape must NOT steal the selection while the dialog is open.
    fireEvent.click(screen.getByText('+ Store Node'));
    const countAfterEdit = getNodeCount();
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();

    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThan(0);

    // Escape closes the dialog without loading the preset...
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'Escape' });
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();

    // ...the edit survives untouched, and the selection was NOT cleared by
    // the editor's window-level handler (the dialog owns the keyboard).
    expect(getNodeCount()).toBe(countAfterEdit);
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();
  });
});
