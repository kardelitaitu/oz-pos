import { screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';
import { loadTopology } from '@/api/topology';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/topology', () => ({
  loadTopology: vi.fn(),
  saveTopology: vi.fn(),
}));

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
  'topology-inspector-title': 'Node Inspector',
  'topology-inspector-node-name': 'Node Name',
  'topology-inspector-subtitle': 'Subtitle / Location',
  'topology-inspector-close-aria': 'Close inspector',
  'topology-inspector-section-coords': 'Coordinates',
  'workspace-type-selector-label': 'Workspace Type',
  'topology-ws-type-store-pos': 'Retail POS',
  'topology-ws-type-restaurant-pos': 'Restaurant POS',
  'topology-ws-type-kds': 'Kitchen Display (KDS)',
  'topology-ws-type-select-aria': 'Select workspace type',
  'topology-confirm-delete-node-title': 'Delete Node',
  'topology-confirm-delete-wire-title': 'Delete Wire',
  'topology-confirm-delete-node-msg':
    'This node has connected wires. Deleting it will remove all its wires too. This action cannot be undone.',
  'topology-confirm-delete-wire-msg': 'Delete this wire connection? This action cannot be undone.',
  'topology-confirm-delete-label': 'Delete',
  'topology-context-node-name': 'Node Name',
  'topology-context-subtitle': 'Subtitle / Location',
  'workspace-store-info-heading': 'Store Info',
  'workspace-store-info-name': 'Name',
  'workspace-store-info-address': 'Address',
  'workspace-store-info-branch': 'Branch',
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
      store: { name: 'Test Store', address: '123 Main St', taxId: '', currency: 'IDR', branch: 'Downtown' },
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

// ── Workspace card mocks — verify they render by checking for key text ──

const mockWorkspaceCardText: Record<string, string> = {
  'WorkspaceStorePosSettings': 'Receipt Settings',
  'WorkspaceRestaurantPosSettings': 'Restaurant Settings',
  'WorkspaceKdsSettings': 'KDS Settings',
  'WorkspaceInventorySettings': 'Inventory Settings',
  'StoreInfoCard': 'Store Info',
};

/** Stub workspace cards so tests don't need the full card dependency tree. */
vi.mock('@/features/settings/workspace-cards', async () => {
  const actual = await vi.importActual('@/features/settings/workspace-cards');
  return {
    ...actual,
    WorkspaceStorePosSettings: ({ variant }: { variant?: string }) =>
      <div data-testid="workspace-store-pos" data-variant={variant}>{mockWorkspaceCardText['WorkspaceStorePosSettings']}</div>,
    WorkspaceRestaurantPosSettings: ({ variant }: { variant?: string }) =>
      <div data-testid="workspace-restaurant-pos" data-variant={variant}>{mockWorkspaceCardText['WorkspaceRestaurantPosSettings']}</div>,
    WorkspaceKdsSettings: ({ variant }: { variant?: string }) =>
      <div data-testid="workspace-kds" data-variant={variant}>{mockWorkspaceCardText['WorkspaceKdsSettings']}</div>,
    WorkspaceInventorySettings: ({ variant }: { variant?: string }) =>
      <div data-testid="workspace-inventory" data-variant={variant}>{mockWorkspaceCardText['WorkspaceInventorySettings']}</div>,
    StoreInfoCard: ({ variant }: { variant?: string }) =>
      <div data-testid="store-info-card" data-variant={variant}>{mockWorkspaceCardText['StoreInfoCard']}</div>,
  };
});

const mockLoadTopology = vi.mocked(loadTopology);

const renderEditor = () =>
  renderWithProvidersSync(<NodeTopologyEditor currentTier="pro" />, multiStoreFtl, sharedFtl);

/** Click the first topology node of a given type to select it. */
const selectNodeByType = (type: string) => {
  const node = document.querySelector(`.node-type-${type}`);
  if (node) fireEvent.mouseDown(node as Element, { button: 0 });
};

describe('Inspector drawer integration (Phase 2)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLoadTopology.mockResolvedValue(null);
  });

  // ── P2-I3-1: Store node renders StoreInfoCard ──────────────

  it('selecting a store node renders StoreInfoCard', async () => {
    renderEditor();

    // Retail preset has a store node named 'Downtown Branch'
    selectNodeByType('store');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
      expect(screen.getByTestId('store-info-card')).toBeInTheDocument();
    });
  });

  // ── P2-I3-2: Workspace node (store-pos) renders correct card ──

  it('selecting a store-pos workspace node renders WorkspaceStorePosSettings', async () => {
    renderEditor();

    selectNodeByType('workspace');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
      expect(screen.getByTestId('workspace-store-pos')).toBeInTheDocument();
    });
  });

  // ── P2-I3-6: Escape deselects — inspector disappears ──────

  it('pressing Escape while inspector is open closes it', async () => {
    renderEditor();

    selectNodeByType('store');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
    });

    // Fire Escape on the canvas
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(canvas).not.toBeNull();
    fireEvent.keyDown(canvas!, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('Node Inspector')).not.toBeInTheDocument();
    });
  });

  // ── P2-I3-7: Workspace typeKey change switches card ──────

  it('changing workspace typeKey dropdown switches the rendered card', async () => {
    renderEditor();

    selectNodeByType('workspace');

    await waitFor(() => {
      expect(screen.getByTestId('workspace-store-pos')).toBeInTheDocument();
    });

    // Change the typeKey select to KDS
    const select = document.querySelector('.inspector-select') as HTMLSelectElement;
    expect(select).not.toBeNull();
    fireEvent.change(select!, { target: { value: 'kds' } });

    await waitFor(() => {
      expect(screen.getByTestId('workspace-kds')).toBeInTheDocument();
      // The old card should no longer be rendered
      expect(screen.queryByTestId('workspace-store-pos')).not.toBeInTheDocument();
    });

    // Change to restaurant-pos
    fireEvent.change(select!, { target: { value: 'restaurant-pos' } });

    await waitFor(() => {
      expect(screen.getByTestId('workspace-restaurant-pos')).toBeInTheDocument();
      expect(screen.queryByTestId('workspace-kds')).not.toBeInTheDocument();
    });
  });

  // ── P2-I3-4: Warehouse node renders InventorySettings ────

  it('selecting a warehouse node renders WorkspaceInventorySettings', async () => {
    renderEditor();

    selectNodeByType('warehouse');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
      expect(screen.getByTestId('workspace-inventory')).toBeInTheDocument();
    });
  });

  // ── P2-I3-5: Hardware node renders its own inspector card ──

  it('selecting a hardware node shows the hardware inspector with editable name/subtitle', async () => {
    renderEditor();

    // Add a hardware node so we can select it
    fireEvent.click(screen.getByText('+ Hardware Node'));

    await waitFor(() => {
      expect(screen.getByText('New Hardware')).toBeInTheDocument();
    });

    selectNodeByType('hardware');

    // The drawer opens with the hardware-specific card and the name prefilled.
    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
      expect(screen.getByTestId('hardware-inspector')).toBeInTheDocument();
      expect(screen.getByText('Hardware Device')).toBeInTheDocument();
    });

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput.value).toBe('New Hardware');

    // Renaming flows through the beginInspectorEdit session — one undo restores.
    fireEvent.change(nameInput, { target: { value: 'Kitchen Printer' } });
    expect(screen.getByText('Kitchen Printer')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Undo (Ctrl+Z)'));

    expect(
      (document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement).value,
    ).toBe('New Hardware');
  });

  // ── P2-I6: Ctrl+I focuses first inspector input ──────────

  it('Ctrl+I focuses the first input in the inspector drawer', async () => {
    renderEditor();

    selectNodeByType('store');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
      expect(screen.getByTestId('store-info-card')).toBeInTheDocument();
    });

    // Fire Ctrl+I on the canvas
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(canvas).not.toBeNull();
    fireEvent.keyDown(canvas!, { key: 'i', ctrlKey: true });

    // The first input in the inspector should now be focused
    const firstInput = document.querySelector('.inspector-content input[type="text"]') as HTMLInputElement;
    expect(firstInput).not.toBeNull();
    expect(document.activeElement).toBe(firstInput);
  });

  it('Ctrl+I does nothing when no node is selected', async () => {
    renderEditor();

    // No node selected — Ctrl+I should be a no-op
    const canvas = document.querySelector('.node-canvas-container') as HTMLElement;
    expect(canvas).not.toBeNull();
    fireEvent.keyDown(canvas!, { key: 'i', ctrlKey: true });

    // No crash, no inspector — no focus shift
    expect(screen.queryByText('Node Inspector')).not.toBeInTheDocument();
  });

  // ── Node name input updates on typing ─────────────────────

  it('typing in the Node Name input updates the node card name', async () => {
    renderEditor();

    selectNodeByType('store');

    await waitFor(() => {
      expect(screen.getByText('Node Inspector')).toBeInTheDocument();
    });

    const nameInput = document.querySelector('.inspector-field input[type="text"]') as HTMLInputElement;
    expect(nameInput).not.toBeNull();

    fireEvent.change(nameInput!, { target: { value: 'Renamed Store' } });

    // The node title on the canvas should update
    expect(screen.getByText('Renamed Store')).toBeInTheDocument();
  });
});
