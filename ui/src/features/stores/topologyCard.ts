import type { ComponentType } from 'react';
import type { ReactLocalization } from '@fluent/react';
import type { FluentVariable } from '@fluent/bundle';
import type { PortName, TopologyNodeData } from './NodeTopologyEditor';
import {
  WorkspaceStorePosSettings,
  WorkspaceRestaurantPosSettings,
  WorkspaceKdsSettings,
  WorkspaceInventorySettings,
} from '@/features/settings/workspace-cards';
import {
  StoreIcon,
  PosIcon,
  WarehouseIcon,
  PrinterIcon,
} from './NodeTopologyIcons';

// ── Registry contract ─────────────────────────────────────────────
//
// Every node type behaves differently on canvas: which ports it exposes,
// what those ports are called, which settings card its inspector opens,
// and which icon heads the card. This module is the single home for that
// per-type knowledge. Adding a new node type (a new workspace typeKey or a
// whole new NodeType) should be a one-entry change here — not a touch in a
// dozen switch statements across the editor.

/** Semantic wire-port identifiers. These are the contract surface the
 *  backend round-trips (from_port_id / to_port_id); presentation (socket
 *  placement, arrow direction) is decoupled from them. */
export type SemanticPortId =
  | 'location-out'
  | 'location-in'
  | 'operation-in'
  | 'stock-out'
  | 'stock-in'
  | 'ticket-out'
  | 'ticket-in'
  | 'device-out'
  | 'generic-in'
  | 'generic-out';

/** True for Kitchen Display workspace instances (metadata typeKey 'kds'). */
export function isKdsNode(node: TopologyNodeData): boolean {
  return node.type === 'workspace' && node.metadata?.['typeKey'] === 'kds';
}

/** True for Inventory Management workspace instances (metadata typeKey 'inventory'). */
export function isInventoryNode(node: TopologyNodeData): boolean {
  return node.type === 'workspace' && node.metadata?.['typeKey'] === 'inventory';
}

/** The workspace settings card the inspector renders for a typeKey. The
 *  default (store-pos) is the baseline card; per-type cards are the
 *  exception list, so adding a workspace type with its own card is a
 *  one-line change here. */
export const WORKSPACE_SETTINGS_CARD: Readonly<Record<string, ComponentType<Record<string, unknown>>>> = {
  'store-pos': WorkspaceStorePosSettings,
  'restaurant-pos': WorkspaceRestaurantPosSettings,
  'kds': WorkspaceKdsSettings,
  'inventory': WorkspaceInventorySettings,
};

/** The settings card for a workspace node, keyed by its typeKey. */
export function settingsCardForTypeKey(
  typeKey: string,
): ComponentType<Record<string, unknown>> {
  return WORKSPACE_SETTINGS_CARD[typeKey] ?? WORKSPACE_SETTINGS_CARD['store-pos']!;
}

/** Type label resolver for the workspace type selector. Pure — the caller
 *  supplies the l10n string resolver so this module stays i18n-framework
 *  agnostic and trivially testable. */
export function workspaceTypeLabel(
  key: string,
  getString: (id: string, vars?: Record<string, string> | null, fallback?: string) => string,
): string {
  const map: Record<string, string> = {
    'store-pos': 'topology-ws-type-store-pos',
    'restaurant-pos': 'topology-ws-type-restaurant-pos',
    'kds': 'topology-ws-type-kds',
    'warehouse': 'topology-ws-type-warehouse',
  };
  const id = map[key];
  return id ? getString(id, null, id) : key;
}

// ── Port model ────────────────────────────────────────────────────

/** Semantic variant for the left input connector of a node. Every node
 *  exposes exactly ONE left input slot; inventory's slot is flexible and
 *  takes its label from the wire actually attached to it (Location or
 *  Operation). Returns plain strings because callers index it against
 *  wire.toPortId (a free-form string). */
export function leftPortVariants(node: TopologyNodeData): string[] {
  if (isKdsNode(node)) return ['operation-in'];
  if (node.type === 'store') return [];
  return ['location-in'];
}

/** Ports exposed by the frontend-only UX. Top/bottom remain load-compatible. */
export function visiblePortsForNode(node: TopologyNodeData): PortName[] {
  // A Kitchen Display is a sink: it consumes a single Operation feed from
  // the left and forwards nothing — it has no output port of its own.
  if (isKdsNode(node)) return ['left'];
  switch (node.type) {
    case 'store':
      return ['right'];
    case 'workspace':
    case 'warehouse':
    case 'hardware':
      return ['left', 'right'];
    default:
      return ['left', 'right'];
  }
}

/** Fluent id for the label of a node's left input. `connectedPortId` is the
 *  wire's recorded toPortId — inventory's flexible input shows Location or
 *  Operation based on what is actually attached, and a neutral "Input" while
 *  unwired. All other nodes keep their fixed label. */
