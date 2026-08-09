import { useState, useMemo, useRef, useEffect, useCallback, memo, type ReactNode } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { Button } from '@/components/Button';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import ErrorBoundary from '@/components/ErrorBoundary';
import { loadTopology } from '@/api/topology';
import { useSettings } from '@/contexts/SettingsContext';
import {
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
  PlusIcon,
  MinusIcon,
  NodesIcon,
  WarningIcon,
} from './NodeTopologyIcons';
import { plainErrorMessage } from '@/utils/app-error';
import {
  clampNodeToViewport,
  edgeAutoPanDelta,
  findFreeSpawnSpot,
  NODE_WIDTH,
  NODE_HEIGHT,
  NODE_PORT_Y,
} from './nodeTopologyClamp';
import { computeAutoLayout } from './nodeTopologyLayout';
import { WarehouseSettingsCard } from './topologyWarehouseCard';
import { pinchTransform, TOUCH_DRAG_THRESHOLD } from './nodeTopologyTouch';
import {
  deserializeTopology,
  serializeTopology,
  saveTemplate,
  loadTemplate,
  listTemplates,
  deleteTemplate,
} from './topologyExport';
import { TopologyNodeCard } from './topologyNodeCard';
import { TopologyWireGroup } from './topologyWireGroup';
import { cubicBezier, polylinePoint } from './topologyWireGeometry';
import {
  normalizeTopologyGraph,
  normalizeWireDirection,
  validateTopologyGraph,
  type TopologyValidationError,
} from './topologyContract';
import {
  leftPortVariants,
  wireRelationshipOptions,
  type WireRelationshipOption,
  NODE_TYPE_ICON,
  workspaceTypeLabel,
  settingsCardForTypeKey,
  topologyUiString,
  sanitizeCopiedNode,
} from './topologyCard';
import './NodeTopologyEditor.css';

// ── Types ──────────────────────────────────────────────────────────

/** Shared stable empty array for cards with no visible validation errors
 *  (a fresh [] per card per render would defeat the card memo). */
const EMPTY_ERRORS: TopologyValidationError[] = [];

export type NodeType = 'store' | 'workspace' | 'warehouse' | 'hardware';
/** Visual flow state of a wire, cycled by clicking it.
 *  'one-way' → left-to-right, 'reverse' → right-to-left,
 *  'two-way' → both. The from/to node ownership is unchanged — this is a
 *  presentation layer over the same semantic edge. */
export type WireDirection = 'one-way' | 'reverse' | 'two-way';

/** Click cycle order for wire direction (1 → 2 → 3 → 1). */
const WIRE_DIRECTION_CYCLE: WireDirection[] = ['one-way', 'reverse', 'two-way'];
export type PortName = 'top' | 'right' | 'bottom' | 'left';

/** Convert legacy vertical anchors to the UX's canonical left/right sides. */
function normalizeVisualPort(port: string | null | undefined, fallback: PortName): PortName {
  if (port === 'top' || port === 'bottom') return fallback;
  if (port === 'left' || port === 'right') return port;
  return fallback;
}
/** Keyboard shortcuts listed in the header's help popover. `key` is the
 *  literal kbd text; `id` is the FTL description key (reuses existing
 *  topology strings where they already name the action). */
const TOPOLOGY_SHORTCUTS: { id: string; key: string }[] = [
  { id: 'topology-shortcuts-help', key: 'F1' },
  { id: 'topology-shortcuts-pan', key: 'Space + Drag' },
  { id: 'topology-shortcuts-duplicate-drag', key: 'Alt + Drag' },
  { id: 'topology-shortcuts-additive-marquee', key: 'Shift + Drag' },
  { id: 'topology-shortcuts-spawn', key: '1–4' },
  { id: 'topology-shortcuts-select-all', key: 'Ctrl+A' },
  { id: 'topology-shortcuts-duplicate', key: 'Ctrl+D' },
  { id: 'topology-shortcuts-copy', key: 'Ctrl+C' },
  { id: 'topology-shortcuts-paste', key: 'Ctrl+V' },
  { id: 'topology-shortcuts-rename', key: 'F2' },
  { id: 'topology-shortcuts-zoom-fit-100', key: 'Ctrl+0 / Ctrl+1' },
  { id: 'topology-shortcuts-zoom-step', key: 'Ctrl++ / Ctrl+-' },
  { id: 'topology-delete-selected', key: 'Del' },
  { id: 'topology-undo', key: 'Ctrl+Z' },
  { id: 'topology-redo', key: 'Ctrl+Y' },
  { id: 'topology-shortcuts-nudge', key: '← ↑ ↓ →' },
  { id: 'topology-shortcuts-esc', key: 'Esc' },
  { id: 'topology-shortcuts-inspector', key: 'Ctrl+I' },
  { id: 'topology-shortcuts-find', key: 'Ctrl+F' },
];

/** Alignment / distribution modes for the multi-select toolbar. */
type AlignMode = 'left' | 'hcenter' | 'right' | 'top' | 'vcenter' | 'bottom' | 'dist-h' | 'dist-v';

const ALIGN_ACTIONS: { mode: AlignMode; ariaId: string }[] = [
  { mode: 'left', ariaId: 'topology-align-left' },
  { mode: 'hcenter', ariaId: 'topology-align-hcenter' },
  { mode: 'right', ariaId: 'topology-align-right' },
  { mode: 'top', ariaId: 'topology-align-top' },
  { mode: 'vcenter', ariaId: 'topology-align-vcenter' },
  { mode: 'bottom', ariaId: 'topology-align-bottom' },
  { mode: 'dist-h', ariaId: 'topology-distribute-h' },
  { mode: 'dist-v', ariaId: 'topology-distribute-v' },
];

/** Node types offered by the right-click canvas context menu. */
const CONTEXT_ADD_TYPES: NodeType[] = ['store', 'workspace', 'warehouse', 'hardware'];

/** Minimap overview widget geometry (bottom-left of the canvas). */
const MINIMAP_W = 176;
const MINIMAP_H = 120;
const MINIMAP_PAD = 8;
const MINIMAP_VIEWPORT_MIN = 8;

/** Orthogonal elbow routing: from the source port, run horizontally to the
 *  midpoint, drop/rise to the target row, then run into the target port.
 *  When the target sits BEHIND the source (reverse flows), detour right
 *  past the source before dropping, so the elbow never folds back through
 *  the source card. Returns the polyline vertices in canvas coords. */
function elbowPoints(x1: number, y1: number, x2: number, y2: number): Array<[number, number]> {
  if (x2 < x1) {
    const detour = x1 + 48;
    return [[x1, y1], [detour, y1], [detour, y2], [x2, y2]];
  }
  const mx = (x1 + x2) / 2;
  return [[x1, y1], [mx, y1], [mx, y2], [x2, y2]];
}

/** SVG path for a polyline of H/V segments. */
function polylineD(pts: Array<[number, number]>): string {
  if (pts.length === 0) return '';
  return `M ${pts[0]![0]} ${pts[0]![1]} ${pts.slice(1).map(([px, py]) => `L ${px} ${py}`).join(' ')}`;
}

/** Point at parameter t (0..1) along an axis-aligned polyline — drives the
 *  simulation pulse so it rides the elbow instead of a phantom curve. */
/** Compact alignment glyphs — three bars whose arrangement encodes the
 *  mode (edges, centers, or even spacing), matching the standard diagram-
 *  tool icon language. */
function AlignGlyph({ mode }: { mode: AlignMode }) {
  let bars: JSX.Element[];
  switch (mode) {
    case 'left':
      bars = [0, 4, 8].map((y) => <rect key={y} x={0} y={y} width={16 - y} height={3} rx={1} fill="currentColor" />);
      break;
    case 'hcenter':
      bars = [0, 4, 8].map((y) => <rect key={y} x={y / 2} y={y} width={16 - y} height={3} rx={1} fill="currentColor" />);
      break;
    case 'right':
      bars = [0, 4, 8].map((y) => <rect key={y} x={y} y={y} width={16 - y} height={3} rx={1} fill="currentColor" />);
      break;
    case 'top':
      bars = [0, 4, 8].map((x) => <rect key={x} x={x} y={0} width={3} height={16 - x} rx={1} fill="currentColor" />);
      break;
    case 'vcenter':
      bars = [0, 4, 8].map((x) => <rect key={x} x={x} y={x / 2} width={3} height={16 - x} rx={1} fill="currentColor" />);
      break;
    case 'bottom':
      bars = [0, 4, 8].map((x) => <rect key={x} x={x} y={x} width={3} height={16 - x} rx={1} fill="currentColor" />);
      break;
    case 'dist-h':
      bars = [0, 6.5, 13].map((x) => <rect key={x} x={x} y={6.5} width={3} height={3} rx={1} fill="currentColor" />);
      break;
    case 'dist-v':
      bars = [0, 6.5, 13].map((y) => <rect key={y} x={6.5} y={y} width={3} height={3} rx={1} fill="currentColor" />);
      break;
  }
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      {bars}
    </svg>
  );
}

export type SemanticRelationshipType =
  | 'location'
  | 'stock-routing'
  | 'ticket-routing'
  | 'hardware-connection'
  | 'inventory-transfer'
  | 'generic';

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
  /** Stable Branch Location identity when this node is a store alias. */
  storeProfileId?: string;
}

export interface TopologyWireData {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  direction: WireDirection;
  label?: string;
  /** Which port on the source node the wire originates from (default: 'right'). */
  fromPort?: PortName;
  /** Which port on the target node the wire connects to (default: 'left'). */
  toPort?: PortName;
  /** Orthogonal bend points (absolute canvas coords) the wire routes
   *  through, in order from source to target. User-authored geometry that
   *  replaces the auto curve/elbow when present; persisted with the diagram. */
  bends?: Array<{ x: number; y: number }>;
  /** Semantic source port; geometry remains presentation-only. */
  fromPortId?: string;
  /** Semantic target port; geometry remains presentation-only. For nodes
   *  with stacked left inputs (inventory: 'location-in' | 'operation-in'),
   *  this doubles as the slot discriminator — the renderer resolves the
   *  vertical socket from it and the backend round-trips it as to_port_id. */
  toPortId?: string;
  /** Typed relationship represented by this wire. */
  relationshipType?: SemanticRelationshipType;
}

export interface BranchLocationSeed {
  /** Canonical store_profiles.id. */
  id: string;
  /** User-visible location name. */
  name: string;
}

export interface WorkspaceInstanceSeed {
  /** Instance id from workspace_instances — becomes the node id. */
  instanceId: string;
  /** Workspace type key (store-pos, restaurant-pos, kds, warehouse). */
  typeKey: string;
  /** Controlled business purpose, independent from type and instance label. */
  purposeKey?: string;
  /** Canonical Branch Location identity for ownership compilation. */
  storeId?: string;
  /** Branch Location display name used only for presentation. */
  storeName?: string;
  name: string;
  subtitle?: string;
  colour?: string;
}

export interface NodeTopologyEditorProps {
  currentTier?: 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise';
  /**
   * Optional toolbar content rendered inside the topology header, above the
   * title/actions row. The parent screen uses this slot to merge its branch
   * (graph) selector toolbar into the editor's header instead of rendering a
   * separate stacked bar.
   */
  branchToolbar?: ReactNode;
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
  /** Branch Locations available to seed the ownership graph. */
  branchLocations?: BranchLocationSeed[];
  /** Persist a Branch Location (store profile) rename from the node card.
   *  Resolves true on success so the card can close its inline form;
   *  false keeps the draft open for a retry (the parent toasts errors). */
  onRenameBranch?: (id: string, name: string) => Promise<boolean> | boolean | void;
  /** Persist a workspace instance rename from the node card (the live
   *  instance row, not just the canvas label). Same contract as
   *  onRenameBranch: true closes the form, false keeps the draft open. */
  onRenameWorkspace?: (id: string, name: string) => Promise<boolean> | boolean | void;
  /** Allow Apply before the parent supplies real branch identities. */
  allowLegacyApply?: boolean;
  /** Branch (graph) identity for viewport memory — pan/zoom persist per
   *  branch so switching branches (which remounts the editor) lands back
   *  where the user left off instead of resetting to identity. The parent
   *  passes the same value it uses to key the remount. */
  branchId?: string;
  /** Reports the canvas dirty state upward (true after any edit, false after
   *  Apply/undo-to-snapshot). The parent uses it to guard branch switches
   *  against silently discarding unsaved edits — the editor cannot veto its
   *  own remount, so the guard must live in the parent. */
  onDirtyChange?: (dirty: boolean) => void;
}

/** Valid workspace type keys selectable when creating a workspace node.
 *  Labels are resolved at render time via l10n.getString for i18n. */
const WORKSPACE_TYPE_KEYS = ['store-pos', 'restaurant-pos', 'kds', 'warehouse'] as const;

// ── Presets ────────────────────────────────────────────────────────

const PRESET_RETAIL: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    // Cards are 240px tall: workspace rows sit at y 80/320 (240px apart on
    // the 24px grid) so no cards overlap on first load. The store keeps its
    // historical (80, 140) position — the geometry tests pin it.
    { id: 'store-1', type: 'store', name: 'Downtown Branch', subtitle: 'Primary Store', x: 80, y: 140, telemetryBadge: 'Online (2 POS)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Retail POS #1', subtitle: 'Main Checkout', x: 380, y: 80, metadata: { typeKey: 'store-pos' }, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-1', type: 'warehouse', name: 'Main Stock Room', subtitle: 'Primary Storage', x: 680, y: 140, telemetryBadge: '1,250 items', telemetryStatus: 'online' },
  ],
  wires: [
    // Natural left-to-right flow: store right → workspace left, workspace right → warehouse left
    { id: 'w-1', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-1', toPort: 'left', fromPortId: 'location-out', toPortId: 'location-in', relationshipType: 'location', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'ws-1', fromPort: 'right', toNodeId: 'wh-1', toPort: 'left', fromPortId: 'stock-out', toPortId: 'stock-in', relationshipType: 'stock-routing', direction: 'one-way', label: 'Stock Deduct (P1)' },
  ],
};

const PRESET_RESTAURANT: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    // 240px cards on a two-row grid (workspace y: 80 / 320, x: 380 / 680)
    // so nothing overlaps on load. The store keeps its historical
    // (80, 180) position — geometry tests pin the retail preset only.
    { id: 'store-1', type: 'store', name: 'Grand Bistro', subtitle: 'Main Branch', x: 80, y: 180, telemetryBadge: 'Online (3 Terminals)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Resto POS #1', subtitle: 'Dining Room', x: 380, y: 80, metadata: { typeKey: 'restaurant-pos' }, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'ws-kds', type: 'workspace', name: 'Kitchen KDS', subtitle: 'Line Cook Display', x: 380, y: 320, metadata: { typeKey: 'kds' }, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-kitchen', type: 'warehouse', name: 'Kitchen Pantry', subtitle: 'Cold & Dry Storage', x: 680, y: 80, telemetryBadge: '⚠️ 12 Low Stock', telemetryStatus: 'warning' },
    { id: 'hw-prn', type: 'hardware', name: 'Kitchen Thermal Printer', subtitle: 'LAN 192.168.1.100', x: 680, y: 320, telemetryBadge: 'Ready', telemetryStatus: 'online' },
  ],
  wires: [
    // Left-to-right: store right → workspace left; then workspace right → warehouse/printer left
    { id: 'w-1', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-1', toPort: 'left', fromPortId: 'location-out', toPortId: 'location-in', relationshipType: 'location', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'store-1', fromPort: 'right', toNodeId: 'ws-kds', toPort: 'left', fromPortId: 'location-out', toPortId: 'location-in', relationshipType: 'location', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-3', fromNodeId: 'ws-1', fromPort: 'right', toNodeId: 'wh-kitchen', toPort: 'left', fromPortId: 'stock-out', toPortId: 'stock-in', relationshipType: 'stock-routing', direction: 'one-way', label: 'Stock Deduct' },
    { id: 'w-4', fromNodeId: 'ws-kds', fromPort: 'right', toNodeId: 'hw-prn', toPort: 'left', fromPortId: 'ticket-out', toPortId: 'ticket-in', relationshipType: 'ticket-routing', direction: 'one-way', label: 'Ticket Print' },
  ],
};

/**
 * Exact dirty check: true when two canvas states differ in their PERSISTED
 * fields. Transient fields are excluded — telemetryBadge/telemetryStatus
 * (never edited), and metadata.persisted (an internal sync bookkeeping flag
 * flipped by the save-triggered instance reload, not user content). Projecting
 * to a fixed key order keeps the JSON comparison order-stable across the
 * spread-based mutation paths (drag, inspector, idMap remap, preset load).
 *
 * The persisted-field set is TRIPLE-COUPLED: it lives here, in the load
 * effect's backend mapping, and in the onSave serialization. Adding a new
 * persisted field must touch all three, or the dirty check silently weakens.
 */
function canvasStateEqual(
  aNodes: TopologyNodeData[],
  aWires: TopologyWireData[],
  bNodes: TopologyNodeData[],
  bWires: TopologyWireData[],
): boolean {
  if (aNodes.length !== bNodes.length || aWires.length !== bWires.length) return false;
  const projNodes = (ns: TopologyNodeData[]) =>
    ns.map((n) => ({
      id: n.id,
      type: n.type,
      name: n.name,
      subtitle: n.subtitle,
      x: n.x,
      y: n.y,
      ...(n.tierRequirement !== undefined ? { tierRequirement: n.tierRequirement } : {}),
      // metadata is typed with an index signature — bracket access required.
      ...(n.metadata ? {
        metadata: {
          typeKey: n.metadata['typeKey'],
          purposeKey: n.metadata['purposeKey'],
          enabled: n.metadata['enabled'],
          capacity: n.metadata['capacity'],
          lowStockThreshold: n.metadata['lowStockThreshold'],
        },
      } : {}),
    }));
  const projWires = (ws: TopologyWireData[]) =>
    ws.map((w) => ({
      id: w.id,
      fromNodeId: w.fromNodeId,
      toNodeId: w.toNodeId,
      ...(w.fromPort !== undefined ? { fromPort: w.fromPort } : {}),
      ...(w.toPort !== undefined ? { toPort: w.toPort } : {}),
      direction: w.direction,
      ...(w.label !== undefined ? { label: w.label } : {}),
      ...(w.bends !== undefined ? { bends: w.bends } : {}),
    }));
  return JSON.stringify(projNodes(aNodes)) === JSON.stringify(projNodes(bNodes))
    && JSON.stringify(projWires(aWires)) === JSON.stringify(projWires(bWires));
}

/** Canvas px the dragged node's edge/center may drift from a stationary
 *  node's before the alignment guide snaps it (Figma-style). */
const ALIGN_THRESHOLD = 6;

interface AlignmentResult {
  /** Guide line coordinate (canvas units) for a vertical (x) / horizontal
   *  (y) guide — present only while that axis is actively aligned. */
  x?: number;
  y?: number;
  /** Delta applied to every dragged node so the primary lands exactly on
   *  the aligned axis (keeps the group rigid). Zero when not aligned. */
  dx: number;
  dy: number;
  /** Whether each axis is under an active alignment — the aligned axis
   *  skips grid snapping (guides beat the grid). */
  alignedX: boolean;
  alignedY: boolean;
}

/** Figma-style COLLECTIVE alignment snap: match ANY edge/center of ANY
 *  dragged node against ANY edge/center of every STATIONARY node (all 9
 *  combos per node pair — left↔left, right↔left, centerX↔centerX, …).
 *  Within the threshold, the closest match across ALL dragged members wins
 *  per axis — so a group can snap on a non-grabbed member's edge, exactly
 *  like Figma. The resulting delta shifts the whole group rigidly. The
 *  dragged set is excluded from the reference pool so a group never aligns
 *  to itself. */
function computeAlignmentGuides(
  targets: Map<string, { x: number; y: number }>,
  draggedIds: Set<string>,
  nodes: TopologyNodeData[],
): AlignmentResult {
  let bestX: { target: number; dist: number } | null = null;
  let bestY: { target: number; dist: number } | null = null;
  for (const other of nodes) {
    if (draggedIds.has(other.id)) continue;
    const rAxesX = [other.x, other.x + NODE_WIDTH / 2, other.x + NODE_WIDTH];
    const rAxesY = [other.y, other.y + NODE_HEIGHT / 2, other.y + NODE_HEIGHT];
    for (const target of targets.values()) {
      const pAxesX = [target.x, target.x + NODE_WIDTH / 2, target.x + NODE_WIDTH];
      const pAxesY = [target.y, target.y + NODE_HEIGHT / 2, target.y + NODE_HEIGHT];
      for (const pAxis of pAxesX) {
        for (const rAxis of rAxesX) {
          const dx = pAxis - rAxis;
          if (Math.abs(dx) <= ALIGN_THRESHOLD && (!bestX || Math.abs(dx) < bestX.dist)) {
            bestX = { target: rAxis, dist: dx };
          }
        }
      }
      for (const pAxis of pAxesY) {
        for (const rAxis of rAxesY) {
          const dy = pAxis - rAxis;
          if (Math.abs(dy) <= ALIGN_THRESHOLD && (!bestY || Math.abs(dy) < bestY.dist)) {
            bestY = { target: rAxis, dist: dy };
          }
        }
      }
    }
  }
  return {
    ...(bestX ? { x: bestX.target } : {}),
    ...(bestY ? { y: bestY.target } : {}),
    dx: bestX?.dist ?? 0,
    dy: bestY?.dist ?? 0,
    alignedX: bestX !== null,
    alignedY: bestY !== null,
  };
}

/** True when the diagram's bounding box (plus zoomToFit's breathing room)
 *  exceeds the MEASURED canvas viewport — the trigger for the one-shot
 *  load auto-fit. A zero/negative measured size (jsdom, pre-layout) returns
 *  false so the identity view is never yanked by a phantom constraint. */
function diagramOverflowsCanvas(canvas: HTMLElement, nodes: TopologyNodeData[]): boolean {
  const viewW = canvas.clientWidth;
  const viewH = canvas.clientHeight;
  if (viewW <= 0 || viewH <= 0 || nodes.length === 0) return false;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const n of nodes) {
    minX = Math.min(minX, n.x);
    minY = Math.min(minY, n.y);
    maxX = Math.max(maxX, n.x + NODE_WIDTH);
    maxY = Math.max(maxY, n.y + NODE_HEIGHT);
  }
  const pad = 60;
  return maxX - minX + pad * 2 > viewW || maxY - minY + pad * 2 > viewH;
}

/** Port offset from node origin (left, top) for each port name.
 *  Left/right wire endpoints sit on the card edge, exactly at the center
 *  of the visible connector circles. The hit areas may overhang, but the
 *  geometry contract never does. */
const PORT_OFFSET: Record<PortName, { dx: number; dy: number }> = {
  // Kept for legacy loaded wires. New UX renders and authorizes only left/right.
  top:    { dx: NODE_WIDTH / 2, dy: -6 },
  right:  { dx: NODE_WIDTH, dy: NODE_PORT_Y },
  bottom: { dx: NODE_WIDTH / 2, dy: NODE_HEIGHT + 6 },
  left:   { dx: 0,             dy: NODE_PORT_Y },
};

/** Canvas-space dy (top edge) for a left port — all nodes share the rail
 *  centerline now that stacked inputs are gone. */
function leftPortDy(_node: TopologyNodeData, _variantIndex: number): number {
  return NODE_PORT_Y;
}

/** Evaluate a cubic bezier at parameter t (0-1). */
const GRID_SIZE = 24;
const snap = (v: number) => Math.round(v / GRID_SIZE) * GRID_SIZE;
type HistoryEntry = { nodes: TopologyNodeData[]; wires: TopologyWireData[] };

/** Stable keys identifying a validation issue for mark-issue-resolved
 *  persistence: a node issue is scoped by its card + message, a graph-level
 *  issue by its message alone. Module-scope so every surface (panel, banner,
 *  card notes) derives the same key from the same error. */
const issueKey = (nodeId: string, messageId: string) => `node:${nodeId}:${messageId}`;
const graphIssueKey = (messageId: string) => `graph:${messageId}`;

/** Milliseconds the issues-count readout waits after the LAST validation
 *  change before animating to the new count. Long enough to absorb the
 *  flicker of a drag or connect gesture that temporarily changes the
 *  issue set, short enough to feel responsive. */
const ISSUES_COUNT_SETTLE_MS = 300;

/** HUD cursor-position readout, isolated in its own memo component with
 *  its own document mousemove listener and rAF throttle. The readout is
 *  display-only, so a burst of moves coalesces into at most ONE state
 *  update per frame — and that update is LOCAL to this span, so pointer
 *  movement over a large diagram never re-renders the editor (which used
 *  to re-render every node card and wire path up to 60×/sec). pan/zoom
 *  come in as props so the conversion to canvas coords stays current. */
const CanvasCursorReadout = memo(function CanvasCursorReadout({ pan, zoom }: { pan: { x: number; y: number }; zoom: number }) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const pendingRef = useRef<{ x: number; y: number } | null>(null);
  const rafRef = useRef<number | null>(null);
  const elRef = useRef<HTMLSpanElement>(null);
  // Mount-once listener: pan/zoom are read via refs inside the handler so a
  // pan/zoom change never re-arms (and cancels a pending) rAF. Re-arming on
  // every pan would ALSO cancel an in-flight frame — leaving the readout
  // stuck until the next move re-schedules.
  const panRef = useRef(pan);
  panRef.current = pan;
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      // The readout lives inside the canvas container; its rect is the
      // viewport origin for the pan/zoom conversion.
      const rect = elRef.current?.closest('.node-canvas-container')?.getBoundingClientRect();
      if (!rect) return;
      pendingRef.current = {
        x: Math.round((e.clientX - rect.left - panRef.current.x) / zoomRef.current),
        y: Math.round((e.clientY - rect.top - panRef.current.y) / zoomRef.current),
      };
      if (rafRef.current === null) {
        rafRef.current = requestAnimationFrame(() => {
          rafRef.current = null;
          setPos(pendingRef.current);
        });
      }
    };
    document.addEventListener('mousemove', onMove);
    return () => {
      document.removeEventListener('mousemove', onMove);
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  return (
    <span ref={elRef} className="canvas-hud-item canvas-hud-cursor">
      {pos ? `${pos.x}, ${pos.y}` : '—'}
    </span>
  );
});

/** Settled issues-count readout for the validation button. Receives the
 *  LIVE count on every validation recompute but only commits it (with a
 *  pop animation) once the value holds steady for
 *  [`ISSUES_COUNT_SETTLE_MS`] — a drag that flicks 1→2→1 never animates
 *  twice. Isolated as a memo component so the settle timer's re-renders
 *  are local to this label and never touch the canvas. */
const ValidationIssuesLabel = memo(function ValidationIssuesLabel({ count }: { count: number }) {
  const { l10n } = useLocalization();
  const [displayCount, setDisplayCount] = useState(count);
  const settleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevCountRef = useRef(count);

  useEffect(() => {
    if (count === prevCountRef.current) return;
    prevCountRef.current = count;
    if (settleTimerRef.current) clearTimeout(settleTimerRef.current);
    settleTimerRef.current = setTimeout(() => {
      settleTimerRef.current = null;
      setDisplayCount(count);
    }, ISSUES_COUNT_SETTLE_MS);
  }, [count]);

  useEffect(
    () => () => {
      if (settleTimerRef.current) clearTimeout(settleTimerRef.current);
    },
    [],
  );

  // Re-keying on the settled count remounts the span so the pop keyframe
  // replays exactly when the readout settles on a new value.
  return (
    <span key={displayCount} className="topology-issues-label topology-issues-label-pop">
      {l10n.getString('topology-validation-details', { count: displayCount })}
    </span>
  );
});

/** An ambiguous wire drop in flight: the source socket and the target
 *  socket admit MULTIPLE relationships (ADR #34), so the editor asks the
 *  user which one the wire means before drawing anything. */
interface RelationshipPickerState {
  fromNodeId: string;
  fromPort: PortName;
  toNodeId: string;
  toPort: PortName;
  options: WireRelationshipOption[];
}

/** Validate the editor's RAW canvas under the Apply gate. Legacy/demo
 *  canvases (no canonical branch identity) keep their non-blocking path
 *  unless the real topology screen opts into strict validation
 *  (allowLegacyApply=false). Shared by the live badge surface AND the
 *  Apply handler so the two can never drift apart. */
function validateEditorGraph(
  nodes: TopologyNodeData[],
  wires: TopologyWireData[],
  allowLegacyApply: boolean,
  tier: string,
): TopologyValidationError[] {
  const semanticGraph = normalizeTopologyGraph(nodes, wires);
  const hasCanonicalBranchIdentity = semanticGraph.nodes.some(
    (node) => node.kind === 'branch-location' && node.storeProfileId !== undefined,
  );
  const errors = hasCanonicalBranchIdentity || !allowLegacyApply
    ? validateTopologyGraph(semanticGraph)
    : [];
  // The multi-warehouse tier cap is an Apply-gate invariant that the
  // creation paths also enforce (every spawn/duplicate path refuses a
  // second warehouse below Pro). Enforcing it here catches the remaining
  // routes in: a tier downgrade while a 2-warehouse diagram exists, or a
  // loaded legacy diagram — so Apply can never persist 2+ warehouses on a
  // non-Pro install.
  if (!['pro', 'enterprise'].includes(tier)) {
    const warehouseCount = semanticGraph.nodes.filter((n) => n.kind === 'warehouse').length;
    if (warehouseCount >= 2) {
      errors.push({ code: 'warehouse-tier-limit', messageId: 'topology-toast-multi-warehouse' });
    }
  }
  return errors;
}

