import { useState, useMemo, useRef, useEffect, useCallback } from 'react';
import { Localized } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { Button } from '@/components/Button';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import {
  StoreIcon,
  PosIcon,
  WarehouseIcon,
  PrinterIcon,
  FlaskIcon,
  StopIcon,
  CartIcon,
  UtensilsIcon,
  CheckIcon,
  TrashIcon,
  CloseIcon,
  LockIcon,
} from './NodeTopologyIcons';
import './NodeTopologyEditor.css';

// ── Types ──────────────────────────────────────────────────────────

export type NodeType = 'store' | 'workspace' | 'warehouse' | 'hardware';
export type WireDirection = 'one-way' | 'two-way';
export type PortName = 'top' | 'right' | 'bottom' | 'left';

export interface TopologyNodeData {
  id: string;
  type: NodeType;
  name: string;
  subtitle?: string;
  x: number;
  y: number;
  tierRequirement?: 'pro' | 'enterprise';
  telemetryBadge?: string;
  telemetryStatus?: 'online' | 'warning' | 'offline';
  metadata?: Record<string, string>;
}

export interface TopologyWireData {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  direction: WireDirection;
  label?: string;
  /** Which port on the source node the wire originates from (default: 'right'). */
  fromPort?: PortName;
  /** Which port on the target node the wire connects to (default: 'right'). */
  toPort?: PortName;
}

export interface NodeTopologyEditorProps {
  currentTier?: 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise';
  onSave?: (nodes: TopologyNodeData[], wires: TopologyWireData[]) => void;
}

// ── Presets ────────────────────────────────────────────────────────

const PRESET_RETAIL: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    { id: 'store-1', type: 'store', name: 'Downtown Branch', subtitle: 'Primary Store', x: 80, y: 140, telemetryBadge: 'Online (2 POS)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Retail POS #1', subtitle: 'Main Checkout', x: 340, y: 80, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-1', type: 'warehouse', name: 'Main Warehouse', subtitle: 'Primary Storage', x: 600, y: 140, telemetryBadge: '1,250 items', telemetryStatus: 'online' },
  ],
  wires: [
    // Natural left-to-right flow: store right → workspace left, workspace right → warehouse left
    { id: 'w-1', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-1', toPort: 'left', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'ws-1', fromPort: 'right', toNodeId: 'wh-1', toPort: 'left', direction: 'one-way', label: 'Stock Deduct (P1)' },
  ],
};

const PRESET_RESTAURANT: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    { id: 'store-1', type: 'store', name: 'Grand Bistro', subtitle: 'Main Branch', x: 80, y: 180, telemetryBadge: 'Online (3 Terminals)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Resto POS #1', subtitle: 'Dining Room', x: 340, y: 80, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'ws-kds', type: 'workspace', name: 'Kitchen KDS', subtitle: 'Line Cook Display', x: 340, y: 260, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-kitchen', type: 'warehouse', name: 'Kitchen Pantry', subtitle: 'Cold & Dry Storage', x: 600, y: 180, telemetryBadge: '⚠️ 12 Low Stock', telemetryStatus: 'warning' },
    { id: 'hw-prn', type: 'hardware', name: 'Kitchen Thermal Printer', subtitle: 'LAN 192.168.1.100', x: 600, y: 320, telemetryBadge: 'Ready', telemetryStatus: 'online' },
  ],
  wires: [
    // Left-to-right: store right → workspace left; then workspace right → warehouse/printer left
    { id: 'w-1', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-1', toPort: 'left', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-kds', toPort: 'left', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-3', fromNodeId: 'ws-1', fromPort: 'right', toNodeId: 'wh-kitchen', toPort: 'left', direction: 'one-way', label: 'Stock Deduct' },
    { id: 'w-4', fromNodeId: 'ws-kds', fromPort: 'right', toNodeId: 'hw-prn', toPort: 'left', direction: 'one-way', label: 'Ticket Print' },
  ],
};

// Estimated node card dimensions for wire endpoint positioning.
// Header ~46px + body ~64px + gaps ≈ 112px. Width ~200px for typical content.
const NODE_WIDTH = 200;
const NODE_HEIGHT = 112;

