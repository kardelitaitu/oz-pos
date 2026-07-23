import { screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';
import { loadTopology, saveTopology } from '@/api/topology';

vi.mock('@/api/topology', () => ({
  loadTopology: vi.fn(),
  saveTopology: vi.fn(),
}));

const mockLoadTopology = vi.mocked(loadTopology);
const mockSaveTopology = vi.mocked(saveTopology);

const renderEditor = (props?: { onSave?: (nodes: unknown, wires: unknown) => void }) =>
  renderWithProvidersSync(<NodeTopologyEditor currentTier="standard" {...props} />);

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
});
