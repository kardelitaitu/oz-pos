//! Isolated unit tests for the topology editor's extracted overlay
//! components — shortcuts help popover, node finder, and canvas minimap.
//!
//! These pin the extracted modules' contracts directly (props in → behavior
//! out) rather than through the full editor mount, so a future edit to one
//! overlay is caught without re-running the 541-test editor suite.

import { render, screen, fireEvent, within, cleanup } from '@testing-library/react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import { TopologyShortcutsHelp } from '../features/stores/topologyShortcutsHelp';
import { TopologyNodeFinder } from '../features/stores/topologyNodeFinder';
import { TopologyMinimap } from '../features/stores/topologyMinimap';
import { NODE_WIDTH, NODE_HEIGHT } from '../features/stores/nodeTopologyClamp';
import type { TopologyNodeData, TopologyWireData } from '../features/stores/NodeTopologyEditor';

// These overlays only read l10n via useLocalization().getString; a raw-id
// fallback keeps the tests focused on behavior, not translation content.
vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => id,
    },
  }),
}));

afterEach(cleanup);

describe('TopologyShortcutsHelp', () => {
  it('renders the help button and toggles on click', () => {
    const onToggle = vi.fn();
    const onClose = vi.fn();
    render(<TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />);

    const button = screen.getByRole('button');
    expect(button).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(button);
    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('renders the shortcut rows only while open', () => {
    const { rerender } = render(
      <TopologyShortcutsHelp open={false} onToggle={vi.fn()} onClose={vi.fn()} />,
    );
    expect(document.querySelector('.topology-shortcuts-popover')).toBeNull();

    rerender(<TopologyShortcutsHelp open onToggle={vi.fn()} onClose={vi.fn()} />);
    const popover = document.querySelector('.topology-shortcuts-popover');
    expect(popover).not.toBeNull();
    expect(popover!.querySelectorAll('.topology-shortcuts-row').length).toBeGreaterThan(0);
  });

  it('closes on Escape and on an outside mousedown', () => {
    const onClose = vi.fn();
    render(<TopologyShortcutsHelp open onToggle={vi.fn()} onClose={onClose} />);

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.mouseDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

describe('TopologyNodeFinder', () => {
  const nodes: TopologyNodeData[] = [
    { id: 'a', type: 'store', name: 'Alpha Store', x: 0, y: 0 },
    { id: 'b', type: 'warehouse', name: 'Bravo Warehouse', x: 200, y: 100 },
    { id: 'c', type: 'workspace', name: 'Charlie POS', x: 400, y: 200 },
  ];

  it('renders nothing while closed', () => {
    const { container } = render(
      <TopologyNodeFinder open={false} nodes={nodes} onJump={vi.fn()} onClose={vi.fn()} />,
    );
    expect(container.querySelector('.topology-finder')).toBeNull();
  });

  it('lists every node for an empty query and filters by name', () => {
    render(<TopologyNodeFinder open nodes={nodes} onJump={vi.fn()} onClose={vi.fn()} />);

    const input = screen.getByRole('combobox');
    const list = screen.getByRole('listbox');
    expect(within(list).getAllByRole('option')).toHaveLength(3);

    fireEvent.change(input, { target: { value: 'bravo' } });
    const options = within(list).getAllByRole('option');
    expect(options).toHaveLength(1);
    expect(options[0]).toHaveTextContent('Bravo Warehouse');
  });

  it('jumps to the highlighted match on Enter and closes on Escape', () => {
    const onJump = vi.fn();
    const onClose = vi.fn();
    render(<TopologyNodeFinder open nodes={nodes} onJump={onJump} onClose={onClose} />);

    const input = screen.getByRole('combobox');
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onJump).toHaveBeenCalledWith(nodes[0]); // first node highlighted

    fireEvent.keyDown(input, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe('TopologyMinimap', () => {
  const minimapNodes: TopologyNodeData[] = [
    { id: 'a', type: 'store', name: 'A', x: 0, y: 0 },
    { id: 'b', type: 'warehouse', name: 'B', x: 300, y: 200 },
  ];
  const minimapWires: TopologyWireData[] = [
    { id: 'w1', fromNodeId: 'a', toNodeId: 'b', direction: 'one-way' },
  ];
  const nodeMap = new Map(minimapNodes.map((n) => [n.id, n]));

  const renderMinimap = (overrides: Partial<Parameters<typeof TopologyMinimap>[0]> = {}) =>
    render(
      <TopologyMinimap
        nodes={minimapNodes}
        wires={minimapWires}
        nodeMap={nodeMap}
        pan={{ x: 0, y: 0 }}
        zoom={1}
        canvasWidth={800}
        canvasHeight={600}
        onCenter={vi.fn()}
        onNudge={vi.fn()}
        {...overrides}
      />,
    );

  it('renders nothing when there are no nodes', () => {
    const { container } = render(
      <TopologyMinimap
        nodes={[]}
        wires={[]}
        nodeMap={new Map()}
        pan={{ x: 0, y: 0 }}
        zoom={1}
        canvasWidth={800}
        canvasHeight={600}
        onCenter={vi.fn()}
        onNudge={vi.fn()}
      />,
    );
    expect(container.querySelector('.topology-minimap')).toBeNull();
  });

  it('renders a node per diagram node, a wire per diagram wire, and the viewport box', () => {
    const { container } = renderMinimap();
    const map = container.querySelector('.topology-minimap');
    expect(map).not.toBeNull();
    expect(map!.querySelectorAll('.topology-minimap-node')).toHaveLength(2);
    expect(map!.querySelectorAll('.topology-minimap-wire')).toHaveLength(1);
    expect(map!.querySelector('.topology-minimap-viewport')).not.toBeNull();
  });

  it('centers on the content box on Enter and nudges on arrows', () => {
    const onCenter = vi.fn();
    const onNudge = vi.fn();
    renderMinimap({ onCenter, onNudge });

    const map = screen.getByRole('button');
    fireEvent.keyDown(map, { key: 'Enter' });
    // Content box: minX=0, maxX=300+NODE_WIDTH; minY=0, maxY=200+NODE_HEIGHT.
    expect(onCenter).toHaveBeenCalledWith((300 + NODE_WIDTH) / 2, (200 + NODE_HEIGHT) / 2);

    fireEvent.keyDown(map, { key: 'ArrowLeft' });
    expect(onNudge).toHaveBeenCalledWith(-40, 0);

    fireEvent.keyDown(map, { key: 'ArrowDown' });
    expect(onNudge).toHaveBeenCalledWith(0, 40);
  });
});