export function leftPortLabelId(node: TopologyNodeData, variantIndex: number, connectedPortId?: string): string {
  const variant = leftPortVariants(node)[variantIndex];
  if (isInventoryNode(node)) {
    if (connectedPortId === 'operation-in') return 'topology-port-operation-in';
    if (connectedPortId === 'location-in') return 'topology-port-location-in';
    return 'topology-port-generic-in';
  }
  if (variant === 'operation-in') return 'topology-port-operation-in';
  if (variant === 'location-in') return 'topology-port-location-in';
  return variant === 'stock-in' ? 'topology-port-stock-in' : 'topology-port-generic-in';
}

export function portLabelId(node: TopologyNodeData, port: PortName): string {
  if (port === 'left') return leftPortLabelId(node, 0);
  if (node.type === 'store' && port === 'right') return 'topology-port-location-out';
  if (node.type === 'workspace' && port === 'right') return 'topology-port-workspace-out';
  if (node.type === 'warehouse' && port === 'right') return 'topology-port-stock-out';
  if (node.type === 'hardware' && port === 'right') return 'topology-port-device-out';
  return 'topology-port-generic-out';
}

export function portAriaLabelId(node: TopologyNodeData, port: PortName, variantIndex = 0): string {
  if (port === 'left') {
    const variant = leftPortVariants(node)[variantIndex];
    if (variant === 'operation-in') return 'topology-port-operation-in-aria';
    if (variant === 'location-in') return 'topology-port-location-in-aria';
  }
  if (node.type === 'store' && port === 'right') return 'topology-port-location-out-aria';
  return 'topology-port-aria';
}

/** Canonical semantic port id for a socket — the only bridge from a
 *  rendered socket to the semantic wire contract. Returns undefined for
 *  ports that carry no semantic (presentation-only sockets). */
export function semanticPortId(node: TopologyNodeData, port: PortName, variantIndex = 0): SemanticPortId | undefined {
  if (node.type === 'store' && port === 'right') return 'location-out';
  if (node.type === 'workspace' && port === 'left') {
    // Inventory's single input accepts either Location or Operation; the
    // wire records which semantic it carries via toPortId. Everything
    // else keeps its fixed left-in semantic.
    if (isInventoryNode(node)) return variantIndex === 1 ? 'operation-in' : 'location-in';
    if (isKdsNode(node)) return 'operation-in';
    return 'location-in';
  }
  return undefined;
}

// ── Header chrome ─────────────────────────────────────────────────

/** Header icon component for each node type. */
export const NODE_TYPE_ICON: Readonly<Record<TopologyNodeData['type'], ComponentType<{ size?: number }>>> = {
  store: StoreIcon,
  workspace: PosIcon,
  warehouse: WarehouseIcon,
  hardware: PrinterIcon,
};

// ── UI string fallbacks ───────────────────────────────────────────

/** Safe fallbacks for topology chrome so a stale or partial locale bundle
 *  never exposes a Fluent message id in the node canvas. */
export const TOPOLOGY_UI_FALLBACKS: Readonly<Record<string, string>> = {
  'topology-port-location-out': 'Location',
  'topology-port-location-in': 'Location',
  'topology-port-location-out-aria': 'Location port',
  'topology-port-location-in-aria': 'Location port',
  'topology-port-workspace-out': 'Operation',
  'topology-port-operation-in': 'Operation',
  'topology-port-operation-in-aria': 'Operation port',
  'topology-port-stock-in': 'Stock In',
  'topology-port-stock-out': 'Stock Out',
  'topology-port-ticket-in': 'Ticket In',
  'topology-port-device-out': 'Device Out',
  'topology-port-generic-in': 'Input',
  'topology-port-generic-out': 'Output',
  'topology-port-aria': 'Topology port',
  'topology-field-name': 'Name',
  'topology-field-name-aria': 'Edit name',
  'topology-field-enabled': 'Enabled',
  'topology-field-enabled-aria': 'Toggle enabled state',
  'topology-ws-type-store-pos': 'Store POS',
  'topology-ws-type-restaurant-pos': 'Restaurant POS',
  'topology-ws-type-kds': 'Kitchen Display (KDS)',
  'topology-ws-type-warehouse': 'Warehouse',
};

/** Resolve topology chrome with a safe fallback so a stale or partial
 *  locale bundle never exposes a Fluent message id in the node canvas. */
export function topologyUiString(
  l10n: Pick<ReactLocalization, 'getString'>,
  id: string,
  vars?: Record<string, FluentVariable> | null,
): string {
  return l10n.getString(id, vars ?? null, TOPOLOGY_UI_FALLBACKS[id] ?? id);
}
