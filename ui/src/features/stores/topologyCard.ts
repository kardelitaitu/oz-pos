import type { ComponentType } from 'react';
import type { ReactLocalization } from '@fluent/react';
import type { FluentVariable } from '@fluent/bundle';
import type { PortName, SemanticRelationshipType, TopologyNodeData } from './NodeTopologyEditor';
import {
  WorkspaceStorePosSettings,
  WorkspaceRestaurantPosSettings,
  WorkspaceKdsSettings,
} from '@/features/settings/workspace-cards';
import {
  StoreIcon,
  PosIcon,
  WarehouseIcon,
  PrinterIcon,
} from './NodeTopologyIcons';
import topologySemantics from './topologySemantics.json';

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
  | 'operation-out'
  | 'operation-in'
  | 'stock-out'
  | 'stock-in'
  | 'transfer-out'
  | 'transfer-in'
  | 'ticket-out'
  | 'ticket-in'
  | 'device-out'
  | 'generic-in'
  | 'generic-out';

/** True for Kitchen Display workspace instances (metadata typeKey 'kds'). */
export function isKdsNode(node: TopologyNodeData): boolean {
  return node.type === 'workspace' && node.metadata?.['typeKey'] === 'kds';
}

/** True for Restaurant POS workspace instances. Restaurant POS emits an
 * operational feed consumed by the KDS operation input. */
export function isRestaurantPosNode(node: TopologyNodeData): boolean {
  return node.type === 'workspace' && node.metadata?.['typeKey'] === 'restaurant-pos';
}

/** The workspace settings card the inspector renders for a typeKey. The
 *  default (store-pos) is the baseline card; per-type cards are the
 *  exception list, so adding a workspace type with its own card is a
 *  one-line change here. */
export const WORKSPACE_SETTINGS_CARD: Readonly<Record<string, ComponentType<Record<string, unknown>>>> = {
  'store-pos': WorkspaceStorePosSettings,
  'restaurant-pos': WorkspaceRestaurantPosSettings,
  'kds': WorkspaceKdsSettings,
};

/** The settings card for a workspace node, keyed by its typeKey. */
export function settingsCardForTypeKey(
  typeKey: string,
): ComponentType<Record<string, unknown>> {
  return WORKSPACE_SETTINGS_CARD[typeKey] ?? WORKSPACE_SETTINGS_CARD['store-pos']!;
}

/** A node prepared for canvas duplication/paste. A Branch Location copy must
 *  NOT carry the original's canonical store identity: the graph keeps exactly
 *  one branch, so the copy is a diagram-only card (the same model as a
 *  palette-spawned store) instead of a second card impersonating the real
 *  branch. Every duplicate path (Ctrl+D, Ctrl+V, Alt+drag, the mid-drag
 *  conversion) routes through this. Non-store nodes pass through untouched. */