/** Port offset from node origin (left, top) for each port name.
 *  Offsets include the 6px port socket overhang so the wire path
 *  connects to the center of the port circle, not the card edge. */
const PORT_OFFSET: Record<PortName, { dx: number; dy: number }> = {
  top:    { dx: NODE_WIDTH / 2, dy: -6 },
  right:  { dx: NODE_WIDTH + 6, dy: NODE_HEIGHT / 2 },
  bottom: { dx: NODE_WIDTH / 2, dy: NODE_HEIGHT + 6 },
  left:   { dx: -6,             dy: NODE_HEIGHT / 2 },
};

/** Evaluate a cubic bezier at parameter t (0-1). */
function cubicBezier(
  t: number,
  p0: number,
  p1: number,
  p2: number,
  p3: number,
): number {
  const u = 1 - t;
  return u * u * u * p0 + 3 * u * u * t * p1 + 3 * u * t * t * p2 + t * t * t * p3;
}

const GRID_SIZE = 24;
const snap = (v: number) => Math.round(v / GRID_SIZE) * GRID_SIZE;
type HistoryEntry = { nodes: TopologyNodeData[]; wires: TopologyWireData[] };

export default function NodeTopologyEditor({
  currentTier = 'standard',
  onSave,
}: NodeTopologyEditorProps) {
  const { addToast } = useToast();
  const canvasRef = useRef<HTMLDivElement>(null);

  const [nodes, setNodes] = useState<TopologyNodeData[]>(PRESET_RETAIL.nodes);
  const [wires, setWires] = useState<TopologyWireData[]>(PRESET_RETAIL.wires);

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedWireId, setSelectedWireId] = useState<string | null>(null);

  const [isSimulating, setIsSimulating] = useState(false);
  const [simPulseStep, setSimPulseStep] = useState(0);

  const [draggingNodeId, setDraggingNodeId] = useState<string | null>(null);
  const dragOffsetRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const isPanningRef = useRef(false);
  const panStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const panCleanupRef = useRef<(() => void) | null>(null);

  const [connectingFromNodeId, setConnectingFromNodeId] = useState<string | null>(null);
  const [connectingFromPort, setConnectingFromPort] = useState<PortName | null>(null);
  /** Nearest target port while dragging a connection, for snap-to-port preview. */
  const [hoveredTarget, setHoveredTarget] = useState<{ nodeId: string; port: PortName } | null>(null);
  const mousePosRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const undoInProgressRef = useRef(false);

  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const isProAllowed = useMemo(() => ['pro', 'enterprise'].includes(currentTier), [currentTier]);

  const pushHistory = useCallback(() => {
    if (undoInProgressRef.current) return;
    setHistory((prev) => {
      const entry: HistoryEntry = { nodes: nodes.map((n) => ({ ...n })), wires: wires.map((w) => ({ ...w })) };
      const next = [...prev, entry];
      if (next.length > 50) next.shift();
      return next;
    });
  }, [nodes, wires]);

  // Clean up pan listeners on unmount to prevent leaks
  useEffect(() => {
    return () => {
      panCleanupRef.current?.();
    };
  }, []);

  useEffect(() => {
    if (!isSimulating) return;
    const interval = setInterval(() => {
      setSimPulseStep((prev) => (prev + 1) % 100);
    }, 30);
    return () => clearInterval(interval);
  }, [isSimulating]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setConnectingFromNodeId(null);
        setConnectingFromPort(null);
        setSelectedNodeId(null);
        setSelectedWireId(null);
        return;
      }
      if ((e.key === 'Delete' || e.key === 'Backspace') && (selectedNodeId || selectedWireId)) {
        e.preventDefault();
        if (selectedNodeId) {
          const hasWires = wires.some((w) => w.fromNodeId === selectedNodeId || w.toNodeId === selectedNodeId);
          if (hasWires) {
            setConfirmDelete(selectedNodeId);
          } else {
            // No connected wires — delete immediately without dialog.
            pushHistory();
            setNodes((prev) => prev.filter((n) => n.id !== selectedNodeId));
            setSelectedNodeId(null);
          }
        } else {
          setConfirmDelete('');
        }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        popUndo();
        return;
      }
      if (selectedNodeId && (e.key === 'ArrowUp' || e.key === 'ArrowDown' || e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault();
        pushHistory();
        const step = e.shiftKey ? GRID_SIZE : 8;
        setNodes((prev) =>
          prev.map((n) =>
            n.id === selectedNodeId
              ? {
                  ...n,
                  x: snap(n.x + (e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0)),
                  y: snap(n.y + (e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0)),
                }
              : n,
          ),
        );
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  });

  const popUndo = useCallback(() => {
    setHistory((prev) => {
      if (prev.length === 0) return prev;
      undoInProgressRef.current = true;
      const entry = prev[prev.length - 1];
      if (entry) {
        setNodes(entry.nodes);
        setWires(entry.wires);
      }
      setTimeout(() => { undoInProgressRef.current = false; }, 0);
      return prev.slice(0, -1);
    });
  }, []);

  const executeDelete = useCallback(() => {
    if (confirmDelete === '') {
      if (selectedWireId) {
        setWires((prev) => prev.filter((w) => w.id !== selectedWireId));
        setSelectedWireId(null);
      }
    } else if (confirmDelete) {
      pushHistory();
      setNodes((prev) => prev.filter((n) => n.id !== confirmDelete));
      setWires((prev) => prev.filter((w) => w.fromNodeId !== confirmDelete && w.toNodeId !== confirmDelete));
      setSelectedNodeId(null);
    }
    setConfirmDelete(null);
  }, [confirmDelete, selectedWireId, pushHistory]);

  const zoomToFit = useCallback(() => {
    if (nodes.length === 0) return;
    const minX = Math.min(...nodes.map((n) => n.x));
    const minY = Math.min(...nodes.map((n) => n.y));
    const maxX = Math.max(...nodes.map((n) => n.x + NODE_WIDTH));
    const maxY = Math.max(...nodes.map((n) => n.y + NODE_HEIGHT));
    const padding = 60;
    const viewW = (canvasRef.current?.clientWidth ?? 800) - padding * 2;
    const viewH = (canvasRef.current?.clientHeight ?? 600) - padding * 2;
    const fitZoom = Math.min(
      Math.min(viewW / Math.max(maxX - minX, 1), viewH / Math.max(maxY - minY, 1)),
      1.5,
    );
    setZoom(Math.max(0.4, Math.min(2.0, fitZoom)));
    setPan({ x: padding - minX * fitZoom, y: padding - minY * fitZoom });
  }, [nodes]);

  const handleNodeMouseDown = (e: React.MouseEvent, nodeId: string) => {
    e.stopPropagation();
    if (e.button !== 0) return;
    pushHistory();
    setSelectedNodeId(nodeId);
    setSelectedWireId(null);
    setDraggingNodeId(nodeId);

    const node = nodes.find((n) => n.id === nodeId);
    if (node) {
      dragOffsetRef.current = {
        x: e.clientX / zoom - node.x,
        y: e.clientY / zoom - node.y,
      };
    }
  };

  const handleCanvasMouseMove = (e: React.MouseEvent) => {
    mousePosRef.current = { x: e.clientX, y: e.clientY };

    if (draggingNodeId) {
      const newX = snap(Math.max(20, e.clientX / zoom - dragOffsetRef.current.x));
      const newY = snap(Math.max(20, e.clientY / zoom - dragOffsetRef.current.y));

      setNodes((prev) =>
        prev.map((n) => (n.id === draggingNodeId ? { ...n, x: newX, y: newY } : n)),
      );
    } else if (connectingFromNodeId) {
      // Find nearest target port when dragging a connection
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;
      const mx = (e.clientX - rect.left - pan.x) / zoom;
      const my = (e.clientY - rect.top - pan.y) / zoom;
      const SNAP_DIST = 30;
      let closest: { nodeId: string; port: PortName; dist: number } | null = null;
      for (const n of nodes) {
        if (n.id === connectingFromNodeId) continue;
        for (const p of ['top', 'right', 'bottom', 'left'] as PortName[]) {
          const off = PORT_OFFSET[p];
          const px = n.x + off.dx;
          const py = n.y + off.dy;
          const dist = Math.sqrt((mx - px) ** 2 + (my - py) ** 2);
          if (dist < SNAP_DIST && (!closest || dist < closest.dist)) {
            closest = { nodeId: n.id, port: p, dist };
          }
        }
      }
      setHoveredTarget(closest ? { nodeId: closest.nodeId, port: closest.port } : null);
    }
  };

  const handleCanvasMouseUp = () => {
    setDraggingNodeId(null);
  };

  // Clear hoveredTarget when connection mode ends
  useEffect(() => {
    if (!connectingFromNodeId) {
      setHoveredTarget(null);
    }
  }, [connectingFromNodeId]);

  const handleCanvasMouseDown = (e: React.MouseEvent) => {
    const targetEl = e.target as HTMLElement;
    if (targetEl === e.currentTarget || targetEl.classList.contains('node-canvas-viewport') || targetEl.tagName === 'svg') {
      setSelectedNodeId(null);
      setSelectedWireId(null);
      if (e.button === 0 || e.button === 1) {
        isPanningRef.current = true;
        panStartRef.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };

        const handleMouseMove = (ev: MouseEvent) => {
          if (!isPanningRef.current) return;
          setPan({
            x: ev.clientX - panStartRef.current.x,
            y: ev.clientY - panStartRef.current.y,
          });
        };

        const handleMouseUp = () => {
          isPanningRef.current = false;
          document.removeEventListener('mousemove', handleMouseMove);
          document.removeEventListener('mouseup', handleMouseUp);
          panCleanupRef.current = null;
        };

        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);

        panCleanupRef.current = () => {
          document.removeEventListener('mousemove', handleMouseMove);
          document.removeEventListener('mouseup', handleMouseUp);
          isPanningRef.current = false;
          panCleanupRef.current = null;
        };
      }
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
    setZoom((prev) => Math.min(2.0, Math.max(0.4, prev * zoomFactor)));
  };

  const handleAddNode = (type: NodeType) => {
    if (type === 'warehouse' && !isProAllowed && nodes.filter((n) => n.type === 'warehouse').length >= 1) {
      addToast({ message: 'Multi-Warehouse storage locations require a Pro Tier license.', type: 'warning' });
      return;
    }
    pushHistory();

    const id = `${type}-${Date.now()}`;
    const newNode: TopologyNodeData = {
      id,
      type,
      name: `New ${type.charAt(0).toUpperCase() + type.slice(1)}`,
      subtitle: type === 'store' ? 'Branch' : type === 'workspace' ? 'Register' : type === 'warehouse' ? 'Storage' : 'Peripheral',
      x: snap(200 + Math.random() * 100),
      y: snap(150 + Math.random() * 100),
      telemetryBadge: 'Ready',
      telemetryStatus: 'online',
    };

    setNodes((prev) => [...prev, newNode]);
    setSelectedNodeId(id);
  };

  const handlePortClick = (e: React.MouseEvent, nodeId: string, port: PortName) => {
    e.stopPropagation();

    if (!connectingFromNodeId) {
      setConnectingFromNodeId(nodeId);
      setConnectingFromPort(port);
      return;
    }

    if (connectingFromNodeId === nodeId) {
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    const fromNode = nodes.find((n) => n.id === connectingFromNodeId);
    const toNode = nodes.find((n) => n.id === nodeId);
    if (!fromNode || !toNode) { setConnectingFromNodeId(null); setConnectingFromPort(null); return; }

    const duplicate = wires.some(
      (w) =>
        (w.fromNodeId === connectingFromNodeId && w.toNodeId === nodeId
          && w.fromPort === connectingFromPort && w.toPort === port)
        || (w.fromNodeId === nodeId && w.toNodeId === connectingFromNodeId
          && w.fromPort === port && w.toPort === connectingFromPort),
    );
    if (duplicate) {
      addToast({ message: 'A wire already connects these ports.', type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    pushHistory();

    const existingWarehouseWires = wires.filter((w) => {
      const fn = nodes.find((n) => n.id === w.fromNodeId);
      const tn = nodes.find((n) => n.id === w.toNodeId);
      return fn?.type === 'workspace' && tn?.type === 'warehouse';
    });

    if (fromNode.type === 'workspace' && toNode.type === 'warehouse' && existingWarehouseWires.length >= 1 && !isProAllowed) {
      addToast({ message: 'Multi-warehouse stock deduction fallback wires require a Pro Tier license.', type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    const newWireId = `wire-${Date.now()}`;
    const isWarehouseWire = fromNode.type === 'workspace' && toNode.type === 'warehouse';
    const label = isWarehouseWire
      ? existingWarehouseWires.length === 0 ? 'Stock Deduct (P1)' : `Fallback (P${existingWarehouseWires.length + 1})`
      : 'Connected';

    setWires((prev) => [
      ...prev,
      { id: newWireId, fromNodeId: connectingFromNodeId, fromPort: connectingFromPort!, toNodeId: nodeId, toPort: port, direction: 'one-way', label },
    ]);
    setConnectingFromNodeId(null);
    setConnectingFromPort(null);
  };

  const handleToggleWireDirection = (wireId: string) => {
    pushHistory();
    setWires((prev) =>
      prev.map((w) => {
        if (w.id === wireId) {
          return { ...w, direction: w.direction === 'one-way' ? 'two-way' : 'one-way' };
        }
        return w;
      }),
    );
  };

  const handleDeleteRequest = () => {
    if (selectedNodeId) {
      const hasWires = wires.some((w) => w.fromNodeId === selectedNodeId || w.toNodeId === selectedNodeId);
      if (hasWires) {
        setConfirmDelete(selectedNodeId);
      } else {
        // No connected wires — delete immediately without dialog.
        pushHistory();
        setNodes((prev) => prev.filter((n) => n.id !== selectedNodeId));
        setSelectedNodeId(null);
      }
    } else if (selectedWireId) {
      setConfirmDelete('');
    }
  };

  const wirePreviewLine = useMemo(() => {
    if (!connectingFromNodeId || !connectingFromPort) return null;
    const fromNode = nodes.find((n) => n.id === connectingFromNodeId);
    if (!fromNode) return null;
    const portOff = PORT_OFFSET[connectingFromPort];
    const x1 = fromNode.x + portOff.dx;
    const y1 = fromNode.y + portOff.dy;

    // If hovering near a target port, snap the preview to it
    let mx: number;
    let my: number;
    if (hoveredTarget) {
      const targetNode = nodes.find((n) => n.id === hoveredTarget.nodeId);
      if (targetNode) {
        const targetOff = PORT_OFFSET[hoveredTarget.port];
        mx = targetNode.x + targetOff.dx;
        my = targetNode.y + targetOff.dy;
      } else {
        const canvas = canvasRef.current;
        if (!canvas) return null;
        const rect = canvas.getBoundingClientRect();
        mx = (mousePosRef.current.x - rect.left - pan.x) / zoom;
        my = (mousePosRef.current.y - rect.top - pan.y) / zoom;
      }
    } else {
      const canvas = canvasRef.current;
      if (!canvas) return null;
      const rect = canvas.getBoundingClientRect();
      mx = (mousePosRef.current.x - rect.left - pan.x) / zoom;
      my = (mousePosRef.current.y - rect.top - pan.y) / zoom;
    }

    const dx = Math.abs(mx - x1) * 0.5;
    return { d: `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${mx - dx} ${my}, ${mx} ${my}` };
  }, [connectingFromNodeId, connectingFromPort, nodes, zoom, pan, hoveredTarget]);

  const selectedNode = useMemo(() => nodes.find((n) => n.id === selectedNodeId), [nodes, selectedNodeId]);

  /* eslint-disable jsx-a11y/no-static-element-interactions, jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions -- interactive drag/pan canvas requires these */
  return (
    <div className="node-topology-editor">
      {/* ── Confirm delete dialog ── */}
      {confirmDelete !== null && (
        <ConfirmDialog
          open
          onCancel={() => setConfirmDelete(null)}
          onConfirm={executeDelete}
          title={confirmDelete ? 'Delete Node' : 'Delete Wire'}
          message={
            confirmDelete
              ? 'This node has connected wires. Deleting it will remove all its wires too. This action cannot be undone.'
              : 'Delete this wire connection? This action cannot be undone.'
          }
          variant="danger"
          confirmLabel="Delete"
        />
      )}

      <div className="node-topology-header">
        <div className="node-topology-header-title">
          <Localized id="topology-builder-title">
            <h2>Visual Store & Workspace Topology Builder</h2>
          </Localized>
          <span className={`topology-tier-badge tier-${currentTier}`}>
            {currentTier.toUpperCase()} TIER
          </span>
        </div>

        <div className="node-topology-header-actions">
          <Button
            variant={isSimulating ? 'primary' : 'secondary'}
            onClick={() => setIsSimulating(!isSimulating)}
            className="simulation-btn"
            icon={isSimulating ? <StopIcon size={16} /> : <FlaskIcon size={16} />}
          >
            {isSimulating ? 'Stop Simulation' : 'Test Order Simulation'}
          </Button>

          <Button
            variant="secondary"
            onClick={() => { setNodes(PRESET_RETAIL.nodes); setWires(PRESET_RETAIL.wires); }}
            icon={<CartIcon size={16} />}
          >
            Retail Preset
          </Button>

          <Button
            variant="secondary"
            onClick={() => { setNodes(PRESET_RESTAURANT.nodes); setWires(PRESET_RESTAURANT.wires); }}
            icon={<UtensilsIcon size={16} />}
          >
            Resto & KDS Preset
          </Button>

          <Button
            variant="primary"
            onClick={() => onSave?.(nodes, wires)}
            icon={<CheckIcon size={16} />}
          >
            Apply Topology Changes
          </Button>
        </div>
      </div>

      <div className="node-topology-main">
        <div className="node-tool-rack">
          <h3>Palette Tools</h3>
          <p className="tool-rack-desc">Drag or click to spawn topology nodes:</p>

          <button className="tool-card" onClick={() => handleAddNode('store')}>
            <span className="tool-card-icon"><StoreIcon size={22} /></span>
            <div className="tool-card-info">
              <strong>+ Store Node</strong>
              <span>Store Branch Profile</span>
            </div>
          </button>

          <button className="tool-card" onClick={() => handleAddNode('workspace')}>
            <span className="tool-card-icon"><PosIcon size={22} /></span>
            <div className="tool-card-info">
              <strong>+ Workspace Node</strong>
              <span>POS / Register Instance</span>
            </div>
          </button>

          <button
            className={`tool-card ${!isProAllowed && nodes.some((n) => n.type === 'warehouse') ? 'locked' : ''}`}
            onClick={() => handleAddNode('warehouse')}
          >
            <span className="tool-card-icon"><WarehouseIcon size={22} /></span>
            <div className="tool-card-info">
              <strong>+ Warehouse Node</strong>
              <span>Storage Location</span>
            </div>
            {!isProAllowed && nodes.some((n) => n.type === 'warehouse') && (
              <span className="lock-badge"><LockIcon size={12} /> Pro</span>
            )}
          </button>

          <button className="tool-card" onClick={() => handleAddNode('hardware')}>
            <span className="tool-card-icon"><PrinterIcon size={22} /></span>
            <div className="tool-card-info">
              <strong>+ Hardware Node</strong>
              <span>Printer / KDS Peripheral</span>
            </div>
          </button>

          <hr className="tool-rack-divider" />

          {selectedNodeId || selectedWireId ? (
            <Button variant="secondary" onClick={handleDeleteRequest} className="delete-btn" icon={<TrashIcon size={16} />}>
              Delete Selected Element
            </Button>
          ) : null}

          {history.length > 0 && (
            <Button variant="secondary" onClick={popUndo} style={{ fontSize: 'var(--text-xs)' }}>
              Undo (Ctrl+Z)
            </Button>
          )}

          <div className="canvas-controls-mini">
            <span>Zoom: {Math.round(zoom * 100)}%</span>
            <Button variant="secondary" onClick={zoomToFit}>
              Fit All
            </Button>
            <Button variant="secondary" onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}>
              Reset View
            </Button>
          </div>
        </div>

        <div
          ref={canvasRef}
          className="node-canvas-container"
          tabIndex={0}
          role="application"
          aria-label="Topology editor canvas. Use arrow keys to nudge selected nodes, Ctrl+Z to undo."
          onMouseMove={handleCanvasMouseMove}
          onMouseUp={handleCanvasMouseUp}
          onMouseDown={handleCanvasMouseDown}
          onWheel={handleWheel}
        >
          <div
            className="node-canvas-viewport"
            style={{
              transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            }}
          >
            <svg className="node-wires-svg">
              <defs>
                <marker
                  id="arrow-end"
                  viewBox="0 0 6 6"
                  refX="5"
                  refY="3"
                  markerWidth="4"
                  markerHeight="4"
                  orient="auto-start-reverse"
                >
                  <path d="M 0 0 L 6 3 L 0 6 z" fill="var(--color-accent, #5a9fd4)" />
                </marker>

                <marker
                  id="arrow-start"
                  viewBox="0 0 6 6"
                  refX="5"
                  refY="3"
                  markerWidth="4"
                  markerHeight="4"
                  orient="auto-start-reverse"
                >
                  <path d="M 0 0 L 6 3 L 0 6 z" fill="var(--color-accent, #5a9fd4)" />
                </marker>
              </defs>

              {wires.map((wire) => {
                const fromNode = nodes.find((n) => n.id === wire.fromNodeId);
                const toNode = nodes.find((n) => n.id === wire.toNodeId);
                if (!fromNode || !toNode) return null;

                // Wire connects from port to port (default: right edge)
                const fromPort = wire.fromPort ?? 'right';
                const toPort = wire.toPort ?? 'right';
                const fromOff = PORT_OFFSET[fromPort];
                const toOff = PORT_OFFSET[toPort];
                const x1 = fromNode.x + fromOff.dx;
                const y1 = fromNode.y + fromOff.dy;
                const x2 = toNode.x + toOff.dx;
                const y2 = toNode.y + toOff.dy;

                const dx = Math.abs(x2 - x1) * 0.5;
                const pathD = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;

                // Label position: bezier midpoint offset perpendicular to the curve
                // so labels float above the wire arc instead of sitting on the
                // geometric midpoint (which can overlap with node cards).
                const labelT = 0.5;
                const lx = cubicBezier(labelT, x1, x1 + dx, x2 - dx, x2);
                const ly = cubicBezier(labelT, y1, y1, y2, y2);
                // Tangent at bezier midpoint
                const tangentX = 1.5 * (x2 - x1 - dx);
                const tangentY = 1.5 * (y2 - y1);
                // Perpendicular vector (ty, -tx) points "upward" for rightward wires
                const perpLen = Math.max(Math.sqrt(tangentX * tangentX + tangentY * tangentY), 12);
                const LABEL_OFFSET = 24;
                const labelOffX = (tangentY / perpLen) * LABEL_OFFSET;
                const labelOffY = (-tangentX / perpLen) * LABEL_OFFSET;

                // Pulse follows the cubic bezier curve, not a straight line
                const t = simPulseStep / 100;
                const pulseX = cubicBezier(t, x1, x1 + dx, x2 - dx, x2);
                const pulseY = cubicBezier(t, y1, y1, y2, y2);

                const isSelected = selectedWireId === wire.id;

                return (
                  <g key={wire.id} className={`wire-group ${isSelected ? 'wire-selected' : ''}`}>
                    <path
                      d={pathD}
                      className="wire-hitbox"
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedWireId(wire.id);
                        setSelectedNodeId(null);
                      }}
                    />

                    {/* Explicit endpoint dot ensures the wire always starts
                        exactly at the port socket center, regardless of SVG
                        renderer quirks with stroke-dasharray at path boundaries. */}
                    <circle cx={x1} cy={y1} r="1.5" className="wire-end-dot" />

                    <path
                      d={pathD}
                      className={`wire-path ${wire.direction}`}
                      markerEnd="url(#arrow-end)"
                      markerStart={wire.direction === 'two-way' ? 'url(#arrow-start)' : undefined}
                    />

                    <g
                      transform={`translate(${lx + labelOffX}, ${ly + labelOffY})`}
                      className="wire-label-group"
                      onClick={() => handleToggleWireDirection(wire.id)}
                      role="button"
                      aria-label="Toggle wire direction"
                    >
                      <rect x="-40" y="-12" width="80" height="22" rx="11" className="wire-label-bg" />
                      <text x="0" y="3" textAnchor="middle" className="wire-label-text">
                        {wire.direction === 'two-way' ? '\u2194' : '\u2192'} {wire.label || ''}
                      </text>
                    </g>

                    {isSimulating && (
                      <circle
                        cx={pulseX}
                        cy={pulseY}
                        r="6"
                        className="wire-simulation-pulse"
                      />
                    )}
                  </g>
                );
              })}

              {wirePreviewLine && (
                <path d={wirePreviewLine.d} className="wire-path" opacity="0.5" pointerEvents="none" />
              )}
            </svg>

            {nodes.map((node) => {
              const isSelected = selectedNodeId === node.id;
              const isConnectingSource = connectingFromNodeId === node.id;

              return (
                <div
                  key={node.id}
                  className={`topology-node node-type-${node.type} ${isSelected ? 'node-selected' : ''} ${isConnectingSource ? 'node-connecting-source' : ''}`}
                  style={{ left: `${node.x}px`, top: `${node.y}px` }}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') setSelectedNodeId(node.id); }}
                  onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
                >
                  <div className="node-header">
                    <span className="node-type-icon">
                      {node.type === 'store' ? <StoreIcon size={16} /> : node.type === 'workspace' ? <PosIcon size={16} /> : node.type === 'warehouse' ? <WarehouseIcon size={16} /> : <PrinterIcon size={16} />}
                    </span>
                    <span className="node-title">{node.name}</span>
                  </div>

                  <div className="node-body">
                    <span className="node-subtitle">{node.subtitle}</span>
                    {node.telemetryBadge && (
                      <span className={`node-telemetry-badge telemetry-${node.telemetryStatus || 'online'}`}>
                        {node.telemetryBadge}
                      </span>
                    )}
                  </div>

                  <div className="node-port-sockets-group">
                    {(['top', 'right', 'bottom', 'left'] as PortName[]).map((port) => {
                      const isActive = connectingFromNodeId === node.id && connectingFromPort === port;
                      const isHovered = hoveredTarget?.nodeId === node.id && hoveredTarget?.port === port;
                      const showHighlight = connectingFromNodeId && connectingFromNodeId !== node.id && isHovered;
                      return (
                        <button
                          key={port}
                          className={`node-port-socket port-${port} ${isActive ? 'port-active' : ''} ${showHighlight ? 'port-highlight' : ''}`}
                          onClick={(e) => handlePortClick(e, node.id, port)}
                          aria-label={`${node.name} ${port} port`}
                        >
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {selectedNode && (
          <div className="node-inspector-drawer">
            <div className="inspector-header">
              <h3>Node Inspector</h3>
              <Button variant="secondary" onClick={() => setSelectedNodeId(null)} icon={<CloseIcon size={14} />} aria-label="Close inspector">{null}</Button>
            </div>

            <div className="inspector-content">
              <label className="inspector-field">
                <span>Node Name</span>
                <input
                  type="text"
                  value={selectedNode.name}
                  onChange={(e) => {
                    const name = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, name } : n)));
                  }}
                />
              </label>

              <label className="inspector-field">
                <span>Subtitle / Location</span>
                <input
                  type="text"
                  value={selectedNode.subtitle || ''}
                  onChange={(e) => {
                    const subtitle = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, subtitle } : n)));
                  }}
                />
              </label>

              <div className="inspector-info-box">
                <strong>Node Type:</strong> {selectedNode.type.toUpperCase()}<br />
                <strong>Coordinates:</strong> X: {selectedNode.x}, Y: {selectedNode.y}
              </div>

              {selectedNode.type === 'warehouse' && (
                <div className="inspector-section">
                  <h4>Warehouse Settings</h4>
                  <label className="inspector-checkbox">
                    <input type="checkbox" defaultChecked /> Require Manager PIN for Stock Adjustments
                  </label>
                </div>
              )}

              {selectedNode.type === 'workspace' && (
                <div className="inspector-section">
                  <h4>Workspace Access</h4>
                  <label className="inspector-checkbox">
                    <input type="checkbox" defaultChecked /> Allow Cashiers to Void Items
                  </label>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
/* eslint-enable jsx-a11y/no-static-element-interactions, jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions */
