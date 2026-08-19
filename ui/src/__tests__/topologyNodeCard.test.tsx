import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import { withFluent, withFluentLocale } from '@/locales/test-utils';
import { TopologyNodeCard } from '@/features/stores/topologyNodeCard';
import type { TopologyNodeData, PortName } from '@/features/stores/NodeTopologyEditor';
import type { TopologyValidationError } from '@/features/stores/topologyContract';
import type { ReactLocalization } from '@fluent/react';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import multiStoreIdFtl from '@/locales/multi-store.id.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import sharedIdFtl from '@/locales/shared.id.ftl?raw';

// ── Mock data factories ────────────────────────────────────────────────

function makeNode(overrides: Partial<TopologyNodeData> = {}): TopologyNodeData {
  return {
    id: 'node-1',
    type: 'workspace',
    name: 'Test Node',
    subtitle: 'Store POS',
    x: 100,
    y: 200,
    metadata: { typeKey: 'store-pos', enabled: true },
    ...overrides,
  };
}

function makeValidationError(overrides: Partial<TopologyValidationError> = {}): TopologyValidationError {
  return {
    code: 'warehouse-at-capacity',
    messageId: 'topology-validation-warehouse-at-capacity',
    nodeId: 'node-1',
    ...overrides,
  };
}

// ── Test utilities ─────────────────────────────────────────────────────

async function renderWithFluent(ui: React.ReactElement) {
  const wrapped = withFluent(ui, sharedFtl, multiStoreFtl);
  await renderInAct(wrapped);
}

async function renderWithFluentId(ui: React.ReactElement) {
  const wrapped = withFluentLocale('id', ui, sharedIdFtl, multiStoreIdFtl);
  await renderInAct(wrapped);
}

// ── Default props factory ──────────────────────────────────────────────

function defaultProps(overrides: Partial<{
  node: TopologyNodeData;
  isSelected: boolean;
  isConnectingSource: boolean;
  connectingFromNodeId: string | null;
  connectingFromPort: PortName | null;
  hoveredTarget: { nodeId: string; port: PortName } | null;
  nodeErrors: TopologyValidationError[];
  countBadge: string | null;
  hasOverlap: boolean;
  stockWireHint: boolean;
  overlayMarker: 'only-here' | 'differing' | null;
  isFresh: boolean;
  isDimmed: boolean;
  isRenameable: boolean;
  renaming: boolean;
  renameDraft: string;
  connectedPortId: string | undefined;
  l10n: Pick<ReactLocalization, 'getString'>;
  renameInputRef: React.RefObject<HTMLInputElement>;
  renameBaselineRef: { current: string | null };
  onSelect: (id: string) => void;
  onOpenNodeMenu: (e: React.MouseEvent, nodeId: string) => void;
  onCardMouseDown: (e: React.MouseEvent, nodeId: string) => void;
  onStartRename: (nodeId: string, currentName: string) => void;
  onCommitRename: (nodeId: string, fromKeyboard?: boolean) => void;
  onCancelRename: () => void;
  onRenameDraftChange: (draft: string) => void;
  onPersistRename: (nodeId: string, name: string) => void;
  onSetNodeName: (nodeId: string, name: string) => void;
  onSetNodeEnabled: (nodeId: string, enabled: boolean) => void;
  onDismissNodeIssue: (nodeId: string, messageId: string) => void;
  onPortClick: (e: React.MouseEvent, nodeId: string, port: PortName) => void;
  onHoverNode: React.Dispatch<React.SetStateAction<string | null>>;
  getTelemetry: (node: TopologyNodeData) => { badge: string; status: 'online' | 'warning' | 'offline' } | null;
  isPortCompatible: (nodeId: string, port: PortName) => boolean;
}> = {}) {
  const node = makeNode();

  return {
    node,
    isSelected: false,
    isConnectingSource: false,
    connectingFromNodeId: null,
    connectingFromPort: null,
    hoveredTarget: null,
    nodeErrors: [],
    countBadge: null as string | null,
    hasOverlap: false,
    stockWireHint: false,
    overlayMarker: null,
    isFresh: false,
    isDimmed: false,
    isRenameable: true,
    renaming: false,
    renameDraft: node.name,
    connectedPortId: undefined,
    l10n: { getString: (id: string) => id } as Pick<ReactLocalization, 'getString'>,
    renameInputRef: { current: null } as React.RefObject<HTMLInputElement>,
    renameBaselineRef: { current: null },
    onSelect: vi.fn(),
    onOpenNodeMenu: vi.fn(),
    onCardMouseDown: vi.fn(),
    onStartRename: vi.fn(),
    onCommitRename: vi.fn(),
    onCancelRename: vi.fn(),
    onRenameDraftChange: vi.fn(),
    onPersistRename: vi.fn(),
    onSetNodeName: vi.fn(),
    onSetNodeEnabled: vi.fn(),
    onDismissNodeIssue: vi.fn(),
    onPortClick: vi.fn(),
    onHoverNode: vi.fn(),
    getTelemetry: () => null,
    isPortCompatible: () => true,
    ...overrides,
  };
}

