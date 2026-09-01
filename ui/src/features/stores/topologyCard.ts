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

/** Fluent id for the label of ONE stacked semantic port row (round 174).
 *  Every semantic a socket exposes gets its own labeled row in the card
 *  footer; this resolves that row's label from the semantic id — the single
 *  mapping used by the row renderer, the relationship picker options, and
 *  the wire-endpoint row resolution. Falls back to the legacy per-side
 *  label for unknown semantics so nothing renders a raw id. */
export function semanticPortLabelId(
  node: TopologyNodeData,
  port: PortName,
  semanticId: SemanticPortId,
): string {
  const map: Record<SemanticPortId, string> = {
    'location-out': 'topology-port-location-out',
    'location-in': 'topology-port-location-in',
    'operation-out': 'topology-port-operation-out',
    'operation-in': 'topology-port-operation-in',
    'stock-out': 'topology-port-stock-out',
    'stock-in': 'topology-port-stock-in',
    'transfer-out': 'topology-port-transfer-out',
    'transfer-in': 'topology-port-transfer-in',
    'ticket-out': 'topology-port-ticket-out',
    'ticket-in': 'topology-port-ticket-in',
    'device-out': 'topology-port-device-out',
    'generic-in': 'topology-port-generic-in',
    'generic-out': 'topology-port-generic-out',
  };
  return map[semanticId] ?? portLabelId(node, port);
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
 *  PRIMARY relationship of a pair always comes first.
 *
 *  Since ADR #45 the row also carries `endpoints`: the closed list of
 *  node-kind pairs that may sit on the ends of the row. Before that, the row
 *  table was the only shared part of the rule and the endpoint predicates
 *  were re-written by hand on each side. */
interface SemanticPairingRow {
  source: SemanticPortId;
  target: SemanticPortId;
  relationshipType: SemanticRelationshipType;
  labelId: string;
  endpoints: readonly TopologyEndpointPair[];
}

/** One admitted (source kind → target kind) pair for a row. Deliberately a
 *  flat tuple list rather than a boolean expression: the contract has to be
 *  evaluable identically by the TypeScript gate and the Rust gate, and an
 *  expression language would need two parsers — which is how the two drifted
 *  apart in the first place. Rows whose admitted pairs are NOT the cross
 *  product of their endpoints (the generic operation row) are written out
 *  pair by pair. See ADR #45 §1. */
export interface TopologyEndpointPair {
  from: string;
  to: string;
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

/** Matches any node kind. Used only by the future-facing `generic-out` row,
 *  which has no registered producer yet. */
export const ANY_NODE_KIND = '*';

/** Matches the graph's Branch Location node. Whether the graph has exactly one
 *  such node is a separate rule both gates enforce before the wire loop
 *  (`multiple-branch-locations`), so at endpoint level this reduces to the
 *  node's kind. */
export const BRANCH_ROOT_KIND = '@branch-root';

/** Build the canonical kind token from a node's two identity halves. Both
 *  languages already carry `type`/`kind` plus a workspace `typeKey`;
 *  collapsing them into one string is what makes every endpoint predicate a set
 *  lookup instead of a hand-written condition. A workspace with no recorded
 *  typeKey is the Store POS baseline, matching the Apply-boundary default. */
export function nodeKindToken(kind: string, typeKey?: string | null): string {
  if (kind === 'workspace') return `workspace:${typeKey ?? 'store-pos'}`;
  // `store` is the serialized compatibility alias for `branch-location`
  // (ADR #34 §1); the contract speaks the canonical name.
  if (kind === 'store') return 'branch-location';
  return kind;
}

/** The kind token for a canvas node. */
export function nodeKindOf(node: TopologyNodeData): string {
  return nodeKindToken(node.type, node.metadata?.['typeKey'] as string | undefined);
}

/** True when a contract endpoint token admits a node's kind token.
 *
 *  Matching is exact, with one addition: a token written without a `:` suffix
 *  also covers that family. So `workspace` admits `workspace:store-pos` and
 *  `workspace:kds`, while `workspace:store-pos` admits only itself. The
 *  contract needs both — the Location row means "any workspace", the Operation
 *  row means "this one" — and a single prefix rule keeps the comparison to one
 *  line in each language rather than an expression language in JSON. */
function kindTokenAdmits(endpointToken: string, nodeKind: string): boolean {
  return endpointToken === nodeKind || nodeKind.startsWith(`${endpointToken}:`);
}

/** True when one endpoint pair admits this (source kind, target kind) tuple.
 *  Unknown kinds and unknown `@`-tokens fail closed — only `*` and
 *  `@branch-root` are special, everything else goes through
 *  {@link kindTokenAdmits}. This is THE endpoint predicate; the Rust gate
 *  implements the same function over the same JSON and ADR #45 §2 keeps them
 *  provably identical. */
function endpointPairAllows(pair: TopologyEndpointPair, fromKind: string, toKind: string): boolean {
  const sourceKind = fromKind === 'branch-location' ? BRANCH_ROOT_KIND : fromKind;
  const fromOk = pair.from === ANY_NODE_KIND || kindTokenAdmits(pair.from, sourceKind);
  const toOk = pair.to === ANY_NODE_KIND || kindTokenAdmits(pair.to, toKind);
  return fromOk && toOk;
}

/** True when a row's declared endpoints admit this node pair. An empty list
 *  admits nothing, so a payload that lost its endpoints degrades to "no wire
 *  may be authored" rather than to the looser row-only check. */
export function pairingAllowsEndpoints(
  row: SemanticPairingRow,
  fromKind: string,
  toKind: string,
): boolean {
  return row.endpoints.some((pair) => endpointPairAllows(pair, fromKind, toKind));
}

/** Kind-level form of the endpoint gate, for callers that hold a normalized
 *  graph node (`kind` + `typeKey`) rather than a canvas node. The Apply-boundary
 *  contract validator uses this so it evaluates the exact same endpoint list
 *  the canvas offered — previously it carried its own hand-written copy of the
 *  per-row predicates, which is how the two drifted. */
export function pairingAdmitsKinds(
  sourceSemantic: string,
  targetSemantic: string,
  fromKind: string,
  toKind: string,
): boolean {
  const row = SEMANTIC_PORT_PAIRINGS.find(
    (r) => r.source === sourceSemantic && r.target === targetSemantic,
  );
  return row ? pairingAllowsEndpoints(row, fromKind, toKind) : false;
}

/** The full authoring gate for one semantic pair between two specific nodes:
 *  the table must declare a row for the semantics AND that row must admit the
 *  node kinds. Returns the row so callers can build the relationship option
 *  without a second lookup. Every authoring surface — socket drop, stacked
 *  row, legacy migration — goes through this, so the canvas can never offer a
 *  wire the Apply gate would reject. */
export function pairingAllowsNodes(
  source: TopologyNodeData,
  target: TopologyNodeData,
  sourceSemantic: SemanticPortId,
  targetSemantic: SemanticPortId,
): SemanticPairingRow | undefined {
  const row = SEMANTIC_PORT_PAIRINGS.find(
    (r) => r.source === sourceSemantic && r.target === targetSemantic,
  );
  if (!row) return undefined;
  return pairingAllowsEndpoints(row, nodeKindOf(source), nodeKindOf(target)) ? row : undefined;
}

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
      const row = pairingAllowsNodes(source, target, src, tgt);
      if (row) {
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

/** Resolve the SINGLE relationship between one source semantic ROW and one
 *  target semantic ROW (round 174 stacked ports). Each stacked row is one
 *  semantic, so the pair is fixed — zero options means incompatible, one
 *  means the drop commits directly. Kept separate from the socket-wide
 *  `wireRelationshipOptions` (which enumerates every source semantic for
 *  the legacy one-socket-per-side picker flow and for compatibility
 *  checks); the editor uses this when both endpoints are specific rows. */
export function rowRelationshipOptions(
  source: TopologyNodeData,
  sourcePort: PortName,
  sourceVariantIndex: number,
  target: TopologyNodeData,
  targetPort: PortName,
  targetVariantIndex: number,
): WireRelationshipOption[] {
  const src = socketSemanticIds(source, sourcePort)[sourceVariantIndex];
  const tgt = socketSemanticIds(target, targetPort, targetVariantIndex)[targetVariantIndex];
  if (!src || !tgt) return [];
  const row = pairingAllowsNodes(source, target, src, tgt);
  if (!row) return [];
  return [{
    fromPortId: src,
    toPortId: tgt,
    relationshipType: row.relationshipType,
    labelId: row.labelId,
  }];
}

/** Node-level legacy-wire migration: every legal relationship a legacy
 *  wire between these two nodes may mean, in pairing-table order. A
 *  fully-unknown legacy wire (folded to the legacy-out/legacy-in
 *  placeholders by normalizeTopologyGraph) carries no socket semantics,
 *  so the resolution enumerates the source node's OUTPUT semantics × the
 *  target node's INPUT semantics over the pairing table — the same
 *  contract the drag gate and relationship picker use, without a specific
 *  socket. Zero options means no legal relationship exists between the
 *  pair: the wire cannot be migrated and must be deleted and recreated
 *  with the labeled ports (never silently reinterpreted). */
export function legacyWireResolutionOptions(
  source: TopologyNodeData,
  target: TopologyNodeData,
): WireRelationshipOption[] {
  // Mirror wireRelationshipOptions' iteration exactly — socket-semantics
  // order (not raw pairing-row order), same first-row lookup — so the
  // migration UI offers options in the same order the relationship picker
  // shows for a live drop between the same nodes.
  const options: WireRelationshipOption[] = [];
  for (const src of socketSemanticIds(source, 'right')) {
    for (const tgt of socketSemanticIds(target, 'left')) {
      const row = pairingAllowsNodes(source, target, src, tgt);
      if (row) {
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
  'topology-port-transfer-out': 'Transfer Out',
  'topology-port-operation-out': 'Operation',
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
  'topology-node-type-store': 'Branch Location',
  'topology-node-type-workspace': 'Workspace',
  'topology-node-type-warehouse': 'Warehouse',
  'topology-node-type-hardware': 'Hardware Device',
  'topology-hardware-thermal-receipt': 'Thermal Receipt Printer',
  'topology-hardware-thermal-kitchen': 'Kitchen Printer',
  'topology-hardware-barcode-scanner': 'Barcode Scanner',
  'topology-hardware-cash-drawer': 'Cash Drawer',
  'topology-hardware-display-customer': 'Customer Display',
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
