import { useState, useMemo, useRef, useEffect, useCallback, memo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { Button } from '@/components/Button';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { loadTopology } from '@/api/topology';
import { useSettings } from '@/contexts/SettingsContext';
import {
  WorkspaceStorePosSettings,
  WorkspaceRestaurantPosSettings,
  WorkspaceKdsSettings,
  WorkspaceInventorySettings,
  StoreInfoCard,
  type WorkspaceCardProps,
} from '@/features/settings/workspace-cards';
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
  metadata?: Record<string, unknown>;
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

export interface WorkspaceInstanceSeed {
  /** Instance id from workspace_instances — becomes the node id. */
  instanceId: string;
  /** Workspace type key (store-pos, restaurant-pos, kds, warehouse). */
  typeKey: string;
  name: string;
  subtitle?: string;
  colour?: string;
}

export interface NodeTopologyEditorProps {
  currentTier?: 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise';
  /**
   * Called when the user clicks "Apply Topology Changes". Returns an
   * optional `oldId -> newId` map so the editor can remap its local
   * state when archive+recreate assigns new UUIDs (Critical #1).
   */
  onSave?: (nodes: TopologyNodeData[], wires: TopologyWireData[]) => Promise<Record<string, string> | void>;
  /**
   * Real workspace instances to seed the canvas with. When provided, the
   * editor renders one workspace node per instance (positions restored from
   * the saved topology diagram when available) instead of the demo preset.
   * This makes the canvas reflect the actual `workspace_instances` table so
   * the parent's onSave diff can create / update / archive correctly.
   */
  workspaceInstances?: WorkspaceInstanceSeed[];
}

/** Valid workspace type keys selectable when creating a workspace node.
 *  Labels are resolved at render time via l10n.getString for i18n. */
const WORKSPACE_TYPE_KEYS = ['store-pos', 'restaurant-pos', 'kds', 'warehouse'] as const;

function getWorkspaceTypeLabel(key: string, l10n: ReturnType<typeof useLocalization>['l10n']): string {
  const map: Record<string, string> = {
    'store-pos': l10n.getString('topology-ws-type-store-pos'),
    'restaurant-pos': l10n.getString('topology-ws-type-restaurant-pos'),
    'kds': l10n.getString('topology-ws-type-kds'),
    'warehouse': l10n.getString('topology-ws-type-warehouse'),
  };
  return map[key] ?? key;
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

/** Isolated simulation pulse circle so the 30ms tick doesn't re-render the whole canvas. */
const SimulationPulse = memo(function SimulationPulse({ x, y }: { x: number; y: number }) {
  return <circle cx={x} cy={y} r="6" className="wire-simulation-pulse" />;
});

export default function NodeTopologyEditor({
  currentTier = 'standard',
  onSave,
  workspaceInstances,
}: NodeTopologyEditorProps) {
  const { addToast } = useToast();
  const { l10n } = useLocalization();
  const { settings } = useSettings();
  const canvasRef = useRef<HTMLDivElement>(null);

  const [nodes, setNodes] = useState<TopologyNodeData[]>(PRESET_RETAIL.nodes);
  const [wires, setWires] = useState<TopologyWireData[]>(PRESET_RETAIL.wires);

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedWireId, setSelectedWireId] = useState<string | null>(null);

  const [isSimulating, setIsSimulating] = useState(false);
  const [simPulseStep, setSimPulseStep] = useState(0);

  const [draggingNodeId, setDraggingNodeId] = useState<string | null>(null);
  const dragOffsetRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  /** Set of node ids that were just added (for scale-in animation). */
  const [freshNodeIds, setFreshNodeIds] = useState<Set<string>>(new Set());
  /** Timers for fresh-node animation cleanup; cleared on unmount to prevent leaks. */
  const freshTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

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
  /** Mirror of `history` state for synchronous reads in undo/redo handlers. */
  const historyRef = useRef<HistoryEntry[]>([]);
  historyRef.current = history;
  const [redo, setRedo] = useState<HistoryEntry[]>([]);

  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [confirmPreset, setConfirmPreset] = useState<'retail' | 'restaurant' | null>(null);

  /** Skip the next workspaceInstances-triggered reload (set before calling onSave). */
  const skipNextLoadRef = useRef(false);
  /** Track whether user has made any edits since last preset load. */
  const isDirtyRef = useRef(false);

  const isProAllowed = useMemo(() => ['pro', 'enterprise'].includes(currentTier), [currentTier]);

  /** O(1) node lookup by id — replaces `nodes.find` in hot paths (wire rendering, etc.). */
  const nodeMap = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);

  /** Precomputed wire path geometry — avoids recomputing bezier curves on every render. */
  const wireGeometries = useMemo(() => {
    const geo = new Map<string, {
      x1: number; y1: number; x2: number; y2: number;
      dx: number;
      pathD: string;
      labelX: number; labelY: number;
    }>();
    for (const wire of wires) {
      const fromNode = nodeMap.get(wire.fromNodeId);
      const toNode = nodeMap.get(wire.toNodeId);
      if (!fromNode || !toNode) continue;
      const fromPort = wire.fromPort ?? 'right';
      const toPort = wire.toPort ?? 'left';
      const fromOff = PORT_OFFSET[fromPort];
      const toOff = PORT_OFFSET[toPort];
      const x1 = fromNode.x + fromOff.dx;
      const y1 = fromNode.y + fromOff.dy;
      const x2 = toNode.x + toOff.dx;
      const y2 = toNode.y + toOff.dy;
      const dx = Math.abs(x2 - x1) * 0.5;
      geo.set(wire.id, {
        x1, y1, x2, y2, dx,
        pathD: `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`,
        labelX: cubicBezier(0.5, x1, x1 + dx, x2 - dx, x2),
        labelY: cubicBezier(0.5, y1, y1, y2, y2),
      });
    }
    return geo;
  }, [wires, nodeMap]);

  /** Dynamic SVG bounds derived from node positions — replaces fixed 5000×5000px clipping. */
  const svgBounds = useMemo(() => {
    if (nodes.length === 0) return { width: 0, height: 0 };
    const maxX = nodes.reduce((acc, n) => Math.max(acc, n.x + NODE_WIDTH), -Infinity);
    const maxY = nodes.reduce((acc, n) => Math.max(acc, n.y + NODE_HEIGHT), -Infinity);
    if (!isFinite(maxX) || !isFinite(maxY)) return { width: 0, height: 0 };
    return { width: maxX + 200, height: maxY + 200 };
  }, [nodes]);

  // Load persisted topology on mount, fall back to retail preset.
  useEffect(() => {
    let cancelled = false;
    loadTopology()
      .then((data) => {
        // Build a lookup of saved node positions/metadata (the diagram layer).
        const savedById = new Map<string, TopologyNodeData>();
        if (data && data.nodes) {
          for (const n of data.nodes) {
            const node: TopologyNodeData = {
              id: n.id,
              type: n.type as NodeType,
              name: n.name,
              x: n.x,
              y: n.y,
            };
            if (n.subtitle !== undefined) node.subtitle = n.subtitle;
            if (n.tier_requirement !== undefined) node.tierRequirement = n.tier_requirement as 'pro' | 'enterprise';
            if (n.telemetry_badge !== undefined) node.telemetryBadge = n.telemetry_badge;
            if (n.telemetry_status !== undefined) node.telemetryStatus = n.telemetry_status as 'online' | 'warning' | 'offline';
            if (n.metadata !== undefined) node.metadata = n.metadata;
            savedById.set(n.id, node);
          }
        }

        // When real workspace instances are supplied, they are authoritative
        // for which workspace nodes exist. Restore positions from the saved
        // diagram, but never resurrect a workspace node that no longer maps
        // to a live instance (that would undo an archive). Non-workspace
        // nodes (store/warehouse/hardware) still come from the saved diagram.
        if (workspaceInstances) {
          if (cancelled) return;
          // Skip the full rebuild when our own save triggered this reload —
          // only update persisted flags, preserving in-flight canvas edits (#8).
          if (skipNextLoadRef.current) {
            skipNextLoadRef.current = false;
            setNodes((prev) =>
              prev.map((n) => {
                if (n.type === 'workspace') {
                  return { ...n, metadata: { ...n.metadata, persisted: true } };
                }
                return n;
              }),
            );
            return;
          }
          const wsNodes: TopologyNodeData[] = workspaceInstances.map((inst, i) => {
            const saved = savedById.get(inst.instanceId);
            const node: TopologyNodeData = {
              id: inst.instanceId,
              type: 'workspace',
              name: inst.name,
              subtitle: inst.subtitle ?? saved?.subtitle ?? '',
              x: saved?.x ?? snap(340),
              y: saved?.y ?? snap(80 + i * 140),
              telemetryBadge: saved?.telemetryBadge ?? 'Active',
              telemetryStatus: saved?.telemetryStatus ?? 'online',
              metadata: { ...(saved?.metadata ?? {}), typeKey: inst.typeKey, persisted: true },
            };
            return node;
          });
          // Keep any saved non-workspace nodes (diagram-only in this pass).
          const otherNodes = [...savedById.values()].filter((n) => n.type !== 'workspace');
          const mergedNodes = [...otherNodes, ...wsNodes];
          const validIds = new Set(mergedNodes.map((n) => n.id));
          const loadedWires: TopologyWireData[] = (data?.wires ?? [])
            .filter((w) => validIds.has(w.from_node_id) && validIds.has(w.to_node_id))
            .map((w) => {
              const wire: TopologyWireData = {
                id: w.id,
                fromNodeId: w.from_node_id,
                toNodeId: w.to_node_id,
                direction: w.direction as WireDirection,
              };
              if (w.label !== undefined) wire.label = w.label;
              if (w.from_port !== undefined) wire.fromPort = w.from_port as PortName;
              if (w.to_port !== undefined) wire.toPort = w.to_port as PortName;
              return wire;
            });
          setNodes(mergedNodes);
          setWires(loadedWires);
          isDirtyRef.current = false;
          return;
        }

        // No real instances supplied — legacy/demo behaviour: use the saved
        // diagram verbatim, or fall back to the retail preset.
        if (cancelled || !data || !data.nodes || data.nodes.length === 0) return;
        if (skipNextLoadRef.current) { skipNextLoadRef.current = false; return; }
        setNodes([...savedById.values()]);
        const loadedWires: TopologyWireData[] = data.wires.map((w) => {
          const wire: TopologyWireData = {
            id: w.id,
            fromNodeId: w.from_node_id,
            toNodeId: w.to_node_id,
            direction: w.direction as WireDirection,
          };
          if (w.label !== undefined) wire.label = w.label;
          if (w.from_port !== undefined) wire.fromPort = w.from_port as PortName;
          if (w.to_port !== undefined) wire.toPort = w.to_port as PortName;
          return wire;
        });
        setWires(loadedWires);
        isDirtyRef.current = false;
      })
      .catch((err) => {
        // Only "no saved topology" (null result) is expected — that is
        // handled in the .then() above. Any thrown error (corrupt DB,
        // serialisation failure, etc.) should be surfaced to the user
        // rather than silently swallowed.
        if (cancelled) return;
        addToast({
          message: `${l10n.getString('topology-toast-load-error')}: ${err instanceof Error ? err.message : String(err)}`,
          type: 'error',
        });
      });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceInstances]);

  const pushHistory = useCallback(() => {
    isDirtyRef.current = true;
    setRedo([]); // new edit invalidates the redo branch
    setHistory((prev) => {
      const entry: HistoryEntry = { nodes: nodes.map((n) => ({ ...n })), wires: wires.map((w) => ({ ...w })) };
      const next = [...prev, entry];
      if (next.length > 50) next.shift();
      return next;
    });
  }, [nodes, wires]);

  const loadPreset = useCallback((preset: 'retail' | 'restaurant') => {
    const data = preset === 'retail' ? PRESET_RETAIL : PRESET_RESTAURANT;
    pushHistory();
    setNodes(data.nodes);
    setWires(data.wires);
    setFreshNodeIds(new Set());
    isDirtyRef.current = false;
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [pushHistory]);

  const popUndo = useCallback(() => {
    const stack = historyRef.current;
    if (stack.length === 0) return;
    const entry = stack[stack.length - 1]!;
    // Push current state to redo before restoring
    setRedo((prev) => [...prev, { nodes: nodes.map((n) => ({ ...n })), wires: wires.map((w) => ({ ...w })) }]);
    // Sibling setState calls (not nested in updater — fixes ADR audit #6)
    setNodes(entry.nodes);
    setWires(entry.wires);
    setHistory((prev) => prev.slice(0, -1));
  }, [nodes, wires]);

  const popRedo = useCallback(() => {
    if (redo.length === 0) return;
    const entry = redo[redo.length - 1]!;
    // Push current state to history before restoring
    setHistory((prev) => [...prev, { nodes: nodes.map((n) => ({ ...n })), wires: wires.map((w) => ({ ...w })) }]);
    setNodes(entry.nodes);
    setWires(entry.wires);
    setRedo((prev) => prev.slice(0, -1));
  }, [redo, nodes, wires]);

  // Clean up pan listeners and fresh-node timers on unmount
  useEffect(() => {
    const timers = freshTimersRef.current;
    return () => {
      panCleanupRef.current?.();
      timers.forEach(clearTimeout);
      timers.clear();
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
      // Guard: don't handle canvas shortcuts while the user is typing in a text field.
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
        return;
      }
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
        if (e.shiftKey) {
          popRedo();
        } else {
          popUndo();
        }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
        e.preventDefault();
        popRedo();
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
  }, [selectedNodeId, selectedWireId, wires, pushHistory, popUndo, popRedo]);

  const executePresetLoad = useCallback(() => {
    if (confirmPreset) {
      loadPreset(confirmPreset);
    }
    setConfirmPreset(null);
  }, [confirmPreset, loadPreset]);

  const executeDelete = useCallback(() => {
    if (confirmDelete === '') {
      if (selectedWireId) {
        pushHistory();
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
    const minX = nodes.reduce((acc, n) => Math.min(acc, n.x), Infinity);
    const minY = nodes.reduce((acc, n) => Math.min(acc, n.y), Infinity);
    const maxX = nodes.reduce((acc, n) => Math.max(acc, n.x + NODE_WIDTH), -Infinity);
    const maxY = nodes.reduce((acc, n) => Math.max(acc, n.y + NODE_HEIGHT), -Infinity);
    // Guard against degenerate bounding box with zero or negative dimensions
    if (!isFinite(minX) || !isFinite(maxX) || maxX <= minX || maxY <= minY) return;
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

    const node = nodeMap.get(nodeId);
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
          panCleanupRef.current?.();
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
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
    setZoom((prev) => {
      const newZoom = Math.min(2.0, Math.max(0.4, prev * zoomFactor));
      // Zoom towards cursor: adjust pan so cursor position stays fixed
      const cursorX = e.clientX - rect.left;
      const cursorY = e.clientY - rect.top;
      setPan((p) => ({
        x: cursorX - (cursorX - p.x) * (newZoom / prev),
        y: cursorY - (cursorY - p.y) * (newZoom / prev),
      }));
      return newZoom;
    });
  };

  const handleAddNode = (type: NodeType) => {
    if (type === 'warehouse' && !isProAllowed && nodes.filter((n) => n.type === 'warehouse').length >= 1) {
      addToast({ message: l10n.getString('topology-toast-multi-warehouse'), type: 'warning' });
      return;
    }
    pushHistory();

    const id = `${type}-${crypto.randomUUID()}`;
    const newNode: TopologyNodeData = {
      id,
      type,
      name: l10n.getString(`topology-new-${type}`),
      subtitle: l10n.getString(`topology-new-${type}-subtitle`),
      x: snap(200 + Math.random() * 100),
      y: snap(150 + Math.random() * 100),
      telemetryBadge: l10n.getString('topology-new-ready'),
      telemetryStatus: 'online',
      // New workspace nodes default to the retail POS type until the user
      // picks another in the inspector. `persisted: false` marks it as not
      // yet backed by a workspace_instances row so onSave will create it.
      ...(type === 'workspace' ? { metadata: { typeKey: 'store-pos', persisted: false } } : {}),
    };

    setNodes((prev) => [...prev, newNode]);
    setFreshNodeIds((prev) => new Set(prev).add(id));
    // Remove from fresh set after animation completes
    const freshTimer = setTimeout(() => {
      setFreshNodeIds((prev) => { const next = new Set(prev); next.delete(id); return next; });
      freshTimersRef.current.delete(freshTimer);
    }, 400);
    freshTimersRef.current.add(freshTimer);
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

    const fromNode = nodeMap.get(connectingFromNodeId);
    const toNode = nodeMap.get(nodeId);
    if (!fromNode || !toNode) { setConnectingFromNodeId(null); setConnectingFromPort(null); return; }

    const duplicate = wires.some(
      (w) =>
        (w.fromNodeId === connectingFromNodeId && w.toNodeId === nodeId
          && w.fromPort === connectingFromPort && w.toPort === port)
        || (w.fromNodeId === nodeId && w.toNodeId === connectingFromNodeId
          && w.fromPort === port && w.toPort === connectingFromPort),
    );
    if (duplicate) {
      addToast({ message: l10n.getString('topology-toast-wire-duplicate'), type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    pushHistory();

    const existingWarehouseWires = wires.filter((w) => {
      const fn = nodeMap.get(w.fromNodeId);
      const tn = nodeMap.get(w.toNodeId);
      return fn?.type === 'workspace' && tn?.type === 'warehouse';
    });

    if (fromNode.type === 'workspace' && toNode.type === 'warehouse' && existingWarehouseWires.length >= 1 && !isProAllowed) {
      addToast({ message: l10n.getString('topology-toast-fallback-warehouse'), type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    const newWireId = `wire-${crypto.randomUUID()}`;
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
    const fromNode = nodeMap.get(connectingFromNodeId);
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
  }, [connectingFromNodeId, connectingFromPort, nodeMap, nodes, zoom, pan, hoveredTarget]);

  const selectedNode = useMemo(() => nodes.find((n) => n.id === selectedNodeId), [nodes, selectedNodeId]);

  // ── Workspace card adapter (ADR #22 Phase 2) ────────────────

  /** Map a workspace node's typeKey to the correct settings card. */
  const renderWorkspaceCard = useCallback((node: TopologyNodeData) => {
    const typeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';
    const cardProps: WorkspaceCardProps = {
      variant: 'inspector-drawer',
      terminalId: node.id,
    };

    switch (typeKey) {
      case 'restaurant-pos':
        return <WorkspaceRestaurantPosSettings key={node.id} {...cardProps} />;
      case 'kds':
        return <WorkspaceKdsSettings key={node.id} {...cardProps} />;
      default:
        return <WorkspaceStorePosSettings key={node.id} {...cardProps} />;
    }
  }, []);

  // ── Live telemetry (ADR #22 Phase 2) ─────────────────────────

  /** Compute live telemetry for a node from SettingsContext. */
  const getTelemetry = useCallback((node: TopologyNodeData): { badge: string; status: 'online' | 'warning' | 'offline' } | null => {
    if (node.type === 'store') {
      return { badge: settings.store.name ? 'Active' : 'Unconfigured', status: settings.store.name ? 'online' : 'warning' };
    }
    if (node.type === 'workspace') {
      const typeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';
      if (typeKey === 'kds') {
        return { badge: 'KDS Ready', status: 'online' };
      }
      return {
        badge: settings.receipt.paperWidth === 'standard' ? 'Receipt ✓' : 'Receipt 58mm',
        status: 'online',
      };
    }
    if (node.type === 'warehouse') {
      // Inventory settings are not yet wired into SettingsContext.
      // When the inventory scope is added (Phase 3+), update this to
      // show live low-stock counts from settings.inventory.
      return { badge: 'Inventory: n/a', status: 'online' };
    }
    return node.telemetryBadge
      ? { badge: node.telemetryBadge, status: node.telemetryStatus ?? 'online' }
      : null;
  }, [settings]);

  /* eslint-disable jsx-a11y/no-static-element-interactions, jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions -- interactive drag/pan canvas requires these */
  return (
    <div className="node-topology-editor">
      {/* ── Confirm delete dialog ── */}
      {confirmDelete !== null && (
        <ConfirmDialog
          open
          onCancel={() => setConfirmDelete(null)}
          onConfirm={executeDelete}
          title={confirmDelete
            ? l10n.getString('topology-confirm-delete-node-title')
            : l10n.getString('topology-confirm-delete-wire-title')}
          message={
            confirmDelete
              ? l10n.getString('topology-confirm-delete-node-msg')
              : l10n.getString('topology-confirm-delete-wire-msg')
          }
          variant="danger"
          confirmLabel={l10n.getString('topology-confirm-delete-label')}
        />
      )}

      {/* ── Confirm preset overwrite dialog ── */}
      {confirmPreset !== null && (
        <ConfirmDialog
          open
          onCancel={() => setConfirmPreset(null)}
          onConfirm={executePresetLoad}
          title={l10n.getString('topology-confirm-preset-title')}
          message={l10n.getString('topology-confirm-preset-msg')}
          variant="warning"
          confirmLabel={l10n.getString('topology-confirm-preset-label')}
        />
      )}

      <div className="node-topology-header">
        <div className="node-topology-header-title">
          <Localized id="topology-builder-title">
            <h2>Visual Store & Workspace Topology Builder</h2>
          </Localized>
          <span className={`topology-tier-badge tier-${currentTier}`}>
            <Localized id="topology-tier-suffix" vars={{ tier: currentTier.toUpperCase() }}>
              {currentTier.toUpperCase()} TIER
            </Localized>
          </span>
        </div>

        <div className="node-topology-header-actions">
          <Button
            variant={isSimulating ? 'primary' : 'secondary'}
            onClick={() => setIsSimulating(!isSimulating)}
            className="simulation-btn"
            icon={isSimulating ? <StopIcon size={16} /> : <FlaskIcon size={16} />}
          >
            <Localized id={isSimulating ? 'topology-sim-stop' : 'topology-sim-start'}>
              {isSimulating ? 'Stop Simulation' : 'Test Order Simulation'}
            </Localized>
          </Button>

          <Button
            variant="secondary"
            onClick={() => { isDirtyRef.current ? setConfirmPreset('retail') : loadPreset('retail'); }}
            icon={<CartIcon size={16} />}
          >
            <Localized id="topology-preset-retail">Retail Preset</Localized>
          </Button>

          <Button
            variant="secondary"
            onClick={() => { isDirtyRef.current ? setConfirmPreset('restaurant') : loadPreset('restaurant'); }}
            icon={<UtensilsIcon size={16} />}
          >
            <Localized id="topology-preset-restaurant">Resto & KDS Preset</Localized>
          </Button>            <Button
              variant="primary"
              onClick={async () => {
                skipNextLoadRef.current = true;
                const idMap = await onSave?.(nodes, wires);
                if (idMap && Object.keys(idMap).length > 0) {
                  // Remap old UUIDs to new UUIDs from archive+recreate
                  // operations so the canvas stays in sync with the backend.
                  // Clear selection to avoid dangling references to old IDs.
                  setSelectedNodeId(null);
                  setSelectedWireId(null);
                  setNodes((prev) =>
                    prev.map((n) => {
                      const newId = idMap[n.id];
                      return newId ? { ...n, id: newId } : n;
                    }),
                  );
                  setWires((prev) =>
                    prev.map((w) => {
                      const newFrom = idMap[w.fromNodeId];
                      const newTo = idMap[w.toNodeId];
                      if (newFrom || newTo) {
                        return {
                          ...w,
                          fromNodeId: newFrom ?? w.fromNodeId,
                          toNodeId: newTo ?? w.toNodeId,
                        };
                      }
                      return w;
                    }),
                  );
                }
              }}
              icon={<CheckIcon size={16} />}
            >
            <Localized id="topology-apply-changes">Apply Topology Changes</Localized>
          </Button>
        </div>
      </div>

      <div className="node-topology-main">
        <div className="node-tool-rack">
          <h3><Localized id="topology-palette-title">Palette Tools</Localized></h3>
          <p className="tool-rack-desc"><Localized id="topology-palette-desc">Drag or click to spawn topology nodes:</Localized></p>

          <button type="button" className="tool-card" onClick={() => handleAddNode('store')}>
            <span className="tool-card-icon"><StoreIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-store">+ Store Node</Localized></strong>
              <span><Localized id="topology-tool-store-desc">Store Branch Profile</Localized></span>
            </div>
          </button>

          <button type="button" className="tool-card" onClick={() => handleAddNode('workspace')}>
            <span className="tool-card-icon"><PosIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-workspace">+ Workspace Node</Localized></strong>
              <span><Localized id="topology-tool-workspace-desc">POS / Register Instance</Localized></span>
            </div>
          </button>

          <button
            className={`tool-card ${!isProAllowed && nodes.some((n) => n.type === 'warehouse') ? 'locked' : ''}`}
            onClick={() => handleAddNode('warehouse')}
          >
            <span className="tool-card-icon"><WarehouseIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-warehouse">+ Warehouse Node</Localized></strong>
              <span><Localized id="topology-tool-warehouse-desc">Storage Location</Localized></span>
            </div>
            {!isProAllowed && nodes.some((n) => n.type === 'warehouse') && (
              <span className="lock-badge"><LockIcon size={12} /> <Localized id="topology-lock-pro">Pro</Localized></span>
            )}
          </button>

          <button type="button" className="tool-card" onClick={() => handleAddNode('hardware')}>
            <span className="tool-card-icon"><PrinterIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-hardware">+ Hardware Node</Localized></strong>
              <span><Localized id="topology-tool-hardware-desc">Printer / KDS Peripheral</Localized></span>
            </div>
          </button>

          <hr className="tool-rack-divider" />

          {selectedNodeId || selectedWireId ? (
            <Button variant="secondary" onClick={handleDeleteRequest} className="delete-btn" icon={<TrashIcon size={16} />}>
              <Localized id="topology-delete-selected">Delete Selected Element</Localized>
            </Button>
          ) : null}

          {history.length > 0 && (
            <Button variant="secondary" onClick={popUndo} style={{ fontSize: 'var(--text-xs)' }}>
              <Localized id="topology-undo">Undo (Ctrl+Z)</Localized>
            </Button>
          )}

          {redo.length > 0 && (
            <Button variant="secondary" onClick={popRedo} style={{ fontSize: 'var(--text-xs)' }}>
              <Localized id="topology-redo">Redo (Ctrl+Y)</Localized>
            </Button>
          )}

          <div className="canvas-controls-mini">
            <Localized id="topology-zoom" vars={{ zoom: Math.round(zoom * 100) }}>
              <span>Zoom: {Math.round(zoom * 100)}%</span>
            </Localized>
            <Button variant="secondary" onClick={zoomToFit}>
              <Localized id="topology-fit-all">Fit All</Localized>
            </Button>
            <Button variant="secondary" onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}>
              <Localized id="topology-reset-view">Reset View</Localized>
            </Button>
          </div>
        </div>

        <div
          ref={canvasRef}
          className="node-canvas-container"
          tabIndex={0}
          role="application"
          aria-label={l10n.getString('topology-canvas-aria-label')}
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
            <svg className="node-wires-svg" style={{ width: svgBounds.width, height: svgBounds.height }}>
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
                // Wire geometry precomputed in wireGeometries useMemo — O(1) lookup vs O(n) find
                const geo = wireGeometries.get(wire.id);
                if (!geo) return null;

                const { x1, y1, x2, y2, dx, pathD, labelX: lx, labelY: ly } = geo;
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
                      transform={`translate(${lx}, ${ly})`}
                      className="wire-label-group"
                      onClick={() => handleToggleWireDirection(wire.id)}
                      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); handleToggleWireDirection(wire.id); } }}
                      role="button"
                      tabIndex={0}
                      aria-label="Toggle wire direction"
                    >
                      <rect x="-55" y="-12" width="110" height="24" rx="12" className="wire-label-bg" />
                      <text x="0" y="4" textAnchor="middle" className="wire-label-text">
                        {wire.direction === 'two-way' ? '\u2194' : '\u2192'} {wire.label || ''}
                      </text>
                    </g>

                    {isSimulating && <SimulationPulse x={pulseX} y={pulseY} />}
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
                  className={`topology-node node-type-${node.type} ${isSelected ? 'node-selected' : ''} ${isConnectingSource ? 'node-connecting-source' : ''}${freshNodeIds.has(node.id) ? ' node-fresh' : ''}`}
                  style={{ left: `${node.x}px`, top: `${node.y}px` }}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') setSelectedNodeId(node.id); }}
                  onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
                >
                  <div className="node-header">
                    <span className="node-type-accent" />
                    <span className="node-grip" aria-hidden="true" title="Drag to move">
                      <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor" aria-hidden="true">
                        <circle cx="9" cy="6" r="1.5" /><circle cx="15" cy="6" r="1.5" />
                        <circle cx="9" cy="12" r="1.5" /><circle cx="15" cy="12" r="1.5" />
                        <circle cx="9" cy="18" r="1.5" /><circle cx="15" cy="18" r="1.5" />
                      </svg>
                    </span>
                    <div className="node-title-wrapper">
                      <span className="node-type-icon">
                        {node.type === 'store' ? <StoreIcon size={16} /> : node.type === 'workspace' ? <PosIcon size={16} /> : node.type === 'warehouse' ? <WarehouseIcon size={16} /> : <PrinterIcon size={16} />}
                      </span>
                      <span className="node-title">{node.name}</span>
                    </div>
                  </div>

                  <div className="node-body">
                    <span className="node-subtitle">{node.subtitle}</span>
                    {(() => {
                      const telemetry = getTelemetry(node);
                      if (!telemetry) {
                        return node.telemetryBadge ? (
                          <span className={`node-telemetry-badge telemetry-${node.telemetryStatus || 'online'}`}>
                            {node.telemetryBadge}
                          </span>
                        ) : null;
                      }
                      return (
                        <span className={`node-telemetry-badge telemetry-${telemetry.status}`}>
                          {telemetry.badge}
                        </span>
                      );
                    })()}
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

          {/* ── Canvas HUD ────────────────────────────────── */}
          <div className="canvas-hud" aria-hidden="true">
            <span className="canvas-hud-item">{Math.round(zoom * 100)}%</span>
            <span className="canvas-hud-divider" />
            <span className="canvas-hud-item">{nodes.length} node{nodes.length !== 1 ? 's' : ''}</span>
            <span className="canvas-hud-divider" />
            <span className="canvas-hud-item">{wires.length} wire{wires.length !== 1 ? 's' : ''}</span>
          </div>
        </div>

        {selectedNode && (
          <div className="node-inspector-drawer">
            <div className="inspector-header">
              <h3><Localized id="topology-inspector-title">Node Inspector</Localized></h3>
              <Button variant="secondary" onClick={() => setSelectedNodeId(null)} icon={<CloseIcon size={14} />} aria-label="Close inspector">{null}</Button>
            </div>

            <div className="inspector-content">
              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
              <label className="inspector-field">
                <span><Localized id="topology-inspector-node-name">Node Name</Localized></span>
                <input
                  type="text"
                  value={selectedNode.name}
                  onChange={(e) => {
                    const name = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, name } : n)));
                  }}
                />
              </label>

              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
              <label className="inspector-field">
                <span><Localized id="topology-inspector-subtitle">Subtitle / Location</Localized></span>
                <input
                  type="text"
                  value={selectedNode.subtitle || ''}
                  onChange={(e) => {
                    const subtitle = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, subtitle } : n)));
                  }}
                />
              </label>

              {/* Workspace type selector + settings card */}
              {selectedNode.type === 'workspace' && (
                <div className="inspector-section">
                  <h4>
                    <Localized id="workspace-type-selector-label">Workspace Type</Localized>
                  </h4>
                  <select
                    className="inspector-select"
                    value={(selectedNode.metadata?.['typeKey'] as string) ?? 'store-pos'}
                    onChange={(e) => {
                      const newTypeKey = e.target.value;
                      setNodes((prev) =>
                        prev.map((n) =>
                          n.id === selectedNode.id
                            ? { ...n, metadata: { ...n.metadata, typeKey: newTypeKey } }
                            : n,
                        ),
                      );
                    }}
                    aria-label="Select workspace type"
                  >
                    {WORKSPACE_TYPE_KEYS.filter((k) => k !== 'warehouse').map((k) => (
                      <option key={k} value={k}>
                        {getWorkspaceTypeLabel(k, l10n)}
                      </option>
                    ))}
                  </select>
                  {renderWorkspaceCard(selectedNode)}
                </div>
              )}
              {selectedNode.type === 'warehouse' && (
                <WorkspaceInventorySettings
                  variant="inspector-drawer"
                  locationId={selectedNode.id}
                />
              )}
              {selectedNode.type === 'store' && (
                <StoreInfoCard variant="inspector-drawer" />
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
/* eslint-enable jsx-a11y/no-static-element-interactions, jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions */
