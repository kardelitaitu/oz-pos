import { useState, type ComponentProps } from 'react';
import { screen, fireEvent, waitFor, act, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor, { type WorkspaceInstanceSeed, type BranchLocationSeed } from '../features/stores/NodeTopologyEditor';
import {
  clampNodeToViewport,
  NODE_HEIGHT,
  NODE_PORT_ROW_H,
  NODE_PORT_MARKER,
  NODE_PORT_Y,
  NODE_WIDTH,
} from '../features/stores/nodeTopologyClamp';
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
  'topology-port-operation-in': 'Operation',
  'topology-ws-type-warehouse': 'Warehouse',
  'topology-port-location-out': 'Location',
  'topology-port-location-in': 'Location',
  'topology-port-location-out-aria': 'Location port',
  'topology-port-location-in-aria': 'Location port',
  'topology-port-aria': 'Topology port',
  'topology-wire-flip-hint-connecting':
    'Flip direction? Clicking keeps your connection in progress.',
  'topology-port-workspace-out': 'Operation',
  'topology-port-stock-in': 'Stock In',
  'topology-port-stock-out': 'Stock Out',
  'topology-port-ticket-in': 'Ticket In',
  'topology-port-device-out': 'Device Out',
  'topology-port-generic-in': 'Input',
  'topology-port-generic-out': 'Output',
  'topology-port-input-only': 'Input connectors receive connections; choose an output connector first.',
  'topology-field-name': 'Name',
  'topology-field-name-aria': 'Edit name',
  'topology-field-enabled': 'Enabled',
  'topology-field-enabled-aria': 'Toggle enabled state',
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
  branchLocations?: BranchLocationSeed[];
  workspaceInstances?: WorkspaceInstanceSeed[];
  onRenameBranch?: (id: string, name: string) => Promise<boolean> | boolean | void;
  onRenameWorkspace?: (id: string, name: string) => Promise<boolean> | boolean | void;
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

/** Stable workspace seed — identity never changes, so only the
 *  branchLocations prop drives the load-effect re-run in the rename flow. */
const renameWsInstances: WorkspaceInstanceSeed[] = [
  { instanceId: 'ws-rename', typeKey: 'store-pos', name: 'POS #1' },
];

/**
 * Harness simulating the TopologyScreen parent: a successful card rename
 * swaps the branchLocations identity (the parent's stores-state update),
 * re-triggering the editor's load effect with ONLY branch locations changed.
 */
function BranchRenameHarness() {
  const [locations, setLocations] = useState<BranchLocationSeed[]>([
    { id: 'store-1', name: 'Downtown Branch' },
  ]);
  return (
    <NodeTopologyEditor
      currentTier="standard"
      workspaceInstances={renameWsInstances}
      branchLocations={locations}
      onRenameBranch={async (id, name) => {
        setLocations((prev) => prev.map((l) => (l.id === id ? { ...l, name } : l)));
        return true;
      }}
    />
  );
}

/** Harness simulating the TopologyScreen parent for workspace renames: the
 *  parent persists via the instance API and refreshes the instances array
 *  with the SAME ids (only the name changed). */
function WorkspaceRenameHarness() {
  const [instances, setInstances] = useState<WorkspaceInstanceSeed[]>([
    { instanceId: 'ws-rename', typeKey: 'store-pos', name: 'POS #1' },
  ]);
  return (
    <NodeTopologyEditor
      currentTier="standard"
      workspaceInstances={instances}
      branchLocations={[{ id: 'store-1', name: 'Downtown Branch' }]}
      onRenameWorkspace={async (id, name) => {
        setInstances((prev) => prev.map((i) => (i.instanceId === id ? { ...i, name } : i)));
        return true;
      }}
    />
  );
}

/** Harness that swaps instances AND branch locations in one action — the
 *  instances-changed-wins case the light-merge guard must NOT intercept. */
function BothChangeHarness() {
  const [instances, setInstances] = useState<WorkspaceInstanceSeed[] | undefined>(undefined);
  const [locations, setLocations] = useState<BranchLocationSeed[] | undefined>(undefined);
  return (
    <>
      <button type="button" onClick={() => {
        setInstances([{ instanceId: 'ws-both', typeKey: 'store-pos', name: 'Both POS' }]);
        setLocations([{ id: 'store-9', name: 'Both Branch' }]);
      }}>
        both-change
      </button>
      <NodeTopologyEditor
        currentTier="standard"
        {...(instances ? { workspaceInstances: instances } : {})}
        {...(locations ? { branchLocations: locations } : {})}
      />
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
const typeSelect = () => document.querySelector('select.inspector-select') as HTMLSelectElement;
const previewLine = () => document.querySelector('path.wire-path[opacity="0.5"]');

const selectFirstNode = () => {
  const firstNode = document.querySelector('.topology-node');
  if (firstNode) fireEvent.mouseDown(firstNode as Element, { button: 0 });
};

/** Lay the canvas out in jsdom so viewport-relative clamping has a real size. */
const mockCanvasSize = (width: number, height: number) => {
  const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
  Object.defineProperty(canvas, 'clientWidth', { value: width, configurable: true });
  Object.defineProperty(canvas, 'clientHeight', { value: height, configurable: true });
};

describe('NodeTopologyEditor Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLoadTopology.mockResolvedValue(null);
    mockSaveTopology.mockResolvedValue(undefined);
  });

  it('renders tier badge and default retail preset nodes', () => {
    renderEditor();

    // The header no longer carries a title — the tier badge (default
    // currentTier = 'standard') marks the header instead.
    expect(screen.getByText('STANDARD TIER')).toBeInTheDocument();
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

  it('renders the UX-first titlebar, left/right labeled ports, textbox, and toggle', () => {
    renderEditor();

    expect(document.querySelectorAll('.node-titlebar')).toHaveLength(3);
    expect(screen.getAllByText('Location').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Operation').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByRole('textbox', { name: /Edit name/ })).toHaveLength(1);
    expect(screen.getAllByRole('checkbox', { name: /Toggle enabled state/ })).toHaveLength(1);
    expect(document.querySelectorAll('.node-port-socket.port-top')).toHaveLength(0);
    expect(document.querySelectorAll('.node-port-socket.port-bottom')).toHaveLength(0);
    expect(document.body.textContent).not.toContain('topology-port-');
    expect(document.body.textContent).not.toContain('topology-field-');

    // The widened card reserves a dedicated connector footer: labels and
    // sockets are outside the content flow, so they cannot collide with
    // telemetry or inline workspace controls.
    const workspace = nodeAt(1);
    expect(workspace.classList.contains('node-type-workspace')).toBe(true);
    expect(workspace.querySelector('.node-body')).not.toBeNull();
    expect(workspace.querySelector('.node-port-sockets-group')).not.toBeNull();
    expect(workspace.querySelector('.node-port-label-left')).not.toBeNull();
    expect(workspace.querySelector('.node-port-label-right')).not.toBeNull();
  });

  it('keeps telemetry badges in the header so they never collide with connector labels', () => {
    renderEditor();

    // The status badge (e.g. "Receipt ✓" / "KDS Ready" / "Active") is a
    // header chip. If it ever returns to the body it sits immediately above
    // the connector rail and visually collides with the left/right port
    // labels — this test pins the ownership contract.
    for (let i = 0; i < 3; i += 1) {
      const card = nodeAt(i);
      const header = card.querySelector('.node-header');
      const body = card.querySelector('.node-body');
      const badge = card.querySelector('.node-telemetry-badge');
      expect(header, `node ${i} header exists`).not.toBeNull();
      expect(body, `node ${i} body exists`).not.toBeNull();
      if (badge) {
        expect(header?.contains(badge), `node ${i} badge lives in the header`).toBe(true);
        expect(body?.contains(badge), `node ${i} badge is NOT in the body`).toBe(false);
      }
    }
  });

  it('keeps connector geometry aligned to the card edge and footer centerline', () => {
    renderEditor();

    const workspace = nodeAt(1);
    const sockets = workspace.querySelector('.node-port-sockets-group') as HTMLElement;
    const left = workspace.querySelector('.node-port-socket.port-left') as HTMLElement;
    const right = workspace.querySelector('.node-port-socket.port-right') as HTMLElement;
    expect(sockets).not.toBeNull();
    expect(left).not.toBeNull();
    expect(right).not.toBeNull();

    // The shared contract is intentionally explicit: wire endpoints are at
    // x=0 / x=NODE_WIDTH and y=NODE_PORT_Y, while CSS centers each circle at
    // those same card-edge coordinates inside the footer hit area. The rail
    // formula mirrors the CSS marker centering (top = (32 − 12) / 2 = 10).
    expect(NODE_WIDTH).toBe(240);
    expect(NODE_PORT_Y).toBe(
      NODE_HEIGHT - NODE_PORT_ROW_H + NODE_PORT_ROW_H / 2,
    );
    expect(NODE_PORT_ROW_H - NODE_PORT_MARKER).toBe(20);
    expect(left.className).toContain('port-left');
    expect(right.className).toContain('port-right');
    expect(sockets.className).toContain('node-port-sockets-group');
  });

  it('keeps long workspace titles visually bounded by the titlebar', () => {
    renderEditor();

    const title = nodeAt(1).querySelector('.node-title') as HTMLElement;
    expect(title).not.toBeNull();
    expect(title.parentElement?.classList.contains('node-title-wrapper')).toBe(true);
  });

  it('edits inline workspace controls without dragging the node', () => {
    renderEditor();

    const workspace = nodeAt(1);
    const nameInput = screen.getAllByRole('textbox', { name: /Edit name/ })[0]!;
    const enabled = screen.getAllByRole('checkbox', { name: /Toggle enabled state/ })[0]!;
    fireEvent.change(nameInput, { target: { value: 'Counter POS' } });
    fireEvent.click(enabled);

    expect(screen.getByText('Counter POS')).toBeInTheDocument();
    expect((enabled as HTMLInputElement).checked).toBe(false);
    // Retail preset spaces workspace cards at x 380 so the 240px-wide cards
    // never overlap the store column.
    expect(workspace.style.left).toBe('380px');
  });

  it('drags from the titlebar while connector clicks remain interaction-only', () => {
    renderEditor();

    const workspace = nodeAt(1);
    const titlebar = workspace.querySelector('.node-titlebar') as HTMLElement;
    const port = portOf(workspace, 'right');
    const before = workspace.style.left;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.mouseDown(titlebar, { button: 0, clientX: 340, clientY: 80 });
    fireEvent.mouseMove(canvas, { clientX: 388, clientY: 80 });
    expect(workspace.style.left).not.toBe(before);
    fireEvent.mouseUp(canvas, { button: 0 });

    const afterDrag = workspace.style.left;
    fireEvent.mouseDown(port, { button: 0, clientX: 0, clientY: 0 });
    expect(workspace.style.left).toBe(afterDrag);
  });

  it('exposes ONE flexible left input on Inventory nodes (Input → Location/Operation by wire)', async () => {
    // An Inventory Manager takes a single left feed — Location or Operation
    // (from another store-pos/inventory). Unwired it reads "Input"; once a
    // wire attaches it adopts that wire's semantic label.
    mockLoadTopology.mockResolvedValueOnce({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Branch', x: 80, y: 140 },
        { id: 'ws-inv', type: 'workspace', name: 'Inventory', x: 380, y: 140, metadata: { typeKey: 'inventory' } },
      ],
      wires: [],
    } as never);
    renderEditor();

    await waitFor(() => expect(document.querySelectorAll('.topology-node')).toHaveLength(2));
    const inv = nodeAt(1);
    // Single left socket, neutral label while unwired; one right output.
    expect(inv.querySelectorAll('.node-port-socket.port-left')).toHaveLength(1);
    expect(inv.querySelectorAll('.node-port-socket.port-right')).toHaveLength(1);
    expect(inv.querySelector('.node-port-label-left')?.textContent).toBe('Input');
  });

  it('labels an Inventory input by the wire attached (Location for location-in)', async () => {
    mockLoadTopology.mockResolvedValueOnce({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Branch', x: 80, y: 140 },
        { id: 'ws-inv', type: 'workspace', name: 'Inventory', x: 380, y: 140, metadata: { typeKey: 'inventory' } },
      ],
      wires: [
        { id: 'w-loc', from_node_id: 'store-1', from_port: 'right', to_node_id: 'ws-inv', to_port: 'left', direction: 'one-way', to_port_id: 'location-in' },
      ],
    } as never);
    renderEditor();

    await waitFor(() => expect(document.querySelectorAll('.topology-node')).toHaveLength(2));
    const inv = nodeAt(1);
    expect(inv.querySelectorAll('.node-port-socket.port-left')).toHaveLength(1);
    expect(inv.querySelector('.node-port-label-left')?.textContent).toBe('Location');
  });

  it('exposes a single left Operation port on Kitchen Display nodes', async () => {
    // A KDS is a sink: it consumes one Operation feed and forwards nothing,
    // so it must expose exactly one left connector labeled "Operation" —
    // never a Location In pair or a right-side Operational Out.
    mockLoadTopology.mockResolvedValueOnce({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Branch', x: 80, y: 140 },
        { id: 'ws-kds', type: 'workspace', name: 'Kitchen Display', x: 380, y: 80, metadata: { typeKey: 'kds' } },
      ],
      wires: [],
    } as never);
    renderEditor();

    await waitFor(() => expect(document.querySelectorAll('.topology-node')).toHaveLength(2));
    const kds = nodeAt(1);
    expect(kds.querySelector('.node-title')?.textContent).toBe('Kitchen Display');
    expect(kds.querySelectorAll('.node-port-socket.port-left')).toHaveLength(1);
    expect(kds.querySelectorAll('.node-port-socket.port-right')).toHaveLength(0);
    expect(kds.querySelector('.node-port-label-left')?.textContent).toBe('Operation');
    expect(kds.querySelector('.node-port-label-right')).toBeNull();
  });

  it('renders legacy vertical wire ports on canonical left/right sides', async () => {
    mockLoadTopology.mockResolvedValueOnce({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Branch', x: 80, y: 140 },
        { id: 'ws-1', type: 'workspace', name: 'POS', x: 340, y: 80 },
      ],
      wires: [{ id: 'legacy-wire', from_node_id: 'store-1', from_port: 'top', to_node_id: 'ws-1', to_port: 'bottom', direction: 'one-way' }],
    } as never);
    renderEditor();

    await waitFor(() => expect(getWireCount()).toBe(1));
    expect(portOf(nodeAt(0), 'right')).not.toBeNull();
    expect(portOf(nodeAt(0), 'top')).toBeNull();
    expect(portOf(nodeAt(1), 'left')).not.toBeNull();
    expect(portOf(nodeAt(1), 'bottom')).toBeNull();
  });

  it('switches to restaurant & KDS preset when clicked', () => {
    renderEditor();

    const restoBtn = screen.getByText('Resto & KDS Preset');
    fireEvent.click(restoBtn);

    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(screen.getByText('Kitchen KDS')).toBeInTheDocument();
    expect(screen.getByText('Kitchen Thermal Printer')).toBeInTheDocument();
  });

  it('preset Kitchen KDS exposes only a left Operation port', () => {
    // The Resto & KDS preset seeds the KDS node WITHOUT metadata.typeKey
    // unless it is defined on the preset data itself — and without it,
    // isKdsNode() fails and the sink wrongly renders a Location In pair
    // plus a right-side output. Pin the preset's own data so the port
    // contract cannot regress when the preset is edited.
    renderEditor();
    fireEvent.click(screen.getByText('Resto & KDS Preset'));

    const nodes = [...document.querySelectorAll('.topology-node')];
    const kds = nodes.find((n) => n.querySelector('.node-title')?.textContent === 'Kitchen KDS');
    expect(kds).not.toBeUndefined();
    expect(kds!.querySelectorAll('.node-port-socket.port-left')).toHaveLength(1);
    expect(kds!.querySelectorAll('.node-port-socket.port-right')).toHaveLength(0);
    expect(kds!.querySelector('.node-port-label-left')?.textContent).toBe('Operation');
    expect(kds!.querySelector('.node-port-label-right')).toBeNull();
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

  // ── Dynamic edge clamp: north/west freedom past the old 20px floor ──

  it('lets a node drag freely north/west to the viewport edge (not the 20px floor)', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(firstNode).not.toBeNull();
    expect(canvas).not.toBeNull();
    // Lay the canvas out so the clamp has a real viewport to honour.
    mockCanvasSize(800, 600);
    expect(firstNode.style.left).toBe('80px');
    expect(firstNode.style.top).toBe('140px');

    // mousedown at clientX/Y 0 → dragOffset = (0 - 80, 0 - 140).
    fireEvent.mouseDown(firstNode, { button: 0, clientX: 0, clientY: 0 });

    // Drag far north-west: raw = (-320, -260) → clamped to the viewport
    // edge (-200, -200) → snapped (-192, -192). The old 20px floor would
    // have parked the node at (24, 24).
    fireEvent.mouseMove(canvas, { clientX: -400, clientY: -400 });
    expect(firstNode.style.left).toBe('-192px');
    expect(firstNode.style.top).toBe('-192px');

    // Dragging even further must HOLD the node at the edge — the clamp
    // guarantees a node can never be pushed off-canvas and lost.
    fireEvent.mouseMove(canvas, { clientX: -1000, clientY: -1000 });
    expect(firstNode.style.left).toBe('-192px');
    expect(firstNode.style.top).toBe('-192px');

    fireEvent.mouseUp(canvas, { button: 0 });
  });

  it('arrow keys respect the same viewport clamp as mouse dragging', () => {
    renderEditor();

    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    mockCanvasSize(800, 600);

    selectFirstNode();
    // Shift nudges move in full grid steps (24px), so every press actually
    // advances: 80 → 48 → 24 → 0 → -24 → … → clamps at minX = -200,
    // snaps to the grid bound -192. (A plain 8px nudge oscillates on the
    // 24px grid and would never reach the edge.)
    for (let i = 0; i < 15; i += 1) {
      fireEvent.keyDown(canvas, { key: 'ArrowLeft', shiftKey: true });
    }
    expect(firstNode.style.left).toBe('-192px');

    // North: 140 → 120 → 96 → … → clamps at minY = -200,
    // snapping to the nearest grid position -192.
    for (let i = 0; i < 15; i += 1) {
      fireEvent.keyDown(canvas, { key: 'ArrowUp', shiftKey: true });
    }
    expect(firstNode.style.top).toBe('-192px');
  });

  // ── clampNodeToViewport unit contract ────────────────────────────

  describe('clampNodeToViewport (view-relative edge clamp)', () => {
    it('at identity transform, the west/north bound keeps only the margin visible', () => {
    // 800×600 canvas, pan 0, zoom 1, default node 240×240, margin 40.
    expect(clampNodeToViewport(-320, -260, { panX: 0, panY: 0, zoom: 1, canvasW: 800, canvasH: 600 }))
      .toEqual({ x: -200, y: -200 });
    });

    it('keeps the node inside the east/south edges with the same margin', () => {
      expect(clampNodeToViewport(9999, 9999, { panX: 0, panY: 0, zoom: 1, canvasW: 800, canvasH: 600 }))
        .toEqual({ x: 760, y: 560 });
    });

    it('is pan-aware: panning extends the reachable west bound', () => {
      // Pan +200 shifts content right, so the canvas-space origin sits
      // further left: minX = (40 - 200) - 240 = -400.
      expect(clampNodeToViewport(-500, -260, { panX: 200, panY: 0, zoom: 1, canvasW: 800, canvasH: 600 }))
        .toEqual({ x: -400, y: -200 });
    });

    it('is zoom-aware: zooming out widens the reachable bounds', () => {
      // Margin is in screen px, so at zoom 0.5 the 40px margin becomes
      // 80 canvas px: minX = (40/0.5) - 240 = -160.
      expect(clampNodeToViewport(-320, -260, { panX: 0, panY: 0, zoom: 0.5, canvasW: 800, canvasH: 600 }))
        .toEqual({ x: -160, y: -160 });
    });

    it('returns the position unchanged when the canvas has no measured size', () => {
      // jsdom / pre-layout canvases report 0 — no viewport constraint exists.
      expect(clampNodeToViewport(120, 90, { panX: 0, panY: 0, zoom: 1, canvasW: 0, canvasH: 0 }))
        .toEqual({ x: 120, y: 90 });
    });
  });

  // ── Branch Location rename from the node card ───────────────────

  it('renames the branch from its card and closes the inline form', async () => {
    const onRenameBranch = vi.fn().mockResolvedValue(true);
    renderEditor({ onRenameBranch });

    // The retail preset's store card is the first node on canvas.
    const storeCard = document.querySelectorAll('.topology-node')[0] as HTMLElement;
    expect(storeCard.className).toContain('node-type-store');

    // Pencil → inline input pre-filled with the current name → Enter.
    fireEvent.click(within(storeCard).getByRole('button', { name: 'topology-branch-rename-label' }));
    const input = within(storeCard).getByLabelText('topology-branch-rename-placeholder');
    expect((input as HTMLInputElement).value).toBe('Downtown Branch');
    fireEvent.change(input, { target: { value: 'Main Street HQ' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(onRenameBranch).toHaveBeenCalledWith('store-1', 'Main Street HQ'));
    // On success the form closes and the card title reflects the new name.
    await waitFor(() => expect(within(storeCard).queryByLabelText('topology-branch-rename-placeholder')).toBeNull());
    await waitFor(() => expect(within(storeCard).getByText('Main Street HQ')).toBeTruthy());
    // A keyboard commit returns focus to the store card, not the canvas body.
    expect(document.activeElement).toBe(storeCard);
  });

  it('closes the rename form when the name is unchanged (no callback round-trip)', () => {
    const onRenameBranch = vi.fn();
    renderEditor({ onRenameBranch });

    const storeCard = document.querySelectorAll('.topology-node')[0] as HTMLElement;
    fireEvent.click(within(storeCard).getByRole('button', { name: 'topology-branch-rename-label' }));
    const input = within(storeCard).getByLabelText('topology-branch-rename-placeholder');
    // Enter without editing — the pre-filled draft equals the current name.
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(onRenameBranch).not.toHaveBeenCalled();
    expect(within(storeCard).queryByLabelText('topology-branch-rename-placeholder')).toBeNull();
    expect(within(storeCard).getByText('Downtown Branch')).toBeTruthy();
    // An unchanged-name Enter is still a keyboard close — focus returns to the card.
    expect(document.activeElement).toBe(storeCard);
  });

  it('Escape cancels the card rename without calling the callback', () => {
    const onRenameBranch = vi.fn();
    renderEditor({ onRenameBranch });

    const storeCard = document.querySelectorAll('.topology-node')[0] as HTMLElement;
    fireEvent.click(within(storeCard).getByRole('button', { name: 'topology-branch-rename-label' }));
    const input = within(storeCard).getByLabelText('topology-branch-rename-placeholder');
    fireEvent.keyDown(input, { key: 'Escape' });

    expect(onRenameBranch).not.toHaveBeenCalled();
    expect(within(storeCard).queryByLabelText('topology-branch-rename-placeholder')).toBeNull();
    expect(within(storeCard).getByText('Downtown Branch')).toBeTruthy();
    // Escape is a keyboard close — focus returns to the store card.
    expect(document.activeElement).toBe(storeCard);
  });

  it('does not steal focus back when the rename commits via blur', async () => {
    // Click-away commits must NOT yank focus back to the card — the user
    // deliberately clicked elsewhere (e.g. the Apply button).
    const onRenameBranch = vi.fn().mockResolvedValue(true);
    renderEditor({ onRenameBranch });

    const storeCard = document.querySelectorAll('.topology-node')[0] as HTMLElement;
    fireEvent.click(within(storeCard).getByRole('button', { name: 'topology-branch-rename-label' }));
    const input = within(storeCard).getByLabelText('topology-branch-rename-placeholder');
    fireEvent.change(input, { target: { value: 'Blur Renamed' } });
    fireEvent.blur(input);

    await waitFor(() => expect(onRenameBranch).toHaveBeenCalledWith('store-1', 'Blur Renamed'));
    await waitFor(() => expect(within(storeCard).queryByLabelText('topology-branch-rename-placeholder')).toBeNull());
    expect(document.activeElement).not.toBe(storeCard);
  });

  /** Harness simulating the TopologyScreen parent deleting a store profile:
 *  removing a branch swaps the branchLocations identity (the parent's
 *  stores-state update) WITHOUT touching workspace instances — the
 *  light-merge path must drop the deleted branch's card and wires. */
function BranchDeleteHarness() {
  const [locations, setLocations] = useState<BranchLocationSeed[]>([
    { id: 'store-1', name: 'Downtown Branch' },
    { id: 'store-2', name: 'Uptown Branch' },
  ]);
  return (
    <div>
      <button onClick={() => setLocations((prev) => prev.filter((l) => l.id !== 'store-1'))}>
        delete-store-1
      </button>
      <NodeTopologyEditor
        currentTier="standard"
        workspaceInstances={renameWsInstances}
        branchLocations={locations}
      />
    </div>
  );
}

// ── Branch rename must not clobber unsaved canvas edits ─────────

  it('keeps unsaved canvas edits when a branch rename refreshes branch locations', async () => {
    // The TopologyScreen parent reacts to a successful rename by updating
    // its stores state, which swaps the branchLocations identity the editor
    // receives. A full rebuild on that change would discard in-flight drags
    // and wires — the rename must merge the new name into the live canvas.
    renderWithProvidersSync(<BranchRenameHarness />, multiStoreFtl, sharedFtl);
    await waitFor(() => expect(document.querySelector('.node-canvas-container')).not.toBeNull());
    mockCanvasSize(800, 600);

    await waitFor(() => expect(screen.getByText('POS #1')).toBeInTheDocument());

    // Unsaved edit: drag the workspace node across the canvas.
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const wsCard = nodeAt(1);
    fireEvent.mouseDown(wsCard, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 200, clientY: 120 });
    fireEvent.mouseUp(canvas, { button: 0 });
    const draggedLeft = wsCard.style.left;
    const draggedTop = wsCard.style.top;
    expect(draggedLeft).not.toBe('336px'); // the drag actually moved it

    // Rename the branch through its card (name change → parent refreshes
    // branchLocations with a new identity).
    const storeCard = nodeAt(0);
    fireEvent.click(within(storeCard).getByRole('button', { name: 'topology-branch-rename-label' }));
    const input = within(storeCard).getByLabelText('topology-branch-rename-placeholder');
    fireEvent.change(input, { target: { value: 'Renamed HQ' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    // The rename propagated to the card title…
    await waitFor(() => expect(within(storeCard).getByText('Renamed HQ')).toBeTruthy());
    // …and the unsaved drag survived the branchLocations refresh.
    await waitFor(() => expect(wsCard.style.left).toBe(draggedLeft));
    expect(wsCard.style.top).toBe(draggedTop);
  });

  // ── Branch deletion: card + wires leave the canvas cleanly ─────

  it('removes a deleted branch location card and its wires (light merge)', async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Downtown Branch', x: 80, y: 140, store_profile_id: 'store-1' },
        { id: 'store-2', type: 'store', name: 'Uptown Branch', x: 380, y: 140, store_profile_id: 'store-2' },
      ],
      wires: [
        { id: 'w-1', from_node_id: 'store-1', from_port: 'right', to_node_id: 'ws-rename', to_port: 'left', direction: 'one-way' },
        { id: 'w-2', from_node_id: 'store-2', from_port: 'right', to_node_id: 'ws-rename', to_port: 'left', direction: 'one-way' },
      ],
    });
    renderWithProvidersSync(<BranchDeleteHarness />, multiStoreFtl, sharedFtl);
    await waitFor(() => expect(document.querySelector('.node-canvas-container')).not.toBeNull());
    mockCanvasSize(800, 600);
    await waitFor(() => expect(screen.getByText('POS #1')).toBeInTheDocument());
    expect(getNodeCount()).toBe(3);
    expect(getWireCount()).toBe(2);

    // The parent deletes store-1 — the canvas must drop its card and the
    // wire attached to it, keeping the surviving branch + workspace intact.
    fireEvent.click(screen.getByRole('button', { name: 'delete-store-1' }));

    await waitFor(() => expect(getNodeCount()).toBe(2));
    expect(getWireCount()).toBe(1);
    expect(screen.queryByText('Downtown Branch')).not.toBeInTheDocument();
    expect(screen.getByText('Uptown Branch')).toBeInTheDocument();
    expect(screen.getByText('POS #1')).toBeInTheDocument();
  });

  it('drops a saved store node whose branch was deleted (full rebuild)', async () => {
    // The real delete flow remounts the editor for the next branch: the
    // saved diagram may still carry the deleted branch's node, but the
    // rebuild must not resurrect it — its card and wires leave cleanly.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Downtown Branch', x: 80, y: 140, store_profile_id: 'store-1' },
        { id: 'store-2', type: 'store', name: 'Uptown Branch', x: 380, y: 140, store_profile_id: 'store-2' },
      ],
      wires: [
        { id: 'w-1', from_node_id: 'store-1', from_port: 'right', to_node_id: 'ws-rename', to_port: 'left', direction: 'one-way' },
        { id: 'w-2', from_node_id: 'store-2', from_port: 'right', to_node_id: 'ws-rename', to_port: 'left', direction: 'one-way' },
      ],
    });
    renderEditor({
      branchLocations: [{ id: 'store-2', name: 'Uptown Branch' }],
      workspaceInstances: renameWsInstances,
    });
    await waitFor(() => expect(screen.getByText('POS #1')).toBeInTheDocument());
    expect(getNodeCount()).toBe(2);
    expect(getWireCount()).toBe(1);
    expect(screen.queryByText('Downtown Branch')).not.toBeInTheDocument();
    expect(screen.getByText('Uptown Branch')).toBeInTheDocument();
  });

  it('renames a workspace node from its card', async () => {
    const onRenameWorkspace = vi.fn().mockResolvedValue(true);
    renderEditor({ onRenameWorkspace });

    // The retail preset's workspace card is the second node on canvas.
    const wsCard = nodeAt(1);
    expect(wsCard.className).toContain('node-type-workspace');

    // Pencil → inline input pre-filled with the current name → Enter.
    fireEvent.click(within(wsCard).getByRole('button', { name: 'topology-workspace-rename-label' }));
    const input = within(wsCard).getByLabelText('topology-workspace-rename-placeholder');
    expect((input as HTMLInputElement).value).toBe('Retail POS #1');
    fireEvent.change(input, { target: { value: 'Front Register' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(onRenameWorkspace).toHaveBeenCalledWith('ws-1', 'Front Register'));
    await waitFor(() => expect(within(wsCard).getByText('Front Register')).toBeTruthy());
    // Keyboard commit returns focus to the workspace card.
    expect(document.activeElement).toBe(wsCard);
  });

  it('keeps unsaved canvas edits when a workspace rename refreshes instances (same ids)', async () => {
    // The parent persists the rename via the instance API and refreshes the
    // instances array with the SAME ids — the editor must merge the new
    // name in place, not rebuild (a rebuild would discard the unsaved drag).
    renderWithProvidersSync(<WorkspaceRenameHarness />, multiStoreFtl, sharedFtl);
    await waitFor(() => expect(document.querySelector('.node-canvas-container')).not.toBeNull());
    mockCanvasSize(800, 600);
    await waitFor(() => expect(screen.getByText('POS #1')).toBeInTheDocument());

    // Unsaved edit: drag the workspace node across the canvas.
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const wsCard = nodeAt(1);
    fireEvent.mouseDown(wsCard, { button: 0, clientX: 0, clientY: 0 });
    fireEvent.mouseMove(canvas, { clientX: 200, clientY: 120 });
    fireEvent.mouseUp(canvas, { button: 0 });
    const draggedLeft = wsCard.style.left;
    expect(draggedLeft).not.toBe('336px'); // the drag actually moved it

    // Rename the workspace through its card.
    fireEvent.click(within(wsCard).getByRole('button', { name: 'topology-workspace-rename-label' }));
    const input = within(wsCard).getByLabelText('topology-workspace-rename-placeholder');
    fireEvent.change(input, { target: { value: 'POS #1 (Renamed)' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    // The rename propagated to the card title…
    await waitFor(() => expect(within(wsCard).getByText('POS #1 (Renamed)')).toBeTruthy());
    // …and the unsaved drag survived the instances refresh.
    await waitFor(() => expect(wsCard.style.left).toBe(draggedLeft));
  });

  it('takes the full reload path when instances AND branch locations change together', async () => {
    // The light-merge guard must only intercept branchLocations-only
    // changes: when instances change in the same batch, the full rebuild
    // (authoritative instances win) still runs.
    renderWithProvidersSync(<BothChangeHarness />, multiStoreFtl, sharedFtl);
    await waitFor(() => expect(screen.getByText('Downtown Branch')).toBeInTheDocument());

    fireEvent.click(screen.getByText('both-change'));

    await waitFor(() => expect(screen.getByText('Both POS')).toBeInTheDocument());
    expect(screen.getByText('Both Branch')).toBeInTheDocument();
    expect(screen.queryByText('Downtown Branch')).not.toBeInTheDocument();
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

  it('cycles wire direction through one-way → reverse → two-way on click', () => {
    renderEditor();

    // The whole wire is the affordance: clicking the hitbox cycles the
    // flow. Retail preset wires start one-way.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(path().getAttribute('data-direction')).toBe('one-way');

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('reverse');

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('two-way');

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('one-way');
  });

  it('renders no visible label pills — the label is a hover tooltip only', async () => {
    // Wire labels are presentation chrome: they surface as a native SVG
    // tooltip on hover, never as a permanent canvas pill.
    mockLoadTopology.mockResolvedValueOnce({
      nodes: [
        { id: 'store-1', type: 'store', name: 'S', x: 80, y: 80 },
        { id: 'ws-1', type: 'workspace', name: 'W', x: 380, y: 80, metadata: { typeKey: 'store-pos' } },
      ],
      wires: [
        { id: 'w-unlabeled', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way' },
        { id: 'w-labeled', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way', label: 'Binds Store' },
      ],
    } as never);
    renderEditor();

    // The load effect resolves async — wait for the two mocked wires first.
    await waitFor(() => expect(document.querySelectorAll('.wire-path')).toHaveLength(2));
    // No label group elements anywhere — the old pill chrome is gone.
    expect(document.querySelectorAll('.wire-label-group')).toHaveLength(0);
    // The labeled wire's hitbox carries the label as its native tooltip.
    const titles = [...document.querySelectorAll('.wire-hitbox title')];
    expect(titles).toHaveLength(2);
    expect(titles.some((t) => t.textContent?.includes('Binds Store'))).toBe(true);
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

  it('confirms on preset when Undo diverges from the last Apply, but not when Redo restores it exactly', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    // Build a 5-node canvas (preset 3 + A + B), saving after each add so
    // both additions are persisted and the canvas is clean afterwards.
    fireEvent.click(screen.getByText('+ Store Node')); // node A → 4
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByText('+ Store Node')); // node B → 5
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(2));

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    // Undo drops node B — the canvas now DIVERGES from the saved 5-node
    // state, so a preset load must re-confirm instead of silently
    // discarding the undone-to canvas.
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(4);

    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThanOrEqual(1);

    // Cancel the dialog — the undone-to canvas must survive.
    fireEvent.keyDown(canvas, { key: 'Escape' });
    expect(getNodeCount()).toBe(4);

    // Redo re-applies node B — the canvas is now EXACTLY the last applied
    // 5-node state, so exact tracking must load the preset directly with
    // NO confirm (the conservative boolean over-approximation would have
    // shown a spurious dialog here).
    fireEvent.keyDown(canvas, { key: 'y', ctrlKey: true });
    expect(getNodeCount()).toBe(5);
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
  });

  it('does not confirm when Undo returns the canvas to the last loaded preset', () => {
    renderEditor();

    // Loading the same preset is not an edit — it loads directly.
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();

    // Undo restores the IDENTICAL retail canvas — it still matches the last
    // preset load, so it is NOT dirty. Exact tracking must not confirm here
    // (the conservative isDirtyRef over-approximation did, spuriously).
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(3);

    // Clicking the preset again must load directly — no "Load Preset" dialog.
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
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

  it('loads a preset directly after an Apply with idMap remapping (snapshot holds remapped ids)', async () => {
    const onSave = vi.fn().mockResolvedValue({ 'ws-1': 'ws-remapped-id' });
    renderEditor({ onSave });

    await waitFor(() => {
      expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    });

    // Apply with a non-empty idMap — every workspace node id changes on
    // screen via the client-side remap.
    fireEvent.click(screen.getByText('Apply Topology Changes'));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    // The applied snapshot must contain the REMAPPED ids (the exact arrays
    // set on the canvas) — so a preset click right after the save loads
    // directly with no spurious confirm, even though ids changed on screen.
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
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

  it('normalizes a corrupt wire direction at load instead of keeping it verbatim', async () => {
    // The load path must apply the same closed-union discipline as the
    // semantic contract: a garbage direction from the backend (manual
    // edit, stale JSON) must fold to a legal value in the EDITOR MODEL,
    // not survive verbatim — otherwise it renders with wrong markers and
    // round-trips back to the backend on the next Apply.
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Store', x: 100, y: 100 },
        { id: 'ws-1', type: 'workspace', name: 'POS', x: 300, y: 100 },
      ],
      wires: [{ id: 'w-bad', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'bidirectional' }],
    });

    renderEditor();

    await waitFor(() => {
      expect(getWireCount()).toBe(1);
    });

    // data-direction is the live render contract: it must be a legal
    // WireDirection, not the corrupt stored value.
    const wirePath = document.querySelector('.wire-path') as SVGPathElement;
    expect(wirePath.getAttribute('data-direction')).toBe('one-way');
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

    const select = typeSelect();
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

  it('preserves a still-valid wire selection when undoing a direction cycle', () => {
    renderEditor();

    // Click the first retail wire via its hitbox — selects AND cycles to
    // reverse in one click (the whole wire is the affordance now).
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    expect(hitbox).not.toBeNull();
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    fireEvent.click(hitbox);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
    expect(path().getAttribute('data-direction')).toBe('reverse');

    // Cycle again (pushes another undo entry), then undo twice: the first
    // undo restores reverse, the second restores one-way.
    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('two-way');
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(path().getAttribute('data-direction')).toBe('reverse');
    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));
    expect(path().getAttribute('data-direction')).toBe('one-way');

    // Direction restored and the wire still exists — selection preserved.
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
  });

  it('clears a wire selection when a preset load removes the selected wire', () => {
    renderEditor();

    // Restaurant preset has 4 wires (w-1..w-4); retail has only 2 (w-1, w-2).
    // Select w-3 — it exists only in the restaurant preset. (Clicking a
    // wire also cycles its direction, which dirties the canvas — the next
    // preset click therefore asks for confirmation, which the test accepts.)
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    const hitboxes = document.querySelectorAll('.wire-hitbox');
    expect(hitboxes.length).toBe(4);
    fireEvent.click(hitboxes[2]!);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    // Clicking Retail Preset confirms replacing the (now-dirty) canvas.
    fireEvent.click(screen.getByText('Retail Preset'));
    const confirm = screen.getAllByText('Load Preset').find((el) => el.tagName === 'BUTTON');
    fireEvent.click(confirm!);

    // w-3 no longer exists, so its selection must not dangle at a removed wire.
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

    // Retail preset has only w-1/w-2 — the wire selection is dropped. The
    // click dirties the canvas (direction cycled), so confirm the load.
    fireEvent.click(screen.getByText('Retail Preset'));
    const confirm = screen.getAllByText('Load Preset').find((el) => el.tagName === 'BUTTON');
    fireEvent.click(confirm!);

    expect(
      screen.getByText('The selected element is not part of this preset and was deselected.'),
    ).toBeInTheDocument();
  });

// ── Wire creation via port connections ──────────────────────────

describe('NodeTopologyEditor — wire creation', () => {
  it('creates a wire when two ports on different nodes are connected', () => {
    renderEditor();
    const baseline = getWireCount();

    // warehouse output → workspace input (not an existing connection).
    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));

    expect(getWireCount()).toBe(baseline + 1);
  });

  it('rejects a duplicate connection with a toast and no new wire', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);

    // Same two ports again — duplicate.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(screen.getByText('A wire already connects these ports.')).toBeInTheDocument();
  });

  it('cancels the connection when clicking the same node again', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(0), 'right'));
    fireEvent.click(portOf(nodeAt(0), 'right')); // same node → cancel

    expect(getWireCount()).toBe(baseline);
  });

  it('undoes a created wire in a single undo step', () => {
    renderEditor();
    const baseline = getWireCount();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);

    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getWireCount()).toBe(baseline);
  });

  it('blocks a second workspace→warehouse fallback wire on the standard tier', () => {
    renderEditor();
    const baseline = getWireCount();

    // Add a second workspace, then connect it to the existing warehouse.
    fireEvent.click(screen.getByText('+ Workspace Node'));
    fireEvent.click(portOf(nodeAt(3), 'right'));
    fireEvent.click(portOf(nodeAt(2), 'left'));

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
    // Inputs cannot start a connection in the left/right UX.
    fireEvent.click(portOf(nodeAt(1), 'left'));

    expect(getWireCount()).toBe(baseline);
    expect(screen.getByText('Input connectors receive connections; choose an output connector first.')).toBeInTheDocument();
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

// ── Canvas shortcuts vs focused chrome controls ─────────────────

describe('NodeTopologyEditor — canvas shortcuts vs focused chrome', () => {
  it('does not delete the canvas when a tool-rack button has keyboard focus', () => {
    renderEditor();

    // Clicking '+ Store Node' adds AND selects the new node; the button
    // keeps keyboard focus after the click (browser behavior). A stray
    // Delete/Backspace must NOT instantly delete the just-added node via
    // the immediate-delete (no-wires) path.
    const addBtn = screen.getByText('+ Store Node');
    fireEvent.click(addBtn);
    expect(getNodeCount()).toBe(4);

    addBtn.focus();
    fireEvent.keyDown(addBtn, { key: 'Delete' });

    expect(getNodeCount()).toBe(4);
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
  });

  it('keeps arrow-nudge inert while a tool-card has focus', () => {
    renderEditor();

    // Select a node WITHOUT an edit — a plain selection pushes no history,
    // so the Undo button's absence after the arrow key proves no nudge
    // (the nudge path would pushHistory).
    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    fireEvent.mouseDown(firstNode, { button: 0 });
    fireEvent.mouseUp(firstNode);
    expect(document.querySelector('.node-selected')).not.toBeNull();
    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();

    const toolCard = screen.getByText('+ Store Node');
    toolCard.focus();
    fireEvent.keyDown(toolCard, { key: 'ArrowDown' });

    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();
  });

  it('keeps arrow-nudge and Escape inert while a header button has focus', () => {
    renderEditor();

    // Select a node so the nudge/selection paths would otherwise fire.
    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    fireEvent.mouseDown(firstNode, { button: 0 });
    fireEvent.mouseUp(firstNode); // end the drag cleanly (no ghost drag)
    expect(document.querySelector('.node-selected')).not.toBeNull();

    const simBtn = screen.getByText('Test Order Simulation');
    simBtn.focus();

    // Arrow keys must not nudge the canvas (no history entry → no Undo).
    fireEvent.keyDown(simBtn, { key: 'ArrowDown' });
    expect(screen.queryByText('Undo (Ctrl+Z)')).not.toBeInTheDocument();

    // Escape must not clear the selection under the focused control.
    fireEvent.keyDown(simBtn, { key: 'Escape' });
    expect(document.querySelector('.node-selected')).not.toBeNull();
  });

  it('still deletes via Delete when a canvas node card itself has focus', () => {
    renderEditor();

    // The guard is chrome-scoped: canvas-internal elements (node cards,
    // ports, wire labels) keep their keyboard shortcuts.
    fireEvent.click(screen.getByText('+ Store Node'));
    const count = getNodeCount();
    const addedNode = document.querySelectorAll('.topology-node')[count - 1] as HTMLElement;
    addedNode.focus();
    fireEvent.keyDown(addedNode, { key: 'Delete' });

    expect(getNodeCount()).toBe(count - 1);
  });

  it('does not open the delete dialog when Apply has focus and a wired node is selected', () => {
    renderEditor();

    // Select a WIRED node (store-1 has preset wires) — without the chrome
    // guard, Delete on the focused Apply button would hit the hasWires
    // branch and open the 'Delete Node' confirm dialog.
    const firstNode = document.querySelector('.topology-node') as HTMLElement;
    fireEvent.mouseDown(firstNode, { button: 0 });
    fireEvent.mouseUp(firstNode);
    expect(document.querySelector('.node-selected')).not.toBeNull();

    const applyBtn = screen.getByText('Apply Topology Changes');
    applyBtn.focus();
    fireEvent.keyDown(applyBtn, { key: 'Delete' });

    // Chrome owns the keyboard while focused: no confirm dialog, and the
    // selection survives (the button itself is untouched).
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
    expect(document.querySelector('.node-selected')).not.toBeNull();
    expect(getNodeCount()).toBe(3);
  });

  it('does not delete the fresh node via Backspace when a tool-card keeps focus', () => {
    renderEditor();

    // Backspace shares the Delete branch — the guard must cover it too, or
    // a stray Backspace instantly deletes the just-added (no-wires) node.
    const addBtn = screen.getByText('+ Store Node');
    fireEvent.click(addBtn);
    expect(getNodeCount()).toBe(4);

    addBtn.focus();
    fireEvent.keyDown(addBtn, { key: 'Backspace' });

    expect(getNodeCount()).toBe(4);
    expect(screen.queryByText('Delete Node')).not.toBeInTheDocument();
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

// ── Wire deletion vs an in-flight connection ───────────────────
// Deleting a wire is a single-wire mutation (like toggling direction), so
// it must NOT cancel a connection in flight — the source node and every
// port the pending connection references stay valid. The one exception:
// if the deleted wire is the EXACT duplicate pair the connection would
// create, the pending state must be cancelled — otherwise completing the
// connection after the delete silently recreates the wire the user just
// removed, bypassing the duplicate detector.

describe('NodeTopologyEditor — wire deletion keeps an in-flight connection', () => {
  it('deleting an unrelated wire keeps the connection in flight and it completes', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from the warehouse output.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(nodeAt(2).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    // Select and delete w-2 (workspace right -> warehouse left) — unrelated
    // to the store-1 -> ws-1 connection being built.
    const hitboxes = document.querySelectorAll('.wire-hitbox');
    fireEvent.click(hitboxes[1] as Element);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Delete' });
    expect(screen.getByText('Delete Wire')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Delete'));
    expect(getWireCount()).toBe(baseline - 1);

    // The connection SURVIVED the unrelated wire delete.
    expect(nodeAt(2).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    // And it still completes normally.
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline);
  });

  it('deleting the exact duplicate pair cancels the in-flight connection', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection that is an EXACT duplicate of w-1 (store-1 right
    // -> ws-1 left — the same endpoints AND the same normalized ports).
    fireEvent.click(portOf(nodeAt(0), 'right'));
    expect(nodeAt(0).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    // Delete w-1 mid-connection — this is the wire the connection would
    // recreate. The pending state must be cancelled, not left dangling to
    // silently duplicate the deleted wire on completion.
    const hitboxes = document.querySelectorAll('.wire-hitbox');
    fireEvent.click(hitboxes[0] as Element);
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Delete' });
    expect(screen.getByText('Delete Wire')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Delete'));
    expect(getWireCount()).toBe(baseline - 1);

    // Completing the connection must NOT recreate the deleted wire — it
    // should behave as a cancelled in-flight connection (no new wire).
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline - 1);
  });

  it('deleting the duplicate pair cancels a connection started from the REVERSED endpoint', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start the connection from the output side of w-2 (workspace right ->
    // warehouse left). Inputs cannot start connections in the UX.
    fireEvent.click(portOf(nodeAt(1), 'right'));
    expect(nodeAt(1).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    const hitboxes = document.querySelectorAll('.wire-hitbox');
    fireEvent.click(hitboxes[1] as Element); // w-2: workspace right <-> warehouse left
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Delete' });
    expect(screen.getByText('Delete Wire')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Delete'));
    expect(getWireCount()).toBe(baseline - 1);

    // The pending state was cancelled — completing cannot recreate w-2.
    fireEvent.click(portOf(nodeAt(2), 'left'));
    expect(getWireCount()).toBe(baseline - 1);
  });
});

// ── Escape connection-cancel flow ───────────────────────────────

describe('NodeTopologyEditor — Escape connection-cancel flow', () => {
  it('Escape cancels an in-flight connection before it completes', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from store-1's bottom port — a ghost preview line appears.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(previewLine()).not.toBeNull();

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'Escape' });

    // Preview gone, and clicking a target port on another node does NOT
    // complete a wire — the connection was cancelled, not left dangling.
    expect(previewLine()).toBeNull();
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline);
  });

  it('Escape during a connection also clears the current selection', () => {
    renderEditor();

    selectFirstNode();
    expect(document.querySelector('.topology-node.node-selected')).not.toBeNull();

    fireEvent.click(portOf(nodeAt(2), 'right'));
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
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(previewLine()).not.toBeNull();

    const nameInput = document.querySelector(
      '.inspector-field input[type="text"]',
    ) as HTMLInputElement;
    expect(nameInput).not.toBeNull();
    nameInput.focus();
    fireEvent.keyDown(nameInput, { key: 'Escape' });

    // The connection is still in flight — completing it creates the wire.
    expect(previewLine()).not.toBeNull();
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);
  });
});

// ── Pro-tier warehouse fallback wire label ──────────────────────

describe('NodeTopologyEditor — Pro-tier warehouse fallback label', () => {
  it('allows a second workspace→warehouse wire with the fallback label on Pro', () => {
    renderEditor({ currentTier: 'pro' });
    const baseline = getWireCount();

    // Add a second warehouse, then connect the workspace to it. The retail
    // preset already has one warehouse wire, so this is fallback priority 2.
    fireEvent.click(screen.getByText('+ Warehouse Node'));
    fireEvent.click(portOf(nodeAt(1), 'right'));
    fireEvent.click(portOf(nodeAt(3), 'left'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(
      screen.queryByText(
        'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
      ),
    ).not.toBeInTheDocument();

    // The new wire carries the fallback label — surfaced as its hover
    // tooltip (raw key in the identity-l10n mock), never as a canvas pill.
    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    const title = last.querySelector('.wire-hitbox title');
    expect(title?.textContent).toContain('topology-wire-label-fallback');
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
    fireEvent.click(portOf(nodeAt(1), 'right'));
    fireEvent.click(portOf(nodeAt(2), 'left'));

    expect(getWireCount()).toBe(baseline + 1);
    expect(
      screen.queryByText(
        'Multi-warehouse stock deduction fallback wires require a Pro Tier license.',
      ),
    ).not.toBeInTheDocument();

    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    const title = last.querySelector('.wire-hitbox title');
    expect(title?.textContent).toContain('topology-wire-label-stock-deduct');
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

// ── Simulation pulse vs canvas mutations ────────────────────────
//
// Contract (pinned): the pulse ALWAYS reflects the current canvas — a
// fresh node adds no pulse (it has no wires), an undone wire's pulse
// vanishes with it, and a PRESET LOAD STOPS the simulation entirely
// (canvas replacement resets transient editor state, exactly like it
// cancels in-flight connections — a pulse animating a "test order" on a
// topology it was never run against would be misleading). The 30ms
// interval must never leak: stop and unmount both clear it.

describe('NodeTopologyEditor — simulation pulse vs canvas mutations', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  const pulseCount = () => document.querySelectorAll('.wire-simulation-pulse').length;

  it('a fresh node add during simulation adds no pulse and keeps the tick running', () => {
    vi.useFakeTimers();
    renderEditor();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(pulseCount()).toBe(2); // retail preset wires

    fireEvent.click(screen.getByText('+ Store Node'));
    expect(getNodeCount()).toBe(4);
    // The new node has no wires — it must not spawn a stale pulse.
    expect(pulseCount()).toBe(2);

    // The interval keeps ticking on the remaining wires.
    const cxBefore = document.querySelector('.wire-simulation-pulse')!.getAttribute('cx');
    act(() => {
      vi.advanceTimersByTime(30);
    });
    expect(document.querySelector('.wire-simulation-pulse')!.getAttribute('cx')).not.toBe(cxBefore);
  });

  it('undoing a wire during simulation removes its pulse and keeps the rest animating', () => {
    vi.useFakeTimers();
    renderEditor();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(pulseCount()).toBe(2);

    // Create a third wire, then undo it mid-simulation.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(3);
    expect(pulseCount()).toBe(3);

    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getWireCount()).toBe(2);
    // The undone wire's pulse is gone — no stale pulse on dead geometry.
    expect(pulseCount()).toBe(2);

    // Remaining wires still animate.
    const cxBefore = document.querySelector('.wire-simulation-pulse')!.getAttribute('cx');
    act(() => {
      vi.advanceTimersByTime(30);
    });
    expect(document.querySelector('.wire-simulation-pulse')!.getAttribute('cx')).not.toBe(cxBefore);
  });

  it('loading a preset stops the simulation: pulse gone, interval cleared', () => {
    // Scoped timers keep getTimerCount() to real timers only — the default
    // also fakes queueMicrotask/nextTick, so a stray promise resolution could
    // shift the baseline between the delta assertions.
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout'] });
    renderEditor();
    const timerBaseline = vi.getTimerCount();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(pulseCount()).toBeGreaterThan(0);
    // Exactly one real timer was added: the 30ms interval.
    expect(vi.getTimerCount()).toBe(timerBaseline + 1);

    // Canvas-replacement rule: a preset replaces the topology, so the
    // transient simulation state must reset — pulse gone, interval cleared.
    fireEvent.click(screen.getByText('Resto & KDS Preset'));
    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(document.querySelector('.wire-simulation-pulse')).toBeNull();
    // The sim button shows the START label — the simulation stopped.
    expect(screen.getByText('Test Order Simulation')).toBeInTheDocument();
    expect(vi.getTimerCount()).toBe(timerBaseline);
  });

  it('never leaks the 30ms interval: stop and unmount both clear it', () => {
    // Assert DELTAS, not absolute counts: the provider stack (toast,
    // workspace, React scheduling) arms unrelated timers, and vitest's
    // default fake timers also fake queueMicrotask/nextTick — so only the
    // interval added by starting the simulation is attributable.
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout'] });
    const { unmount } = renderEditor();
    const timerBaseline = vi.getTimerCount();

    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(vi.getTimerCount()).toBe(timerBaseline + 1); // exactly one interval

    fireEvent.click(screen.getByText('Stop Simulation'));
    expect(vi.getTimerCount()).toBe(timerBaseline); // interval cleared

    // Restart and unmount while running — the effect cleanup must clear it.
    fireEvent.click(screen.getByText('Test Order Simulation'));
    expect(vi.getTimerCount()).toBe(timerBaseline + 1);
    unmount();
    // Teardown also clears component-owned timers that were part of the
    // baseline, so assert only that the running interval is definitely gone
    // (count strictly below start-time count), not an exact post-teardown
    // number.
    expect(vi.getTimerCount()).toBeLessThan(timerBaseline + 1);
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
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    fireEvent.click(screen.getByText('Apply Topology Changes'));

    // The failure surfaces as an error toast (plainErrorMessage sanitizes the
    // thrown Error to the generic fallback, so pin the save-error key itself).
    await waitFor(() =>
      expect(screen.getByText(/topology-toast-save-error/)).toBeInTheDocument(),
    );

    // The in-memory edit survives the failed save.
    expect(getNodeCount()).toBe(countAfterEdit);

    // Still dirty WHILE the edit is present: a preset click asks about
    // unsaved changes (confirm dialog title + the unsaved-changes message
    // body are both rendered).
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.getAllByText('Load Preset').length).toBeGreaterThan(0);
    expect(
      screen.getByText(/Loading a preset will replace your current topology/),
    ).toBeInTheDocument();
    fireEvent.keyDown(canvas, { key: 'Escape' });
    expect(getNodeCount()).toBe(countAfterEdit);

    // Undo still works — the pre-save history entry was not cleared.
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(getNodeCount()).toBe(countAfterEdit - 1);

    // The undone-to canvas equals the last applied state (the failed save
    // never updated the applied snapshot), so exact tracking loads the
    // preset directly — NO spurious confirm.
    fireEvent.click(screen.getByText('Retail Preset'));
    expect(screen.queryByText('Load Preset')).not.toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
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

// ── Wire click cycles direction ─────────────────────────────────

describe('NodeTopologyEditor — wire click direction cycle', () => {
  it('a single click on the wire selects it and cycles the direction', () => {
    renderEditor();

    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(path().getAttribute('data-direction')).toBe('one-way');

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('reverse');
    // The click also selects the wire (Delete button appears).
    expect(screen.getByText('Delete Selected Element')).toBeInTheDocument();

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('two-way');

    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('one-way');
  });

  it('cycles wire direction with Enter and Space (keyboard parity)', () => {
    renderEditor();

    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(path().getAttribute('data-direction')).toBe('one-way');

    fireEvent.keyDown(hitbox, { key: 'Enter' });
    expect(path().getAttribute('data-direction')).toBe('reverse');

    fireEvent.keyDown(hitbox, { key: ' ' });
    expect(path().getAttribute('data-direction')).toBe('two-way');
  });
});

// ── Hover-target preview snap ───────────────────────────────────

describe('NodeTopologyEditor — hover-target preview snap', () => {
  it('snaps the in-flight preview to a port when hovering near it', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;

    // Start a connection from warehouse's output.
    fireEvent.click(portOf(nodeAt(2), 'right'));

    // Move to ws-1's left input port: the UX exposes only left/right
    // connectors, so the preview should snap to the labeled Location In.
    const wsX = parseFloat(nodeAt(1).style.left);
    const wsY = parseFloat(nodeAt(1).style.top);
    const targetX = wsX;
    const targetY = wsY + NODE_HEIGHT - 16;
    fireEvent.mouseMove(canvas, { clientX: targetX, clientY: targetY });

    const preview = document.querySelector(
      'path.wire-path[opacity="0.5"]',
    ) as SVGPathElement | null;
    expect(preview).not.toBeNull();
    const d = preview!.getAttribute('d')!;
    const nums = d.match(/-?\d+(\.\d+)?/g)!.map(Number);
    const endX = nums[nums.length - 2]!;
    const endY = nums[nums.length - 1]!;
    // The preview endpoint snapped to the ws-1 left input port.
    expect(endX).toBeCloseTo(targetX, 1);
    expect(endY).toBeCloseTo(targetY, 1);
  });
});

// ── Wire arrow markers ──────────────────────────────────────────

describe('NodeTopologyEditor — wire arrow markers', () => {
  it('renders direction markers for one-way, reverse, and two-way wires', () => {
    renderEditor();

    // Retail preset wires are one-way: end marker only, no start marker.
    const oneWayPath = document.querySelector('path.wire-path.one-way');
    expect(oneWayPath).not.toBeNull();
    expect(oneWayPath!.getAttribute('marker-start')).toBeNull();
    expect(oneWayPath!.getAttribute('marker-end')).toBe('url(#arrow-end)');

    // Click the first wire: one-way → reverse. Reverse renders only the
    // START marker (pointing back along the path) and no end marker.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    fireEvent.click(hitbox);
    const reversePath = hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(reversePath.getAttribute('data-direction')).toBe('reverse');
    expect(reversePath.getAttribute('marker-end')).toBeNull();
    expect(reversePath.getAttribute('marker-start')).toBe('url(#arrow-start)');

    // reverse → two-way: both markers render.
    fireEvent.click(hitbox);
    const twoWayPath = hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(twoWayPath.getAttribute('data-direction')).toBe('two-way');
    expect(twoWayPath.getAttribute('marker-start')).toBe('url(#arrow-start)');
    expect(twoWayPath.getAttribute('marker-end')).toBe('url(#arrow-end)');

    // The other one-way wire is untouched.
    expect(document.querySelectorAll('path.wire-path.one-way').length).toBe(1);
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

describe('NodeTopologyEditor — direction cycle undo/redo', () => {
  it('undo restores the previous wire direction and redo re-applies it', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;

    // one-way → reverse: the cycle pushes one history entry.
    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('reverse');

    // One undo returns to one-way; one redo re-applies reverse.
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(path().getAttribute('data-direction')).toBe('one-way');
    fireEvent.keyDown(canvas, { key: 'y', ctrlKey: true });
    expect(path().getAttribute('data-direction')).toBe('reverse');
  });
});

// ── Connected label on regular wires ────────────────────────────

describe('NodeTopologyEditor — connected wire label', () => {
  it('labels a regular store→workspace wire as connected', () => {
    renderEditor();
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(2), 'right'));
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);

    // Non-warehouse wires carry the plain connected label (raw identity key)
    // — surfaced as the wire's hover tooltip.
    const wires = document.querySelectorAll('.wire-group');
    const last = wires[wires.length - 1]!;
    const title = last.querySelector('.wire-hitbox title');
    expect(title?.textContent).toContain('topology-wire-label-connected');
  });
});

// ── Preset load cancels in-flight connection ────────────────────

describe('NodeTopologyEditor — preset load cancels in-flight connection', () => {
  it('cancels an in-flight connection when a preset is loaded', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from warehouse's output — ghost preview appears.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(previewLine()).not.toBeNull();

    // Load the SAME preset mid-connection (no edits yet, so it loads
    // directly without the unsaved-changes dialog).
    fireEvent.click(screen.getByText('Retail Preset'));

    // The canvas was replaced — the in-flight connection must be cancelled:
    // no ghost preview may survive the replacement...
    expect(previewLine()).toBeNull();
    // ...and a subsequent target-port click must start a NEW connection
    // instead of completing the stale one (no wire may be created).
    fireEvent.click(portOf(nodeAt(1), 'left'));
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

    fireEvent.click(portOf(nodeAt(0), 'right'));
    expect(previewLine()).not.toBeNull();

    // Trigger the workspaceInstances reload — the canvas is replaced.
    fireEvent.click(screen.getByText('reload-instances'));
    await waitFor(() => expect(previewLine()).toBeNull());

    // The stale connection cannot complete: a target click creates no wire.
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(0);
  });
});

// ── Wire-label toggle keeps an in-flight connection ─────────────
//
// Decision (pinned): a direction toggle is a SINGLE-WIRE mutation that
// leaves every node/port an in-flight connection references valid, so it
// must NOT cancel the connection — the codebase's rule is to cancel only
// when the CANVAS is replaced (preset load, instance reload), where a
// stale source node could mis-wire the new canvas. Cancelling on a toggle
// would destroy a deliberate two-step connection intent for an unrelated
// edit, and no other single-element interaction (node drag, selection
// click) cancels connections either. The toggle's history push is
// orthogonal: connection state is transient UI and is never captured in
// history, so neither the push nor its undo affects it.

describe('NodeTopologyEditor — wire click keeps an in-flight connection', () => {
  it('keeps the connection in flight across a direction cycle and completes it correctly', () => {
    renderEditor();
    const baseline = getWireCount();

    // Start a connection from warehouse's output — ghost preview + source highlight.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(previewLine()).not.toBeNull();
    expect(nodeAt(2).className).toContain('node-connecting-source');

    // Click the first wire (store right → workspace left): one-way → reverse.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(path().getAttribute('data-direction')).toBe('one-way');
    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('reverse');

    // The connection SURVIVED the cycle: source highlight + ghost preview intact.
    expect(nodeAt(2).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    // Completing the connection still creates the expected store→workspace
    // wire from the in-flight source — the cycle's history push did not
    // corrupt the pending state.
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);
    const wires = document.querySelectorAll('.wire-group');
    const created = wires[wires.length - 1]!;
    const title = created.querySelector('.wire-hitbox title');
    expect(title?.textContent).toContain('topology-wire-label-connected');
  });

  it('keeps the connection in flight when the cycle is undone (history push is orthogonal)', () => {
    renderEditor();
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    const baseline = getWireCount();

    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(previewLine()).not.toBeNull();

    // Cycle to reverse, then undo the cycle mid-connection.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    fireEvent.click(hitbox);
    expect(path().getAttribute('data-direction')).toBe('reverse');
    fireEvent.keyDown(canvas, { key: 'z', ctrlKey: true });
    expect(path().getAttribute('data-direction')).toBe('one-way');

    // The connection survived BOTH the cycle's history push and its undo.
    expect(nodeAt(2).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();

    // And it still completes normally afterwards.
    fireEvent.click(portOf(nodeAt(1), 'left'));
    expect(getWireCount()).toBe(baseline + 1);
  });

  it('wire click does not bubble a cancel to the canvas (stopPropagation contract)', () => {
    // The wire's onClick must never reach a canvas-level background handler
    // (e.g. a future background-click-cancels-connection listener). Without
    // stopPropagation the cycle click bubbles to a container onClick — where
    // a background-click cancel would wrongly kill the in-flight connection
    // the cycle is supposed to leave untouched. The spy is a REACT-level
    // onClick on a wrapper container: native listeners on intermediate
    // elements fire regardless of React's synthetic stopPropagation (React
    // 17+ delegates at the root), so only the delegation-level handler
    // discriminates the fix.
    let canvasClicked = false;
    const onCanvasClick = () => { canvasClicked = true; };
    // Native <button> as the wrapper: an interactive element that would
    // receive the bubbled wire click, standing in for a future
    // canvas-level background-click-cancels-connection handler.
    function Wrap() {
      return <button type="button" onClick={onCanvasClick}><NodeTopologyEditor currentTier="standard" /></button>;
    }
    renderWithProvidersSync(<Wrap />, multiStoreFtl, sharedFtl);

    // Start a connection from the warehouse output.
    fireEvent.click(portOf(nodeAt(2), 'right'));
    expect(nodeAt(2).className).toContain('node-connecting-source');

    // Click the first wire to cycle its direction.
    const hitbox = document.querySelector('.wire-hitbox') as Element;
    fireEvent.click(hitbox);
    const path = () => hitbox.parentElement!.querySelector('path.wire-path') as Element;
    expect(path().getAttribute('data-direction')).toBe('reverse');

    // The click must NOT have bubbled to the wrapper's React onClick — a
    // background-click cancel handler must never fire for a wire
    // interaction.
    expect(canvasClicked).toBe(false);

    // The user's contract: a background mousedown on the canvas AFTER the
    // wire click must not cancel the in-flight connection — the cycle's
    // click never reached (and cannot arm) a background-click cancel.
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    fireEvent.mouseDown(canvas, { button: 0 });
    expect(nodeAt(2).className).toContain('node-connecting-source');
    expect(previewLine()).not.toBeNull();
  });

  it('surfaces the label + cycle hint as the wire hover tooltip', () => {
    renderEditor();

    // Every wire's native tooltip carries its label plus the click-to-cycle
    // hint (raw FTL key in the identity-l10n mock).
    const titles = [...document.querySelectorAll('.wire-hitbox title')];
    expect(titles.length).toBeGreaterThan(0);
    for (const title of titles) {
      expect(title.textContent).toContain('topology-wire-toggle-hint');
    }
    // The first retail wire's label appears in its tooltip.
    expect(titles[0]!.textContent).toContain('Binds Store');

    // And the tooltip never renders as visible canvas chrome.
    expect(document.querySelector('.wire-label-group')).toBeNull();
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