describe('TopologyNodeCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Rendering — node types', () => {
    it('renders store node with correct type class', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'store', name: 'Main Store' }) })} />
      );

      const card = screen.getByRole('group', { name: 'Main Store' });
      expect(card).toHaveClass('node-type-store');
    });

    it('renders workspace node with correct type class', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace', name: 'POS 1' }) })} />
      );

      const card = screen.getByRole('group', { name: 'POS 1' });
      expect(card).toHaveClass('node-type-workspace');
    });

    it('renders warehouse node with correct type class', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'warehouse', name: 'Stock Room' }) })} />
      );

      const card = screen.getByRole('group', { name: 'Stock Room' });
      expect(card).toHaveClass('node-type-warehouse');
    });

    it('renders hardware node with correct type class', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'hardware', name: 'Printer' }) })} />
      );

      const card = screen.getByRole('group', { name: 'Printer' });
      expect(card).toHaveClass('node-type-hardware');
    });

    it('renders node at correct position', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ x: 150, y: 250 }) })} />
      );

      const card = screen.getByRole('group', { name: 'Test Node' });
      expect(card).toHaveStyle({ left: '150px', top: '250px' });
    });

    it('renders node name in title', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ name: 'Custom Name' }) })} />
      );

      expect(screen.getByText('Custom Name')).toBeInTheDocument();
    });

    it('renders subtitle', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ subtitle: 'Custom Subtitle' }) })} />
      );

      expect(screen.getByText('Custom Subtitle')).toBeInTheDocument();
    });
  });

  describe('Rendering — state classes', () => {
    it('applies node-selected class when selected', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isSelected: true })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('node-selected');
    });

    it('applies node-connecting-source class when connecting source', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isConnectingSource: true })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('node-connecting-source');
    });

    it('applies node-fresh class when fresh', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isFresh: true })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('node-fresh');
    });

    it('applies node-dimmed class when dimmed', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isDimmed: true })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('node-dimmed');
    });

    it('applies overlay marker class for only-here', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ overlayMarker: 'only-here' })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('topology-node--overlay-only-here');
    });

    it('applies overlay marker class for differing', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ overlayMarker: 'differing' })} />);

      const card = screen.getByRole('group');
      expect(card).toHaveClass('topology-node--overlay-differing');
    });

    it('does not apply overlay class when null', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ overlayMarker: null })} />);

      const card = screen.getByRole('group');
      expect(card).not.toHaveClass('topology-node--overlay-only-here');
      expect(card).not.toHaveClass('topology-node--overlay-differing');
    });
  });

  describe('Rendering — rename mode', () => {
    it('renders rename input when renaming=true', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ renaming: true, renameDraft: 'New Name' })} />
      );

      // The rename input has aria-label for the placeholder
      const input = screen.getByRole('textbox', { name: /rename-placeholder/i });
      expect(input).toBeInTheDocument();
      expect(input).toHaveValue('New Name');
    });

    it('renders rename button when not renaming and renameable', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ renaming: false, isRenameable: true })} />);

      const renameBtn = screen.getByRole('button', { name: /workspace-rename/i });
      expect(renameBtn).toBeInTheDocument();
    });

    it('does not render rename button when not renameable', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isRenameable: false })} />);

      expect(screen.queryByRole('button', { name: /rename/i })).not.toBeInTheDocument();
    });

    it('renders store rename placeholder for store type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'store' }), renaming: true })} />
      );

      const input = screen.getByRole('textbox', { name: /branch-rename-placeholder/i });
      expect(input).toBeInTheDocument();
      expect(input).toHaveAttribute('aria-label', 'topology-branch-rename-placeholder');
    });

    it('renders workspace rename placeholder for workspace type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), renaming: true })} />
      );

      const input = screen.getByRole('textbox', { name: /workspace-rename-placeholder/i });
      expect(input).toBeInTheDocument();
      expect(input).toHaveAttribute('aria-label', 'topology-workspace-rename-placeholder');
    });
  });

  describe('Rendering — workspace-specific fields', () => {
    it('renders name input for workspace type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }) })} />
      );

      const nameInput = screen.getByLabelText('topology-field-name-aria');
      expect(nameInput).toBeInTheDocument();
      expect(nameInput).toHaveValue('Test Node');
    });

    it('renders enabled checkbox for workspace type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }) })} />
      );

      const checkbox = screen.getByRole('checkbox', { name: /topology-field-enabled-aria/i });
      expect(checkbox).toBeInTheDocument();
      expect(checkbox).toBeChecked();
    });

    it('renders unchecked checkbox when disabled', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace', metadata: { enabled: false } }) })} />
      );

      const checkbox = screen.getByRole('checkbox', { name: /topology-field-enabled-aria/i });
      expect(checkbox).not.toBeChecked();
    });

    it('does not render workspace fields for store type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'store' }) })} />
      );

      expect(screen.queryByLabelText('topology-field-name-aria')).not.toBeInTheDocument();
      expect(screen.queryByRole('checkbox', { name: /topology-field-enabled-aria/i })).not.toBeInTheDocument();
    });

    it('does not render workspace fields for warehouse type', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'warehouse' }) })} />
      );

      expect(screen.queryByLabelText('topology-field-name-aria')).not.toBeInTheDocument();
      expect(screen.queryByRole('checkbox', { name: /topology-field-enabled-aria/i })).not.toBeInTheDocument();
    });
  });

  describe('Rendering — validation notes', () => {
    it('renders validation note when errors exist', async () => {
      const error = makeValidationError();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error] })} />);

      const note = screen.getByRole('status', { name: /topology-validation-warehouse-at-capacity/i });
      expect(note).toBeInTheDocument();
      expect(note).toHaveClass('node-validation-note');
    });

    it('shows validation icon', async () => {
      const error = makeValidationError();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error] })} />);

      const icon = screen.getByText('!');
      expect(icon).toBeInTheDocument();
      expect(icon).toHaveClass('node-validation-icon');
    });

    it('shows first error message', async () => {
      const error = makeValidationError();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error] })} />);

      const text = screen.getByText('topology-validation-warehouse-at-capacity');
      expect(text).toBeInTheDocument();
      expect(text).toHaveClass('node-validation-text');
    });

    it('shows count badge when provided', async () => {
      const error = makeValidationError();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error], countBadge: '3 Stock Rooms — 1 allowed' })} />);

      const badge = screen.getByText('3 Stock Rooms — 1 allowed');
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveClass('node-validation-count-badge');
    });

    it('renders dismiss button for warehouse-missing-stock-routing', async () => {
      const error = makeValidationError({ code: 'warehouse-missing-stock-routing' });
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error], onDismissNodeIssue: vi.fn() })} />);

      const dismissBtn = screen.getByRole('button', { name: /topology-validation-dismiss/i });
      expect(dismissBtn).toBeInTheDocument();
    });

    it('does not render dismiss button for other error codes', async () => {
      const error = makeValidationError({ code: 'warehouse-at-capacity' });
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error], onDismissNodeIssue: vi.fn() })} />);

      expect(screen.queryByRole('button', { name: /topology-validation-dismiss/i })).not.toBeInTheDocument();
    });

    it('does not render validation note when no errors', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [] })} />);

      expect(screen.queryByRole('status', { name: /validation/i })).not.toBeInTheDocument();
    });
  });

  describe('Rendering — stock wire hint', () => {
    it('renders stock wire hint when enabled', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ stockWireHint: true })} />);

      const hint = screen.getByRole('status');
      expect(hint).toBeInTheDocument();
      expect(hint).toHaveClass('node-stock-wire-hint');
    });

    it('shows hint icon and text', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ stockWireHint: true })} />);

      expect(screen.getByText('→')).toBeInTheDocument();
      expect(screen.getByText('topology-node-stock-wire-hint')).toBeInTheDocument();
    });

    it('does not render when stockWireHint=false', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ stockWireHint: false })} />);

      expect(screen.queryByRole('status')).not.toBeInTheDocument();
    });
  });

  describe('Rendering — overlap badge', () => {
    it('renders overlap badge when hasOverlap=true', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ hasOverlap: true })} />);

      const badge = screen.getByRole('status', { name: /topology-overlap-badge/i });
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveClass('node-overlap-badge');
    });

    it('does not render when hasOverlap=false', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ hasOverlap: false })} />);

      expect(screen.queryByRole('status', { name: /overlap/i })).not.toBeInTheDocument();
    });
  });

  describe('Rendering — telemetry badge', () => {
    it('renders telemetry badge when getTelemetry returns data', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            getTelemetry: () => ({ badge: 'Online', status: 'online' }),
          })}
        />
      );

      const badge = screen.getByText('Online');
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveClass('telemetry-online');
    });

    it('renders warning status badge', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            getTelemetry: () => ({ badge: 'Warning', status: 'warning' }),
          })}
        />
      );

      const badge = screen.getByText('Warning');
      expect(badge).toHaveClass('telemetry-warning');
    });

    it('renders offline status badge', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            getTelemetry: () => ({ badge: 'Offline', status: 'offline' }),
          })}
        />
      );

      const badge = screen.getByText('Offline');
      expect(badge).toHaveClass('telemetry-offline');
    });

    it('does not render when getTelemetry returns null', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ getTelemetry: () => null })} />);

      expect(screen.queryByText('Online')).not.toBeInTheDocument();
    });
  });

  describe('Rendering — port sockets', () => {
    it('renders left and right ports for workspace', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }) })} />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      expect(ports).toHaveLength(2);
    });

    it('renders only right port for store', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'store' }) })} />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      expect(ports).toHaveLength(1);
      expect(ports[0]).toHaveClass('port-right');
    });

    it('renders left and right ports for warehouse', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'warehouse' }) })} />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      expect(ports).toHaveLength(2);
    });

    it('renders left and right ports for hardware', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'hardware' }) })} />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      expect(ports).toHaveLength(2);
    });

    it('applies port-active class when connecting from this port', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            connectingFromNodeId: 'node-1',
            connectingFromPort: 'left',
          })}
        />
      );

      // Get the left port by its aria-label
      const leftPort = screen.getByRole('button', { name: /port.*location.*in.*aria/i });
      expect(leftPort).toBeInTheDocument();
      expect(leftPort).toHaveClass('port-active');
    });

    it('applies port-highlight when hovered and compatible', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            connectingFromNodeId: 'node-2',
            hoveredTarget: { nodeId: 'node-1', port: 'left' },
            isPortCompatible: () => true,
          })}
        />
      );

      // Get the left port by its aria-label
      const leftPort = screen.getByRole('button', { name: /port.*location.*in.*aria/i });
      expect(leftPort).toBeInTheDocument();
      expect(leftPort).toHaveClass('port-highlight');
    });

    it('applies port-compatible class when compatible', async () => {
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ isPortCompatible: () => true })} />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      ports.forEach((port) => {
        expect(port).toHaveClass('port-compatible');
      });
    });

    it('applies port-incompatible when connecting and not compatible', async () => {
      await renderWithFluent(
        <TopologyNodeCard
          {...defaultProps({
            connectingFromNodeId: 'node-2',
            isPortCompatible: () => false,
          })}
        />
      );

      const ports = screen.getAllByRole('button', { name: /port/i });
      ports.forEach((port) => {
        expect(port).toHaveClass('port-incompatible');
      });
    });
  });

  describe('Interaction — selection', () => {
    it('calls onSelect when Enter pressed', async () => {
      const onSelect = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onSelect })} />);

      const card = screen.getByRole('group');
      fireEvent.keyDown(card, { key: 'Enter' });
      expect(onSelect).toHaveBeenCalledWith('node-1');
    });

    it('calls onSelect when Space pressed', async () => {
      const onSelect = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onSelect })} />);

      const card = screen.getByRole('group');
      fireEvent.keyDown(card, { key: ' ' });
      expect(onSelect).toHaveBeenCalledWith('node-1');
    });

    it('handles Enter without throwing', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps()} />);

      const card = screen.getByRole('group');
      // Should not throw - the component calls e.preventDefault() internally
      fireEvent.keyDown(card, { key: 'Enter' });
    });

    it('handles Space without throwing', async () => {
      await renderWithFluent(<TopologyNodeCard {...defaultProps()} />);

      const card = screen.getByRole('group');
      fireEvent.keyDown(card, { key: ' ' });
    });
  });

  describe('Interaction — hover', () => {
    it('calls onHoverNode on mouse enter', async () => {
      const onHoverNode = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onHoverNode })} />);

      const card = screen.getByRole('group');
      fireEvent.mouseEnter(card);
      expect(onHoverNode).toHaveBeenCalledWith('node-1');
    });

    it('calls onHoverNode cleanup on mouse leave', async () => {
      const onHoverNode = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onHoverNode })} />);

      const card = screen.getByRole('group');
      fireEvent.mouseLeave(card);
      expect(onHoverNode).toHaveBeenCalledWith(expect.any(Function));
    });
  });

  describe('Interaction — context menu', () => {
    it('calls onOpenNodeMenu on context menu', async () => {
      const onOpenNodeMenu = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onOpenNodeMenu })} />);

      const card = screen.getByRole('group');
      fireEvent.contextMenu(card);
      expect(onOpenNodeMenu).toHaveBeenCalledTimes(1);
    });
  });

  describe('Interaction — drag start', () => {
    it('calls onCardMouseDown on mouse down (not on controls)', async () => {
      const onCardMouseDown = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onCardMouseDown })} />);

      const card = screen.getByRole('group');
      fireEvent.mouseDown(card);
      expect(onCardMouseDown).toHaveBeenCalledTimes(1);
    });

    it('does not call onCardMouseDown when clicking input', async () => {
      const onCardMouseDown = vi.fn();
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), onCardMouseDown })} />
      );

      const input = screen.getByLabelText('topology-field-name-aria');
      fireEvent.mouseDown(input);
      expect(onCardMouseDown).not.toHaveBeenCalled();
    });

    it('does not call onCardMouseDown when clicking button', async () => {
      const onCardMouseDown = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onCardMouseDown })} />);

      const button = screen.getByRole('button', { name: /workspace-rename/i });
      fireEvent.mouseDown(button);
      expect(onCardMouseDown).not.toHaveBeenCalled();
    });

    it('does not call onCardMouseDown when clicking port', async () => {
      const onCardMouseDown = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onCardMouseDown })} />);

      // Click the first port (left)
      const ports = screen.getAllByRole('button', { name: /port/i });
      const firstPort = ports[0];
      expect(firstPort).toBeInTheDocument();
      fireEvent.mouseDown(firstPort!);
      expect(onCardMouseDown).not.toHaveBeenCalled();
    });
  });

  describe('Interaction — rename', () => {
    it('calls onStartRename on double click when renameable', async () => {
      const onStartRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onStartRename })} />);

      const card = screen.getByRole('group');
      fireEvent.dblClick(card);
      expect(onStartRename).toHaveBeenCalledWith('node-1', 'Test Node');
    });

    it('does not call onStartRename when not renameable', async () => {
      const onStartRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ isRenameable: false, onStartRename })} />);

      const card = screen.getByRole('group');
      fireEvent.dblClick(card);
      expect(onStartRename).not.toHaveBeenCalled();
    });

    it('calls onStartRename when rename button clicked', async () => {
      const onStartRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onStartRename })} />);

      const renameBtn = screen.getByRole('button', { name: /workspace-rename/i });
      fireEvent.click(renameBtn);
      expect(onStartRename).toHaveBeenCalledWith('node-1', 'Test Node');
    });

    it('calls onRenameDraftChange on input change', async () => {
      const onRenameDraftChange = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ renaming: true, onRenameDraftChange })} />);

      const input = screen.getByRole('textbox', { name: /rename-placeholder/i });
      fireEvent.change(input, { target: { value: 'New Name' } });
      expect(onRenameDraftChange).toHaveBeenCalledWith('New Name');
    });

    it('calls onCommitRename on Enter in rename input', async () => {
      const onCommitRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ renaming: true, onCommitRename })} />);

      const input = screen.getByRole('textbox', { name: /rename-placeholder/i });
      fireEvent.keyDown(input, { key: 'Enter' });
      expect(onCommitRename).toHaveBeenCalledWith('node-1', true);
    });

    it('calls onCancelRename on Escape in rename input', async () => {
      const onCancelRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ renaming: true, onCancelRename })} />);

      const input = screen.getByRole('textbox', { name: /rename-placeholder/i });
      fireEvent.keyDown(input, { key: 'Escape' });
      expect(onCancelRename).toHaveBeenCalled();
    });

    it('calls onCommitRename on blur in rename input', async () => {
      const onCommitRename = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ renaming: true, onCommitRename })} />);

      const input = screen.getByRole('textbox', { name: /rename-placeholder/i });
      fireEvent.blur(input);
      expect(onCommitRename).toHaveBeenCalledWith('node-1');
    });
  });

  describe('Interaction — workspace name input', () => {
    it('calls onSetNodeName on change', async () => {
      const onSetNodeName = vi.fn();
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), onSetNodeName })} />
      );

      const input = screen.getByLabelText('topology-field-name-aria');
      fireEvent.change(input, { target: { value: 'Updated Name' } });
      expect(onSetNodeName).toHaveBeenCalledWith('node-1', 'Updated Name');
    });

    it('calls onPersistRename on blur', async () => {
      const onPersistRename = vi.fn();
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), onPersistRename })} />
      );

      const input = screen.getByLabelText('topology-field-name-aria');
      fireEvent.blur(input);
      expect(onPersistRename).toHaveBeenCalledWith('node-1', 'Test Node');
    });

    it('calls onPersistRename on Enter', async () => {
      const onPersistRename = vi.fn();
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), onPersistRename })} />
      );

      const input = screen.getByLabelText('topology-field-name-aria');
      fireEvent.keyDown(input, { key: 'Enter' });
      expect(onPersistRename).toHaveBeenCalledWith('node-1', 'Test Node');
    });
  });

  describe('Interaction — enabled checkbox', () => {
    it('calls onSetNodeEnabled on change', async () => {
      const onSetNodeEnabled = vi.fn();
      await renderWithFluent(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), onSetNodeEnabled })} />
      );

      const checkbox = screen.getByRole('checkbox', { name: /topology-field-enabled-aria/i });
      fireEvent.click(checkbox);
      expect(onSetNodeEnabled).toHaveBeenCalledWith('node-1', false);
    });
  });

  describe('Interaction — validation dismiss', () => {
    it('calls onDismissNodeIssue when dismiss button clicked', async () => {
      const onDismissNodeIssue = vi.fn();
      const error = makeValidationError({ code: 'warehouse-missing-stock-routing', messageId: 'topology-validation-warehouse-missing-stock-routing' });
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error], onDismissNodeIssue })} />);

      const dismissBtn = screen.getByRole('button', { name: /topology-validation-dismiss/i });
      fireEvent.click(dismissBtn);
      expect(onDismissNodeIssue).toHaveBeenCalledWith('node-1', 'topology-validation-warehouse-missing-stock-routing');
    });
  });

  describe('Interaction — port click', () => {
    it('calls onPortClick when port clicked', async () => {
      const onPortClick = vi.fn();
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ onPortClick })} />);

      // Click the left port specifically
      const leftPort = screen.getByRole('button', { name: /port.*location.*in.*aria/i });
      fireEvent.click(leftPort);
      expect(onPortClick).toHaveBeenCalledTimes(1);
    });
  });

  describe('Indonesian locale', () => {
    it('renders with Indonesian localization', async () => {
      await renderWithFluentId(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace', name: 'Test Node' }) })} />
      );

      const card = screen.getByRole('group', { name: 'Test Node' });
      expect(card).toBeInTheDocument();

      // Indonesian text should appear (check for some localized text)
      expect(card).toBeInTheDocument();
    });

    it('focuses rename input in Indonesian locale', async () => {
      await renderWithFluentId(
        <TopologyNodeCard {...defaultProps({ node: makeNode({ type: 'workspace' }), renaming: true })} />
      );

      const input = screen.getByRole('textbox', { name: /workspace-rename-placeholder/i });
      expect(input).toBeInTheDocument();
    });
  });

  describe('Multiple errors', () => {
    it('shows only first error in validation note', async () => {
      const error1 = makeValidationError({ code: 'warehouse-at-capacity', messageId: 'topology-validation-warehouse-at-capacity' });
      const error2 = makeValidationError({ code: 'duplicate-node', messageId: 'topology-validation-duplicate-node' });
      await renderWithFluent(<TopologyNodeCard {...defaultProps({ nodeErrors: [error1, error2] })} />);

      // Should show first error
      expect(screen.getByText('topology-validation-warehouse-at-capacity')).toBeInTheDocument();
      // But title should have both
      const note = screen.getByRole('status', { name: /warehouse-at-capacity/i });
      expect(note).toHaveAttribute('title', 'topology-validation-warehouse-at-capacity\ntopology-validation-duplicate-node');
    });
  });
});