export function sanitizeCopiedNode(node: TopologyNodeData): TopologyNodeData {
  if (node.type !== 'store') return node;
  const { storeProfileId: _stripped, ...rest } = node;
  return rest;
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
 *  exposes exactly ONE left input slot. Returns plain strings because
 *  callers index it against wire.toPortId (a free-form string). */
export function leftPortVariants(node: TopologyNodeData): string[] {
  if (isKdsNode(node)) return ['operation-in'];
  if (node.type === 'store') return [];
  if (node.type === 'warehouse') return ['location-in', 'operation-in'];
  if (node.type === 'hardware') return ['generic-in'];
  return ['location-in'];
}

/** Ports exposed by the frontend-only UX. Top/bottom remain load-compatible. */
export function visiblePortsForNode(node: TopologyNodeData): PortName[] {
  // A Kitchen Display consumes a single Operation feed from the left and
  // forwards ticket feeds to a printer from the right — one left input and
  // one right ticket-out output.
  if (isKdsNode(node)) return ['left', 'right'];
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
 *  wire's recorded toPortId — a warehouse input shows Stock or Transfer
 *  based on what is attached; every other node keeps its fixed label. */
export function leftPortLabelId(node: TopologyNodeData, variantIndex: number, connectedPortId?: string): string {
  // A warehouse has one primary input: Branch Location or Retail POS
  // Operation. Legacy stock/transfer wires remain load-compatible and keep
  // their historical labels when present.
  if (node.type === 'warehouse') {
    if (connectedPortId === 'operation-in') return 'topology-port-operation-in';
    if (connectedPortId === 'stock-in') return 'topology-port-stock-in';
    if (connectedPortId === 'transfer-in') return 'topology-port-transfer-in';
    return 'topology-port-location-in';
  }
  const variant = leftPortVariants(node)[variantIndex];
  if (variant === 'operation-in') return 'topology-port-operation-in';
  if (variant === 'location-in') return 'topology-port-location-in';
  // A hardware input receives device or ticket feeds — the label follows
  // the wire: Ticket In for a KDS ticket feed, neutral Input otherwise.
  if (node.type === 'hardware') {
    return connectedPortId === 'ticket-in' ? 'topology-port-ticket-in' : 'topology-port-generic-in';
  }
  return variant === 'stock-in' ? 'topology-port-stock-in' : 'topology-port-generic-in';
}

export function portLabelId(node: TopologyNodeData, port: PortName): string {
  if (port === 'left') return leftPortLabelId(node, 0);
  if (node.type === 'store' && port === 'right') return 'topology-port-location-out';
  // A KDS right socket is the ticket feed to a printer, not a generic
  // Operation output.
  if (isKdsNode(node) && port === 'right') return 'topology-port-ticket-out';
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
  if (isKdsNode(node) && port === 'right') return 'topology-port-ticket-out-aria';
  return 'topology-port-aria';
}

/** Canonical semantic port id for a socket — the only bridge from a
 *  rendered socket to the semantic wire contract. Returns undefined for
 *  ports that carry no semantic (presentation-only sockets). */
export function semanticPortId(node: TopologyNodeData, port: PortName, _variantIndex = 0): SemanticPortId | undefined {
  if (node.type === 'store' && port === 'right') return 'location-out';
  if (node.type === 'workspace' && port === 'left') {
    // A KDS consumes the Operation feed; every other workspace keeps its
    // fixed Location-in semantic.
    if (isKdsNode(node)) return 'operation-in';
    return 'location-in';
  }
  return undefined;
}

/** ALL semantic ids a socket can represent, in canonical order. The first
 *  entry is the socket's PRIMARY semantic (what gatingSemanticId resolves);
 *  extra entries exist when one socket admits multiple relationships — a
 *  Restaurant POS output admits an operational feed in addition to its
 *  stock/transfer feeds, while a warehouse input receives stock or transfer.
 *  The relationship picker (ADR #34) disambiguates multi-entry sockets at
 *  drop time. */
export function socketSemanticIds(
  node: TopologyNodeData,
  port: PortName,
  _variantIndex = 0,
): SemanticPortId[] {
  if (port === 'left') {
    // Inputs.
    if (node.type === 'store') return [];
    if (node.type === 'warehouse') return ['location-in', 'operation-in', 'stock-in', 'transfer-in'];
    // A hardware input receives device feeds AND KDS ticket feeds — the
    // ticket-in semantic is what the Resto preset's kds→printer wire
    // records, so the pairing row ticket-out → ticket-in is authorable.
    if (node.type === 'hardware') return ['generic-in', 'ticket-in'];
    // Workspace left: a KDS takes the Operation feed; every other
    // workspace takes Location.
    if (isKdsNode(node)) return ['operation-in'];
    return ['location-in'];
  }
  // Outputs.
  if (node.type === 'store') return ['location-out'];
  if (node.type === 'warehouse') return ['stock-out'];
  if (node.type === 'hardware') return ['device-out'];
  // Workspace right: a KDS forwards ticket feeds. POS workspaces emit an
  // Operation feed for a Warehouse or KDS, while retaining stock/transfer
  // routing for the existing inventory runtime. Other workspace types keep
  // their existing stock/transfer semantics.
  if (isKdsNode(node)) return ['ticket-out'];
  if (node.type === 'workspace' && node.metadata?.['typeKey'] === 'store-pos') {
    return ['stock-out', 'transfer-out', 'operation-out'];
  }
  if (isRestaurantPosNode(node)) return ['operation-out', 'stock-out', 'transfer-out'];
  return ['stock-out', 'transfer-out'];
}

/** The full typed socket map used by connection gating (ADR #34). Unlike
 *  the recording-side semanticPortId (which deliberately stays minimal so
 *  persisted wire semantics and duplicate detection are stable), this
 *  resolves EVERY socket's semantic — outputs (POS/warehouse stock-out,
 *  KDS ticket-out, hardware device-out) and non-workspace inputs
 *  (warehouse stock-in, hardware generic-in) included — so a drag in
 *  progress can tell compatible targets from incompatible ones before any
 *  wire is drawn. Name chosen to avoid colliding with semanticPortId.
 *  Defined as the socket's PRIMARY semantic so it can never disagree with
 *  the multi-semantic resolution used by the picker. */
export function gatingSemanticId(
  node: TopologyNodeData,
  port: PortName,
  variantIndex = 0,
): SemanticPortId | undefined {
  return socketSemanticIds(node, port, variantIndex)[0];
}

/** One row of the ADR #34 pairing table: a source semantic that may feed a
 *  target semantic, the typed relationship that combination represents,
 *  and the Fluent id for its human-readable label. Kept as an ordered row
 *  list so (a) the gate and the relationship picker share ONE source of
 *  truth and (b) picker options render in the order the rows appear — the
 *  PRIMARY relationship of a pair always comes first. */
interface SemanticPairingRow {
  source: SemanticPortId;
  target: SemanticPortId;
  relationshipType: SemanticRelationshipType;
  labelId: string;
}

/** The pairing table: which source semantic may feed which target
 *  semantic, and what relationship that wire then represents. Inputs are
 *  never sources, and mismatched semantics (a Location feed into a stock
 *  rack, a stock feed into a Location input) gate closed.
 *
 *  Reachability note: `operation-out` is produced by Restaurant POS for its
 *  operational feed into KDS; `ticket-in` is admitted by hardware inputs so
 *  the Resto preset's loaded kds→printer wire (ticket-out/ticket-in) is also
 *  authorable. Other unused semantic members remain contract-level and
 *  future-facing rather than authorable today. */
const SEMANTIC_PORT_PAIRINGS = topologySemantics.semanticPairings as readonly SemanticPairingRow[];

/** True when the source semantic may feed the target semantic under the
 *  typed pairing table. Unknown or input-side sources always return false
 *  — the gate fails closed. */
export function canSemanticPortsConnect(
  source: SemanticPortId,
  target: SemanticPortId,
): boolean {
  return SEMANTIC_PORT_PAIRINGS.some((r) => r.source === source && r.target === target);
}

/** Validate persisted semantic fields against the same pairing matrix used by
 *  drag gating and relationship selection. Unknown strings fail closed so
 *  hand-edited or stale payloads cannot bypass the contract. */
export function isSemanticWireCompatible(
  source: string,
  target: string,
  relationshipType: string,
): boolean {
  return SEMANTIC_PORT_PAIRINGS.some((row) =>
    row.source === source && row.target === target && row.relationshipType === relationshipType
  );
}

/** One admissible relationship for a specific socket pair — the semantic
 *  ids to RECORD on the wire (fromPortId/toPortId round-trip through the
 *  backend), the typed relationship, and its label. */
export interface WireRelationshipOption {
  fromPortId: SemanticPortId;
  toPortId: SemanticPortId;
  relationshipType: SemanticRelationshipType;
  labelId: string;
}

/** All relationships a drop between a source socket and a target socket
 *  may create, in pairing-table order (primary first). Zero options means
 *  the pair is incompatible; one option means the drop creates that wire
 *  directly; TWO OR MORE means the drop is ambiguous and the UI must ask
 *  the user which relationship they mean (the picker). */
export function wireRelationshipOptions(
  source: TopologyNodeData,
  sourcePort: PortName,
  target: TopologyNodeData,
  targetPort: PortName,
  targetVariantIndex = 0,
): WireRelationshipOption[] {
  const options: WireRelationshipOption[] = [];
  for (const src of socketSemanticIds(source, sourcePort)) {
    for (const tgt of socketSemanticIds(target, targetPort, targetVariantIndex)) {
      const row = SEMANTIC_PORT_PAIRINGS.find((r) => r.source === src && r.target === tgt);
      const operationTargetAllowed = row?.relationshipType !== 'generic'
        || (target.type === 'warehouse'
          ? source.type === 'workspace' && source.metadata?.['typeKey'] === 'store-pos'
          : target.type === 'workspace'
            && target.metadata?.['typeKey'] === 'kds'
            && source.type === 'workspace'
            && source.metadata?.['typeKey'] === 'restaurant-pos');
      if (row && operationTargetAllowed) {
        options.push({
          fromPortId: src,
          toPortId: tgt,
          relationshipType: row.relationshipType,
          labelId: row.labelId,
        });
      }
    }
  }
  return options;
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
  'topology-port-transfer-in': 'Transfer In',
  'topology-port-ticket-in': 'Ticket In',
  'topology-port-ticket-out': 'Ticket Out',
  'topology-port-ticket-out-aria': 'Ticket port',
  'topology-port-device-out': 'Device Out',
  'topology-port-generic-in': 'Input',
  'topology-port-generic-out': 'Output',
  'topology-port-aria': 'Topology port',
  'topology-relationship-location': 'Location',
  'topology-relationship-stock-routing': 'Stock routing',
  'topology-relationship-inventory-transfer': 'Transfer',
  'topology-relationship-ticket-routing': 'Ticket routing',
  'topology-relationship-hardware-connection': 'Device connection',
  'topology-relationship-operation': 'Operation',
  'topology-relationship-generic': 'Generic',
  'topology-relationship-picker-title': 'Choose connection type',
  'topology-relationship-picker-cancel': 'Cancel',
  'topology-wire-label-transfer': 'Transfer',
  'topology-wire-label-ticket': 'Ticket Print',
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