export default function NodeTopologyEditor({
  currentTier = 'standard',
  onSave,
  workspaceInstances,
  branchLocations,
  onRenameBranch,
  onRenameWorkspace,
  allowLegacyApply = true,
  branchToolbar,
  branchId,
  onDirtyChange,
}: NodeTopologyEditorProps) {
  const { sessionToken } = useWorkspace();
  const { addToast } = useToast();
  const { l10n } = useLocalization();
  /** Latest l10n for ref-based callbacks (duplicate commit/cancel) so the
   *  announcement strings always come from the current bundle. */
  const l10nRef = useRef(l10n);
  l10nRef.current = l10n;
  const { settings } = useSettings();
  const canvasRef = useRef<HTMLDivElement>(null);

  const [nodes, setNodes] = useState<TopologyNodeData[]>(PRESET_RETAIL.nodes);
  /** True once the first authoritative topology load has settled. The editor
   *  mounts on the retail preset while the async load is in flight, and the
   *  validation-issue dismissal forget-effect must not treat that placeholder
   *  graph as the real diagram (it would drop restored dismissals on every
   *  reload). Set in the load chain's finally, after every branch settles. */
  const [topologyLoaded, setTopologyLoaded] = useState(false);
  const [wires, setWires] = useState<TopologyWireData[]>(PRESET_RETAIL.wires);

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  /** Full multi-selection set; selectedNodeId is the primary (inspector
   *  target / last-picked). All selection writes go through selectOnly /
   *  clearSelection so the two can never disagree. */
  const [selectedNodeIds, setSelectedNodeIds] = useState<Set<string>>(new Set());
  /** Render-time mirror so the memoized card handlers read the CURRENT
   *  selection without taking it as a useCallback dep (a dep would churn
   *  the handler identity on every selection change and defeat the card
   *  memo for unrelated cards). */
  const selectedNodeIdsRef = useRef<Set<string>>(selectedNodeIds);
  selectedNodeIdsRef.current = selectedNodeIds;
  const [selectedWireId, setSelectedWireId] = useState<string | null>(null);

  /** Replace the whole node selection with a single primary node. */
  const selectOnly = useCallback((id: string) => {
    setSelectedNodeIds(new Set([id]));
    setSelectedNodeId(id);
  }, []);
  /** Clear the node selection entirely (wire selection untouched). Stable
   *  so the memoized wire/card layers can receive it as a prop. */
  const clearSelection = useCallback(() => {
    setSelectedNodeIds(new Set());
    setSelectedNodeId(null);
  }, []);

  const [isSimulating, setIsSimulating] = useState(false);
  const [simPulseStep, setSimPulseStep] = useState(0);

  /** Set of node ids being dragged together (a multi-selection drags as
   *  one group; each node keeps its own pointer offset). */
  const [draggingNodeIds, setDraggingNodeIds] = useState<Set<string>>(new Set());
  const dragOffsetsRef = useRef<Map<string, { x: number; y: number }>>(new Map());
  /** Alt+drag duplicate mode: true while an in-flight drag is duplicating
   *  (the copies follow the cursor, the originals stay). Committed (one
   *  undo entry) on mouseup, cancelled by Escape. */
  const duplicateDragRef = useRef(false);
  /** Ids of the live duplicate copies during an Alt+drag (the drag set). */
  const duplicateCopyIdsRef = useRef<string[]>([]);
  /** True when an Alt+drag was converted MID-move: the move's history entry
   *  (pushed at first movement) IS the pre-drag state, so the commit must
   *  not push a duplicate entry and the cancel must pop it. */
  const duplicateHistoryPushedRef = useRef(false);
  /** Live alignment guide lines while dragging: the snapped edge/center
   *  coordinate (canvas units) for a vertical (x) and/or horizontal (y)
   *  guide. Null while idle. Cleared on mouseup. */
  const [alignmentGuide, setAlignmentGuide] = useState<{ x?: number; y?: number } | null>(null);
  /** Accessible snap feedback: the alignment guides are aria-hidden, so a
   *  visually-hidden live region announces when a drag/nudge SNAPS. The
   *  announcement fires on ENTRY only (null → guide); while the guide stays
   *  visible (snapped), the recreated guide object must not re-announce on
   *  every mousemove — the mouseup clear resets the latch so the next
   *  approach re-announces. */
  const [liveAnnouncement, setLiveAnnouncement] = useState('');
  const prevGuideRef = useRef<{ x?: number; y?: number } | null>(null);
  useEffect(() => {
    const prev = prevGuideRef.current;
    prevGuideRef.current = alignmentGuide;
    if (alignmentGuide && !prev) {
      setLiveAnnouncement(l10nRef.current.getString('topology-snap-announce'));
    }
  }, [alignmentGuide]);
  /** Marquee box selection: null while idle, a rect in container-relative
   *  screen px while left-dragging on empty background. */
  const [marquee, setMarquee] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  /** Mirror of the rendered marquee rect so the document-level finalizer
   *  (armed at mousedown) always reads the LATEST box — refs never go
   *  stale across renders, and a release event may carry no pointer coords. */
  const marqueeRef = useRef<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const marqueeStartRef = useRef<{ x: number; y: number } | null>(null);
  /** True while a marquee drag runs with Shift held — its result UNIONs into
   *  the pre-drag selection at release instead of replacing it. Reset by the
   *  finalizer (or the next mousedown), so it can never leak across drags. */
  const marqueeAdditiveRef = useRef(false);
  /** Cancels an in-flight marquee when the pointer is released outside the
   *  canvas — the canvas onMouseUp never fires there, so without a
   *  document-level listener the box would linger and the next mousemove
   *  would re-open it. Mirrors dragCleanupRef for node drags. */
  const marqueeCleanupRef = useRef<(() => void) | null>(null);
  /** Set once a drag has actually moved the node — history is pushed on the
   *  first movement, not on mousedown, so a plain click-to-select never
   *  creates a no-op undo entry or marks the canvas dirty. */
  const dragHasMovedRef = useRef(false);
  /** Pre-drag positions of the nodes in an in-flight MOVE (filled at node
   *  mousedown, cleared on mouseup/cancel). Escape mid-move restores these
   *  so the drag snaps back to where it started (Figma semantics). Empty
   *  while idle — which is also what distinguishes "a move is in flight"
   *  from a plain Escape in the keydown handler. */
  const dragStartRef = useRef<Map<string, { x: number; y: number }>>(new Map());
  /** In-flight bend drag: which wire, which bend index, its pre-drag
   *  position (Escape restores it), whether it has moved yet (history is
   *  pushed on first movement, one entry per drag), and whether the bend
   *  was CREATED by this gesture (a ghost insert — cancel then removes it
   *  entirely instead of restoring). */
  const bendDragRef = useRef<{
    wireId: string;
    index: number;
    moved: boolean;
    startX: number;
    startY: number;
    created: boolean;
  } | null>(null);
  /** Document-listener cleanup for a bend drag (minimap pattern) — the
   *  drag must keep tracking when the pointer leaves the handle. */
  const bendDragCleanupRef = useRef<(() => void) | null>(null);
  /** Set of node ids that were just added (for scale-in animation). */
  const [freshNodeIds, setFreshNodeIds] = useState<Set<string>>(new Set());
  /** Timers for fresh-node animation cleanup; cleared on unmount to prevent leaks. */
  const freshTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  /** Per-branch viewport memory: pan/zoom persist per branch id so a branch
   *  switch (which remounts the editor) lands back where the user left off
   *  instead of resetting to identity. 'unassigned' mirrors the parent's
   *  key fallback for diagrams with no selected branch. */
  const viewKey = `oz-topology-viewport:${branchId ?? 'unassigned'}`;
  /** Lazy mount-time read of the saved view. Restoring one marks the view as
   *  user-owned so the auto-fit effect (which fits overflowing NEW diagrams)
   *  never yanks a saved position. */
  const [savedView] = useState<{ zoom: number; pan: { x: number; y: number } } | null>(() => {
    try {
      const raw = localStorage.getItem(viewKey);
      if (raw) {
        const parsed = JSON.parse(raw) as { zoom?: number; pan?: { x: number; y: number } } | null;
        if (parsed && typeof parsed.zoom === 'number' && parsed.pan
          && typeof parsed.pan.x === 'number' && typeof parsed.pan.y === 'number') {
          return { zoom: parsed.zoom, pan: parsed.pan };
        }
      }
    } catch { /* corrupted view — fall back to identity */ }
    return null;
  });
  const restoredViewRef = useRef(savedView !== null);
  const [zoom, setZoom] = useState(() => (savedView ? Math.max(0.4, Math.min(2.0, savedView.zoom)) : 1));
  const [pan, setPan] = useState<{ x: number; y: number }>(() => savedView?.pan ?? { x: 0, y: 0 });

  useEffect(() => {
    try {
      localStorage.setItem(viewKey, JSON.stringify({ zoom, pan }));
    } catch { /* storage may be unavailable (private mode) — view pref only */ }
  }, [viewKey, zoom, pan]);

  /** Node finder (Ctrl+F): a quick-jump overlay. finderQuery drives the
   *  filtered match list; finderIndex is the highlighted row (clamped to the
   *  list at render). The input owns its own keydown (Esc closes, arrows
   *  move, Enter jumps); while the overlay is open, a canvas-focus Escape
   *  closes it too (see the keydown effect). */
  const [finderOpen, setFinderOpen] = useState(false);
  const [finderQuery, setFinderQuery] = useState('');
  const [finderIndex, setFinderIndex] = useState(0);
  const finderInputRef = useRef<HTMLInputElement>(null);
  /** Latest zoom for ref-based math (finder centering) without re-arming
   *  document listeners. */
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;

  useEffect(() => {
    if (finderOpen) finderInputRef.current?.focus();
  }, [finderOpen]);

  /** Nodes matching the finder query (name or subtitle, case-insensitive).
   *  An empty query lists every node so Enter always has a target. */
  const finderMatches = useMemo(() => {
    const q = finderQuery.trim().toLowerCase();
    if (!q) return nodes;
    return nodes.filter((n) => n.name.toLowerCase().includes(q) || (n.subtitle ?? '').toLowerCase().includes(q));
  }, [nodes, finderQuery]);

  /** Jump the viewport to a finder match: select it, center it at the
   *  current zoom, and close the overlay. */
  const jumpToFinderMatch = useCallback((match: TopologyNodeData) => {
    selectOnly(match.id);
    const canvas = canvasRef.current;
    if (canvas) {
      setPan({
        x: canvas.clientWidth / 2 - (match.x + NODE_WIDTH / 2) * zoomRef.current,
        y: canvas.clientHeight / 2 - (match.y + NODE_HEIGHT / 2) * zoomRef.current,
      });
    }
    setFinderOpen(false);
  }, [selectOnly]);
  /** Wire routing style: smooth cubic beziers (default) or orthogonal
   *  elbow segments. Persisted per diagram (branch) in localStorage — the
   *  same key scheme as the viewport memory and minimap — so each diagram
   *  keeps its own routing across branch switches and reloads. A legacy
   *  per-install value is inherited once when no per-diagram choice exists
   *  yet (the write-back effect then migrates it to the branch key). */
  const routingKey = `oz-topology-view-routing:${branchId ?? 'unassigned'}`;
  const [wireRouting, setWireRouting] = useState<'curved' | 'elbow'>(() => {
    try {
      const saved = localStorage.getItem(routingKey);
      const value = saved ?? localStorage.getItem('oz-topology-view-routing');
      return value === 'elbow' ? 'elbow' : 'curved';
    } catch {
      return 'curved';
    }
  });
  const snapKey = `oz-topology-view-snap:${branchId ?? 'unassigned'}`;
  /** Snap interactive placement (drag/nudge/spawn) to the 24px grid.
   *  Persisted per branch alongside the routing preference; a legacy
   *  per-install value is inherited once when no per-branch choice exists. */
  const [snapEnabled, setSnapEnabled] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem(snapKey);
      const value = saved ?? localStorage.getItem('oz-topology-view-snap');
      return value !== '0';
    } catch {
      return true;
    }
  });
  /** Pan tool: while active, left-drags on the empty canvas pan instead of
   *  marqueeing — the touchscreen-friendly twin of Space+drag. */
  const [panToolActive, setPanToolActive] = useState(false);
  /** Wire label pills: optional permanent labels at each wire's midpoint
   *  (clicking one opens the round-20 rename editor). Persisted with the
   *  other view prefs; default off to keep the current clean look. */
  const wireLabelsKey = `oz-topology-view-wire-labels:${branchId ?? 'unassigned'}`;
  const [wireLabelsVisible, setWireLabelsVisible] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem(wireLabelsKey);
      const value = saved ?? localStorage.getItem('oz-topology-view-wire-labels');
      return value === '1';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(wireLabelsKey, wireLabelsVisible ? '1' : '0');
    } catch { /* storage may be unavailable (private mode) — view pref only */ }
  }, [wireLabelsKey, wireLabelsVisible]);

  /** Any wire carrying authored bends. The elbow/curved toggle then applies
   *  only to UNBENT wires (authored geometry wins), so the View rack shows
   *  an override note instead of letting the toggle silently lie. */
  const anyBentWires = wires.some((w) => (w.bends?.length ?? 0) > 0);

  useEffect(() => {
    try {
      localStorage.setItem(routingKey, wireRouting);
    } catch { /* storage may be unavailable (private mode) — view pref only */ }
  }, [routingKey, wireRouting]);

  useEffect(() => {
    try {
      localStorage.setItem(snapKey, snapEnabled ? '1' : '0');
    } catch { /* storage may be unavailable (private mode) — view pref only */ }
  }, [snapKey, snapEnabled]);
  /** Grid-aware placement: identity when the snap toggle is off. */
  const snapOrNot = (v: number) => (snapEnabled ? snap(v) : v);
  /** Live cursor position in canvas coords while a connection is in flight
   *  — drives the wire preview so it follows the pointer, not just the
   *  last hovered target. */
  const [previewCursor, setPreviewCursor] = useState<{ x: number; y: number } | null>(null);
  /** Latest pointer position for the in-flight wire preview (the
   *  connection line follows the last cursor position when no socket is
   *  hovered). Ref-only — the readout no longer needs it; CanvasCursorReadout
   *  owns its own listener and rAF. */
  const mousePosRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const isPanningRef = useRef(false);
  const panStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const panCleanupRef = useRef<(() => void) | null>(null);
  /** Space held → the next left-drag pans (Figma-style) instead of
   *  marqueeing. Mirrored in state purely for the grab cursor class. */
  const spaceDownRef = useRef(false);
  const [spacePanArmed, setSpacePanArmed] = useState(false);

  useEffect(() => {
    // Space is the pan modifier. It must NOT arm while typing or when a
    // control owns the key (e.g. the wire hitbox's Space cycle-to-direction
    // — a focused role=button keeps its own Space behavior).
    const isTypingOrControl = (t: EventTarget | null) =>
      t instanceof HTMLElement && !!t.closest('input, textarea, select, [contenteditable="true"], button, [role="button"]');
    const down = (e: KeyboardEvent) => {
      if (e.code !== 'Space') return;
      if (isTypingOrControl(e.target)) return;
      e.preventDefault(); // keep the page from scrolling while arming pan
      spaceDownRef.current = true;
      setSpacePanArmed(true);
    };
    const up = (e: KeyboardEvent) => {
      if (e.code !== 'Space') return;
      spaceDownRef.current = false;
      setSpacePanArmed(false);
    };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
    };
  }, []);
  /** Cancels an in-flight node drag when the pointer is released outside
   *  the canvas — the canvas onMouseUp never fires there, so without this
   *  the node would keep following the cursor on re-entry (ghost drag). */
  const dragCleanupRef = useRef<(() => void) | null>(null);

  const [connectingFromNodeId, setConnectingFromNodeId] = useState<string | null>(null);
  const [connectingFromPort, setConnectingFromPort] = useState<PortName | null>(null);
  /** Nearest target port while dragging a connection, for snap-to-port preview. */
  const [hoveredTarget, setHoveredTarget] = useState<{ nodeId: string; port: PortName; variantIndex: number } | null>(null);

  const [history, setHistory] = useState<HistoryEntry[]>([]);
  /** Mirror of `history` state for synchronous reads in undo/redo handlers. */
  const historyRef = useRef<HistoryEntry[]>([]);
  historyRef.current = history;
  const [redo, setRedo] = useState<HistoryEntry[]>([]);

  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  /** Batch delete confirmation (2+ nodes). Single nodes keep confirmDelete
   *  so the established single-node dialog text stays untouched. */
  const [confirmDeleteMany, setConfirmDeleteMany] = useState<string[] | null>(null);
  /** An ambiguous drop awaiting a relationship choice (ADR #34): the
   *  socket pair admits 2+ semantics, so no wire is drawn until the user
   *  picks one. The in-flight connection stays visible meanwhile. */
  const [relationshipPicker, setRelationshipPicker] = useState<RelationshipPickerState | null>(null);
  const [confirmPreset, setConfirmPreset] = useState<'retail' | 'restaurant' | null>(null);

  /** Shortcuts help popover (header "?" button) — KDS pattern: Escape or an
   *  outside click closes it. While open, Escape is stopPropagation'd so the
   *  canvas's own Escape (deselect) does not also fire. */
  const [showShortcuts, setShowShortcuts] = useState(false);
  const shortcutsBtnRef = useRef<HTMLButtonElement | null>(null);
  const shortcutsRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!showShortcuts) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setShowShortcuts(false);
      }
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (
        shortcutsRef.current && !shortcutsRef.current.contains(e.target as Node) &&
        shortcutsBtnRef.current && !shortcutsBtnRef.current.contains(e.target as Node)
      ) {
        setShowShortcuts(false);
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showShortcuts]);

  /** Anchor for the relationship picker popover (focus + positioning). */
  const relationshipPickerRef = useRef<HTMLDivElement | null>(null);

  /** Cancel the relationship picker AND the in-flight connection it
   *  belongs to (same cleanup as an incompatible drop). Declared early so
   *  the keyboard effect's deps can reference it (const TDZ). */
  const cancelRelationshipPicker = useCallback(() => {
    setRelationshipPicker(null);
    setConnectingFromNodeId(null);
    setConnectingFromPort(null);
    // Return focus to the canvas so keyboard users resume where they left off.
    canvasRef.current?.focus();
  }, []);

  /** Move focus into the picker (first option) when it opens, so keyboard
   *  users land on the choice instead of Tab-ing blindly. */
  useEffect(() => {
    if (relationshipPicker) {
      relationshipPickerRef.current?.querySelector<HTMLButtonElement>('.topology-relationship-option')?.focus();
    }
  }, [relationshipPicker]);

  /** Skip the next workspaceInstances-triggered reload (set before calling onSave). */
  const skipNextLoadRef = useRef(false);
  /** Previous prop identities — a branchLocations-only change (profile
   *  renamed or added while the canvas holds in-flight edits) is a light
   *  merge into the live canvas, NEVER a rebuild from the saved diagram: a
   *  rebuild would silently discard unsaved drags and wires. Deletions are
   *  intentionally not handled here — the full path also keeps saved store
   *  nodes whose profile is gone. */
  const prevBranchLocationsRef = useRef<BranchLocationSeed[] | undefined>(branchLocations);
  const prevInstancesRef = useRef<WorkspaceInstanceSeed[] | undefined>(workspaceInstances);
  /**
   * Exact dirty tracking: the canvas as of the last Apply success, preset
   * load, or authoritative load. Dirty is DERIVED at preset-click time by
   * comparing the current canvas against this snapshot (canvasStateEqual),
   * instead of the previous conservative boolean that was armed by every
   * pushHistory/undo/redo — that over-approximated by marking a canvas
   * dirty even when undo/redo had returned it to EXACTLY the last applied
   * state (e.g. undoing a same-preset load showed a spurious confirm).
   * A null snapshot (never applied) counts as dirty.
   */
  const appliedSnapshotRef = useRef<{ nodes: TopologyNodeData[]; wires: TopologyWireData[] } | null>(
    { nodes: PRESET_RETAIL.nodes, wires: PRESET_RETAIL.wires },
  );
  // Live mirrors so isCanvasDirty stays stable and always reads the latest canvas.
  const nodesRef = useRef<TopologyNodeData[]>(nodes);
  nodesRef.current = nodes;
  const wiresRef = useRef<TopologyWireData[]>(wires);
  wiresRef.current = wires;
  /** Mirror of the dragging set for the touch gesture loop: the document
   *  pointer listeners installed at pointerdown run in a stale closure, so
   *  state reads there must go through refs (drag moves would otherwise see
   *  the pre-drag empty set forever). */
  const draggingNodeIdsRef = useRef<Set<string>>(draggingNodeIds);
  draggingNodeIdsRef.current = draggingNodeIds;
  /** Pan mirror for the same stale-closure reason as draggingNodeIdsRef:
   *  applyDragMove auto-pans the viewport mid-drag, so the drag math must
   *  always read the CURRENT pan (a down-time closure would compute targets
   *  against the pre-pan view and the dragged node would lag the pointer).
   *  zoom has an existing mirror (zoomRef, used by the finder centering). */
  const panRef = useRef(pan);
  panRef.current = pan;
  /** Last pointer position fed to applyDragMove, for edge auto-pan's
   *  direction gate: the viewport only pans when the drag moves TOWARD the
   *  edge the pointer sits in — a drag that drifts away from the edge (or
   *  holds still) must not scroll. Seeded at drag start so the first move
   *  has a baseline. */
  const lastDragMovePosRef = useRef<{ x: number; y: number } | null>(null);
  const isCanvasDirty = useCallback(() => {
    const snap = appliedSnapshotRef.current;
    if (!snap) return true;
    return !canvasStateEqual(snap.nodes, snap.wires, nodesRef.current, wiresRef.current);
  }, []);

  /** Reactive twin of isCanvasDirty for the header's "Unsaved changes"
   *  chip: the ref above is the click-time source of truth, but a ref
   *  cannot drive re-renders — so commitSnapshot bumps this version and
   *  the memo re-derives the flag after every Apply/load/preset. */
  const [snapshotVersion, setSnapshotVersion] = useState(0);
  const commitSnapshot = useCallback((next: { nodes: TopologyNodeData[]; wires: TopologyWireData[] }) => {
    appliedSnapshotRef.current = next;
    setSnapshotVersion((v) => v + 1);
  }, []);
  const isDirty = useMemo(() => {
    const snap = appliedSnapshotRef.current;
    if (!snap) return true;
    // snapshotVersion has no bearing on the comparison itself — its only
    // job is to be a dependency so the memo re-derives after commitSnapshot
    // bumps it (a ref alone cannot trigger a re-render).
    void snapshotVersion;
    return !canvasStateEqual(snap.nodes, snap.wires, nodes, wires);
  }, [nodes, wires, snapshotVersion]);

  /** Surface the dirty flag upward for the parent's branch-switch guard.
   *  Fires on mount (post-load clean) and on every dirty transition; a
   *  stable parent callback makes this effect fire only on real changes. */
  useEffect(() => {
    onDirtyChange?.(isDirty);
  }, [isDirty, onDirtyChange]);

  /** Hover-focus mode: while a node card is hovered, non-connected nodes
   *  and wires dim so the neighbourhood reads at a glance (Figma-style
   *  focus). Null when nothing is hovered — no dimming at all. */
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  /** Wire under the pointer — reveals the midpoint bend ghosts so the
   *  editing affordance is discoverable without selecting the wire first.
   *  Selection alone shows the full handles; hover shows only the ghosts. */
  const [hoveredWireId, setHoveredWireId] = useState<string | null>(null);
  const hoverConnections = useMemo(() => {
    if (!hoveredNodeId) return null;
    const ids = new Set([hoveredNodeId]);
    for (const w of wires) {
      if (w.fromNodeId === hoveredNodeId) ids.add(w.toNodeId);
      if (w.toNodeId === hoveredNodeId) ids.add(w.fromNodeId);
    }
    return ids;
  }, [hoveredNodeId, wires]);

  /** Right-click canvas context menu position (container-relative screen
   *  px). Null while closed. Closed by Escape, any document mousedown
   *  outside the menu, a canvas left-click, or picking an item. */
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; nodeId?: string; wireId?: string } | null>(null);

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        close();
      }
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [contextMenu]);

  /** Whether the zoom-level button's slider popover is open. Closed by
   *  Escape or any document mousedown outside the picker (the picker
   *  wrapper stops propagation, so slider drags never close it). */
  const [zoomPickerOpen, setZoomPickerOpen] = useState(false);
  /** Save-template popover: open flag + the in-flight template name. */
  const [templateSaveOpen, setTemplateSaveOpen] = useState(false);
  const [templateName, setTemplateName] = useState('');
  /** Templates popover: open flag + the last-listed names (re-listed on
   *  open and after a delete so the list never goes stale). */
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [savedTemplates, setSavedTemplates] = useState<string[]>([]);
  /** Minimap visibility per diagram (branch) — mirrors the viewport memory's
   *  per-branch key scheme so a branch switch (which remounts the editor)
   *  restores the user's hide/show choice for that diagram instead of
   *  resetting to a global default. 'unassigned' mirrors the viewport key's
   *  fallback for diagrams with no selected branch. */
  const minimapKey = `oz-topology-view-minimap:${branchId ?? 'unassigned'}`;
  const [minimapVisible, setMinimapVisible] = useState<boolean>(() => {
    try {
      return localStorage.getItem(minimapKey) !== '0';
    } catch { /* storage unavailable or corrupted — default visible */ }
    return true;
  });

  useEffect(() => {
    try {
      localStorage.setItem(minimapKey, minimapVisible ? '1' : '0');
    } catch { /* storage may be unavailable (private mode) — view pref only */ }
  }, [minimapKey, minimapVisible]);

  useEffect(() => {
    if (!zoomPickerOpen) return;
    const close = () => setZoomPickerOpen(false);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        close();
      }
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [zoomPickerOpen]);

  /** Bounding box of the current multi-selection in canvas coords — the
   *  align/distribute toolbar floats above it. Null unless 2+ nodes are
   *  selected (alignment needs a pair, distribution needs the box). */
  const selectionBounds = useMemo(() => {
    const sel = nodes.filter((n) => selectedNodeIds.has(n.id));
    if (sel.length < 2) return null;
    const minX = Math.min(...sel.map((n) => n.x));
    const minY = Math.min(...sel.map((n) => n.y));
    const maxX = Math.max(...sel.map((n) => n.x + NODE_WIDTH));
    const maxY = Math.max(...sel.map((n) => n.y + NODE_HEIGHT));
    return { minX, minY, maxX, maxY };
  }, [nodes, selectedNodeIds]);

  /** Content bounding box in canvas coords — the minimap's projection frame. */
  const contentBounds = useMemo(() => {
    if (nodes.length === 0) return null;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const n of nodes) {
      minX = Math.min(minX, n.x);
      minY = Math.min(minY, n.y);
      maxX = Math.max(maxX, n.x + NODE_WIDTH);
      maxY = Math.max(maxY, n.y + NODE_HEIGHT);
    }
    return { minX, minY, maxX, maxY };
  }, [nodes]);

  /** Uniform scale mapping canvas coords onto the fixed-size minimap. */
  const minimapScale = useMemo(() => {
    if (!contentBounds) return 1;
    const cw = contentBounds.maxX - contentBounds.minX;
    const ch = contentBounds.maxY - contentBounds.minY;
    if (cw <= 0 || ch <= 0) return 1;
    return Math.min(
      (MINIMAP_W - MINIMAP_PAD * 2) / cw,
      (MINIMAP_H - MINIMAP_PAD * 2) / ch,
    );
  }, [contentBounds]);

  /** Select every node on the canvas (context menu action). */
  const selectAllNodes = useCallback(() => {
    setSelectedNodeIds(new Set(nodes.map((n) => n.id)));
    setSelectedNodeId(null);
    setSelectedWireId(null);
  }, [nodes]);

  /**
   * Node id for which an inspector edit already pushed an undo entry in
   * the current selection session. Inspector fields push history once on
   * the FIRST change after selecting a node, so a whole typing burst in
   * the name/subtitle/type controls is a single undo step — not one
   * entry per keystroke. Reset on selection change and undo/redo. */
  const inspectorHistoryPushedForRef = useRef<string | null>(null);

  const isProAllowed = useMemo(() => ['pro', 'enterprise'].includes(currentTier), [currentTier]);
  /** True when adding `extra` warehouse nodes would exceed the tier cap
   *  (one warehouse per install below Pro). The palette spawn, Ctrl+D,
   *  Ctrl+V, Alt+drag, and the mid-drag Alt conversion ALL share this gate
   *  so no creation path can bypass it. Reads nodesRef for freshness inside
   *  callbacks with stable deps. */
  const wouldExceedWarehouseCap = useCallback(
    (extra: number) =>
      !isProAllowed && nodesRef.current.filter((n) => n.type === 'warehouse').length + extra > 1,
    [isProAllowed],
  );
  /** Validate a pending duplicate/paste BEFORE any mutation: warehouses obey
   *  the Pro-tier cap. Returns the FTL toast id to refuse with, or null when
   *  the gesture is allowed. Every duplicate path shares it so the gate can
   *  never be bypassed by an alternate route. */
  const duplicateRefusal = useCallback(
    (copies: TopologyNodeData[]): string | null => {
      const whCopies = copies.filter((n) => n.type === 'warehouse').length;
      if (whCopies > 0 && wouldExceedWarehouseCap(whCopies)) return 'topology-toast-multi-warehouse';
      return null;
    },
    [wouldExceedWarehouseCap],
  );

  /** O(1) node lookup by id — replaces `nodes.find` in hot paths (wire rendering, etc.). */
  const nodeMap = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);

  /** User-visible wire label: the custom label, else the endpoint-name join,
   *  else the generic connection fallback. Shared by the context-menu title
   *  and the label pills so the two surfaces can never disagree. */
  const wireDisplayLabel = (wire: TopologyWireData) =>
    wire.label
      || [nodeMap.get(wire.fromNodeId)?.name, nodeMap.get(wire.toNodeId)?.name].filter(Boolean).join(' → ')
      || l10n.getString('topology-wire-label-connected');

  /** Precomputed wire path geometry — avoids recomputing bezier curves on every render. */
  const wireGeometries = useMemo(() => {
    const geo = new Map<string, {
      x1: number; y1: number; x2: number; y2: number;
      dx: number;
      pathD: string;
      polyline?: Array<[number, number]>;
    }>();
    for (const wire of wires) {
      const fromNode = nodeMap.get(wire.fromNodeId);
      const toNode = nodeMap.get(wire.toNodeId);
      if (!fromNode || !toNode) continue;
      const fromPort = wire.fromPort ?? 'right';
      const toPort = wire.toPort ?? 'left';
      const fromOff = PORT_OFFSET[fromPort];
      const toOff = PORT_OFFSET[toPort];
      // Wires targeting a left input resolve the vertical slot from their
      // recorded toPortId ('location-in' | 'operation-in') so they terminate
      // on the exact socket (inventory nodes stack two left inputs).
      const toVariantIndex = toPort === 'left'
        ? leftPortVariants(toNode).indexOf(wire.toPortId ?? 'location-in')
        : 0;
      const x1 = fromNode.x + fromOff.dx;
      const y1 = fromNode.y + fromOff.dy;
      const x2 = toNode.x + toOff.dx;
      const y2 = toNode.y + (toPort === 'left' ? leftPortDy(toNode, Math.max(0, toVariantIndex)) : toOff.dy);
      const dx = Math.abs(x2 - x1) * 0.5;
      if (wire.bends && wire.bends.length > 0) {
        // User-authored bends take precedence over auto routing: the wire
        // becomes a polyline through the bend points (the pulse rides the
        // same polyline, so the simulation follows the bent path).
        const pts: Array<[number, number]> = [
          [x1, y1],
          ...wire.bends.map((b) => [b.x, b.y] as [number, number]),
          [x2, y2],
        ];
        geo.set(wire.id, { x1, y1, x2, y2, dx, pathD: polylineD(pts), polyline: pts });
      } else if (wireRouting === 'elbow') {
        const pts = elbowPoints(x1, y1, x2, y2);
        geo.set(wire.id, { x1, y1, x2, y2, dx, pathD: polylineD(pts), polyline: pts });
      } else {
        geo.set(wire.id, {
          x1, y1, x2, y2, dx,
          pathD: `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`,
        });
      }
    }
    return geo;
  }, [wires, nodeMap, wireRouting]);

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
    // Branch locations changed but workspace instances did not: the parent
    // renamed, added, or DELETED a store profile. Merge names into the
    // existing store nodes, seed any new locations, and drop the cards (and
    // wires) of deleted locations — a removed branch must leave the canvas
    // cleanly instead of stranding an orphaned card. Positions, history, and
    // every other in-flight edit stay untouched. The initial mount and
    // instance-driven reloads take the full rebuild path below.
    const prevLocations = prevBranchLocationsRef.current;
    const prevInstances = prevInstancesRef.current;
    prevBranchLocationsRef.current = branchLocations;
    prevInstancesRef.current = workspaceInstances;
    if (prevLocations !== branchLocations && prevInstances === workspaceInstances) {
      // Store node ids equal location ids (seeding uses loc.id), so the
      // removed set can be derived from the location delta alone and applied
      // to BOTH the node and wire states in lockstep.
      const locationIds = new Set((branchLocations ?? []).map((l) => l.id));
      const removedLocationIds = new Set(
        (prevLocations ?? []).map((l) => l.id).filter((id) => !locationIds.has(id)),
      );
      setNodes((prev) => {
        const nameById = new Map((branchLocations ?? []).map((l) => [l.id, l.name]));
        const next = prev
          .filter((n) => !(n.type === 'store' && n.storeProfileId && !locationIds.has(n.storeProfileId)))
          .map((n) => {
            if (n.type !== 'store' || !n.storeProfileId) return n;
            const name = nameById.get(n.storeProfileId);
            return name !== undefined && name !== n.name ? { ...n, name } : n;
          });
        for (const loc of branchLocations ?? []) {
          if (!next.some((n) => n.type === 'store' && n.storeProfileId === loc.id)) {
            next.push({
              id: loc.id,
              type: 'store',
              name: loc.name,
              subtitle: 'Branch Location',
              x: snap(80),
              y: snap(140),
              storeProfileId: loc.id,
            });
          }
        }
        return next;
      });
      // Wires to a removed branch card must go with it.
      setWires((prev) =>
        prev.filter((w) => !removedLocationIds.has(w.fromNodeId) && !removedLocationIds.has(w.toNodeId)),
      );
      if (removedLocationIds.size > 0) {
        // A removed branch card may host an in-flight wire preview — cancel
        // it like the rebuild path does, so no stale preview can complete.
        setConnectingFromNodeId(null);
        setConnectingFromPort(null);
        setHoveredTarget(null);
      }
      return;
    }
    // Workspace instances changed but the SET of instances is identical
    // (same ids, same order) — the parent refreshed names after a card
    // rename. Merge the new names into the live workspace nodes instead of
    // rebuilding (a rebuild would discard unsaved drags/wires). Structural
    // changes (create / archive / reorder) fall through to the full
    // authoritative rebuild below, and the post-Apply skip guard keeps
    // precedence so persisted-flag marking still runs.
    const instancesSameIds =
      prevInstances !== workspaceInstances
      && (prevInstances?.length ?? 0) === (workspaceInstances?.length ?? 0)
      && (prevInstances ?? []).every((i, idx) => (workspaceInstances?.[idx]?.instanceId ?? '') === i.instanceId);
    if (instancesSameIds && !skipNextLoadRef.current) {
      const nameById = new Map((workspaceInstances ?? []).map((i) => [i.instanceId, i.name]));
      setNodes((prev) => prev.map((n) => {
        if (n.type !== 'workspace') return n;
        const name = nameById.get(n.id);
        return name !== undefined && name !== n.name ? { ...n, name } : n;
      }));
      return;
    }
    let cancelled = false;
    loadTopology(branchId)
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
            if (n.store_profile_id !== undefined) node.storeProfileId = n.store_profile_id;
            savedById.set(n.id, node);
          }
        }

        // When real workspace instances are supplied (or were — the parent
        // may have just deleted the last branch, wiping them to []), the
        // instance list is authoritative for which workspace nodes exist.
        // Restore positions from the saved diagram, but never resurrect a
        // workspace node that no longer maps to a live instance (that would
        // undo an archive). Non-workspace nodes (store/warehouse/hardware)
        // still come from the saved diagram. The initial mount passes EMPTY
        // arrays while the parent's lists load — those must fall through to
        // the saved-diagram/preset path below, or the canvas wipes to empty
        // (and a fresh install shows nothing at all) before the real seeds
        // arrive.
        const hadInstances = (workspaceInstances?.length ?? 0) > 0
          || (prevInstances?.length ?? 0) > 0
          || (branchLocations?.length ?? 0) > 0
          || (prevLocations?.length ?? 0) > 0;
        // The direct truthiness clause also narrows the type for TS — the
        // body uses workspaceInstances.map directly below.
        if (workspaceInstances && hadInstances) {
          if (cancelled) return;
          // Skip the full rebuild when our own save triggered this reload —
          // only update persisted flags, preserving in-flight canvas edits (#8).
          if (skipNextLoadRef.current) {
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
              metadata: { ...(saved?.metadata ?? {}), typeKey: inst.typeKey, purposeKey: inst.purposeKey ?? 'general', persisted: true },
            };
            return node;
          });
          // Keep saved non-workspace nodes. On a first real workspace load,
          // seed a stable Branch Location node from the workspace's canonical
          // store_id so the required Location In graph is authorable without
          // inventing a primary/default store relationship.
          const otherNodes = [...savedById.values()]
            .filter((n) => n.type !== 'workspace')
            .map((n) => {
              // Legacy store nodes carry no store_profile_id (dev-mock seed
              // and pre-canonical diagrams). When branch locations are
              // supplied, the node id IS the location id — adopt the
              // canonical identity so the deletion filter below can drop it
              // when its branch is gone, and so the node keeps its saved
              // position instead of being dropped and re-seeded at the
              // default slot.
              if (n.type !== 'store' || n.storeProfileId) return n;
              const location = (branchLocations ?? []).find((l) => l.id === n.id);
              return location ? { ...n, storeProfileId: location.id } : n;
            })
            .filter((n) => {
              // A deleted store profile leaves its Branch Location card (and
              // wires) behind: when branch locations are supplied (even an
              // empty list) they own the graph, so drop saved store nodes
              // whose branch no longer exists — whether the node carried a
              // store_profile_id or is a legacy node that just failed to
              // adopt one. A provided-but-empty list means the last branch
              // was deleted — the saved diagram must not resurrect it. Only
              // branchLocations === undefined (standalone editor with no
              // branch concept) keeps the legacy diagram so it still
              // renders. Wires to a dropped node are filtered below by
              // validIds.
              if (branchLocations === undefined) return true;
              if (n.type === 'store') {
                return (branchLocations ?? []).some((l) => l.id === (n.storeProfileId ?? n.id));
              }
              return true;
            })
            // The store profile is the source of truth for a branch's name —
            // refresh saved store nodes from the live location list so a
            // rename reaches the card immediately, not on the next Apply.
            .map((n) => {
              if (n.type !== 'store' || !n.storeProfileId) return n;
              const location = (branchLocations ?? []).find((l) => l.id === n.storeProfileId);
              return location ? { ...n, name: location.name } : n;
            });
          const seededStoreIds = new Set(
            otherNodes.flatMap((node) => node.type === 'store' && node.storeProfileId ? [node.storeProfileId] : []),
          );
          for (const location of branchLocations ?? []) {
            if (seededStoreIds.has(location.id)) continue;
            seededStoreIds.add(location.id);
            otherNodes.push({
              id: location.id,
              type: 'store',
              name: location.name,
              subtitle: 'Branch Location',
              x: snap(80),
              y: snap(140),
              storeProfileId: location.id,
            });
          }
          const mergedNodes = [...otherNodes, ...wsNodes];
          const validIds = new Set(mergedNodes.map((n) => n.id));
          const loadedWires: TopologyWireData[] = (data?.wires ?? [])
            .filter((w) => validIds.has(w.from_node_id) && validIds.has(w.to_node_id))
            .map((w) => {
              const wire: TopologyWireData = {
                id: w.id,
                fromNodeId: w.from_node_id,
                toNodeId: w.to_node_id,
                // Same closed-union discipline as the semantic contract: a
                // corrupt stored direction folds to a legal value here so
                // the editor model (and the Apply round-trip) never carries
                // garbage that would render wrong markers.
                direction: normalizeWireDirection(w.direction),
              };
              if (w.label !== undefined) wire.label = w.label;
              if (w.bends !== undefined) wire.bends = w.bends;
              if (w.from_port != null) wire.fromPort = normalizeVisualPort(w.from_port, 'right');
              if (w.to_port != null) wire.toPort = normalizeVisualPort(w.to_port, 'left');
              if (w.from_port_id !== undefined) wire.fromPortId = w.from_port_id;
              if (w.to_port_id !== undefined) wire.toPortId = w.to_port_id;
              if (w.relationship_type !== undefined) wire.relationshipType = w.relationship_type as SemanticRelationshipType;
              return wire;
            });
          setNodes(mergedNodes);
          setWires(loadedWires);
          // A fresh authoritative load replaces the canvas — the undo/redo
          // stacks hold stale pre-reload states that contradict the loaded
          // instances. Clear them so Undo can never restore a phantom canvas.
          setHistory([]);
          setRedo([]);
          // Same rule as preset loads: cancel any in-flight port connection
          // so a later port click cannot complete a wire from a stale source.
          setConnectingFromNodeId(null);
          setConnectingFromPort(null);
          setHoveredTarget(null);
          // A reloaded node with a surviving id must start a fresh inspector
          // edit session, or its next edit would silently skip pushHistory.
          inspectorHistoryPushedForRef.current = null;
          commitSnapshot({ nodes: mergedNodes, wires: loadedWires });
          return;
        }

        // An explicit unassigned branch owns an empty graph. This is distinct
        // from the initial loading state (where the parent omits seed props),
        // so a saved diagram from a previously selected branch cannot leak
        // into the unassigned canvas after the last branch is deleted.
        const unassignedGraph = branchId === 'unassigned'
          && workspaceInstances !== undefined
          && branchLocations !== undefined
          && workspaceInstances.length === 0
          && branchLocations.length === 0;
        if (unassignedGraph) {
          setNodes([]);
          setWires([]);
          setHistory([]);
          setRedo([]);
          setConnectingFromNodeId(null);
          setConnectingFromPort(null);
          setHoveredTarget(null);
          inspectorHistoryPushedForRef.current = null;
          commitSnapshot({ nodes: [], wires: [] });
          return;
        }

        // No real instances/locations ever supplied — legacy/demo behaviour:
        // use the saved diagram verbatim, or fall back to the retail preset.
        if (cancelled || !data || !data.nodes || data.nodes.length === 0) {
          // No saved diagram. A standalone editor (seeds never supplied) keeps
          // the demo preset; a parent that EXPLICITLY supplied empty seeds
          // owns the graph — a fresh or fully-deleted store must show the
          // empty canvas (onboarding hint) rather than demo data.
          if (workspaceInstances !== undefined || branchLocations !== undefined) {
            setNodes([]);
            setWires([]);
            setHistory([]);
            setRedo([]);
            commitSnapshot({ nodes: [], wires: [] });
          }
          return;
        }
        if (skipNextLoadRef.current) { return; }
        setNodes([...savedById.values()]);
        const loadedWires: TopologyWireData[] = data.wires.map((w) => {
          const wire: TopologyWireData = {
            id: w.id,
            fromNodeId: w.from_node_id,
            toNodeId: w.to_node_id,
            // Same closed-union discipline as the semantic contract: a
            // corrupt stored direction folds to a legal value here so
            // the editor model (and the Apply round-trip) never carries
            // garbage that would render wrong markers.
            direction: normalizeWireDirection(w.direction),
          };
          if (w.label !== undefined) wire.label = w.label;
          if (w.from_port != null) wire.fromPort = normalizeVisualPort(w.from_port, 'right');
          if (w.to_port != null) wire.toPort = normalizeVisualPort(w.to_port, 'left');
          if (w.from_port_id !== undefined) wire.fromPortId = w.from_port_id;
          if (w.to_port_id !== undefined) wire.toPortId = w.to_port_id;
          if (w.relationship_type !== undefined) wire.relationshipType = w.relationship_type as SemanticRelationshipType;
          return wire;
        });
        setWires(loadedWires);
        // Fresh authoritative load — drop stale pre-load undo/redo state.
        setHistory([]);
        setRedo([]);
        // Same rule as preset loads: cancel any in-flight port connection.
        setConnectingFromNodeId(null);
        setConnectingFromPort(null);
        setHoveredTarget(null);
        // A reloaded node with a surviving id must start a fresh inspector
        // edit session, or its next edit would silently skip pushHistory.
        inspectorHistoryPushedForRef.current = null;
        commitSnapshot({ nodes: [...savedById.values()], wires: loadedWires });
      })
      .catch((err) => {
        // Only "no saved topology" (null result) is expected — that is
        // handled in the .then() above. Any thrown error (corrupt DB,
        // serialisation failure, etc.) should be surfaced to the user
        // rather than silently swallowed.
        if (cancelled) return;
        addToast({
          message: `${l10n.getString('topology-toast-load-error')}: ${plainErrorMessage(err)}`,
          type: 'error',
        });
      })
      .finally(() => {
        // Every .then branch returns, so this runs once the first
        // authoritative load (saved diagram, empty graph, or preset fallback)
        // has been applied — the gate for the dismissal forget-effect.
        if (!cancelled) setTopologyLoaded(true);
      });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceInstances, branchLocations, branchId]);

  // ── Inline node rename on the card (Branch Location + workspace) ──
  const [renamingNodeId, setRenamingNodeId] = useState<string | null>(null);
  const renamingNodeIdRef = useRef<string | null>(renamingNodeId);
  renamingNodeIdRef.current = renamingNodeId;
  const [renameDraft, setRenameDraft] = useState('');
  const [renameSaving, setRenameSaving] = useState(false);
  const renameInputRef = useRef<HTMLInputElement>(null);
  /** Guards the blur-commit against a concurrent Escape/close. */
  const renameCancelledRef = useRef(false);
  /** Focus-time name snapshot for the live-bound rename inputs (body config
   *  / inspector Node Name). They already carry the edited value on blur, so
   *  the baseline is what tells an unedited blur from a real rename. */
  const renameBaselineRef = useRef<string | null>(null);
  /** Focus target when the rename form closes: the node id for keyboard
   *  closes (Enter/Escape), null for blur-commits — a click-away must not
   *  steal focus back from wherever the user actually clicked. */
  const renameFocusReturnRef = useRef<string | null>(null);

  // Move keyboard focus into the card's rename input the moment it opens
  // (autoFocus is banned by jsx-a11y/no-autofocus).
  useEffect(() => {
    if (renamingNodeId) renameInputRef.current?.focus();
  }, [renamingNodeId]);

  // Return focus to the node card after a keyboard-driven close, so the
  // keyboard user lands back on the node they just renamed instead of the
  // canvas body.
  useEffect(() => {
    if (renamingNodeId !== null) return;
    const nodeId = renameFocusReturnRef.current;
    if (nodeId === null) return;
    renameFocusReturnRef.current = null;
    (document.querySelector(`.topology-node[data-node-id="${nodeId}"]`) as HTMLElement | null)?.focus();
  }, [renamingNodeId]);

  const startNodeRename = useCallback((nodeId: string, currentName: string) => {
    renameCancelledRef.current = false;
    renameFocusReturnRef.current = null;
    setRenameDraft(currentName);
    setRenamingNodeId(nodeId);
  }, []);

  const cancelNodeRename = useCallback(() => {
    renameCancelledRef.current = true;
    // Escape is a keyboard close — return focus to the card. Reads the
    // current renaming node via the ref so the callback stays stable (the
    // memoized cards all receive it as a prop).
    renameFocusReturnRef.current = renamingNodeIdRef.current;
    setRenamingNodeId(null);
    setRenameDraft('');
  }, []);

  /** Persist a live-bound rename (the body config input / inspector Node
   *  Name field) through the same parent callback the titlebar F2 rename
   *  uses, so a committed rename survives the authoritative instance/
   *  location refresh instead of being silently reverted by the merge.
   *  Harnesses without the callback keep the local-only path (Apply
   *  persists the diff). A false return means the parent toasted — keep
   *  the local name for a retry, mirroring commitNodeRename. */
  const persistNodeRename = useCallback(async (nodeId: string, name: string) => {
    const node = nodes.find((n) => n.id === nodeId);
    if (!node) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    // Live-bound inputs already carry the edited value on blur — compare
    // against the focus-time baseline so an unedited blur never round-trips
    // a redundant rename through the parent.
    if (trimmed === renameBaselineRef.current) return;
    const persist = node.type === 'store' ? onRenameBranch : onRenameWorkspace;
    if (!persist) return;
    const ok = await persist(nodeId, trimmed);
    if (ok === false) {
      // The parent refused (it toasts the error) — revert the live-bound name
      // to the focus-time (authoritative) baseline so the canvas never holds
      // a name the backend rejected. commitNodeRename keeps its draft open
      // for a retry; a blurred input has no draft to keep, so reverting is
      // the honest state — the alternative (keep the edited name) would
      // silently revert on the next authoritative refresh instead.
      setNodes((prev) => prev.map((n) => (n.id === nodeId ? { ...n, name: renameBaselineRef.current ?? n.name } : n)));
      return;
    }
    renameBaselineRef.current = trimmed;
  }, [nodes, onRenameBranch, onRenameWorkspace]);

  const commitNodeRename = useCallback(async (nodeId: string, fromKeyboard = false) => {
    if (renameSaving || renameCancelledRef.current) return;
    const node = nodes.find((n) => n.id === nodeId);
    const name = renameDraft.trim();
    // Empty or unchanged input is a no-op: close the form silently rather
    // than round-tripping a redundant update. A false return from the
    // parent is reserved for genuine errors (it toasts and we keep the
    // draft open for a retry).
    if (!node || !name || name === node.name) {
      renameCancelledRef.current = true;
      renameFocusReturnRef.current = fromKeyboard ? nodeId : null;
      setRenamingNodeId(null);
      setRenameDraft('');
      return;
    }
    setRenameSaving(true);
    try {
      const persist = node.type === 'store' ? onRenameBranch : onRenameWorkspace;
      const ok = await persist?.(nodeId, name);
      if (ok !== false) {
        // Belt & suspenders: reflect the new name locally AND let the seed
        // refresh (profile / instance is authoritative) confirm it on the
        // next reload.
        setNodes((prev) => prev.map((n) => (n.id === nodeId ? { ...n, name } : n)));
        renameCancelledRef.current = true;
        renameFocusReturnRef.current = fromKeyboard ? nodeId : null;
        setRenamingNodeId(null);
        setRenameDraft('');
      }
      // ok === false → parent toasts; keep the draft open for a retry.
    } finally {
      setRenameSaving(false);
    }
  }, [renameSaving, renameDraft, nodes, onRenameBranch, onRenameWorkspace]);

  // ── Inline wire rename: floating input at the wire's midpoint ──
  const [renamingWireId, setRenamingWireId] = useState<string | null>(null);
  const [wireRenameDraft, setWireRenameDraft] = useState('');
  const wireRenameInputRef = useRef<HTMLInputElement>(null);
  /** Guards the blur-commit against a concurrent Escape/close. */
  const wireRenameCancelledRef = useRef(false);
  /** Focus target when the form closes: the wire id for keyboard closes
   *  (Enter/Escape), null for blur-commits — a click-away must not steal
   *  focus back from wherever the user actually clicked. */
  const wireRenameFocusReturnRef = useRef<string | null>(null);

  // Move keyboard focus into the wire's rename input the moment it opens.
  useEffect(() => {
    if (renamingWireId) wireRenameInputRef.current?.focus();
  }, [renamingWireId]);

  // Return focus to the wire after a keyboard-driven close, so the keyboard
  // user lands back on the object they just relabeled instead of the canvas.
  useEffect(() => {
    if (renamingWireId !== null) return;
    const wireId = wireRenameFocusReturnRef.current;
    if (wireId === null) return;
    wireRenameFocusReturnRef.current = null;
    (document.querySelector(`.wire-hitbox[data-wire-id="${wireId}"]`) as HTMLElement | null)?.focus();
  }, [renamingWireId]);

  const startWireRename = (wireId: string) => {
    const wire = wires.find((w) => w.id === wireId);
    wireRenameCancelledRef.current = false;
    wireRenameFocusReturnRef.current = null;
    setWireRenameDraft(wire?.label ?? '');
    setRenamingWireId(wireId);
  };

  const cancelWireRename = () => {
    wireRenameCancelledRef.current = true;
    // Escape is a keyboard close — return focus to the wire.
    wireRenameFocusReturnRef.current = renamingWireId;
    setRenamingWireId(null);
    setWireRenameDraft('');
  };

  const commitWireRename = (wireId: string, fromKeyboard = false) => {
    if (wireRenameCancelledRef.current) return;
    const wire = wires.find((w) => w.id === wireId);
    const label = wireRenameDraft.trim();
    // Empty or unchanged input is a no-op: close the form silently. An empty
    // label reverts to the endpoint-name display (the label is optional).
    if (!wire || label === (wire.label ?? '')) {
      wireRenameCancelledRef.current = true;
      wireRenameFocusReturnRef.current = fromKeyboard ? wireId : null;
      setRenamingWireId(null);
      setWireRenameDraft('');
      return;
    }
    // One undo entry; label is a persisted field in the dirty projection, so
    // the relabel also marks the canvas dirty and rides Apply Topology.
    pushHistory();
    setWires((prev) =>
      prev.map((w) => {
        if (w.id !== wireId) return w;
        const next: TopologyWireData = { ...w };
        if (label) next.label = label;
        else delete next.label;
        return next;
      }),
    );
    wireRenameCancelledRef.current = true;
    wireRenameFocusReturnRef.current = fromKeyboard ? wireId : null;
    setRenamingWireId(null);
    setWireRenameDraft('');
  };

  const pushHistory = useCallback((snapshot?: { nodes: TopologyNodeData[]; wires: TopologyWireData[] }) => {
    // Dirty is derived (isCanvasDirty compares against appliedSnapshotRef),
    // so no flag needs arming here — the mutation itself is the dirty signal.
    setRedo([]); // new edit invalidates the redo branch
    setHistory((prev) => {
      // An explicit snapshot wins (bend drags capture the pre-gesture wires
      // at mousedown so a ghost-created bend undoes away completely); the
      // default snapshots the refs — identical to the latest render's
      // closure state, but keeps pushHistory referentially STABLE so the
      // memoized card/wire layers don't churn on every nodes/wires change.
      const src = snapshot ?? { nodes: nodesRef.current, wires: wiresRef.current };
      const entry: HistoryEntry = { nodes: src.nodes.map((n) => ({ ...n })), wires: src.wires.map((w) => ({ ...w })) };
      const next = [...prev, entry];
      if (next.length > 50) next.shift();
      return next;
    });
  }, []);
  /** Mirror so the memoized wire handlers (cycle/bends) can call pushHistory
   *  without taking it as a dep — pushHistory re-keys whenever nodes/wires
   *  change, which would churn the handler identity and re-render every wire
   *  on any unrelated edit. The ref reads the latest snapshot at call time. */
  const pushHistoryRef = useRef(pushHistory);
  pushHistoryRef.current = pushHistory;

  /** One-click organize (see computeAutoLayout): a thin wrapper that
   *  pushes ONE undo entry, applies the engine's placements, clears authored
   *  bends (their coordinates described the OLD geometry), and announces the
   *  result. An empty diagram has nothing to organize — no history entry. */
  const autoLayout = useCallback(() => {
    // Elbow-routed wires are orthogonal — snap the placements to the grid
    // so the orthogonal geometry stays clean; curved wires tolerate the
    // free-floating anchor positions.
    const placed = computeAutoLayout(nodes, wires, {
      snapToGrid: snapEnabled && wireRouting === 'elbow',
    });
    if (placed.length === 0) return;
    pushHistory();
    const byId = new Map(placed.map((p) => [p.id, p]));
    setNodes((prev) => prev.map((n) => {
      const p = byId.get(n.id);
      return p ? { ...n, x: p.x, y: p.y } : n;
    }));
    // exactOptionalPropertyTypes forbids `bends: undefined` — destructure
    // the property away so the wires leave with NO bends key at all.
    setWires((prev) => prev.map(({ bends: _bends, ...rest }) => rest));
    setLiveAnnouncement(l10nRef.current.getString('topology-layout-announce'));
  }, [nodes, wires, pushHistory, snapEnabled, wireRouting]);

  /** Copy the diagram to the clipboard as the versioned JSON envelope.
   *  Guards a missing clipboard API (insecure context / WebView) with an
   *  explanatory toast instead of throwing. */
  const handleExport = useCallback(async () => {
    if (!navigator.clipboard?.writeText) {
      addToast({ message: l10nRef.current.getString('topology-toast-clipboard-unavailable'), type: 'warning' });
      return;
    }
    try {
      await navigator.clipboard.writeText(serializeTopology(nodes, wires));
      addToast({ message: l10nRef.current.getString('topology-toast-export-copied'), type: 'info' });
    } catch {
      addToast({ message: l10nRef.current.getString('topology-toast-clipboard-unavailable'), type: 'warning' });
    }
  }, [nodes, wires, addToast]);

  /** Replace the canvas with a clipboard payload under ONE undo entry. A
   *  strict-parse failure (or a missing/unreadable clipboard) leaves the
   *  canvas untouched — a bad paste can never half-load a broken diagram. */
  const handleImport = useCallback(async () => {
    if (!navigator.clipboard?.readText) {
      addToast({ message: l10nRef.current.getString('topology-toast-clipboard-unavailable'), type: 'warning' });
      return;
    }
    let json: string;
    try {
      json = await navigator.clipboard.readText();
    } catch {
      addToast({ message: l10nRef.current.getString('topology-toast-import-invalid'), type: 'warning' });
      return;
    }
    const payload = deserializeTopology(json);
    if (!payload) {
      addToast({ message: l10nRef.current.getString('topology-toast-import-invalid'), type: 'warning' });
      return;
    }
    pushHistory();
    setNodes(payload.nodes.map((n) => ({ ...n })));
    setWires(payload.wires.map((w) => ({ ...w })));
    addToast({ message: l10nRef.current.getString('topology-toast-import-ok'), type: 'info' });
  }, [pushHistory, addToast]);

  /** Save the diagram under `name`; an empty name keeps the popover open
   *  (the pure helper refuses it — nothing to save). */
  const handleSaveTemplate = useCallback((name: string) => {
    if (saveTemplate(name, nodes, wires) === null) return;
    setTemplateSaveOpen(false);
    setTemplateName('');
    addToast({ message: l10nRef.current.getString('topology-toast-template-saved'), type: 'info' });
  }, [nodes, wires, addToast]);

  /** Load a saved template, replacing the canvas under one undo entry. */
  const handleLoadTemplate = useCallback((name: string) => {
    const payload = loadTemplate(name);
    if (!payload) return;
    pushHistory();
    setNodes(payload.nodes.map((n) => ({ ...n })));
    setWires(payload.wires.map((w) => ({ ...w })));
    setTemplatesOpen(false);
    addToast({ message: l10nRef.current.getString('topology-toast-import-ok'), type: 'info' });
  }, [pushHistory, addToast]);

  /** Delete a saved template and re-list, so the popover reflects the
   *  deletion immediately. */
  const handleDeleteTemplate = useCallback((name: string) => {
    deleteTemplate(name);
    setSavedTemplates(listTemplates());
    addToast({ message: l10nRef.current.getString('topology-toast-template-deleted'), type: 'info' });
  }, [addToast]);

  /** Toggle the templates popover, re-listing on every open. */
  const openTemplates = useCallback(() => {
    setSavedTemplates(listTemplates());
    setTemplatesOpen((v) => !v);
  }, []);

  /** Commit an in-flight Alt+drag: the copies stay where they dropped,
   *  become the selection, and the whole duplicate-drop lands as ONE undo
   *  entry (undo removes the copies entirely). The entry is the PRE-drag
   *  state — exactly the current state minus the copy ids, since the
   *  originals never moved during an Alt+drag. When the drag was converted
   *  MID-move, the move's own entry already IS that pre-drag state — skip
   *  the push. Idempotent — both the document and canvas mouseup paths can
   *  fire for the same release. */
  const commitDuplicateDrag = useCallback(() => {
    if (!duplicateDragRef.current) return;
    duplicateDragRef.current = false;
    const copyIds = duplicateCopyIdsRef.current;
    duplicateCopyIdsRef.current = [];
    const entryAlreadyPushed = duplicateHistoryPushedRef.current;
    duplicateHistoryPushedRef.current = false;
    if (copyIds.length > 0) {
      if (!entryAlreadyPushed) {
        const copySet = new Set(copyIds);
        setRedo([]); // new edit invalidates the redo branch
        setHistory((prev) => {
          const entry: HistoryEntry = {
            nodes: nodesRef.current.filter((n) => !copySet.has(n.id)).map((n) => ({ ...n })),
            wires: wiresRef.current
              .filter((w) => !copySet.has(w.fromNodeId) && !copySet.has(w.toNodeId))
              .map((w) => ({ ...w })),
          };
          const next = [...prev, entry];
          if (next.length > 50) next.shift();
          return next;
        });
      }
      setSelectedNodeIds(new Set(copyIds));
      setSelectedNodeId(copyIds[0] ?? null);
      setSelectedWireId(null);
      setLiveAnnouncement(l10nRef.current.getString('topology-duplicate-announce'));
    }
    document.body.style.cursor = '';
  }, []);

  /** Escape during an Alt+drag: discard the preview copies and the drag
   *  itself (originals stay selected, no history entry). When the drag was
   *  converted MID-move, the state after cancel equals the move's history
   *  entry (originals restored to start, no copies) — pop it so Undo is not
   *  a no-op. */
  const cancelDuplicateDrag = useCallback(() => {
    if (!duplicateDragRef.current) return;
    duplicateDragRef.current = false;
    const copyIds = new Set(duplicateCopyIdsRef.current);
    duplicateCopyIdsRef.current = [];
    const entryPushed = duplicateHistoryPushedRef.current;
    duplicateHistoryPushedRef.current = false;
    if (copyIds.size > 0) {
      setNodes((prev) => prev.filter((n) => !copyIds.has(n.id)));
      setWires((prev) => prev.filter((w) => !copyIds.has(w.fromNodeId) && !copyIds.has(w.toNodeId)));
    }
    if (entryPushed) {
      setHistory((prev) => prev.slice(0, -1));
    }
    document.body.style.cursor = '';
    setDraggingNodeIds(new Set());
    dragHasMovedRef.current = false;
    dragOffsetsRef.current.clear();
    dragStartRef.current.clear();
    setAlignmentGuide(null);
    setLiveAnnouncement(l10nRef.current.getString('topology-duplicate-cancel-announce'));
    dragCleanupRef.current?.();
  }, []);

  /** Alt pressed MID-move (Figma semantics): the drag becomes a duplicate
   *  drag. The originals snap back to their pre-drag positions, fresh copies
   *  take over the cursor from the current mid-drag positions, and the drag
   *  offsets re-key to the copies. If the move had already pushed its
   *  history entry, that entry IS the pre-drag state — the commit reuses it
   *  (no duplicate entry) and the cancel pops it. */
  const convertDragToDuplicate = useCallback(() => {
    if (duplicateDragRef.current) return;
    const start = dragStartRef.current;
    if (start.size === 0) return;
    const draggedIds = new Set(start.keys());
    // Refuse the mid-drag conversion when it would copy a Branch Location or
    // duplicate a warehouse past the tier cap — the move simply stays a move.
    const refusal = duplicateRefusal(nodesRef.current.filter((n) => draggedIds.has(n.id)));
    if (refusal) {
      addToast({ message: l10n.getString(refusal), type: 'warning' });
      return;
    }
    duplicateHistoryPushedRef.current = dragHasMovedRef.current;

    // Copies of the dragged set at their CURRENT (mid-drag) positions;
    // wires copy when BOTH endpoints are dragged.
    const originalToCopy = new Map<string, string>();
    const copies = nodesRef.current
      .filter((n) => draggedIds.has(n.id))
      .map((n) => {
        const newId = `${n.type}-${crypto.randomUUID()}`;
        originalToCopy.set(n.id, newId);
        // sanitizeCopiedNode strips a Branch Location copy's canonical
        // identity — the copy is a diagram-only card, never a second
        // branch impersonating the original.
        return { ...sanitizeCopiedNode(n), id: newId };
      });
    const wireCopies = wiresRef.current
      .filter((w) => draggedIds.has(w.fromNodeId) && draggedIds.has(w.toNodeId))
      .map((w) => ({
        ...w,
        id: `wire-${crypto.randomUUID()}`,
        fromNodeId: originalToCopy.get(w.fromNodeId)!,
        toNodeId: originalToCopy.get(w.toNodeId)!,
      }));
    duplicateCopyIdsRef.current = copies.map((c) => c.id);
    duplicateDragRef.current = true;

    // Originals back to start; copies in at their current positions.
    if (copies.length > 0) {
      setNodes((prev) => prev.map((n) => (
        start.has(n.id) ? { ...n, ...start.get(n.id)! } : n
      )));
      setNodes((prev) => [...prev, ...copies]);
      setWires((prev) => [...prev, ...wireCopies]);
    }

    // Re-key the drag offsets to the copies (same cursor-relative offsets).
    const offsets = new Map<string, { x: number; y: number }>();
    for (const [id, off] of dragOffsetsRef.current) {
      const copyId = originalToCopy.get(id);
      if (copyId) offsets.set(copyId, off);
    }
    dragOffsetsRef.current = offsets;
    setDraggingNodeIds(new Set(duplicateCopyIdsRef.current));
    document.body.style.cursor = 'copy';
  }, [duplicateRefusal, addToast, l10n]);

  /** Escape mid-MOVE (Figma semantics): the dragged nodes snap back to
   *  their pre-drag positions, the move's single history entry is popped
   *  (undo would otherwise restore the same state — a no-op entry), and the
   *  selection survives. Idempotent. */
  const cancelNodeMove = useCallback(() => {
    if (dragStartRef.current.size === 0) return;
    const start = dragStartRef.current;
    dragStartRef.current = new Map();
    // Merge the start COORDINATES only — the snapshot is { x, y }, so a
    // wholesale replacement would strip type/name/id and crash the render.
    setNodes((prev) => prev.map((n) => (start.has(n.id) ? { ...n, ...start.get(n.id)! } : n)));
    if (dragHasMovedRef.current) {
      setHistory((prev) => prev.slice(0, -1));
    }
    dragHasMovedRef.current = false;
    setDraggingNodeIds(new Set());
    dragOffsetsRef.current.clear();
    setAlignmentGuide(null);
    dragCleanupRef.current?.();
  }, []);

  /** Escape mid-bend-drag: restore the bend to its start position (a
   *  ghost-created bend is removed entirely) and pop the drag's single
   *  history entry, so a cancelled gesture leaves no undo record. Mirrors
   *  cancelNodeMove for node drags. Defined before the keydown effect that
   *  calls it (the effect's deps evaluate this binding eagerly). */
  const cancelBendDrag = useCallback(() => {
    const d = bendDragRef.current;
    if (!d) return;
    bendDragRef.current = null;
    setWires((prev) =>
      prev.map((w) => {
        if (w.id !== d.wireId) return w;
        if (d.created) {
          return { ...w, bends: (w.bends ?? []).filter((_, i) => i !== d.index) };
        }
        return {
          ...w,
          bends: (w.bends ?? []).map((b, i) => (i === d.index ? { x: d.startX, y: d.startY } : b)),
        };
      }),
    );
    if (d.moved) {
      // The drag pushed exactly one entry (on first movement) — pop it so
      // Undo stays a no-op for a cancelled gesture.
      setHistory((prev) => prev.slice(0, -1));
    }
    bendDragCleanupRef.current?.();
  }, []);

  /** Align or distribute the current multi-selection. One undo entry per
   *  action; the reference geometry is the selection's own bounding box,
   *  so the extremes stay put and the rest move to match. Both use exact
   *  arithmetic — no re-snapping, or an off-grid extreme node would drift
   *  instead of anchoring the alignment (legacy preset ys like 80 are
   *  deliberately off the 24px grid, and geometry tests pin them). */
  const applyAlign = useCallback((mode: AlignMode) => {
    if (selectedNodeIds.size < 2) return;
    pushHistory();
    const ids = new Set(selectedNodeIds);
    setNodes((prev) => {
      const sel = prev.filter((n) => ids.has(n.id));
      if (sel.length < 2) return prev;
      const minX = Math.min(...sel.map((n) => n.x));
      const maxX = Math.max(...sel.map((n) => n.x + NODE_WIDTH));
      const minY = Math.min(...sel.map((n) => n.y));
      const maxY = Math.max(...sel.map((n) => n.y + NODE_HEIGHT));
      return prev.map((n) => {
        if (!ids.has(n.id)) return n;
        switch (mode) {
          case 'left': return { ...n, x: minX };
          case 'hcenter': return { ...n, x: minX + (maxX - minX - NODE_WIDTH) / 2 };
          case 'right': return { ...n, x: maxX - NODE_WIDTH };
          case 'top': return { ...n, y: minY };
          case 'vcenter': return { ...n, y: minY + (maxY - minY - NODE_HEIGHT) / 2 };
          case 'bottom': return { ...n, y: maxY - NODE_HEIGHT };
          case 'dist-h': {
            const sorted = [...sel].sort((a, b) => a.x - b.x);
            if (sorted.length < 3) return n;
            const i = sorted.findIndex((s) => s.id === n.id);
            const span = sorted[sorted.length - 1]!.x - sorted[0]!.x;
            return { ...n, x: sorted[0]!.x + (span * i) / (sorted.length - 1) };
          }
          case 'dist-v': {
            const sorted = [...sel].sort((a, b) => a.y - b.y);
            if (sorted.length < 3) return n;
            const i = sorted.findIndex((s) => s.id === n.id);
            const span = sorted[sorted.length - 1]!.y - sorted[0]!.y;
            return { ...n, y: sorted[0]!.y + (span * i) / (sorted.length - 1) };
          }
          default: return n;
        }
      });
    });
  }, [selectedNodeIds, pushHistory]);

  /**
   * Push history at most once per node selection session when an
   * inspector field changes — the first keystroke/select of a session
   * snapshots the pre-edit state; later changes in the same session
   * mutate without creating more undo entries.
   */
  const beginInspectorEdit = useCallback((nodeId: string) => {
    if (inspectorHistoryPushedForRef.current !== nodeId) {
      inspectorHistoryPushedForRef.current = nodeId;
      pushHistory();
    }
  }, [pushHistory]);

  // A fresh selection starts a fresh inspector edit session.
  useEffect(() => {
    inspectorHistoryPushedForRef.current = null;
  }, [selectedNodeId]);

  /**
   * Re-validate the selection whenever the canvas changes — an undo, redo,
   * preset load, or fresh topology reload can remove the selected node or
   * wire. A dangling selection (pointing at a now-gone element) renders the
   * tool-rack Delete button for nothing and lets arrow keys push no-op undo
   * entries, so clear it; a still-valid selection is preserved.
   */
  useEffect(() => {
    if (selectedNodeId && !nodeMap.has(selectedNodeId)) {
      setSelectedNodeIds(new Set());
      setSelectedNodeId(null);
    } else {
      // Prune any multi-selection members that no longer exist.
      setSelectedNodeIds((prev) => {
        if (prev.size === 0) return prev;
        const next = new Set([...prev].filter((id) => nodeMap.has(id)));
        return next.size === prev.size ? prev : next;
      });
    }
    if (selectedWireId && !wires.some((w) => w.id === selectedWireId)) {
      setSelectedWireId(null);
    }
    // A picker whose target node vanished (preset load, workspace reload,
    // batch delete) must close — otherwise its keyboard guard would keep
    // swallowing canvas shortcuts even though the popover is unrenderable.
    if (relationshipPicker && !nodeMap.has(relationshipPicker.toNodeId)) {
      setRelationshipPicker(null);
    }
  }, [selectedNodeId, selectedWireId, nodeMap, wires, relationshipPicker]);

  const loadPreset = useCallback((preset: 'retail' | 'restaurant') => {
    // A wholesale canvas replacement invalidates any in-flight relationship
    // choice — close the picker (and its connection) before nodes change.
    setRelationshipPicker(null);
    setConnectingFromNodeId(null);
    setConnectingFromPort(null);
    const data = preset === 'retail' ? PRESET_RETAIL : PRESET_RESTAURANT;
    pushHistory();
    setNodes(data.nodes);
    setWires(data.wires);
    // The canvas was replaced — cancel any in-flight port connection so a
    // later port click starts a fresh connection instead of completing a
    // wire from a stale source node (the preset ids overlap, so the stale
    // source could otherwise survive and mis-wire the new canvas).
    setConnectingFromNodeId(null);
    setConnectingFromPort(null);
    setHoveredTarget(null);
    // Same canvas-replacement rule: the simulation pulse animates the OLD
    // wire geometry, so stop it — a pulse must never animate a "test order"
    // on a topology it was never run against. Flipping isSimulating false
    // makes the interval effect's cleanup clear the 30ms interval.
    setIsSimulating(false);
    setSimPulseStep(0);
    setFreshNodeIds(new Set());
    // The preset is now the applied state — the canvas matches it exactly,
    // so a subsequent preset click must not confirm.
    commitSnapshot({ nodes: data.nodes, wires: data.wires });
    // The canvas was replaced — a still-selected node (preset ids overlap)
    // must start a fresh inspector edit session, or its next edit would
    // silently skip pushHistory (no undo entry, no dirty flag).
    inspectorHistoryPushedForRef.current = null;
    setZoom(1);
    setPan({ x: 0, y: 0 });
    // Preset ids only partially overlap — the re-validation effect will
    // clear a selection pointing at an element the new preset lacks.
    // Surface it so the user knows why the inspector closed instead of
    // the drop happening silently.
    if (selectedNodeId && !data.nodes.some((n) => n.id === selectedNodeId)) {
      addToast({ message: l10n.getString('topology-toast-selection-dropped'), type: 'info' });
    } else if (selectedWireId && !data.wires.some((w) => w.id === selectedWireId)) {
      addToast({ message: l10n.getString('topology-toast-selection-dropped'), type: 'info' });
    }
  }, [pushHistory, selectedNodeId, selectedWireId, addToast, l10n, commitSnapshot]);

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
    // Dirty is derived: if the undone-to canvas matches the last applied
    // snapshot (e.g. undoing a same-preset load), no confirm fires; if it
    // diverges (undoing past a save), the preset gate confirms. The stale
    // conservative boolean was removed — it armed a spurious confirm for
    // the exact-equality case.
    // A post-undo edit is a fresh session — it must push a new entry.
    inspectorHistoryPushedForRef.current = null;
    // Undoing a deletion restores the removed node — re-select it so the
    // inspector reopens on the restored element (the delete flow cleared
    // the selection). Exactly one node restored from the entry is the
    // delete signature: an undo of an add/move/toggle restores no nodes
    // and must leave the selection untouched.
    const currentIds = new Set(nodes.map((n) => n.id));
    const restoredNodes = entry.nodes.filter((n) => !currentIds.has(n.id));
    if (restoredNodes.length === 1) {
      selectOnly(restoredNodes[0]!.id);
    }
  }, [nodes, wires, selectOnly]);

  const popRedo = useCallback(() => {
    if (redo.length === 0) return;
    const entry = redo[redo.length - 1]!;
    // Push current state to history before restoring
    setHistory((prev) => [...prev, { nodes: nodes.map((n) => ({ ...n })), wires: wires.map((w) => ({ ...w })) }]);
    setNodes(entry.nodes);
    setWires(entry.wires);
    setRedo((prev) => prev.slice(0, -1));
    // Same derived dirty rule as undo: redo to exactly the applied canvas
    // is clean; redo to anything else confirms on the next preset click.
    // A post-redo edit is a fresh session — it must push a new entry.
    inspectorHistoryPushedForRef.current = null;
  }, [redo, nodes, wires]);

  // Clean up pan/drag listeners and fresh-node timers on unmount
  useEffect(() => {
    const timers = freshTimersRef.current;
    return () => {
      panCleanupRef.current?.();
      dragCleanupRef.current?.();
      minimapDragCleanupRef.current?.();
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

  /** Delete a set of nodes in one history entry — every wire touching any
   *  of them goes too. Single-node and batch deletes share this path. */
  const deleteNodes = useCallback((ids: string[]) => {
    if (ids.length === 0) return;
    const doomed = new Set(ids);
    pushHistory();
    setNodes((prev) => prev.filter((n) => !doomed.has(n.id)));
    setWires((prev) => prev.filter((w) => !doomed.has(w.fromNodeId) && !doomed.has(w.toNodeId)));
    setSelectedNodeIds(new Set());
    setSelectedNodeId(null);
  }, [pushHistory]);

  /** Fit the whole diagram into the viewport (clamped 40%..200%). */
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

  /** Fit the current multi-selection — same bounds math as zoomToFit but
   *  scoped to the selected nodes (context menu action). */
  const zoomToSelection = useCallback(() => {
    if (selectedNodeIds.size === 0) return;
    const sel = nodes.filter((n) => selectedNodeIds.has(n.id));
    if (sel.length === 0) return;
    const minX = Math.min(...sel.map((n) => n.x));
    const minY = Math.min(...sel.map((n) => n.y));
    const maxX = Math.max(...sel.map((n) => n.x + NODE_WIDTH));
    const maxY = Math.max(...sel.map((n) => n.y + NODE_HEIGHT));
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
  }, [nodes, selectedNodeIds]);

  /** Step the zoom by a factor, clamped to the same 40%..200% range the
   *  wheel uses — the floating − / + buttons share one code path. */
  const zoomBy = useCallback((factor: number) => {
    setZoom((prev) => Math.min(2.0, Math.max(0.4, prev * factor)));
  }, []);

  /** Return to the identity transform (100%, no pan). */
  const resetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, []);

  /** One-shot load auto-fit: when a diagram's content first lands (the
   *  mount preset or an async load) on a MEASURED canvas, fit it if it
   *  overflows the viewport — fixes clipped cards on narrow canvases
   *  (tablet) without ever yanking the view during editing. Content-keyed:
   *  refits when a NEW diagram replaces the current one (preset → load,
   *  preset swap), never for in-place edits, and never after the user has
   *  interacted (a click or key press hands the view to the user). */
  const autoFitKeyRef = useRef('');
  const userInteractedRef = useRef(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || canvas.clientWidth <= 0 || canvas.clientHeight <= 0) return;
    const key = nodes.map((n) => n.id).sort().join('|');
    if (autoFitKeyRef.current === key) return;
    autoFitKeyRef.current = key;
    if (!userInteractedRef.current && !restoredViewRef.current && diagramOverflowsCanvas(canvas, nodes)) {
      zoomToFit();
    }
    // The fit decides on the CONTENT (nodes/wires), not on pan/zoom —
    // those are the values it sets, so they must not re-trigger it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes, wires]);

  /** Internal clipboard for Ctrl+C/Ctrl+V. Wires are kept only when BOTH
   *  endpoints were copied — a half-copied wire would dangle on paste. */
  const clipboardRef = useRef<{ nodes: TopologyNodeData[]; wires: TopologyWireData[] }>({
    nodes: [],
    wires: [],
  });
  /** Pastes cascade one grid step per paste (Figma-style) so repeated
   *  Ctrl+V never stacks copies exactly on top of each other. Reset on a
   *  fresh copy. */
  const pasteCascadeRef = useRef(0);

  const copySelection = useCallback(() => {
    if (selectedNodeIds.size === 0) return;
    const ids = new Set(selectedNodeIds);
    clipboardRef.current = {
      nodes: nodes.filter((n) => ids.has(n.id)).map((n) => ({ ...n })),
      wires: wires.filter((w) => ids.has(w.fromNodeId) && ids.has(w.toNodeId)).map((w) => ({ ...w })),
    };
    pasteCascadeRef.current = 0;
  }, [nodes, wires, selectedNodeIds]);

  /** Duplicate the selection in place: copies offset one grid step down-right,
   *  wires copied only when both endpoints are selected, the copies become
   *  the selection (so repeated Ctrl+D cascades), all in one undo entry. */
  const duplicateSelection = useCallback(() => {
    if (selectedNodeIds.size === 0) return;
    const ids = new Set(selectedNodeIds);
    // Creation-path gates (branch + warehouse tier cap) refuse the gesture
    // BEFORE the history entry, so a blocked duplicate leaves no undo
    // record — shared with paste/Alt+drag so no route can bypass them.
    const refusal = duplicateRefusal(nodes.filter((n) => ids.has(n.id)));
    if (refusal) {
      addToast({ message: l10n.getString(refusal), type: 'warning' });
      return;
    }
    pushHistory();
    const idMap = new Map<string, string>();
    const copies = nodes.filter((n) => ids.has(n.id)).map((n) => {
      const newId = `${n.type}-${crypto.randomUUID()}`;
      idMap.set(n.id, newId);
      const clamped = clampNodeToViewport(n.x + GRID_SIZE, n.y + GRID_SIZE, {
        panX: pan.x,
        panY: pan.y,
        zoom,
        canvasW: canvasRef.current?.clientWidth ?? 0,
        canvasH: canvasRef.current?.clientHeight ?? 0,
      });
      // sanitizeCopiedNode strips a Branch Location copy's canonical
      // identity — the copy is a diagram-only card, never a second
      // branch impersonating the original.
      return { ...sanitizeCopiedNode(n), id: newId, x: clamped.x, y: clamped.y };
    });
    const wireCopies = wires
      .filter((w) => ids.has(w.fromNodeId) && ids.has(w.toNodeId))
      .map((w) => ({
        ...w,
        id: `wire-${crypto.randomUUID()}`,
        fromNodeId: idMap.get(w.fromNodeId)!,
        toNodeId: idMap.get(w.toNodeId)!,
      }));
    setNodes((prev) => [...prev, ...copies]);
    setWires((prev) => [...prev, ...wireCopies]);
    setSelectedNodeIds(new Set(copies.map((c) => c.id)));
    setSelectedNodeId(copies[0]?.id ?? null);
    setSelectedWireId(null);
  }, [nodes, wires, selectedNodeIds, pushHistory, pan, zoom, duplicateRefusal, addToast, l10n]);

  /** Paste the clipboard with a per-paste cascade offset; wires whose both
   *  endpoints were copied come along. The pasted copies become the
   *  selection, and each paste is one undo entry. */
  const pasteClipboard = useCallback(() => {
    const clip = clipboardRef.current;
    if (clip.nodes.length === 0) return;
    // Same creation-path gates as the other routes — a clipboard holding a
    // Branch Location or warehouses past the tier cap is refused before any
    // history entry or cascade offset.
    const refusal = duplicateRefusal(clip.nodes);
    if (refusal) {
      addToast({ message: l10n.getString(refusal), type: 'warning' });
      return;
    }
    pushHistory();
    pasteCascadeRef.current += 1;
    const dx = pasteCascadeRef.current * GRID_SIZE;
    const dy = pasteCascadeRef.current * GRID_SIZE;
    const idMap = new Map<string, string>();
    const copies = clip.nodes.map((n) => {
      const newId = `${n.type}-${crypto.randomUUID()}`;
      idMap.set(n.id, newId);
      const clamped = clampNodeToViewport(n.x + dx, n.y + dy, {
        panX: pan.x,
        panY: pan.y,
        zoom,
        canvasW: canvasRef.current?.clientWidth ?? 0,
        canvasH: canvasRef.current?.clientHeight ?? 0,
      });
      // sanitizeCopiedNode strips a Branch Location copy's canonical
      // identity — the copy is a diagram-only card, never a second
      // branch impersonating the original.
      return { ...sanitizeCopiedNode(n), id: newId, x: clamped.x, y: clamped.y };
    });
    const wireCopies = clip.wires
      .filter((w) => idMap.has(w.fromNodeId) && idMap.has(w.toNodeId))
      .map((w) => ({
        ...w,
        id: `wire-${crypto.randomUUID()}`,
        fromNodeId: idMap.get(w.fromNodeId)!,
        toNodeId: idMap.get(w.toNodeId)!,
      }));
    setNodes((prev) => [...prev, ...copies]);
    setWires((prev) => [...prev, ...wireCopies]);
    setSelectedNodeIds(new Set(copies.map((c) => c.id)));
    setSelectedNodeId(copies[0]?.id ?? null);
    setSelectedWireId(null);
  }, [pushHistory, pan, zoom, duplicateRefusal, addToast, l10n]);

  /** Latest-ref for the spawn handler: the keydown effect runs earlier in
   *  the component body than `handleAddNode`'s const declaration, so a
   *  direct dep would hit the TDZ. The ref is assigned after the function
   *  definition and read by the effect — always the current render's fn. */
  const handleAddNodeRef = useRef<((type: NodeType) => void) | null>(null);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Any key press hands the view to the user — the one-shot load
      // auto-fit must never yank it afterwards (even Delete/Undo, which
      // change the content key, must not trigger a refit).
      userInteractedRef.current = true;
      // F1 — help: toggle the shortcuts popover. Deliberately BEFORE the
      // typing/rack guards: help is never an accidental canvas edit, so it
      // works even while typing or with a rack control focused.
      if (e.key === 'F1') {
        e.preventDefault();
        setShowShortcuts((v) => !v);
        return;
      }
      // Guard: don't handle canvas shortcuts while the user is typing in a text field.
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
        return;
      }
      // Guard: while a non-canvas control (tool rack, header, inspector)
      // owns keyboard focus, canvas shortcuts are inert — a stray
      // Delete/Backspace/arrow after clicking a tool-card or header button
      // would otherwise mutate (or instantly delete) the selected element
      // the user is not looking at. Canvas-internal elements (node cards,
      // port sockets, wire labels) are NOT covered and keep their
      // shortcuts, so keyboard Delete on a focused node still works.
      // `closest` only exists on Elements — keydown can target window/document
      // (tests, programmatic dispatch), which must never throw out of the guard.
      if (target && typeof target.closest === 'function'
        && target.closest('.node-tool-rack, .node-topology-header, .node-inspector-drawer')) {
        return;
      }
      // Guard: a confirm dialog owns the keyboard while it is open — Escape
      // (and any canvas shortcut) must not clear the selection or mutate the
      // canvas under an open delete/preset dialog. The Modal's focus trap
      // closes the dialog itself (bubble order: document listener first).
      // NOTE: every editor-owned confirm dialog must be added to this
      // condition, or its Escape/shortcut handling will leak into the canvas.
      if (relationshipPicker) {
        // The relationship picker owns the keyboard while open: Escape
        // closes it (cancelling the in-flight connection); everything
        // else must NOT leak into the canvas.
        if (e.key === 'Escape') cancelRelationshipPicker();
        return;
      }
      if (confirmDelete || confirmDeleteMany || confirmPreset) {
        return;
      }
      // Tool-slot shortcuts: 1-4 spawn nodes from the palette, matching the
      // rack's card order (Store, Workspace, Warehouse, Hardware). Bare keys
      // only — no modifier, no auto-repeat. The guards above already keep
      // these inert while typing or when a non-canvas control owns focus.
      if (!e.ctrlKey && !e.metaKey && !e.altKey && !e.repeat) {
        const spawnBySlot: Record<string, NodeType> = {
          '1': 'store',
          '2': 'workspace',
          '3': 'warehouse',
          '4': 'hardware',
        };
        const spawnType = spawnBySlot[e.key];
        if (spawnType) {
          e.preventDefault();
          handleAddNodeRef.current?.(spawnType);
          return;
        }
      }
      // Alt pressed MID-move converts the drag into a duplicate (Figma):
      // only while a node drag is in flight and not already duplicating.
      if (e.key === 'Alt' && !e.repeat && dragStartRef.current.size > 0 && !duplicateDragRef.current) {
        convertDragToDuplicate();
        return;
      }
      if (e.key === 'Escape') {
        // Escape closes the finder overlay first — it owns the canvas while
        // open, so a plain Escape must not clear the selection underneath.
        if (finderOpen) {
          setFinderOpen(false);
          return;
        }
        // Escape mid-Alt+drag cancels the duplication: the preview copies
        // are discarded and the originals keep the selection (no history
        // entry — nothing was committed).
        if (duplicateDragRef.current) {
          cancelDuplicateDrag();
          return;
        }
        // Escape mid-bend-drag restores the bend (or removes a ghost-created
        // one) and pops the drag's history entry — same Figma semantics as
        // the node-move cancel below. bendDragRef is set only by a real
        // handle/ghost mousedown, so a stale value cannot swallow the plain
        // Escape.
        if (bendDragRef.current) {
          cancelBendDrag();
          return;
        }
        // Escape mid-MOVE snaps the dragged nodes back to their start
        // positions (Figma semantics) and keeps the selection. Guarded on
        // the drag having actually MOVED: a bare mousedown (select-first,
        // port-click sequence) leaves dragStartRef populated but is not a
        // move to cancel — a stale cancel would swallow the normal Escape
        // (connection/selection clear) below.
        if (dragStartRef.current.size > 0 && dragHasMovedRef.current) {
          cancelNodeMove();
          return;
        }
        // Escape mid-marquee cancels the box and disarms its document
        // finalizer — a release after Escape must not commit a selection
        // from a cancelled marquee (the box would otherwise linger until
        // the next mousedown/mouseup cycle).
        if (marqueeStartRef.current) {
          marqueeStartRef.current = null;
          marqueeRef.current = null;
          setMarquee(null);
          marqueeCleanupRef.current?.();
          return;
        }
        setConnectingFromNodeId(null);
        setConnectingFromPort(null);
        clearSelection();
        setSelectedWireId(null);
        return;
      }
      if ((e.key === 'Delete' || e.key === 'Backspace') && (selectedNodeIds.size > 0 || selectedWireId)) {
        e.preventDefault();
        if (selectedNodeIds.size > 0) {
          const targets = [...selectedNodeIds];
          const hasWires = wires.some((w) => targets.includes(w.fromNodeId) || targets.includes(w.toNodeId));
          if (hasWires) {
            // A single wired node keeps the established dialog; 2+ use the
            // count-aware batch dialog.
            if (targets.length === 1) setConfirmDelete(targets[0]!);
            else setConfirmDeleteMany(targets);
          } else {
            // No connected wires — delete immediately without dialog.
            deleteNodes(targets);
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
      // Ctrl+I — jump focus to the first inspector input when a node is selected
      if ((e.ctrlKey || e.metaKey) && e.key === 'i' && selectedNodeIds.size > 0) {
        e.preventDefault();
        const firstInput = document.querySelector('.inspector-content input');
        if (firstInput instanceof HTMLElement) {
          firstInput.focus();
        }
        return;
      }
      // Clipboard & bulk selection: Ctrl+A select all, Ctrl+D duplicate the
      // selection, Ctrl+C copy, Ctrl+V paste. The typing guard at the top of
      // this handler already returns early inside INPUT/TEXTAREA/contentEditable,
      // so native field copy/paste/select-all is never hijacked.
      if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
        e.preventDefault();
        selectAllNodes();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
        e.preventDefault();
        duplicateSelection();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
        e.preventDefault();
        copySelection();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
        e.preventDefault();
        pasteClipboard();
        return;
      }
      // Zoom shortcuts: Ctrl+0 fit the whole diagram, Ctrl+1 100%, Ctrl+= in,
      // Ctrl+- out — the standard diagram-tool set. The typing guard above
      // keeps native browser zoom intact inside text fields.
      if ((e.ctrlKey || e.metaKey) && (e.key === '0' || e.key === '1' || e.key === '=' || e.key === '+' || e.key === '-')) {
        e.preventDefault();
        if (e.key === '0') zoomToFit();
        else if (e.key === '1') resetView();
        else if (e.key === '=' || e.key === '+') zoomBy(1.25);
        else zoomBy(1 / 1.25);
        return;
      }
      // Ctrl+F — open the node finder (typing guard above keeps native
      // browser find intact inside text fields).
      if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
        e.preventDefault();
        setFinderQuery('');
        setFinderIndex(0);
        setFinderOpen(true);
        return;
      }
      // F2 — inline rename of the single selected renameable node, same
      // flow as the card pencil (store/workspace with a rename callback).
      // The typing guard above already keeps F2 inert inside text fields.
      if (e.key === 'F2' && selectedNodeIds.size === 1) {
        e.preventDefault();
        const nodeId = [...selectedNodeIds][0]!;
        const node = nodes.find((n) => n.id === nodeId);
        if (node
          && ((node.type === 'store' && !!onRenameBranch) || (node.type === 'workspace' && !!onRenameWorkspace))) {
          renameCancelledRef.current = false;
          renameFocusReturnRef.current = null;
          setRenameDraft(node.name);
          setRenamingNodeId(nodeId);
        }
        return;
      }
      if (selectedNodeIds.size > 0 && !e.repeat && (e.key === 'ArrowUp' || e.key === 'ArrowDown' || e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault();
        pushHistory();
        // Shift = FINE nudge (1px, pixel-exact — never grid-rounded); plain
        // arrows move one full grid step when snap is on (deterministic,
        // no dead presses on-grid) or the raw 8px step when it is off.
        const step = e.shiftKey ? 1 : (snapEnabled ? GRID_SIZE : 8);
        const fine = e.shiftKey;
        // Arrow nudges share the SAME dynamic edge clamp as mouse dragging,
        // so keyboard and pointer movement agree on the reachable bounds.
        // The whole multi-selection nudges together. Positions are computed
        // UP FRONT (not inside the updater) so the alignment engine below
        // can run on the exact post-nudge geometry.
        const canvas = canvasRef.current;
        const next = new Map<string, { x: number; y: number }>();
        for (const n of nodes) {
          if (!selectedNodeIds.has(n.id)) continue;
          const rawX = n.x + (e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0);
          const rawY = n.y + (e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0);
          const clamped = clampNodeToViewport(rawX, rawY, {
            panX: pan.x,
            panY: pan.y,
            zoom,
            canvasW: canvas?.clientWidth ?? 0,
            canvasH: canvas?.clientHeight ?? 0,
          });
          next.set(n.id, {
            x: fine ? clamped.x : (snapEnabled ? snap(clamped.x) : clamped.x),
            y: fine ? clamped.y : (snapEnabled ? snap(clamped.y) : clamped.y),
          });
        }
        // Figma-style alignment on FINE nudges only: the round-22 guide
        // engine runs on the nudged selection, so a Shift+arrow landing
        // flush against a neighbour shows the live guide. ENTRY-ONLY snap:
        // the correction applies only when the nudge itself crosses INTO the
        // 6px band (the pre-nudge position was outside it) — once inside,
        // raw 1px moves stand, so a snap can never eat every subsequent
        // nudge. The guide lingers while the band is held and clears when a
        // nudge (fine or grid) leaves it. Plain arrows skip the engine: they
        // are grid steps by design and clear any lingering guide.
        let dx = 0;
        let dy = 0;
        if (fine && next.size > 0) {
          const before = new Map<string, { x: number; y: number }>();
          for (const n of nodes) {
            if (selectedNodeIds.has(n.id)) before.set(n.id, { x: n.x, y: n.y });
          }
          const after = computeAlignmentGuides(next, selectedNodeIds, nodes);
          const pre = computeAlignmentGuides(before, selectedNodeIds, nodes);
          const inX = after.alignedX;
          const inY = after.alignedY;
          const enterX = after.alignedX && !pre.alignedX;
          const enterY = after.alignedY && !pre.alignedY;
          // Delta is the reference MINUS the dragged axis: a nudge must land
          // exactly flush on the line. Applied to the whole group, so it stays
          // rigid. (NOTE: the drag path applies the same delta with the
          // opposite sign — a known stick-lead discrepancy, journaled.)
          if (enterX) dx = -after.dx;
          if (enterY) dy = -after.dy;
          setAlignmentGuide(
            inX || inY
              ? { ...(inX ? { x: after.x } : {}), ...(inY ? { y: after.y } : {}) }
              : null,
          );
        } else {
          setAlignmentGuide(null);
        }
        setNodes((prev) =>
          prev.map((n) => {
            const p = next.get(n.id);
            if (!p) return n;
            return { ...n, x: p.x + dx, y: p.y + dy };
          }),
        );
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [selectedNodeIds, selectedWireId, wires, pushHistory, popUndo, popRedo, confirmDelete, confirmDeleteMany, confirmPreset, pan, zoom, deleteNodes, relationshipPicker, cancelRelationshipPicker, selectAllNodes, duplicateSelection, copySelection, pasteClipboard, nodes, onRenameBranch, onRenameWorkspace, zoomToFit, zoomBy, resetView, snapEnabled, cancelDuplicateDrag, cancelNodeMove, convertDragToDuplicate, cancelBendDrag, finderOpen, clearSelection]);

  const executePresetLoad = useCallback(() => {
    if (confirmPreset) {
      loadPreset(confirmPreset);
    }
    setConfirmPreset(null);
  }, [confirmPreset, loadPreset]);

  const executeDelete = useCallback(() => {
    if (confirmDeleteMany) {
      deleteNodes(confirmDeleteMany);
      setConfirmDeleteMany(null);
      return;
    }
    if (confirmDelete === '') {
      if (selectedWireId) {
        // Deleting a wire is a single-wire mutation — it must NOT cancel a
        // connection in flight (mirrors the direction-toggle rule). The one
        // exception: if the deleted wire is the EXACT duplicate pair the
        // pending connection would create, cancel the pending state —
        // otherwise completing the connection after the delete would
        // silently recreate the wire the user just removed, bypassing the
        // duplicate detector in handlePortClick. The target node is unknown
        // until the connection completes, so the source endpoint is the only
        // match signal — conservative by design: a same-source, different-
        // target wire delete also cancels (ghost preview vanishing signals
        // it), which is the safer failure than silently recreating the
        // deleted wire.
        const deleted = wires.find((w) => w.id === selectedWireId);
        if (
          connectingFromNodeId
          && connectingFromPort
          && deleted
          && ((deleted.fromNodeId === connectingFromNodeId
            && (deleted.fromPort ?? 'right') === connectingFromPort)
            || (deleted.toNodeId === connectingFromNodeId
              && (deleted.toPort ?? 'left') === connectingFromPort))
        ) {
          setConnectingFromNodeId(null);
          setConnectingFromPort(null);
        }
        pushHistory();
        setWires((prev) => prev.filter((w) => w.id !== selectedWireId));
        setSelectedWireId(null);
      }
    } else if (confirmDelete) {
      deleteNodes([confirmDelete]);
    }
    setConfirmDelete(null);
  }, [confirmDelete, confirmDeleteMany, selectedWireId, connectingFromNodeId, connectingFromPort, wires, pushHistory, deleteNodes]);

  /** End an in-flight node drag (release / document mouseup / touch up):
   *  commit any Alt-drag copies, clear the drag set and offsets, and drop
   *  the alignment guide. Shared by the mouse document listener, the canvas
   *  onMouseUp, and the touch gesture loop. */
  const finalizeNodeDrag = useCallback(() => {
    commitDuplicateDrag();
    setDraggingNodeIds(new Set());
    draggingNodeIdsRef.current = new Set();
    dragHasMovedRef.current = false;
    dragOffsetsRef.current.clear();
    dragStartRef.current.clear();
    setAlignmentGuide(null);
    lastDragMovePosRef.current = null;
  }, [commitDuplicateDrag]);

  /** Arm a node drag (mouse mousedown or the touch gesture loop): set the
   *  dragging set, compute each node's grip offset from the pointer, and —
   *  for mouse — attach the document mouseup that finalizes the drag when
   *  the pointer releases outside the canvas. The duplicate (Alt+drag) setup
   *  lives here so every creation path shares one gate and one history
   *  contract. Touch passes gesture='touch': the touch loop owns its own
   *  pointermove/pointerup listeners, so only the drag STATE is armed. */
  const beginNodeDrag = useCallback((
    clientX: number,
    clientY: number,
    selection: Set<string>,
    isDuplicateDrag: boolean,
    gesture: 'mouse' | 'touch',
  ) => {
    userInteractedRef.current = true;
    setRelationshipPicker(null);
    setSelectedWireId(null);
    // Alt+drag = Figma-style DUPLICATE drag: the dragged set is replaced by
    // fresh copies (new ids, starting at the originals' positions) that
    // follow the cursor while the originals stay put; the drop commits them
    // as ONE undo entry, Escape discards them. Wires copy only when BOTH
    // endpoints are in the selection (mirrors duplicateSelection).
    // The creation-path gates apply to the duplicate paths too: an Alt+drag
    // that would copy a Branch Location or a warehouse past the tier cap is
    // refused up front (no copies, no drag, no history entry).
    // All reads go through refs so this handler stays referentially stable
    // across nodes/wires/pan/zoom changes — the memoized cards receive it as
    // a prop, and a churn here would re-render every card on any edit or
    // viewport move. The refs mirror the latest committed state, which is
    // exactly what a mousedown needs.
    const currentNodes = nodesRef.current;
    const currentWires = wiresRef.current;
    if (isDuplicateDrag) {
      const refusal = duplicateRefusal(currentNodes.filter((n) => selection.has(n.id)));
      if (refusal) {
        addToast({ message: l10n.getString(refusal), type: 'warning' });
        return;
      }
    }
    duplicateDragRef.current = isDuplicateDrag;
    const originalToCopy = new Map<string, string>();
    let dragIds: string[];
    if (isDuplicateDrag) {
      const copies = currentNodes
        .filter((n) => selection.has(n.id))
        .map((n) => {
          const newId = `${n.type}-${crypto.randomUUID()}`;
          originalToCopy.set(n.id, newId);
          // sanitizeCopiedNode strips a Branch Location copy's canonical
          // identity — the copy is a diagram-only card, never a second
          // branch impersonating the original.
          return { ...sanitizeCopiedNode(n), id: newId };
        });
      const wireCopies = currentWires
        .filter((w) => selection.has(w.fromNodeId) && selection.has(w.toNodeId))
        .map((w) => ({
          ...w,
          id: `wire-${crypto.randomUUID()}`,
          fromNodeId: originalToCopy.get(w.fromNodeId)!,
          toNodeId: originalToCopy.get(w.toNodeId)!,
        }));
      duplicateCopyIdsRef.current = copies.map((c) => c.id);
      dragIds = duplicateCopyIdsRef.current;
      if (copies.length > 0) {
        setNodes((prev) => [...prev, ...copies]);
        setWires((prev) => [...prev, ...wireCopies]);
      }
      document.body.style.cursor = 'copy';
    } else {
      dragIds = [...selection];
    }
    // Copy: the drag set must never share identity with the live selection
    // state (a future mutation of one would corrupt the other). Mirror the
    // ref SYNCHRONOUSLY too — the touch path calls applyDragMove in the same
    // event handler, before React re-renders and the render-time mirror
    // (draggingNodeIdsRef.current = draggingNodeIds) would catch up.
    const nextDragSet = new Set(dragIds);
    setDraggingNodeIds(nextDragSet);
    draggingNodeIdsRef.current = nextDragSet;
    dragHasMovedRef.current = false;
    // Seed the edge auto-pan direction baseline at the grip point.
    lastDragMovePosRef.current = { x: clientX, y: clientY };

    if (gesture === 'mouse') {
      // Cancel any in-flight drag listener from a previous drag, then arm a
      // document-level mouseup so releasing the pointer outside the canvas
      // still ends the drag (the canvas onMouseUp is unreachable there).
      dragCleanupRef.current?.();
      const handleDocumentMouseUp = () => {
        finalizeNodeDrag();
        document.removeEventListener('mouseup', handleDocumentMouseUp);
        dragCleanupRef.current = null;
      };
      document.addEventListener('mouseup', handleDocumentMouseUp);
      dragCleanupRef.current = () => {
        document.removeEventListener('mouseup', handleDocumentMouseUp);
        dragCleanupRef.current = null;
      };
    }
    // Touch: the touch gesture loop's document pointer listeners own the
    // moves and the finalize — nothing to arm here beyond the drag state.

    const rect = canvasRef.current?.getBoundingClientRect();
    const canvasX = (clientX - (rect?.left ?? 0) - panRef.current.x) / zoomRef.current;
    const canvasY = (clientY - (rect?.top ?? 0) - panRef.current.y) / zoomRef.current;
    dragOffsetsRef.current.clear();
    dragStartRef.current.clear();
    const copyToOriginal = new Map([...originalToCopy].map(([k, v]) => [v, k]));
    // Position lookup via a fresh map from the ref — `nodeMap`'s identity
    // tracks nodes, and taking it as a dep would re-key this handler (and
    // every card prop) on any node edit.
    const nodeMapNow = new Map(currentNodes.map((n) => [n.id, n]));
    for (const id of dragIds) {
      // Duplicate-drag offsets come from the ORIGINALS (the copies start at
      // their positions and aren't in the map until the state flush), but
      // are keyed by the copy ids the drag actually moves.
      const srcId = isDuplicateDrag ? copyToOriginal.get(id) : id;
      const n = srcId ? nodeMapNow.get(srcId) : null;
      if (n) {
        dragOffsetsRef.current.set(id, { x: canvasX - n.x, y: canvasY - n.y });
        dragStartRef.current.set(id, { x: n.x, y: n.y });
      }
    }
  }, [duplicateRefusal, addToast, l10n, finalizeNodeDrag]);

  const handleNodeMouseDown = useCallback((e: React.MouseEvent, nodeId: string) => {
    e.stopPropagation();
    setRelationshipPicker(null);
    if (e.button !== 0) return;
    // Multi-select rules: shift+mousedown ADDS the node to the selection;
    // a plain mousedown on an unselected node collapses to just it; a
    // mousedown on a node already inside a multi-selection keeps the group
    // so it can be dragged as a whole. The selection is read via the ref so
    // this handler stays stable across selection changes (the memoized
    // cards all receive it as a prop).
    const currentSelection = selectedNodeIdsRef.current;
    const wasSelected = currentSelection.has(nodeId);
    let selection: Set<string>;
    if (e.shiftKey && !wasSelected) {
      selection = new Set(currentSelection);
      selection.add(nodeId);
      setSelectedNodeIds(selection);
      setSelectedNodeId(nodeId);
    } else if (!wasSelected) {
      selection = new Set([nodeId]);
      selectOnly(nodeId);
    } else {
      selection = new Set(currentSelection);
    }
    beginNodeDrag(e.clientX, e.clientY, selection, e.altKey, 'mouse');
  }, [selectOnly, beginNodeDrag]);

  /** Apply one drag-move to the dragged group (mouse canvas mousemove and
   *  the touch gesture loop share this). Reads the dragging set and nodes
   *  via refs so the touch path — which runs in the document-listener
   *  closure armed at pointerdown — always sees the CURRENT drag state, not
   *  the stale render-time snapshot. */
  const applyDragMove = (clientX: number, clientY: number) => {
    if (draggingNodeIdsRef.current.size === 0) return;
    // Push history once, on the first real movement — a plain click that
    // never moves must not create a no-op undo entry. An Alt+drag defers
    // its entry to the drop (one undo for the whole duplicate).
    if (!dragHasMovedRef.current) {
      dragHasMovedRef.current = true;
      if (!duplicateDragRef.current) pushHistory();
    }
    // Edge auto-pan: a pointer inside an edge band pans the viewport so a
    // drag can keep moving across a large diagram instead of stalling at
    // the viewport clamp. Reads the CURRENT pan via panRef (the touch
    // gesture loop runs in a down-time closure; the mouse path is equally
    // fresh) and derives the drag math from the POST-pan view, so the
    // dragged node tracks the pointer through the scroll. Pointers OUTSIDE
    // the canvas produce no delta — the clamp below then holds the node at
    // the edge (the never-lose-a-node invariant).
    const canvas = canvasRef.current;
    const rect = canvas?.getBoundingClientRect();
    const curPan = panRef.current;
    const curZoom = zoomRef.current;
    let auto = edgeAutoPanDelta(
      clientX - (rect?.left ?? 0),
      clientY - (rect?.top ?? 0),
      canvas?.clientWidth ?? 0,
      canvas?.clientHeight ?? 0,
    );
    // Direction gate: only pan toward the edge the pointer is pushing
    // against. A drag drifting AWAY from the edge (or holding still) must
    // not scroll — proximity alone would pan while dragging toward the
    // diagram's interior near a corner.
    const lastPos = lastDragMovePosRef.current;
    if (lastPos) {
      const moveDx = clientX - lastPos.x;
      const moveDy = clientY - lastPos.y;
      if (auto.dx !== 0 && Math.sign(auto.dx) !== Math.sign(moveDx)) auto = { ...auto, dx: 0 };
      if (auto.dy !== 0 && Math.sign(auto.dy) !== Math.sign(moveDy)) auto = { ...auto, dy: 0 };
    }
    lastDragMovePosRef.current = { x: clientX, y: clientY };
    const nextPan = auto.dx === 0 && auto.dy === 0
      ? curPan
      : { x: curPan.x + auto.dx, y: curPan.y + auto.dy };
    if (nextPan !== curPan) setPan(nextPan);
    const rawX = (clientX - (rect?.left ?? 0) - nextPan.x) / curZoom;
    const rawY = (clientY - (rect?.top ?? 0) - nextPan.y) / curZoom;
    // Dynamic edge clamp: every node in the dragged group may travel
    // north/west until its box nearly leaves the visible canvas, but can
    // never be pushed off-screen and lost. Pan/zoom aware, so the reachable
    // edge follows the current view. Each node clamps independently; the
    // group delta is otherwise identical (same raw cursor → same per-node
    // offset).
    const targets = new Map<string, { x: number; y: number }>();
    for (const [id, off] of dragOffsetsRef.current) {
      if (!draggingNodeIdsRef.current.has(id)) continue;
      targets.set(id, clampNodeToViewport(rawX - off.x, rawY - off.y, {
        panX: nextPan.x,
        panY: nextPan.y,
        zoom: curZoom,
        canvasW: canvas?.clientWidth ?? 0,
        canvasH: canvas?.clientHeight ?? 0,
      }));
    }
    // Figma-style COLLECTIVE alignment: every dragged node's edges/centers
    // snap to stationary nodes' edges/centers within a small threshold; the
    // closest match across the whole group wins per axis and the delta
    // applies to the group so it stays rigid (a non-grabbed member's edge
    // can snap the group — Figma semantics). The aligned axis skips grid
    // snapping (guides beat the grid); the other axis still snaps as
    // configured.
    const align = targets.size > 0
      ? computeAlignmentGuides(targets, draggingNodeIdsRef.current, nodesRef.current)
      : { dx: 0, dy: 0, alignedX: false, alignedY: false };
    setAlignmentGuide(
      align.x !== undefined || align.y !== undefined
        ? { ...(align.x !== undefined ? { x: align.x } : {}), ...(align.y !== undefined ? { y: align.y } : {}) }
        : null,
    );
    setNodes((prev) =>
      prev.map((n) => {
        const off = dragOffsetsRef.current.get(n.id);
        if (!off) return n;
        const clamped = clampNodeToViewport(rawX - off.x, rawY - off.y, {
          panX: nextPan.x,
          panY: nextPan.y,
          zoom: curZoom,
          canvasW: canvas?.clientWidth ?? 0,
          canvasH: canvas?.clientHeight ?? 0,
        });
        // The delta is the dragged axis MINUS the reference (pAxis − rAxis),
        // so SUBTRACTING it lands the edge exactly on the line — a drag that
        // raw-lands 3px off snaps onto it, never parking 2× the miss away.
        let fx = clamped.x - align.dx;
        let fy = clamped.y - align.dy;
        if (snapEnabled) {
          if (!align.alignedX) fx = snap(fx);
          if (!align.alignedY) fy = snap(fy);
        }
        return { ...n, x: fx, y: fy };
      }),
    );
  };

  const handleCanvasMouseMove = (e: React.MouseEvent) => {
    mousePosRef.current = { x: e.clientX, y: e.clientY };
    // NOTE: the HUD cursor readout is NOT fed here — CanvasCursorReadout
    // owns its own document listener + rAF, so canvas mousemoves re-render
    // only that span, never the editor.
    applyDragMove(e.clientX, e.clientY);
    if (marqueeStartRef.current) {
      // Marquee: track the drag rect in container-relative screen px.
      const rect = canvasRef.current?.getBoundingClientRect();
      const next = {
        x0: marqueeStartRef.current.x,
        y0: marqueeStartRef.current.y,
        x1: e.clientX - (rect?.left ?? 0),
        y1: e.clientY - (rect?.top ?? 0),
      };
      setMarquee(next);
      marqueeRef.current = next;
    } else if (connectingFromNodeId) {
      // Find nearest target port when dragging a connection
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;
      const mx = (e.clientX - rect.left - pan.x) / zoom;
      const my = (e.clientY - rect.top - pan.y) / zoom;
      setPreviewCursor({ x: mx, y: my });
      const SNAP_DIST = 30;
      let closest: { nodeId: string; port: PortName; variantIndex: number; dist: number } | null = null;
      for (const n of nodes) {
        if (n.id === connectingFromNodeId) continue;
    // Snap candidates mirror the visible sockets: one left input per node
    // plus a single right output (stores expose output only).
    const candidates: Array<{ port: PortName; variantIndex: number; off: { dx: number; dy: number } }> = [
      { port: 'left', variantIndex: 0, off: { dx: PORT_OFFSET.left.dx, dy: leftPortDy(n, 0) } },
    ];
    if (n.type !== 'store') {
      candidates.push({ port: 'right', variantIndex: 0, off: PORT_OFFSET.right });
    }
        for (const c of candidates) {
          const px = n.x + c.off.dx;
          const py = n.y + c.off.dy;
          const dist = Math.sqrt((mx - px) ** 2 + (my - py) ** 2);
          if (dist < SNAP_DIST && (!closest || dist < closest.dist)) {
            closest = { nodeId: n.id, port: c.port, variantIndex: c.variantIndex, dist };
          }
        }
      }
      setHoveredTarget(closest ? { nodeId: closest.nodeId, port: closest.port, variantIndex: closest.variantIndex } : null);
    }
  };

  const handleCanvasMouseUp = () => {
    finalizeNodeDrag();
    // The marquee is finalized by its own document-level mouseup listener
    // (armed at marquee start), which also fires when the pointer is
    // released OUTSIDE the canvas — the canvas onMouseUp is unreachable
    // there, and without it the box would linger and re-open on the next
    // mousemove.
  };

  /** Minimap: a scaled overview of the whole diagram. Click or drag to
   *  recenter the viewport on that canvas point; arrows nudge the view,
   *  Enter centers on the content box. Drag arms document-level listeners
   *  (cleanup ref, same pattern as node drag) so the map keeps panning
   *  even when the pointer leaves the widget. */
  const minimapRef = useRef<HTMLDivElement>(null);
  const minimapDragCleanupRef = useRef<(() => void) | null>(null);

  const recenterViewOn = (px: number, py: number) => {
    if (!contentBounds) return;
    const cx = contentBounds.minX + (px - MINIMAP_PAD) / minimapScale;
    const cy = contentBounds.minY + (py - MINIMAP_PAD) / minimapScale;
    const canvas = canvasRef.current;
    const cw = canvas?.clientWidth ?? 0;
    const ch = canvas?.clientHeight ?? 0;
    setPan({ x: cw / 2 - cx * zoom, y: ch / 2 - cy * zoom });
  };

  const handleMinimapMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const rect = minimapRef.current?.getBoundingClientRect();
    recenterViewOn(e.clientX - (rect?.left ?? 0), e.clientY - (rect?.top ?? 0));
    minimapDragCleanupRef.current?.();
    const handleMove = (ev: MouseEvent) => {
      const r = minimapRef.current?.getBoundingClientRect();
      recenterViewOn(ev.clientX - (r?.left ?? 0), ev.clientY - (r?.top ?? 0));
    };
    const handleUp = () => {
      document.removeEventListener('mousemove', handleMove);
      document.removeEventListener('mouseup', handleUp);
      minimapDragCleanupRef.current = null;
    };
    document.addEventListener('mousemove', handleMove);
    document.addEventListener('mouseup', handleUp);
    minimapDragCleanupRef.current = () => {
      document.removeEventListener('mousemove', handleMove);
      document.removeEventListener('mouseup', handleUp);
      minimapDragCleanupRef.current = null;
    };
  };

  const handleMinimapKeyDown = (e: React.KeyboardEvent) => {
    if (!contentBounds) return;
    const canvas = canvasRef.current;
    const cw = canvas?.clientWidth ?? 0;
    const ch = canvas?.clientHeight ?? 0;
    if (e.key === 'Enter') {
      const cx = contentBounds.minX + (contentBounds.maxX - contentBounds.minX) / 2;
      const cy = contentBounds.minY + (contentBounds.maxY - contentBounds.minY) / 2;
      setPan({ x: cw / 2 - cx * zoom, y: ch / 2 - cy * zoom });
      return;
    }
    const STEP = 40;
    let dx = 0;
    let dy = 0;
    if (e.key === 'ArrowLeft') dx = -STEP;
    else if (e.key === 'ArrowRight') dx = STEP;
    else if (e.key === 'ArrowUp') dy = -STEP;
    else if (e.key === 'ArrowDown') dy = STEP;
    else return;
    e.preventDefault();
    setPan((p) => ({ x: p.x + dx, y: p.y + dy }));
  };

  // Clear hoveredTarget when connection mode ends
  useEffect(() => {
    if (!connectingFromNodeId) {
      setHoveredTarget(null);
    }
  }, [connectingFromNodeId]);

  /** Commit the marquee at its release point: a forward drag (left→right)
   *  selects only nodes FULLY contained in the box, a backward drag
   *  (right→left) selects every node the box touches (screen space at
   *  identity pan/zoom), or leave the selection cleared if the box captured
   *  nothing (a background click). The rect is derived from the START ref +
   *  release coords, so a document listener armed at mousedown never reads
   *  a stale rect. */
  const finalizeMarquee = () => {
    const start = marqueeStartRef.current;
    marqueeStartRef.current = null;
    if (!start) return;
    // Only a marquee that actually RENDERED (the pointer moved) commits — a
    // mousedown+mouseup without movement is a plain background click, and
    // the selection was already cleared when it started. The ref mirror
    // also keeps this document-armed listener free of stale-closure risk.
    const box = marqueeRef.current;
    marqueeRef.current = null;
    setMarquee(null);
    if (!box) return;
    const mx0 = Math.min(box.x0, box.x1);
    const mx1 = Math.max(box.x0, box.x1);
    const my0 = Math.min(box.y0, box.y1);
    const my1 = Math.max(box.y0, box.y1);
    // A degenerate (click-sized) box selects nothing.
    if (mx1 - mx0 < 1 || my1 - my0 < 1) return;
    // Direction-aware marquee (Figma/draw.io convention): a FORWARD drag
    // (left→right) selects only nodes FULLY contained in the box; a
    // BACKWARD drag (right→left) selects every node the box touches.
    const forward = box.x1 >= box.x0;
    const hit = nodes.filter((n) => {
      const nx = n.x * zoom + pan.x;
      const ny = n.y * zoom + pan.y;
      const nx1 = nx + NODE_WIDTH * zoom;
      const ny1 = ny + NODE_HEIGHT * zoom;
      if (forward) {
        return nx >= mx0 && nx1 <= mx1 && ny >= my0 && ny1 <= my1;
      }
      return nx1 >= mx0 && nx <= mx1 && ny1 >= my0 && ny <= my1;
    });
    const additive = marqueeAdditiveRef.current;
    marqueeAdditiveRef.current = false;
    if (hit.length > 0) {
      if (additive) {
        // Union with the selection captured at mousedown (the finalizer's
        // closure is from that render, so it still holds the pre-drag set).
        const union = new Set(selectedNodeIds);
        for (const n of hit) union.add(n.id);
        setSelectedNodeIds(union);
      } else {
        setSelectedNodeIds(new Set(hit.map((n) => n.id)));
      }
      // The primary (inspector target) is the last node in render order.
      setSelectedNodeId(hit[hit.length - 1]!.id);
      setSelectedWireId(null);
    } else if (!additive) {
      clearSelection();
    }
  };

  /** Start a pan gesture from any button: middle/right drags and the
   *  Space+left-drag modifier. Document-level listeners keep the pan
   *  tracking even when the pointer leaves the canvas. */
  const startPan = (e: React.MouseEvent, clearSelectionFirst: boolean) => {
    if (clearSelectionFirst) clearSelection();
    isPanningRef.current = true;
    panStartRef.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
    document.body.style.cursor = 'grabbing';

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
      document.body.style.cursor = '';
      panCleanupRef.current = null;
    };
  };

  const handleCanvasMouseDown = (e: React.MouseEvent) => {
    userInteractedRef.current = true;
    setRelationshipPicker(null);
    setContextMenu(null);
    const targetEl = e.target as HTMLElement;
    if (targetEl === e.currentTarget || targetEl.classList.contains('node-canvas-viewport') || targetEl.tagName === 'svg') {
      setSelectedWireId(null);
      if (e.button === 0 && (spaceDownRef.current || panToolActive)) {
        // Space+drag (or the active Pan tool) pans like the middle/right
        // button, but Figma-style it preserves the current selection
        // instead of clearing it.
        startPan(e, false);
      } else if (e.button === 0) {
        // Left-drag on empty background is the marquee selector; a plain
        // click (no movement) clears the selection on mouseup. Shift+drag is
        // ADDITIVE: the current selection is kept so the marquee unions into
        // it at release (and a Shift+click on empty canvas clears nothing).
        // The marquee coords are container-relative screen px (the viewport
        // inside is panned/zoomed, so node boxes are compared in screen
        // space too). A document-level mouseup finalizes the box, so
        // releasing outside the canvas still commits the selection instead
        // of leaking a half-open marquee.
        const additive = e.shiftKey;
        marqueeAdditiveRef.current = additive;
        if (!additive) {
          clearSelection();
        }
        const rect = canvasRef.current?.getBoundingClientRect();
        marqueeStartRef.current = {
          x: e.clientX - (rect?.left ?? 0),
          y: e.clientY - (rect?.top ?? 0),
        };
        marqueeRef.current = null;
        marqueeCleanupRef.current?.();
        const handleMarqueeMouseUp = () => {
          finalizeMarquee();
          marqueeCleanupRef.current?.();
        };
        document.addEventListener('mouseup', handleMarqueeMouseUp);
        marqueeCleanupRef.current = () => {
          document.removeEventListener('mouseup', handleMarqueeMouseUp);
          marqueeCleanupRef.current = null;
        };
      } else if (e.button === 1 || e.button === 2) {
        // Middle/right-button drag pans the canvas.
        startPan(e, true);
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

  // ── Touch gestures (pointer parity for tablets) ────────────────
  // Mouse input keeps the mouse handlers above (and all their tests); touch
  // input runs entirely through pointer events. One finger on a node card
  // drags it, one finger on empty canvas pans (a sub-threshold touch is a
  // tap that clears the selection), and two fingers pinch-zoom about the
  // midpoint. The gesture loop runs in DOCUMENT-level pointer listeners
  // armed at the first pointerdown: touch pointers have implicit capture,
  // so moves/ups keep firing even when the finger leaves the canvas, and
  // dispatching on the canvas (tests) still bubbles to the document. All
  // gesture state lives in refs so the stale down-time closure always sees
  // the latest drag/pan/zoom.
  const touchPointersRef = useRef<Map<number, { x: number; y: number }>>(new Map());
  interface TouchGesture {
    mode: 'none' | 'node-drag' | 'pan' | 'pinch';
    startX: number;
    startY: number;
    nodeId: string | null;
    selection: Set<string>;
    panStart: { x: number; y: number };
    pinchZoom0: number;
    pinchPan0: { x: number; y: number };
    pinchMid0: { x: number; y: number };
    pinchDist0: number;
  }
  const touchGestureRef = useRef<TouchGesture | null>(null);
  const touchCleanupRef = useRef<(() => void) | null>(null);

  /** Finish a touch gesture with all fingers lifted: finalize any node drag
   *  (commits Alt-style copies — none on touch — and clears drag state),
   *  end a pan, or resolve a tap (empty-canvas taps clear the selection). */
  const endTouchGesture = (g: TouchGesture) => {
    if (g.mode === 'node-drag') {
      finalizeNodeDrag();
    } else if (g.mode === 'pan') {
      // Touch pans never emit the native contextmenu (no right button), so
      // the contextmenu-suppression ref is a mouse-only concern.
      isPanningRef.current = false;
      document.body.style.cursor = '';
    } else if (g.mode === 'none' && g.nodeId === null) {
      // A tap on empty canvas is the touch equivalent of a plain click
      // (the mouse path finalizes an empty marquee, which clears the
      // selection). Node taps already selected at pointerdown.
      clearSelection();
      setSelectedWireId(null);
    }
    touchPointersRef.current.clear();
    touchGestureRef.current = null;
    touchCleanupRef.current?.();
  };

  const handleTouchPointerMove = (e: PointerEvent) => {
    if (e.pointerType !== 'touch') return;
    const prev = touchPointersRef.current.get(e.pointerId);
    if (prev) touchPointersRef.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const g = touchGestureRef.current;
    if (!g) return;
    if (g.mode === 'pinch') {
      if (touchPointersRef.current.size < 2) return;
      const pts = [...touchPointersRef.current.values()];
      const p1 = pts[0]!;
      const p2 = pts[1]!;
      const out = pinchTransform(
        { zoom: g.pinchZoom0, pan: g.pinchPan0 },
        g.pinchMid0,
        g.pinchDist0,
        { x: (p1.x + p2.x) / 2, y: (p1.y + p2.y) / 2 },
        Math.hypot(p2.x - p1.x, p2.y - p1.y),
      );
      setZoom(out.zoom);
      setPan(out.pan);
      return;
    }
    if (g.mode === 'none') {
      const dx = e.clientX - g.startX;
      const dy = e.clientY - g.startY;
      if (Math.hypot(dx, dy) < TOUCH_DRAG_THRESHOLD) return;
      if (g.nodeId !== null) {
        // The node was already selected at pointerdown; arm the drag with
        // the stored selection (the group, when the touch landed on an
        // already-selected member).
        beginNodeDrag(g.startX, g.startY, g.selection, false, 'touch');
        g.mode = 'node-drag';
      } else {
        isPanningRef.current = true;
        document.body.style.cursor = 'grabbing';
        // Pan baseline from the down-time view (same as startPan's
        // panStartRef: clientX − pan.x).
        g.panStart = { x: g.startX - pan.x, y: g.startY - pan.y };
        g.mode = 'pan';
      }
    }
    if (g.mode === 'node-drag') {
      applyDragMove(e.clientX, e.clientY);
    } else if (g.mode === 'pan') {
      setPan({ x: e.clientX - g.panStart.x, y: e.clientY - g.panStart.y });
    }
  };

  const handleTouchPointerUp = (e: PointerEvent) => {
    if (e.pointerType !== 'touch') return;
    touchPointersRef.current.delete(e.pointerId);
    const g = touchGestureRef.current;
    if (!g) {
      // All fingers lifted outside a gesture (e.g. the inert finger left
      // after a pinch disarmed the gesture) — drop the listeners.
      if (touchPointersRef.current.size === 0) touchCleanupRef.current?.();
      return;
    }
    if (touchPointersRef.current.size > 0) {
      // A finger remains down. After a pinch (or an armed-but-unmoved
      // gesture) the remaining finger must not continue a pan or drag —
      // disarm until all fingers lift.
      if (g.mode === 'pinch' || g.mode === 'none') touchGestureRef.current = null;
      return;
    }
    endTouchGesture(g);
  };

  /** A system gesture stole the touch (scroll, notification) — end the
   *  gesture exactly like a release; the finger is already gone. */
  const handleTouchPointerCancel = (e: PointerEvent) => {
    if (e.pointerType !== 'touch') return;
    handleTouchPointerUp(e);
  };

  /** Arm the document-level touch gesture listeners once, when the first
   *  touch pointer lands. Removed by endTouchGesture when all fingers lift. */
  const armTouchDocumentListeners = () => {
    if (touchCleanupRef.current) return;
    document.addEventListener('pointermove', handleTouchPointerMove);
    document.addEventListener('pointerup', handleTouchPointerUp);
    document.addEventListener('pointercancel', handleTouchPointerCancel);
    touchCleanupRef.current = () => {
      document.removeEventListener('pointermove', handleTouchPointerMove);
      document.removeEventListener('pointerup', handleTouchPointerUp);
      document.removeEventListener('pointercancel', handleTouchPointerCancel);
      touchCleanupRef.current = null;
    };
  };

  const handleCanvasPointerDown = (e: React.PointerEvent) => {
    if (e.pointerType !== 'touch') return;
    // Suppress the compatibility mouse events (mousedown/mouseup) a real
    // browser dispatches after touch — without this, a touch pan would
    // spawn a ghost marquee and a touch node-tap would double-arm a drag.
    e.preventDefault();
    userInteractedRef.current = true;
    setRelationshipPicker(null);
    setContextMenu(null);
    touchPointersRef.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const g = touchGestureRef.current;
    if (touchPointersRef.current.size === 1) {
      // First finger: arm a drag (node card) or pan (background) candidate.
      const target = e.target as HTMLElement;
      const card = target.closest('.topology-node');
      const onNode = !!card && !target.closest('input, button, select, textarea, [data-no-node-drag]');
      if (onNode) {
        const nodeId = (card as HTMLElement).dataset['nodeId'] ?? null;
        if (nodeId) {
          // Selection mirrors the mouse mousedown rules: a tap on an
          // unselected node collapses to it; a tap on an already-selected
          // node keeps the group so it can be dragged as a whole.
          const wasSelected = selectedNodeIds.has(nodeId);
          if (!wasSelected) selectOnly(nodeId);
          setSelectedWireId(null);
          touchGestureRef.current = {
            mode: 'none',
            startX: e.clientX,
            startY: e.clientY,
            nodeId,
            selection: wasSelected ? new Set(selectedNodeIds) : new Set([nodeId]),
            panStart: { x: 0, y: 0 },
            pinchZoom0: 1,
            pinchPan0: { x: 0, y: 0 },
            pinchMid0: { x: 0, y: 0 },
            pinchDist0: 0,
          };
        }
      } else {
        touchGestureRef.current = {
          mode: 'none',
          startX: e.clientX,
          startY: e.clientY,
          nodeId: null,
          selection: new Set(),
          panStart: { x: 0, y: 0 },
          pinchZoom0: 1,
          pinchPan0: { x: 0, y: 0 },
          pinchMid0: { x: 0, y: 0 },
          pinchDist0: 0,
        };
      }
    } else if (touchPointersRef.current.size === 2) {
      // Second finger: commit any in-flight node drag (the node stays where
      // it is; its undo entry was already pushed on first movement), then
      // enter pinch about the two fingers' midpoint.
      if (g?.mode === 'node-drag') finalizeNodeDrag();
      const pts = [...touchPointersRef.current.values()];
      const p1 = pts[0]!;
      const p2 = pts[1]!;
      touchGestureRef.current = {
        mode: 'pinch',
        startX: 0,
        startY: 0,
        nodeId: null,
        selection: new Set(),
        panStart: { x: 0, y: 0 },
        pinchZoom0: zoom,
        pinchPan0: pan,
        pinchMid0: { x: (p1.x + p2.x) / 2, y: (p1.y + p2.y) / 2 },
        pinchDist0: Math.hypot(p2.x - p1.x, p2.y - p1.y),
      };
    }
    armTouchDocumentListeners();
  };

  const handleAddNode = (type: NodeType, at?: { x: number; y: number }) => {
    // Strict mode (the real topology screen) builds the branch card from
    // the authoritative branchLocations list — a palette-spawned store has
    // no storeProfileId and nothing can attach one, so it could never be
    // applied. Refuse the spawn there; the palette slot, context-menu
    // entry, and the 1-slot shortcut are hidden too.
    if (type === 'store' && !allowLegacyApply) return;
    if (type === 'warehouse' && wouldExceedWarehouseCap(1)) {
      addToast({ message: l10n.getString('topology-toast-multi-warehouse'), type: 'warning' });
      return;
    }
    pushHistory();

    const id = `${type}-${crypto.randomUUID()}`;
    // Placement: a context-menu spawn honors the cursor; a palette spawn
    // jitters near the origin then settles into the first collision-free
    // spot (the old jitter box sat entirely inside the preset branch card,
    // so palette spawns stacked invisibly on top of it). Both are clamped
    // into the visible viewport so a node can never land off-canvas, and a
    // palette spot that was outside the view (panned/zoomed away) pans the
    // viewport so the fresh node is revealed instead of silently invisible.
    const raw = at
      ? { x: snapOrNot(at.x), y: snapOrNot(at.y) }
      : { x: snapOrNot(200 + Math.random() * 100), y: snapOrNot(150 + Math.random() * 100) };
    const free = at ? raw : findFreeSpawnSpot(raw, nodes.map((n) => ({ x: n.x, y: n.y })));
    const canvas = canvasRef.current;
    const canvasW = canvas?.clientWidth ?? 0;
    const canvasH = canvas?.clientHeight ?? 0;
    const placed = clampNodeToViewport(free.x, free.y, {
      panX: pan.x,
      panY: pan.y,
      zoom,
      canvasW,
      canvasH,
    });
    if (!at && canvasW > 0 && canvasH > 0
      && (placed.x !== free.x || placed.y !== free.y)) {
      // The natural palette spot was off-view — pan to reveal the node
      // (mirrors the node-finder jump).
      setPan({
        x: canvasW / 2 - (placed.x + NODE_WIDTH / 2) * zoom,
        y: canvasH / 2 - (placed.y + NODE_HEIGHT / 2) * zoom,
      });
    }
    const newNode: TopologyNodeData = {
      id,
      type,
      name: l10n.getString(`topology-new-${type}`),
      subtitle: l10n.getString(`topology-new-${type}-subtitle`),
      x: placed.x,
      y: placed.y,
      telemetryBadge: l10n.getString('topology-new-ready'),
      telemetryStatus: 'online',
      // New workspace nodes default to the retail POS type until the user
      // picks another in the inspector. `persisted: false` marks it as not
      // yet backed by a workspace_instances row so onSave will create it.
      ...(type === 'workspace' ? { metadata: { typeKey: 'store-pos', purposeKey: 'general', persisted: false } } : {}),
    };

    setNodes((prev) => [...prev, newNode]);
    setFreshNodeIds((prev) => new Set(prev).add(id));
    // Remove from fresh set after animation completes
    const freshTimer = setTimeout(() => {
      setFreshNodeIds((prev) => { const next = new Set(prev); next.delete(id); return next; });
      freshTimersRef.current.delete(freshTimer);
    }, 400);
    freshTimersRef.current.add(freshTimer);
    selectOnly(id);
  };
  handleAddNodeRef.current = handleAddNode;

  const portDirection = useCallback((port: PortName): 'input' | 'output' => (
    port === 'left' ? 'input' : 'output'
  ), []);

  const isPortCompatible = useCallback((nodeId: string, port: PortName, variantIndex = 0): boolean => {
    if (!connectingFromNodeId || !connectingFromPort) return false;
    if (nodeId === connectingFromNodeId) return false;
    if (portDirection(port) !== 'input') return false;
    const source = nodeMap.get(connectingFromNodeId);
    const target = nodeMap.get(nodeId);
    if (!source || !target) return false;
    // Compatibility is decided by the semantic pairing table (ADR #34): a
    // drop is only completable into a target socket that admits at least
    // one relationship with the source socket. Untyped sockets gate closed
    // — no valid pair, no connection. A pair admitting MULTIPLE
    // relationships stays compatible; the picker disambiguates at drop.
    return wireRelationshipOptions(source, connectingFromPort, target, port, variantIndex).length > 0;
  }, [connectingFromNodeId, connectingFromPort, nodeMap, portDirection]);

  /** Live semantic validation of the CURRENT canvas (ADR #34 slice 2).
   *  Mirrors the Apply gate exactly — same normalize + validate, same
   *  canonical-identity condition — so the on-canvas badges and the Apply
   *  toast can never disagree. Errors carrying a nodeId pin to that card as
   *  a note; graph-level errors (branch roots, wire integrity) surface as
   *  the canvas banner. */
  const liveValidation = useMemo(() => {
    const errors = validateEditorGraph(nodes, wires, allowLegacyApply, currentTier);
    const byNode = new Map<string, TopologyValidationError[]>();
    const graphLevel: TopologyValidationError[] = [];
    for (const err of errors) {
      if (err.nodeId) {
        const list = byNode.get(err.nodeId);
        if (list) list.push(err);
        else byNode.set(err.nodeId, [err]);
      } else {
        graphLevel.push(err);
      }
    }
    return { byNode, graphLevel };
  }, [nodes, wires, allowLegacyApply, currentTier]);

  /** Aggregated issue list for the validation panel: per-node problems
   *  first (actionable — clicking jumps to the node), then graph-level. */
  const [validationPanelOpen, setValidationPanelOpen] = useState(false);
  const nodeIssues = useMemo(() => {
    const out: Array<{ nodeId: string; nodeName: string; messageId: string }> = [];
    for (const [nodeId, errs] of liveValidation.byNode) {
      const nodeName = nodeMap.get(nodeId)?.name ?? nodeId;
      for (const e of errs) out.push({ nodeId, nodeName, messageId: e.messageId });
    }
    return out;
  }, [liveValidation, nodeMap]);

  /** Mark-issue-resolved: dismissals of validation issues, persisted per
   *  diagram (branch) so a dismissal survives reloads and branch switches.
   *  Dismissals are OCCURRENCE-scoped — the forget effect below drops a
   *  stored key once the issue leaves the live set, so a genuinely NEW
   *  occurrence later surfaces again instead of staying hidden forever.
   *  Cosmetic only: the Apply gate validates the raw graph and is never
   *  bypassed by a dismissal. */
  const resolvedIssuesKey = `oz-topology-resolved-issues:${branchId ?? 'unassigned'}`;
  const [resolvedIssues, setResolvedIssues] = useState<Set<string>>(() => {
    try {
      const raw = localStorage.getItem(resolvedIssuesKey);
      if (raw) {
        const parsed = JSON.parse(raw) as unknown;
        if (Array.isArray(parsed)) {
          return new Set(parsed.filter((k): k is string => typeof k === 'string'));
        }
      }
    } catch { /* corrupted — start empty */ }
    return new Set();
  });
  const dismissIssue = (key: string) =>
    setResolvedIssues((prev) => {
      if (prev.has(key)) return prev;
      const next = new Set(prev);
      next.add(key);
      return next;
    });
  /** Visible (non-dismissed) issues drive the button count, the panel, the
   *  banner, and the card notes — every surface reads the same filtered
   *  lists so they can never disagree. */
  const visibleNodeIssues = useMemo(
    () => nodeIssues.filter((i) => !resolvedIssues.has(issueKey(i.nodeId, i.messageId))),
    [nodeIssues, resolvedIssues],
  );
  const visibleGraphLevel = useMemo(
    () => liveValidation.graphLevel.filter((e) => !resolvedIssues.has(graphIssueKey(e.messageId))),
    [liveValidation, resolvedIssues],
  );
  const totalIssues = visibleNodeIssues.length + visibleGraphLevel.length;

  /** Per-card visible errors, memoized so the memoized node cards receive a
   *  STABLE nodeErrors prop (a per-render `.filter()` would defeat the memo
   *  for every card carrying an issue). */
  const nodeErrorsByNode = useMemo(() => {
    const m = new Map<string, TopologyValidationError[]>();
    for (const n of nodes) {
      const errs = liveValidation.byNode
        .get(n.id)
        ?.filter((e) => !resolvedIssues.has(issueKey(n.id, e.messageId)));
      if (errs && errs.length > 0) m.set(n.id, errs);
    }
    return m;
  }, [nodes, liveValidation, resolvedIssues]);

  /** First-match 'left' input port wiring per node (the flexible inventory
   *  label). Stable across hover/selection so the card memo holds. */
  const connectedPortIdByNode = useMemo(() => {
    const m = new Map<string, string | undefined>();
    for (const w of wires) {
      if ((w.toPort ?? 'left') === 'left' && w.toNodeId && !m.has(w.toNodeId)) {
        m.set(w.toNodeId, w.toPortId);
      }
    }
    return m;
  }, [wires]);

  /** Forget a dismissal once its issue is genuinely gone. Gated on
   *  topologyLoaded so the preset placeholder shown during the async load
   *  can never wipe restored dismissals (see the load effect's finally). */
  useEffect(() => {
    if (!topologyLoaded) return;
    const live = new Set<string>();
    for (const [nodeId, errs] of liveValidation.byNode) {
      for (const e of errs) live.add(issueKey(nodeId, e.messageId));
    }
    for (const e of liveValidation.graphLevel) live.add(graphIssueKey(e.messageId));
    setResolvedIssues((prev) => {
      const kept = new Set<string>();
      let changed = false;
      for (const k of prev) {
        if (live.has(k)) kept.add(k);
        else changed = true;
      }
      return changed ? kept : prev;
    });
  }, [liveValidation, topologyLoaded]);

  useEffect(() => {
    try {
      localStorage.setItem(resolvedIssuesKey, JSON.stringify([...resolvedIssues]));
    } catch { /* storage unavailable — dismissal is session-only */ }
  }, [resolvedIssuesKey, resolvedIssues]);

  /** Create one wire from an ADR #34 relationship option — the single path
   *  for both unambiguous drops (auto-commit) and picker choices.
   *  Duplicate detection compares the CHOSEN toPortId (two relationships
   *  may share a socket pair, and a fully-untyped legacy wire occupies the
   *  pair regardless), and the Pro-tier fallback limit applies only to
   *  stock-routing wires — a transfer is a different relationship. */
  const commitWire = useCallback((
    source: TopologyNodeData,
    sourcePort: PortName,
    target: TopologyNodeData,
    targetPort: PortName,
    option: WireRelationshipOption,
  ) => {
    const currentWires = wiresRef.current;
    const duplicate = currentWires.some(
      (w) =>
        (w.fromNodeId === source.id && w.toNodeId === target.id
          && (w.fromPort ?? 'right') === sourcePort && (w.toPort ?? 'left') === targetPort
          && ((w.relationshipType === undefined && w.fromPortId === undefined && w.toPortId === undefined)
            || (w.toPortId ?? 'location-in') === option.toPortId))
        || (w.fromNodeId === target.id && w.toNodeId === source.id
          && (w.fromPort ?? 'right') === targetPort && (w.toPort ?? 'left') === sourcePort),
    );
    if (duplicate) {
      addToast({ message: l10n.getString('topology-toast-wire-duplicate'), type: 'warning' });
      cancelRelationshipPicker();
      return;
    }

    // The Pro-tier fallback limit covers STOCK-ROUTING wires only — a
    // transfer wire on the same pair is a different relationship and is
    // always authorable. Legacy untyped workspace→warehouse wires count
    // as stock-routing (that is what the pair defaults to).
    const existingStockWires = currentWires.filter((w) => {
      const fn = nodeMap.get(w.fromNodeId);
      const tn = nodeMap.get(w.toNodeId);
      return fn?.type === 'workspace' && tn?.type === 'warehouse'
        && (w.relationshipType === 'stock-routing' || w.relationshipType === undefined);
    });
    if (option.relationshipType === 'stock-routing' && existingStockWires.length >= 1 && !isProAllowed) {
      addToast({ message: l10n.getString('topology-toast-fallback-warehouse'), type: 'warning' });
      cancelRelationshipPicker();
      return;
    }

    pushHistoryRef.current();

    const newWireId = `wire-${crypto.randomUUID()}`;
    const isWarehouseWire = source.type === 'workspace' && target.type === 'warehouse';
    const priority = existingStockWires.length === 0 ? 1 : existingStockWires.length + 1;
    const label = isWarehouseWire
      ? option.relationshipType === 'inventory-transfer'
        ? l10n.getString('topology-wire-label-transfer')
        : existingStockWires.length === 0
          ? l10n.getString('topology-wire-label-stock-deduct', { priority })
          : l10n.getString('topology-wire-label-fallback', { priority })
      : option.relationshipType === 'ticket-routing'
        ? l10n.getString('topology-wire-label-ticket')
        : l10n.getString('topology-wire-label-connected');

    setWires((prev) => [
      ...prev,
      {
        id: newWireId,
        fromNodeId: source.id,
        fromPort: sourcePort,
        toNodeId: target.id,
        toPort: targetPort,
        direction: 'one-way',
        label,
        fromPortId: option.fromPortId,
        toPortId: option.toPortId,
        relationshipType: option.relationshipType,
      },
    ]);
    cancelRelationshipPicker();
  }, [nodeMap, addToast, l10n, isProAllowed, cancelRelationshipPicker, setWires]);

  const handlePortClick = useCallback((e: React.MouseEvent, nodeId: string, port: PortName, variantIndex = 0) => {
    e.stopPropagation();
    setRelationshipPicker(null);

    if (!connectingFromNodeId) {
      if (portDirection(port) !== 'output') {
        addToast({ message: l10n.getString('topology-port-input-only'), type: 'info' });
        return;
      }
      setConnectingFromNodeId(nodeId);
      setConnectingFromPort(port);
      setPreviewCursor(null);
      return;
    }

    if (connectingFromNodeId === nodeId) {
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    if (!isPortCompatible(nodeId, port, variantIndex)) {
      addToast({ message: l10n.getString('topology-wire-incompatible'), type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    const fromNode = nodeMap.get(connectingFromNodeId);
    const toNode = nodeMap.get(nodeId);
    if (!fromNode || !toNode) {
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    const options = wireRelationshipOptions(fromNode, connectingFromPort!, toNode, port, variantIndex);
    if (options.length === 0) {
      addToast({ message: l10n.getString('topology-wire-incompatible'), type: 'warning' });
      setConnectingFromNodeId(null);
      setConnectingFromPort(null);
      return;
    }

    // A drop that admits MULTIPLE relationships must not draw a wire
    // blindly — open the picker and let the user choose which one this
    // wire means. The in-flight connection stays visible (ghost + source
    // highlight) until the choice lands or is cancelled.
    if (options.length > 1) {
      setRelationshipPicker({
        fromNodeId: connectingFromNodeId,
        fromPort: connectingFromPort!,
        toNodeId: nodeId,
        toPort: port,
        options,
      });
      return;
    }

    commitWire(fromNode, connectingFromPort!, toNode, port, options[0]!);
  }, [connectingFromNodeId, connectingFromPort, nodeMap, isPortCompatible, commitWire, addToast, l10n, setRelationshipPicker, setConnectingFromNodeId, setConnectingFromPort, setPreviewCursor, portDirection]);

  /** Cycle a wire's visual flow: one-way → reverse → two-way → one-way.
   *  Clicking the wire itself is the affordance; the from/to ownership is
   *  untouched (only the arrow presentation changes). */
  const handleCycleWireDirection = useCallback((wireId: string) => {
    pushHistoryRef.current();
    setWires((prev) =>
      prev.map((w) => {
        if (w.id !== wireId) return w;
        const current = w.direction === 'reverse' || w.direction === 'two-way' ? w.direction : 'one-way';
        const next = WIRE_DIRECTION_CYCLE[(WIRE_DIRECTION_CYCLE.indexOf(current) + 1) % WIRE_DIRECTION_CYCLE.length]!;
        return { ...w, direction: next };
      }),
    );
  }, [setWires]);

  /** Arm a document-level drag that moves bend `index` on `wireId`.
   *  Canvas coords are derived from client coords with the same pan/zoom
   *  transform as node drags, so bends stay glued to the cursor while
   *  panning/zoomed. History is pushed once, on the first movement. */
  const startBendDrag = useCallback((e: React.MouseEvent, wireId: string, index: number, startX: number, startY: number, created = false) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    setSelectedWireId(wireId);
    clearSelection();
    bendDragCleanupRef.current?.();
    const drag = { wireId, index, moved: false, startX, startY, created };
    bendDragRef.current = drag;
    // Pre-gesture snapshot captured at mousedown: for a ghost-created bend
    // the insertion setWires above hasn't flushed yet, so the refs still
    // hold the UNBENT wires — the exact undo target (one entry, restores
    // the pre-gesture state). Immutable discipline: each setWires replaces
    // the bends array, so the history entry keeps the old array reference.
    const snapshot = { nodes: nodesRef.current, wires: wiresRef.current };
    const handleMove = (ev: MouseEvent) => {
      const rect = canvasRef.current?.getBoundingClientRect();
      const bx = (ev.clientX - (rect?.left ?? 0) - pan.x) / zoom;
      const by = (ev.clientY - (rect?.top ?? 0) - pan.y) / zoom;
      const d = bendDragRef.current;
      if (!d) return;
      if (!d.moved) {
        d.moved = true;
        pushHistoryRef.current(snapshot);
      }
      setWires((prev) =>
        prev.map((w) =>
          w.id !== d.wireId
            ? w
            : { ...w, bends: (w.bends ?? []).map((b, i) => (i === d.index ? { x: bx, y: by } : b)) },
        ),
      );
    };
    const handleUp = () => {
      document.removeEventListener('mousemove', handleMove);
      document.removeEventListener('mouseup', handleUp);
      bendDragCleanupRef.current = null;
      bendDragRef.current = null;
    };
    document.addEventListener('mousemove', handleMove);
    document.addEventListener('mouseup', handleUp);      bendDragCleanupRef.current = () => {
      document.removeEventListener('mousemove', handleMove);
      document.removeEventListener('mouseup', handleUp);
      bendDragCleanupRef.current = null;
      bendDragRef.current = null;
    };
  }, [clearSelection, pan, zoom, setSelectedWireId, canvasRef, setWires]);

  /** Drag on a midpoint ghost: insert a bend there, then drag the new
   *  bend — one gesture creates and positions it. */
  const startGhostBendDrag = useCallback((e: React.MouseEvent, wireId: string, segmentIndex: number, mx: number, my: number) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    setSelectedWireId(wireId);
    clearSelection();
    // Insert the bend at the ghost's midpoint, then drag that fresh bend.
    setWires((prev) =>
      prev.map((w) => {
        if (w.id !== wireId) return w;
        const bends = [...(w.bends ?? [])];
        bends.splice(segmentIndex, 0, { x: mx, y: my });
        return { ...w, bends };
      }),
    );
    startBendDrag(e, wireId, segmentIndex, mx, my, true);
  }, [setSelectedWireId, clearSelection, setWires, startBendDrag]);

  /** Double-click a bend handle to remove it (one undo entry). Stable so
   *  the memoized wire groups can receive it as a prop. */
  const removeBend = useCallback((wireId: string, index: number) => {
    pushHistoryRef.current();
    setWires((prev) =>
      prev.map((w) =>
        w.id !== wireId
          ? w
          : { ...w, bends: (w.bends ?? []).filter((_, i) => i !== index) },
      ),
    );
  }, [setWires]);

  /** Node card context menu (right-click): select the object and open the
   *  NODE menu (rename/duplicate/delete) instead of the canvas menu.
   *  Stable so the memoized cards can receive it as a prop. */
  const openNodeMenu = useCallback((e: React.MouseEvent, nodeId: string) => {
    e.preventDefault();
    e.stopPropagation();
    if (!e.shiftKey) selectOnly(nodeId);
    const rect = canvasRef.current?.getBoundingClientRect();
    setContextMenu({ x: e.clientX - (rect?.left ?? 0), y: e.clientY - (rect?.top ?? 0), nodeId });
  }, [selectOnly, canvasRef, setContextMenu]);

  /** Wire click: select the wire AND cycle its flow direction — the whole
   *  wire is the affordance now (no separate label pill). Stable so the
   *  memoized wire groups can receive it as a prop. */
  const handleWireClick = useCallback((e: { stopPropagation(): void }, wireId: string) => {
    e.stopPropagation();
    setSelectedWireId(wireId);
    clearSelection();
    handleCycleWireDirection(wireId);
  }, [setSelectedWireId, clearSelection, handleCycleWireDirection]);

  /** Wire context menu (right-click): object-scoped wire menu (direction +
   *  delete) instead of the canvas menu. Stable. */
  const openWireMenu = useCallback((e: React.MouseEvent, wireId: string) => {
    e.preventDefault();
    e.stopPropagation();
    const rect = canvasRef.current?.getBoundingClientRect();
    setSelectedWireId(wireId);
    clearSelection();
    setContextMenu({ x: e.clientX - (rect?.left ?? 0), y: e.clientY - (rect?.top ?? 0), wireId });
  }, [setSelectedWireId, clearSelection, canvasRef, setContextMenu]);

  /** Stable name/enabled writers for the memoized workspace cards. */
  const handleSetNodeName = useCallback((nodeId: string, name: string) => {
    beginInspectorEdit(nodeId);
    setNodes((prev) => prev.map((n) => (n.id === nodeId ? { ...n, name } : n)));
  }, [beginInspectorEdit]);

  const handleSetNodeEnabled = useCallback((nodeId: string, enabled: boolean) => {
    beginInspectorEdit(nodeId);
    setNodes((prev) => prev.map((n) => (n.id === nodeId
      ? { ...n, metadata: { ...n.metadata, enabled } }
      : n)));
  }, [beginInspectorEdit]);

  /** Stable metadata writer for the warehouse settings card (capacity,
   *  low-stock threshold). Keeps edits in the beginInspectorEdit dirty
   *  flow so canvasStateEqual can project the new keys. */
  const handleSetNodeMetadata = useCallback((nodeId: string, patch: Record<string, unknown>) => {
    beginInspectorEdit(nodeId);
    setNodes((prev) => prev.map((n) => (n.id === nodeId
      ? { ...n, metadata: { ...n.metadata, ...patch } }
      : n)));
  }, [beginInspectorEdit]);

  const handleDeleteRequest = () => {
    if (selectedNodeIds.size > 0) {
      const targets = [...selectedNodeIds];
      const hasWires = wires.some((w) => targets.includes(w.fromNodeId) || targets.includes(w.toNodeId));
      if (hasWires) {
        if (targets.length === 1) setConfirmDelete(targets[0]!);
        else setConfirmDeleteMany(targets);
      } else {
        // No connected wires — delete immediately without dialog.
        deleteNodes(targets);
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

    // Snap the preview to a hovered target port; otherwise follow the
    // live cursor (previewCursor tracks every mousemove while connecting).
    let mx: number;
    let my: number;
    if (hoveredTarget) {
      const targetNode = nodes.find((n) => n.id === hoveredTarget.nodeId);
      if (targetNode) {
        const targetOff = PORT_OFFSET[hoveredTarget.port];
        const targetDy = hoveredTarget.port === 'left' ? leftPortDy(targetNode, hoveredTarget.variantIndex) : targetOff.dy;
        mx = targetNode.x + targetOff.dx;
        my = targetNode.y + targetDy;
      } else {
        mx = previewCursor?.x ?? mousePosRef.current.x;
        my = previewCursor?.y ?? mousePosRef.current.y;
      }
    } else {
      mx = previewCursor?.x ?? mousePosRef.current.x;
      my = previewCursor?.y ?? mousePosRef.current.y;
    }

    if (wireRouting === 'elbow') {
      return { d: polylineD(elbowPoints(x1, y1, mx, my)) };
    }
    const dx = Math.abs(mx - x1) * 0.5;
    return { d: `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${mx - dx} ${my}, ${mx} ${my}` };
  }, [connectingFromNodeId, connectingFromPort, nodeMap, nodes, hoveredTarget, previewCursor, wireRouting]);

  const selectedNode = useMemo(() => nodes.find((n) => n.id === selectedNodeId), [nodes, selectedNodeId]);

  // ── Workspace card adapter (ADR #22 Phase 2) ────────────────

  /** Map a workspace node's typeKey to the correct settings card. The
   *  per-type card registry lives in topologyCard.ts — adding a workspace
   *  type with its own card is a one-line change there. */
  const renderWorkspaceCard = useCallback((node: TopologyNodeData) => {
    const typeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';
    const cardProps: WorkspaceCardProps = {
      variant: 'inspector-drawer',
      terminalId: node.id,
      ...(sessionToken ? { sessionToken } : {}),
    };
    const Card = settingsCardForTypeKey(typeKey);
    return <Card key={node.id} {...cardProps} />;
  }, [sessionToken]);

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
      // Inventory settings are not yet wired into SettingsContext, so there
      // is no real status to badge yet. Returning null keeps the header clean
      // instead of shipping a placeholder chip that reads as "unfinished".
      // When the inventory scope is added (Phase 3+), show live low-stock
      // counts from settings.inventory here.
      return null;
    }
    return node.telemetryBadge
      ? { badge: node.telemetryBadge, status: node.telemetryStatus ?? 'online' }
      : null;
  }, [settings]);

  /* eslint-disable jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions -- interactive drag/pan canvas requires these */
  // Anchor the relationship picker at the target node's left edge so the
  // user sees WHICH socket the pending choice applies to. Same screen-space
  // math as the marquee (node position × zoom + pan, in canvas-container
  // coordinates).
  const pickerAnchor = relationshipPicker ? nodeMap.get(relationshipPicker.toNodeId) : null;

  return (
    <div className="node-topology-editor">
      {/* Visually-hidden live region: announces alignment snaps and
          Alt-duplicate drops/cancels for assistive tech. The visual
          guides and the copy cursor are aria-hidden, so screen-reader
          users would otherwise get zero feedback that a snap or clone
          happened. role="status" implies aria-live="polite". */}
      <div className="sr-only" role="status" aria-live="polite" data-testid="topology-live-region">{liveAnnouncement}</div>
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

      {/* ── Confirm batch delete dialog (2+ nodes) ── */}
      {confirmDeleteMany !== null && (
        <ConfirmDialog
          open
          onCancel={() => setConfirmDeleteMany(null)}
          onConfirm={executeDelete}
          title={l10n.getString('topology-confirm-delete-many-title', { count: confirmDeleteMany.length })}
          message={l10n.getString('topology-confirm-delete-many-msg', { count: confirmDeleteMany.length })}
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
        {/* Visually-hidden heading keeps the topology screen's heading
            hierarchy (h2 → h3 Palette Tools) intact for assistive tech
            without occupying header space. */}
        <Localized id="topology-builder-title">
          <h2 className="sr-only">Visual Store & Workspace Topology Builder</h2>
        </Localized>
        {branchToolbar}
        <span className={`topology-tier-badge tier-${currentTier}`}>
          <Localized id="topology-tier-suffix" vars={{ tier: currentTier.toUpperCase() }}>
            <span>{currentTier.toUpperCase()} TIER</span>
          </Localized>
        </span>

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
            onClick={() => { if (isCanvasDirty()) setConfirmPreset('retail'); else loadPreset('retail'); }}
            icon={<CartIcon size={16} />}
          >
            <Localized id="topology-preset-retail">Retail Preset</Localized>
          </Button>

          <Button
            variant="secondary"
            onClick={() => { if (isCanvasDirty()) setConfirmPreset('restaurant'); else loadPreset('restaurant'); }}
            icon={<UtensilsIcon size={16} />}
          >
            <Localized id="topology-preset-restaurant">Resto & KDS Preset</Localized>
          </Button>

          <Button variant="secondary" onClick={autoLayout}>
            <Localized id="topology-auto-layout">Auto-layout</Localized>
          </Button>            <Button
              variant="primary"
              onClick={async () => {
                // Same gate as the live badge surface — shared helper keeps
                // the Apply toast and the on-canvas badges in lockstep.
                const validationErrors = validateEditorGraph(nodes, wires, allowLegacyApply, currentTier);
                if (validationErrors.length > 0) {
                  addToast({
                    message: l10n.getString(validationErrors[0]!.messageId),
                    type: 'error',
                  });
                  return;
                }
                skipNextLoadRef.current = true;
                // Hoisted ABOVE the try: the snapshot below is written after
                // the catch, and let/const are block-scoped to the try — a
                // declaration inside would ReferenceError on success.
                let savedNodes = nodes;
                let savedWires = wires;
                try {
                  const idMap = await onSave?.(nodes, wires);
                  if (idMap && Object.keys(idMap).length > 0) {
                    // Remap old UUIDs to new UUIDs from archive+recreate
                    // operations so the canvas stays in sync with the backend.
                    // Clear selection to avoid dangling references to old IDs,
                    // and drop the undo/redo stacks — every pre-save entry
                    // holds the OLD ids, which no longer exist on the canvas
                    // or in the DB. Undo must not restore dangling ids.
                    clearSelection();
                    setSelectedWireId(null);
                    setHistory([]);
                    setRedo([]);
                    savedNodes = nodes.map((n) => {
                      const newId = idMap[n.id];
                      return newId ? { ...n, id: newId } : n;
                    });
                    savedWires = wires.map((w) => {
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
                    });
                    // Direct array set (not the updater form): nothing can
                    // interleave during this handler's synchronous tail after
                    // the await, and the same arrays are snapshotted below.
                    setNodes(savedNodes);
                    setWires(savedWires);
                  }
                } catch (err) {
                  addToast({
                    message: `${l10n.getString('topology-toast-save-error')}: ${plainErrorMessage(err)}`,
                    type: 'error',
                  });
                  skipNextLoadRef.current = false;
                  return;
                }
                // Save succeeded — the canvas now matches the backend, so a
                // preset load must not ask about unsaved changes. (A failed
                // save returned above and stays dirty.) The snapshot is the
                // FINAL canvas — remapped ids included — so exact tracking
                // compares against what is actually on screen post-remap.
                commitSnapshot({ nodes: savedNodes, wires: savedWires });
                // Defer reset so React commits state updates + fires effects first,
                // preventing post-save reload from clobbering in-flight edits (#8).
                setTimeout(() => { skipNextLoadRef.current = false; }, 0);
              }}
              icon={<CheckIcon size={16} />}
            >
            <Localized id="topology-apply-changes">Apply Topology Changes</Localized>
          </Button>

          {isDirty && (
            <span className="topology-dirty-chip" role="status">
              <span className="topology-dirty-dot" aria-hidden="true" />
              <Localized id="topology-unsaved">Unsaved changes</Localized>
            </span>
          )}

          <button
            ref={shortcutsBtnRef}
            type="button"
            className="topology-shortcuts-btn"
            onClick={() => setShowShortcuts((p) => !p)}
            aria-label={l10n.getString('topology-shortcuts-aria')}
            aria-expanded={showShortcuts}
            aria-controls="topology-shortcuts-popover"
          >
            <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
              <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-3a1 1 0 00-.867.5 1 1 0 11-1.731-1A3 3 0 0113 8a3.001 3.001 0 01-2 2.83V11a1 1 0 11-2 0v-1a1 1 0 011-1 1 1 0 100-2zm0 8a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd" />
            </svg>
          </button>
          {showShortcuts && (
            <div
              id="topology-shortcuts-popover"
              ref={shortcutsRef}
              className="topology-shortcuts-popover"
              role="region"
              aria-label={l10n.getString('topology-shortcuts-title')}
            >
              <div className="topology-shortcuts-title">{l10n.getString('topology-shortcuts-title')}</div>
              {TOPOLOGY_SHORTCUTS.map((s) => (
                <div key={s.id} className="topology-shortcuts-row">
                  <span className="topology-shortcuts-desc">{l10n.getString(s.id)}</span>
                  <kbd className="topology-shortcuts-key">{s.key}</kbd>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="node-topology-main">
        <div className="node-tool-rack">
          <h3><Localized id="topology-palette-title">Palette Tools</Localized></h3>
          <p className="tool-rack-desc"><Localized id="topology-palette-desc">Drag or click to spawn topology nodes:</Localized></p>

          <div className="tool-rack-section">
            <h4 className="tool-rack-section-title"><Localized id="topology-rack-add-title">Add Nodes</Localized></h4>

          {allowLegacyApply && (
            <button type="button" className="tool-card" onClick={() => handleAddNode('store')}>
              <span className="tool-card-icon"><StoreIcon size={22} /></span>
              <div className="tool-card-info">
                <strong><Localized id="topology-tool-store">+ Store Node</Localized></strong>
                <span><Localized id="topology-tool-store-desc">Store Branch Profile</Localized></span>
              </div>
              <kbd className="tool-card-shortcut" aria-hidden="true">1</kbd>
            </button>
          )}

          <button type="button" className="tool-card" onClick={() => handleAddNode('workspace')}>
            <span className="tool-card-icon"><PosIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-workspace">+ Workspace Node</Localized></strong>
              <span><Localized id="topology-tool-workspace-desc">POS / Register Instance</Localized></span>
            </div>
            <kbd className="tool-card-shortcut" aria-hidden="true">2</kbd>
          </button>

          <button
            className={`tool-card ${!isProAllowed && nodes.some((n) => n.type === 'warehouse') ? 'locked' : ''}`}
            onClick={() => handleAddNode('warehouse')}
          >
            <span className="tool-card-icon"><WarehouseIcon size={22} /></span>
            <div className="tool-card-info">
              <strong><Localized id="topology-tool-warehouse">+ Stock Room</Localized></strong>
              <span><Localized id="topology-tool-warehouse-desc">Storage Location</Localized></span>
            </div>
            <kbd className="tool-card-shortcut" aria-hidden="true">3</kbd>
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
            <kbd className="tool-card-shortcut" aria-hidden="true">4</kbd>
          </button>
          </div>

          {(selectedNodeIds.size > 0 || selectedWireId || history.length > 0 || redo.length > 0) && (
            <div className="tool-rack-section">
              <h4 className="tool-rack-section-title"><Localized id="topology-rack-edit-title">Edit</Localized></h4>

          {selectedNodeIds.size > 0 || selectedWireId ? (
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
            </div>
          )}

            <div className="tool-rack-section">
              <h4 className="tool-rack-section-title"><Localized id="topology-rack-view-title">View</Localized></h4>                <button
                  type="button"
                  className={`rack-view-toggle ${wireRouting === 'elbow' ? 'is-active' : ''}`}
                  aria-pressed={wireRouting === 'elbow'}
                  onClick={() => setWireRouting((r) => (r === 'elbow' ? 'curved' : 'elbow'))}
                  title={anyBentWires ? l10n.getString('topology-bends-override-note') : undefined}
                >
                  <Localized id="topology-wire-routing-toggle">Elbow wires</Localized>
                </button>
                {anyBentWires && (
                  <span className="rack-view-note" role="status">
                    {l10n.getString('topology-bends-override-note')}
                  </span>
                )}
              <button
                type="button"
                className={`rack-view-toggle ${snapEnabled ? 'is-active' : ''}`}
                aria-pressed={snapEnabled}
                onClick={() => setSnapEnabled((s) => !s)}
              >
                <Localized id="topology-snap-toggle">Snap to grid</Localized>
              </button>
              <button
                type="button"
                className={`rack-view-toggle ${panToolActive ? 'is-active' : ''}`}
                aria-pressed={panToolActive}
                onClick={() => setPanToolActive((v) => !v)}
              >
                <Localized id="topology-pan-tool-toggle">Pan tool</Localized>
              </button>
              <button
                type="button"
                className={`rack-view-toggle ${wireLabelsVisible ? 'is-active' : ''}`}
                aria-pressed={wireLabelsVisible}
                onClick={() => setWireLabelsVisible((v) => !v)}
              >
                <Localized id="topology-wire-labels-toggle">Wire labels</Localized>
              </button>
            </div>

            <div className="tool-rack-section">
              <h4 className="tool-rack-section-title"><Localized id="topology-rack-share-title">Share</Localized></h4>
              <div className="rack-share-row">
                <button type="button" className="rack-view-toggle" onClick={handleExport}>
                  <Localized id="topology-export">Export</Localized>
                </button>
                <button type="button" className="rack-view-toggle" onClick={handleImport}>
                  <Localized id="topology-import">Import</Localized>
                </button>
              </div>
              <div className="rack-share-row">
                <button
                  type="button"
                  className="rack-view-toggle"
                  aria-expanded={templateSaveOpen}
                  onClick={() => setTemplateSaveOpen((v) => !v)}
                >
                  <Localized id="topology-save-template">Save template</Localized>
                </button>
                <button
                  type="button"
                  className="rack-view-toggle"
                  aria-expanded={templatesOpen}
                  onClick={openTemplates}
                >
                  <Localized id="topology-templates">Templates</Localized>
                </button>
              </div>
              {templateSaveOpen && (
                <div
                  className="rack-template-pop"
                  role="group"
                  aria-label={l10n.getString('topology-save-template')}
                >
                  <input
                    type="text"
                    className="rack-template-input"
                    placeholder={l10n.getString('topology-template-name-placeholder')}
                    value={templateName}
                    onChange={(e) => setTemplateName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleSaveTemplate(templateName);
                      else if (e.key === 'Escape') {
                        setTemplateSaveOpen(false);
                        setTemplateName('');
                      }
                    }}
                  />
                  <button
                    type="button"
                    className="rack-template-save"
                    onClick={() => handleSaveTemplate(templateName)}
                  >
                    <Localized id="topology-template-save">Save</Localized>
                  </button>
                </div>
              )}
              {templatesOpen && (
                <div
                  className="rack-template-list"
                  role="group"
                  aria-label={l10n.getString('topology-templates')}
                >
                  {savedTemplates.length === 0 ? (
                    <span className="rack-template-empty">
                      <Localized id="topology-no-templates">No saved templates</Localized>
                    </span>
                  ) : (
                    <ul className="rack-template-items">
                      {savedTemplates.map((name) => (
                        <li key={name} className="rack-template-item">
                          <span className="rack-template-name">{name}</span>
                          <div className="rack-template-actions">
                            <button type="button" onClick={() => handleLoadTemplate(name)}>
                              <Localized id="topology-template-load">Load</Localized>
                            </button>
                            <button type="button" onClick={() => handleDeleteTemplate(name)}>
                              <Localized id="topology-template-delete">Delete</Localized>
                            </button>
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
            </div>
        </div>

        <div
          ref={canvasRef}
          className={`node-canvas-container${spacePanArmed || panToolActive ? ' canvas-space-pan' : ''}`}
          tabIndex={0}
          role="application"
          aria-label={l10n.getString('topology-canvas-aria-label')}
          onMouseMove={handleCanvasMouseMove}
          onMouseUp={handleCanvasMouseUp}
          onMouseDown={handleCanvasMouseDown}
          onPointerDown={handleCanvasPointerDown}
          onWheel={handleWheel}
          onContextMenu={(e) => {
            e.preventDefault();
            const rect = canvasRef.current?.getBoundingClientRect();
            setContextMenu({ x: e.clientX - (rect?.left ?? 0), y: e.clientY - (rect?.top ?? 0) });
          }}
        >
          {visibleGraphLevel.length > 0 && (
            <div
              className="topology-validation-banner"
              role="alert"
              onMouseDown={(e) => e.stopPropagation()}
            >
              {visibleGraphLevel.map((err) => (
                <span key={err.messageId} className="topology-validation-banner-item">
                  {l10n.getString(err.messageId)}
                </span>
              ))}
            </div>
          )}
          {totalIssues > 0 && (
            <div className="topology-validation-widget">
              <button
                type="button"
                className="topology-issues-btn"
                aria-expanded={validationPanelOpen}
                onMouseDown={(e) => e.stopPropagation()}
                onClick={() => setValidationPanelOpen((o) => !o)}
              >
                <WarningIcon size={14} />
                <ValidationIssuesLabel count={totalIssues} />
              </button>
              {validationPanelOpen && (
                <div
                  className="topology-validation-panel"
                  role="dialog"
                  aria-label={l10n.getString('topology-validation-panel-aria')}
                  onMouseDown={(e) => e.stopPropagation()}
                >
                  {visibleNodeIssues.map((issue) => (
                    <div key={`${issue.nodeId}-${issue.messageId}`} className="topology-validation-item">
                      <button
                        type="button"
                        className="topology-validation-item-select"
                        onClick={() => {
                          setValidationPanelOpen(false);
                          selectOnly(issue.nodeId);
                        }}
                      >
                        <span className="topology-validation-item-node">{issue.nodeName}</span>
                        <span className="topology-validation-item-msg">{l10n.getString(issue.messageId)}</span>
                      </button>
                      <button
                        type="button"
                        className="topology-validation-item-dismiss"
                        aria-label={l10n.getString('topology-validation-dismiss')}
                        title={l10n.getString('topology-validation-dismiss')}
                        onClick={() => dismissIssue(issueKey(issue.nodeId, issue.messageId))}
                      >
                        <CloseIcon size={12} />
                      </button>
                    </div>
                  ))}
                  {visibleGraphLevel.map((err) => (
                    <div key={err.messageId} className="topology-validation-item topology-validation-item-static">
                      <span className="topology-validation-item-msg">{l10n.getString(err.messageId)}</span>
                      <button
                        type="button"
                        className="topology-validation-item-dismiss"
                        aria-label={l10n.getString('topology-validation-dismiss')}
                        title={l10n.getString('topology-validation-dismiss')}
                        onClick={() => dismissIssue(graphIssueKey(err.messageId))}
                      >
                        <CloseIcon size={12} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
          {marquee && (
            <div
              className="topology-marquee"
              aria-hidden="true"
              onMouseDown={(e) => e.stopPropagation()}
              style={{
                left: Math.min(marquee.x0, marquee.x1),
                top: Math.min(marquee.y0, marquee.y1),
                width: Math.abs(marquee.x1 - marquee.x0),
                height: Math.abs(marquee.y1 - marquee.y0),
              }}
            />
          )}
          {nodes.length === 0 && (
            <div className="topology-empty-state" aria-live="polite">
              <div className="topology-empty-state-card">
                <NodesIcon size={30} />
                <h3>
                  <Localized id="topology-empty-state-title">Build your store topology</Localized>
                </h3>
                <p>
                  <Localized id="topology-empty-state-body">
                    Drag tools from the palette onto the canvas, or press 1–4 to add a node. Connect nodes with the port sockets on each card.
                  </Localized>
                </p>
              </div>
            </div>
          )}
          {selectionBounds && (
            <div
              className="topology-align-toolbar"
              role="toolbar"
              aria-label={l10n.getString('topology-align-aria')}
              onMouseDown={(e) => e.stopPropagation()}
              style={{
                left: ((selectionBounds.minX + selectionBounds.maxX) / 2) * zoom + pan.x,
                top: selectionBounds.minY * zoom + pan.y,
              }}
            >
              {ALIGN_ACTIONS.map((a, i) => (
                <span key={a.mode} className="topology-align-slot">
                  {i === 6 && <span className="topology-align-divider" aria-hidden="true" />}
                  <button
                    type="button"
                    className="topology-align-btn"
                    aria-label={l10n.getString(a.ariaId)}
                    title={l10n.getString(a.ariaId)}
                    onClick={() => applyAlign(a.mode)}
                  >
                    <AlignGlyph mode={a.mode} />
                  </button>
                </span>
              ))}
            </div>
          )}
          {contextMenu && (
            <div
              className="topology-context-menu"
              role="menu"
              aria-label={l10n.getString('topology-context-add-title')}
              tabIndex={-1}
              onMouseDown={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
                e.preventDefault();
                const items = Array.from(
                  e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
                );
                if (items.length === 0) return;
                const idx = items.indexOf(document.activeElement as HTMLButtonElement);
                const next = e.key === 'ArrowDown'
                  ? (idx + 1) % items.length
                  : (idx - 1 + items.length) % items.length;
                items[next]!.focus();
              }}
              style={{ left: contextMenu.x, top: contextMenu.y }}
            >
              {(() => {
                const menuNode = contextMenu.nodeId ? nodeMap.get(contextMenu.nodeId) : undefined;
                const menuWire = contextMenu.wireId ? wires.find((w) => w.id === contextMenu.wireId) : undefined;
                if (menuWire) {
                  // Wire menu: object-scoped actions (direction + rename + delete).
                  return (
                    <>
                      <div className="topology-context-section-title">{wireDisplayLabel(menuWire)}</div>
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); handleCycleWireDirection(menuWire.id); }}
                      >
                        {l10n.getString('topology-wire-toggle-aria')}
                      </button>
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); startWireRename(menuWire.id); }}
                      >
                        {l10n.getString('topology-context-rename-wire')}
                      </button>
                      <div className="topology-context-divider" />
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); setConfirmDelete(''); }}
                      >
                        {l10n.getString('topology-context-delete-wire')}
                      </button>
                    </>
                  );
                }
                if (menuNode) {
                  // Node menu: object-scoped actions (rename/duplicate/delete).
                  const menuRenameable = (menuNode.type === 'store' && !!onRenameBranch)
                    || (menuNode.type === 'workspace' && !!onRenameWorkspace);
                  return (
                    <>
                      <div className="topology-context-section-title">{menuNode.name}</div>
                      {menuRenameable && (
                        <button
                          type="button"
                          role="menuitem"
                          className="topology-context-item"
                          onClick={() => { setContextMenu(null); startNodeRename(menuNode.id, menuNode.name); }}
                        >
                          {l10n.getString('topology-context-rename')}
                        </button>
                      )}
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); duplicateSelection(); }}
                      >
                        {l10n.getString('topology-context-duplicate')}
                      </button>
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); handleDeleteRequest(); }}
                      >
                        {l10n.getString('topology-confirm-delete-node-title')}
                      </button>
                      <div className="topology-context-divider" />
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); zoomToSelection(); }}
                      >
                        {l10n.getString('topology-context-zoom-selection')}
                      </button>
                    </>
                  );
                }
                // Canvas menu: an active (marquee) selection gets a summary
                // + clear action up top; add node types + view actions follow.
                return (
                  <>
                    {selectedNodeIds.size > 0 && (
                      <>
                        <div className="topology-context-section-title">
                          {l10n.getString('topology-context-selection-title', { count: selectedNodeIds.size })}
                        </div>
                        <button
                          type="button"
                          role="menuitem"
                          className="topology-context-item"
                          onClick={() => { setContextMenu(null); clearSelection(); }}
                        >
                          {l10n.getString('topology-context-clear-selection')}
                        </button>
                        <div className="topology-context-divider" />
                      </>
                    )}
                    <div className="topology-context-section-title">
                      {l10n.getString('topology-context-add-title')}
                    </div>
                    {CONTEXT_ADD_TYPES.filter((t) => allowLegacyApply || t !== 'store').map((type) => {
                      const Icon = NODE_TYPE_ICON[type];
                      return (
                        <button
                          key={type}
                          type="button"
                          role="menuitem"
                          className="topology-context-item"
                          onClick={() => {
                            setContextMenu(null);
                            handleAddNode(type, {
                              x: (contextMenu.x - pan.x) / zoom,
                              y: (contextMenu.y - pan.y) / zoom,
                            });
                          }}
                        >
                          <span className="topology-context-item-icon"><Icon size={14} /></span>
                          {l10n.getString(`topology-new-${type}`)}
                        </button>
                      );
                    })}
                    <div className="topology-context-divider" />
                    <button
                      type="button"
                      role="menuitem"
                      className="topology-context-item"
                      onClick={() => { setContextMenu(null); selectAllNodes(); }}
                    >
                      {l10n.getString('topology-context-select-all')}
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      className="topology-context-item"
                      onClick={() => { setContextMenu(null); zoomToFit(); }}
                    >
                      {l10n.getString('topology-fit-all')}
                    </button>
                    {selectedNodeIds.size > 0 && (
                      <button
                        type="button"
                        role="menuitem"
                        className="topology-context-item"
                        onClick={() => { setContextMenu(null); zoomToSelection(); }}
                      >
                        {l10n.getString('topology-context-zoom-selection')}
                      </button>
                    )}
                    <button
                      type="button"
                      role="menuitem"
                      className="topology-context-item"
                      onClick={() => { setContextMenu(null); resetView(); }}
                    >
                      {l10n.getString('topology-reset-view')}
                    </button>
                  </>
                );
              })()}
            </div>
          )}
          {relationshipPicker && pickerAnchor && (
            <div
              ref={relationshipPickerRef}
              className="topology-relationship-picker"
              role="dialog"
              aria-label={l10n.getString('topology-relationship-picker-title')}
              onMouseDown={(e) => e.stopPropagation()}
              style={{
                left: pickerAnchor.x * zoom + pan.x - 12,
                top: pickerAnchor.y * zoom + pan.y + NODE_HEIGHT / 2,
              }}
            >
              <div className="topology-relationship-picker-title">
                {l10n.getString('topology-relationship-picker-title')}
              </div>
              {relationshipPicker.options.map((option) => (
                <button
                  key={`${option.fromPortId}|${option.toPortId}`}
                  type="button"
                  className="topology-relationship-option"
                  onClick={() => {
                    const from = nodeMap.get(relationshipPicker.fromNodeId);
                    const to = nodeMap.get(relationshipPicker.toNodeId);
                    if (!from || !to) {
                      cancelRelationshipPicker();
                      return;
                    }
                    commitWire(from, relationshipPicker.fromPort, to, relationshipPicker.toPort, option);
                  }}
                >
                  {l10n.getString(option.labelId)}
                </button>
              ))}
              <button
                type="button"
                className="topology-relationship-cancel"
                onClick={cancelRelationshipPicker}
              >
                {l10n.getString('topology-relationship-picker-cancel')}
              </button>
            </div>
          )}
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
                const geo = wireGeometries.get(wire.id);
                if (!geo) return null;
                // Pulse rides the wire's actual geometry: the cubic bezier
                // by default, or the elbow polyline when orthogonal routing
                // is on. Computed here (not inside the memoized wire group)
                // so `pulse` is a plain {x, y} prop — null while the
                // simulation is idle keeps the memo boundary effective.
                const t = simPulseStep / 100;
                const pulsePoint = geo.polyline
                  ? polylinePoint(geo.polyline, t)
                  : { x: cubicBezier(t, geo.x1, geo.x1 + geo.dx, geo.x2 - geo.dx, geo.x2), y: cubicBezier(t, geo.y1, geo.y1, geo.y2, geo.y2) };
                return (
                  <TopologyWireGroup
                    key={wire.id}
                    wire={wire}
                    x1={geo.x1}
                    y1={geo.y1}
                    x2={geo.x2}
                    y2={geo.y2}
                    dx={geo.dx}
                    pathD={geo.pathD}
                    polyline={geo.polyline}
                    selected={selectedWireId === wire.id}
                    dimmed={hoverConnections !== null
                      && wire.fromNodeId !== hoveredNodeId
                      && wire.toNodeId !== hoveredNodeId}
                    hovered={hoveredWireId === wire.id}
                    pulse={isSimulating ? pulsePoint : null}
                    l10n={l10n}
                    onHoverWire={setHoveredWireId}
                    onWireClick={handleWireClick}
                    onOpenWireMenu={openWireMenu}
                    onStartGhostBend={startGhostBendDrag}
                    onStartBendDrag={startBendDrag}
                    onRemoveBend={removeBend}
                  />
                );
              })}

              {wirePreviewLine && (
                <path d={wirePreviewLine.d} className="wire-path" opacity="0.5" pointerEvents="none" />
              )}
            </svg>

            {alignmentGuide?.x !== undefined && (
              <div className="alignment-guide alignment-guide-x" style={{ left: alignmentGuide.x }} aria-hidden="true" />
            )}
            {alignmentGuide?.y !== undefined && (
              <div className="alignment-guide alignment-guide-y" style={{ top: alignmentGuide.y }} aria-hidden="true" />
            )}

            {(() => {
              // Inline wire relabel: a floating input at the wire's midpoint
              // (where a label pill would sit), seeded with the current label.
              if (!renamingWireId) return null;
              const geo = wireGeometries.get(renamingWireId);
              if (!geo) return null;
              const mid = geo.polyline
                ? polylinePoint(geo.polyline, 0.5)
                : {
                    x: cubicBezier(0.5, geo.x1, geo.x1 + geo.dx, geo.x2 - geo.dx, geo.x2),
                    y: cubicBezier(0.5, geo.y1, geo.y1, geo.y2, geo.y2),
                  };
              return (
                <input
                  ref={wireRenameInputRef}
                  className="wire-rename-input"
                  value={wireRenameDraft}
                  onChange={(e) => setWireRenameDraft(e.target.value)}
                  onMouseDown={(e) => e.stopPropagation()}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') { e.preventDefault(); commitWireRename(renamingWireId, true); }
                    if (e.key === 'Escape') { e.preventDefault(); cancelWireRename(); }
                  }}
                  onBlur={() => void commitWireRename(renamingWireId)}
                  aria-label={l10n.getString('topology-wire-rename-placeholder')}
                  style={{ left: mid.x, top: mid.y }}
                />
              );
            })()}

            {wireLabelsVisible && wires.map((wire) => {
              // Permanent label pill at the wire's midpoint (the same point
              // the rename input anchors to). Clicking opens the rename
              // editor — the wire itself stays the direction-cycle
              // affordance, so the pill must not cycle. Hidden while the
              // wire's own rename input is open (it replaces the pill).
              if (renamingWireId === wire.id) return null;
              const geo = wireGeometries.get(wire.id);
              if (!geo) return null;
              const mid = geo.polyline
                ? polylinePoint(geo.polyline, 0.5)
                : {
                    x: cubicBezier(0.5, geo.x1, geo.x1 + geo.dx, geo.x2 - geo.dx, geo.x2),
                    y: cubicBezier(0.5, geo.y1, geo.y1, geo.y2, geo.y2),
                  };
              const isDimmed = hoverConnections !== null
                && wire.fromNodeId !== hoveredNodeId
                && wire.toNodeId !== hoveredNodeId;
              return (
                <button
                  key={wire.id}
                  type="button"
                  className={`wire-label-pill${isDimmed ? ' wire-label-pill-dimmed' : ''}`}
                  style={{ left: mid.x, top: mid.y }}
                  title={l10n.getString('topology-context-rename-wire')}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedWireId(wire.id);
                    clearSelection();
                    startWireRename(wire.id);
                  }}
                >
                  {wireDisplayLabel(wire)}
                </button>
              );
            })}

                        {nodes.map((node) => (
              <TopologyNodeCard
                key={node.id}
                node={node}
                isSelected={selectedNodeIds.has(node.id)}
                isConnectingSource={connectingFromNodeId === node.id}
                connectingFromNodeId={connectingFromNodeId}
                connectingFromPort={connectingFromPort}
                hoveredTarget={hoveredTarget}
                nodeErrors={nodeErrorsByNode.get(node.id) ?? EMPTY_ERRORS}
                isFresh={freshNodeIds.has(node.id)}
                isDimmed={hoverConnections !== null && !hoverConnections.has(node.id)}
                isRenameable={(node.type === 'store' && !!onRenameBranch) || (node.type === 'workspace' && !!onRenameWorkspace)}
                renaming={renamingNodeId === node.id}
                renameDraft={renameDraft}
                connectedPortId={connectedPortIdByNode.get(node.id)}
                l10n={l10n}
                renameInputRef={renameInputRef}
                renameBaselineRef={renameBaselineRef}
                onSelect={selectOnly}
                onOpenNodeMenu={openNodeMenu}
                onCardMouseDown={handleNodeMouseDown}
                onStartRename={startNodeRename}
                onCommitRename={commitNodeRename}
                onCancelRename={cancelNodeRename}
                onRenameDraftChange={setRenameDraft}
                onPersistRename={persistNodeRename}
                onSetNodeName={handleSetNodeName}
                onSetNodeEnabled={handleSetNodeEnabled}
                onPortClick={handlePortClick}
                onHoverNode={setHoveredNodeId}
                getTelemetry={getTelemetry}
                isPortCompatible={isPortCompatible}
              />
            ))}
          </div>

          {/* ── Canvas HUD — status readouts only; the zoom readout
                 lives in the floating zoom cluster ───────────────── */}
          <div className="canvas-hud" aria-hidden="true">
            <span className="canvas-hud-item">{l10n.getString('topology-hud-nodes', { count: nodes.length })}</span>
            <span className="canvas-hud-divider" />
            <span className="canvas-hud-item">{l10n.getString('topology-hud-wires', { count: wires.length })}</span>
            <span className="canvas-hud-divider" />
            <CanvasCursorReadout pan={pan} zoom={zoom} />
            <span className="canvas-hud-divider" />
            <span className="canvas-hud-item">{l10n.getString('topology-status-selection', { count: selectedNodeIds.size })}</span>
          </div>

          {/* ── Node finder (Ctrl+F) — quick jump overlay ─────── */}
          {finderOpen && (() => {
            const activeIndex = Math.min(finderIndex, Math.max(0, finderMatches.length - 1));
            return (
              <div
                className="topology-finder"
                role="dialog"
                aria-label={l10n.getString('topology-finder-aria')}
                onMouseDown={(e) => e.stopPropagation()}
              >
                <input
                  ref={finderInputRef}
                  className="topology-finder-input"
                  type="text"
                  value={finderQuery}
                  placeholder={l10n.getString('topology-finder-placeholder')}
                  aria-label={l10n.getString('topology-finder-aria')}
                  onChange={(e) => {
                    setFinderQuery(e.target.value);
                    setFinderIndex(0);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Escape') {
                      e.preventDefault();
                      e.stopPropagation();
                      setFinderOpen(false);
                    } else if (e.key === 'ArrowDown') {
                      e.preventDefault();
                      setFinderIndex((i) => (finderMatches.length === 0 ? 0 : (i + 1) % finderMatches.length));
                    } else if (e.key === 'ArrowUp') {
                      e.preventDefault();
                      setFinderIndex((i) => (finderMatches.length === 0 ? 0 : (i - 1 + finderMatches.length) % finderMatches.length));
                    } else if (e.key === 'Enter') {
                      e.preventDefault();
                      const match = finderMatches[Math.min(finderIndex, Math.max(0, finderMatches.length - 1))];
                      if (match) jumpToFinderMatch(match);
                    }
                  }}
                />
                <ul className="topology-finder-list" role="listbox">
                  {finderMatches.length === 0 ? (
                    <li className="topology-finder-empty" role="option" aria-selected="false">
                      {l10n.getString('topology-finder-no-matches')}
                    </li>
                  ) : finderMatches.map((n, i) => (
                    <li
                      key={n.id}
                      role="option"
                      aria-selected={i === activeIndex}
                      className={`topology-finder-item ${i === activeIndex ? 'is-active' : ''}`}
                      onMouseDown={(e) => {
                        e.stopPropagation();
                        jumpToFinderMatch(n);
                      }}
                    >
                      <span className="topology-finder-item-name">{n.name}</span>
                      <span className="topology-finder-item-sub">{n.subtitle}</span>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })()}

          {/* ── Canvas zoom controls — the floating bottom-right
                 cluster (standard canvas-tool pattern) ────────── */}
          <div
            className="canvas-zoom-controls"
            role="toolbar"
            aria-label={l10n.getString('topology-canvas-aria-label')}
            onMouseDown={(e) => e.stopPropagation()}
          >
            <button
              type="button"
              className="canvas-zoom-btn"
              aria-label={l10n.getString('topology-zoom-out')}
              onClick={() => zoomBy(1 / 1.25)}
            >
              <MinusIcon size={14} />
            </button>
            <div className="canvas-zoom-picker">
              <button
                type="button"
                className="canvas-zoom-level"
                aria-label={l10n.getString('topology-zoom-level-aria', { count: Math.round(zoom * 100) })}
                aria-expanded={zoomPickerOpen}
                onMouseDown={(e) => e.stopPropagation()}
                onClick={() => setZoomPickerOpen((v) => !v)}
              >
                {Math.round(zoom * 100)}%
              </button>
              {zoomPickerOpen && (
                <div
                  className="canvas-zoom-slider-pop"
                  role="group"
                  aria-label={l10n.getString('topology-zoom-slider-aria')}
                >
                  <input
                    type="range"
                    min={40}
                    max={200}
                    step={5}
                    value={Math.round(zoom * 100)}
                    onChange={(e) => setZoom(Number(e.target.value) / 100)}
                    onMouseDown={(e) => e.stopPropagation()}
                    aria-label={l10n.getString('topology-zoom-slider-aria')}
                  />
                  <span className="canvas-zoom-slider-value" aria-hidden="true">{Math.round(zoom * 100)}%</span>
                </div>
              )}
            </div>
            <button
              type="button"
              className="canvas-zoom-btn"
              aria-label={l10n.getString('topology-zoom-in')}
              onClick={() => zoomBy(1.25)}
            >
              <PlusIcon size={14} />
            </button>
            <span className="canvas-zoom-divider" aria-hidden="true" />
            <button type="button" className="canvas-zoom-btn canvas-zoom-action" onClick={zoomToFit}>
              <Localized id="topology-fit-all">Fit All</Localized>
            </button>
            <button type="button" className="canvas-zoom-btn canvas-zoom-action" onClick={resetView}>
              <Localized id="topology-reset-view">Reset View</Localized>
            </button>
            <button
              type="button"
              className="canvas-zoom-btn canvas-zoom-action"
              aria-pressed={minimapVisible}
              onClick={() => setMinimapVisible((v) => !v)}
            >
              <Localized id={minimapVisible ? 'topology-minimap-hide' : 'topology-minimap-show'}>
                {minimapVisible ? 'Hide Minimap' : 'Show Minimap'}
              </Localized>
            </button>
          </div>

          {/* ── Canvas minimap — bottom-left overview; click/drag to
                 recenter, arrows nudge the view, Enter centers on the
                 content box ────────────────────────────────────── */}
          {contentBounds && minimapVisible && (
            <div
              ref={minimapRef}
              className="topology-minimap"
              role="button"
              tabIndex={0}
              aria-label={l10n.getString('topology-minimap-aria')}
              onMouseDown={handleMinimapMouseDown}
              onKeyDown={handleMinimapKeyDown}
            >
              <svg width={MINIMAP_W} height={MINIMAP_H} aria-hidden="true">
                {wires.map((w) => {
                  const from = nodeMap.get(w.fromNodeId);
                  const to = nodeMap.get(w.toNodeId);
                  if (!from || !to) return null;
                  return (
                    <line
                      key={w.id}
                      className="topology-minimap-wire"
                      x1={MINIMAP_PAD + (from.x + NODE_WIDTH / 2 - contentBounds.minX) * minimapScale}
                      y1={MINIMAP_PAD + (from.y + NODE_HEIGHT / 2 - contentBounds.minY) * minimapScale}
                      x2={MINIMAP_PAD + (to.x + NODE_WIDTH / 2 - contentBounds.minX) * minimapScale}
                      y2={MINIMAP_PAD + (to.y + NODE_HEIGHT / 2 - contentBounds.minY) * minimapScale}
                    />
                  );
                })}
                {nodes.map((n) => (
                  <rect
                    key={n.id}
                    className={`topology-minimap-node node-type-${n.type}`}
                    x={MINIMAP_PAD + (n.x - contentBounds.minX) * minimapScale}
                    y={MINIMAP_PAD + (n.y - contentBounds.minY) * minimapScale}
                    width={Math.max(2, NODE_WIDTH * minimapScale)}
                    height={Math.max(2, NODE_HEIGHT * minimapScale)}
                    rx={2}
                  />
                ))}
                <rect
                  className="topology-minimap-viewport"
                  x={MINIMAP_PAD + (pan.x - contentBounds.minX) * minimapScale}
                  y={MINIMAP_PAD + (pan.y - contentBounds.minY) * minimapScale}
                  width={Math.max(MINIMAP_VIEWPORT_MIN, ((canvasRef.current?.clientWidth ?? 0) / zoom) * minimapScale)}
                  height={Math.max(MINIMAP_VIEWPORT_MIN, ((canvasRef.current?.clientHeight ?? 0) / zoom) * minimapScale)}
                />
              </svg>
            </div>
          )}
        </div>

        {selectedNode && (
          <div className="node-inspector-drawer">
            <div className="inspector-header">
              <h3><Localized id="topology-inspector-title">Node Inspector</Localized></h3>
              <Button variant="secondary" onClick={clearSelection} icon={<CloseIcon size={14} />} aria-label={l10n.getString('topology-inspector-close-aria')}>{null}</Button>
            </div>

            <div className="inspector-content">
              <ErrorBoundary>
              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
              <label className="inspector-field">
                <span><Localized id="topology-inspector-node-name">Node Name</Localized></span>
                <input
                  type="text"
                  value={selectedNode.name}
                  onChange={(e) => {
                    beginInspectorEdit(selectedNode.id);
                    const name = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, name } : n)));
                  }}
                  onFocus={() => { renameBaselineRef.current = selectedNode.name; }}
                  onBlur={() => void persistNodeRename(selectedNode.id, selectedNode.name)}
                  onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); void persistNodeRename(selectedNode.id, selectedNode.name); } }}
                />
              </label>

              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
              <label className="inspector-field">
                <span><Localized id="topology-inspector-subtitle">Subtitle / Location</Localized></span>
                <input
                  type="text"
                  value={selectedNode.subtitle || ''}
                  onChange={(e) => {
                    beginInspectorEdit(selectedNode.id);
                    const subtitle = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, subtitle } : n)));
                  }}
                />
              </label>

              {/* Workspace type selector + settings card */}
              {selectedNode.type === 'workspace' && (                  <div className="inspector-section">
                  <h4>
                    <Localized id="workspace-type-selector-label">Workspace Type</Localized>
                  </h4>
                  <div className="topology-workspace-identity" data-testid="workspace-identity-fields">
                    <span className="topology-identity-row">
                      <Localized id="topology-workspace-purpose-label">Purpose</Localized>:
                      <strong>{l10n.getString(`topology-purpose-${(selectedNode.metadata?.['purposeKey'] as string) ?? 'general'}`)}</strong>
                    </span>
                    <span className="topology-identity-row topology-identity-technical">
                      <Localized id="topology-workspace-technical-type-label">Technical type</Localized>:
                      <code>{(selectedNode.metadata?.['typeKey'] as string) ?? 'store-pos'}</code>
                    </span>
                  </div>
                  <label className="inspector-field">
                    <span><Localized id="topology-workspace-purpose-selector-label">Workspace purpose</Localized></span>
                    <select
                      className="topology-purpose-select"
                      value={(selectedNode.metadata?.['purposeKey'] as string) ?? 'general'}
                      onChange={(e) => {
                        beginInspectorEdit(selectedNode.id);
                        const purposeKey = e.target.value;
                        setNodes((prev) => prev.map((n) => n.id === selectedNode.id
                          ? { ...n, metadata: { ...n.metadata, purposeKey } }
                          : n));
                      }}
                      aria-label={l10n.getString('topology-workspace-purpose-selector-aria')}
                    >
                      <option value="general">{l10n.getString('topology-purpose-general')}</option>
                      <option value="checkout">{l10n.getString('topology-purpose-checkout')}</option>
                      <option value="returns">{l10n.getString('topology-purpose-returns')}</option>
                      <option value="dining-room">{l10n.getString('topology-purpose-dining-room')}</option>
                      <option value="kitchen-hot-line">{l10n.getString('topology-purpose-kitchen-hot-line')}</option>
                      <option value="stock-control">{l10n.getString('topology-purpose-stock-control')}</option>
                      <option value="receiving">{l10n.getString('topology-purpose-receiving')}</option>
                    </select>
                  </label>
                  <select
                    className="inspector-select"
                    value={(selectedNode.metadata?.['typeKey'] as string) ?? 'store-pos'}
                    onChange={(e) => {
                      beginInspectorEdit(selectedNode.id);
                      const newTypeKey = e.target.value;
                      setNodes((prev) =>
                        prev.map((n) =>
                          n.id === selectedNode.id
                            ? { ...n, metadata: { ...n.metadata, typeKey: newTypeKey } }
                            : n,
                        ),
                      );
                    }}
                    aria-label={l10n.getString('topology-ws-type-select-aria')}
                  >
                    {WORKSPACE_TYPE_KEYS.filter((k) => k !== 'warehouse').map((k) => (
                      <option key={k} value={k}>
                        {workspaceTypeLabel(k, (id, vars) => topologyUiString(l10n, id, vars ?? null))}
                      </option>
                    ))}
                  </select>
                  {renderWorkspaceCard(selectedNode)}
                </div>
              )}
              {selectedNode.type === 'warehouse' && (
                <WarehouseSettingsCard node={selectedNode} onChange={handleSetNodeMetadata} />
              )}
              {selectedNode.type === 'store' && (
                <StoreInfoCard variant="inspector-drawer" />
              )}
              {selectedNode.type === 'hardware' && (
                <div className="inspector-section" data-testid="hardware-inspector">
                  <h4>
                    <Localized id="topology-inspector-hardware-title">Hardware Device</Localized>
                  </h4>
                  {selectedNode.telemetryBadge && (
                    <span className={`node-telemetry-badge telemetry-${selectedNode.telemetryStatus ?? 'online'}`}>
                      {selectedNode.telemetryBadge}
                    </span>
                  )}
                </div>
              )}
              </ErrorBoundary>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
/* eslint-enable jsx-a11y/no-noninteractive-tabindex, jsx-a11y/no-noninteractive-element-interactions */
