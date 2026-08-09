import type { TopologyNodeData, TopologyWireData } from './NodeTopologyEditor';
import type { SemanticPortId } from './topologyCard';

type TopologyNodeInput = TopologyNodeData & { storeProfileId?: string };
type TopologyWireInput = TopologyWireData & {
  fromPortId?: string;
  toPortId?: string;
  relationshipType?: SemanticRelationshipType;
};


/** Version of the semantic topology contract understood by this client. */
export const TOPOLOGY_SCHEMA_VERSION = 1;

/** Closed node kinds used by the first ownership slice. */
export type SemanticNodeKind = 'branch-location' | 'workspace' | 'warehouse' | 'hardware';

/** Closed relationship types introduced by the topology ADR. */
export type SemanticRelationshipType =
  | 'location'
  | 'stock-routing'
  | 'ticket-routing'
  | 'hardware-connection'
  | 'inventory-transfer'
  | 'generic';

/** Direction of a semantic port. */
export type SemanticPortDirection = 'input' | 'output';

/** A semantic port definition, independent of canvas geometry. */
export interface SemanticPortDefinition {
  id: string;
  labelId: string;
  direction: SemanticPortDirection;
  relationshipType: SemanticRelationshipType;
  required: boolean;
  cardinality: 'one' | 'many';
}

/** Node definition for the first semantic ownership graph. */
export interface SemanticNodeDefinition {
  kind: SemanticNodeKind;
  ports: readonly SemanticPortDefinition[];
}

/** Stable semantic node in a normalized graph. */
export interface SemanticTopologyNode {
  id: string;
  kind: SemanticNodeKind;
  /** Canonical store_profiles.id for a Branch Location when known. */
  storeProfileId?: string;
  /** Technical workspace template key, when the node is a workspace. */
  typeKey?: string;
  /** Controlled business purpose, independent from type and instance label. */
  purposeKey?: string;
}

/** Stable semantic wire in a normalized graph. */
export interface SemanticTopologyWire {
  id: string;
  fromNodeId: string;
  fromPortId: string;
  toNodeId: string;
  toPortId: string;
  relationshipType: SemanticRelationshipType;
  direction: 'one-way' | 'reverse' | 'two-way';
  /** True when semantic fields were inferred from the legacy geometric graph. */
  legacyInferred: boolean;
}

/** Versioned graph envelope used by client validation and Apply preparation. */
export interface SemanticTopologyGraph {
  schemaVersion: number;
  nodes: SemanticTopologyNode[];
  wires: SemanticTopologyWire[];
}

/** Structured, localizable validation failure. */
export interface TopologyValidationError {
  code:
    | 'unsupported-schema-version'
    | 'warehouse-tier-limit'
    | 'multiple-branch-locations'
    | 'missing-branch-location'
    | 'branch-location-missing-identity'
    | 'duplicate-node'
    | 'invalid-purpose'
    | 'missing-location-input'
    | 'multiple-location-inputs'
    | 'missing-operation-input'
    | 'multiple-operation-inputs'
    | 'invalid-operation-source'
    | 'invalid-location-connection'
    | 'duplicate-wire'
    | 'unknown-wire-endpoint';
  messageId: string;
  nodeId?: string;
  wireId?: string;
  portId?: string;
}

/** The canonical Branch Location definition. `store` is its legacy alias. */
export const BRANCH_LOCATION_DEFINITION: SemanticNodeDefinition = {
  kind: 'branch-location',
  ports: [
    {
      id: 'location-out',
      labelId: 'topology-port-location-out',
      direction: 'output',
      relationshipType: 'location',
      required: true,
      cardinality: 'many',
    },
  ],
};

/** Every workspace type shares this required ownership input. */
/** Controlled business purposes. Keys are persisted; labels are localized. */
export const WORKSPACE_PURPOSES = {
  general: { typeKeys: ['store-pos', 'restaurant-pos', 'kds', 'inventory', 'warehouse'] as const, labelId: 'topology-purpose-general' },
  checkout: { typeKeys: ['store-pos'] as const, labelId: 'topology-purpose-checkout' },
  returns: { typeKeys: ['store-pos'] as const, labelId: 'topology-purpose-returns' },
  'dining-room': { typeKeys: ['restaurant-pos'] as const, labelId: 'topology-purpose-dining-room' },
  'kitchen-hot-line': { typeKeys: ['kds'] as const, labelId: 'topology-purpose-kitchen-hot-line' },
  'stock-control': { typeKeys: ['inventory', 'warehouse'] as const, labelId: 'topology-purpose-stock-control' },
  receiving: { typeKeys: ['inventory', 'warehouse'] as const, labelId: 'topology-purpose-receiving' },
} as const;

/** Every workspace type shares this required ownership input. */
export const WORKSPACE_DEFINITION: SemanticNodeDefinition = {
  kind: 'workspace',
  ports: [
    {
      id: 'location-in',
      labelId: 'topology-port-location-in',
      direction: 'input',
      relationshipType: 'location',
      required: true,
      cardinality: 'one',
    },
  ],
};

/** Operational inventory node definition; ownership is not implied. */
export const WAREHOUSE_DEFINITION: SemanticNodeDefinition = {
  kind: 'warehouse',
  ports: [],
};

/** Operational hardware node definition; ownership is not implied. */
export const HARDWARE_DEFINITION: SemanticNodeDefinition = {
  kind: 'hardware',
  ports: [],
};

/** Closed first-slice registry. Unknown node kinds are not accepted: they
 *  fold to workspace in nodeKind so ownership validation surfaces them. */
export const SEMANTIC_NODE_DEFINITIONS: Readonly<Record<SemanticNodeKind, SemanticNodeDefinition>> = {
  'branch-location': BRANCH_LOCATION_DEFINITION,
  workspace: WORKSPACE_DEFINITION,
  warehouse: WAREHOUSE_DEFINITION,
  hardware: HARDWARE_DEFINITION,
};

function nodeKind(node: TopologyNodeInput): SemanticNodeKind {
  // `store` is deliberately accepted only as a serialized compatibility alias.
  if (node.type === 'store') return 'branch-location';
  // Closed registry: an unknown type (manual edit, stale JSON) must not
  // flow into the semantic graph as an opaque kind — validateTopologyGraph
  // only checks branch-location and workspace, so an unknown kind would
  // silently pass AND round-trip to Apply. Folding to the most common kind
  // makes the ownership checks surface it (missing-location-input) instead.
  // NOTE: the final return below is a runtime-only path — node.type is
  // typed as the closed NodeType union, so TypeScript narrows the three
  // legal kinds away; only corrupt runtime data reaches the fold.
  if (node.type === 'workspace' || node.type === 'warehouse' || node.type === 'hardware') {
    return node.type;
  }
  return 'workspace';
}

function metadataString(node: TopologyNodeInput, key: string): string | undefined {
  const value = node.metadata?.[key];
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

/** The closed set of legal relationship types (mirrors SemanticRelationshipType).
 *  Used to quarantine corrupt values at the contract boundary — a garbage
 *  string (manual edit, stale JSON) must never flow into the semantic graph. */
const RELATIONSHIP_TYPES = new Set<SemanticRelationshipType>([
  'location',
  'stock-routing',
  'ticket-routing',
  'hardware-connection',
  'inventory-transfer',
  'generic',
]);

/** The closed set of legal semantic port ids — the SemanticPortId union
 *  from topologyCard.ts (the single source of truth, imported as a type so
 *  the whitelist cannot drift) plus two contract-internal placeholders for
 *  fully-unknown legacy wires. Corrupt ids must never reach consumers — the
 *  renderer matches wires to sockets by port id, and validation switches on
 *  'location-out'/'location-in'. */
const SEMANTIC_PORT_IDS: ReadonlySet<string> = new Set<SemanticPortId | 'legacy-out' | 'legacy-in'>([
  'location-out',
  'location-in',
  'operation-in',
  'stock-out',
  'stock-in',
  'transfer-out',
  'transfer-in',
  'ticket-out',
  'ticket-in',
  'device-out',
  'generic-in',
  'generic-out',
  // Contract-internal placeholders for fully-unknown legacy wires.
  'legacy-out',
  'legacy-in',
]);

function isLegalPortId(value: string | undefined): value is string {
  return value !== undefined && SEMANTIC_PORT_IDS.has(value);
}

function inferredWire(
  wire: TopologyWireInput,
  fromNode: SemanticTopologyNode | undefined,
  toNode: SemanticTopologyNode | undefined,
): Pick<SemanticTopologyWire, 'fromPortId' | 'toPortId' | 'relationshipType' | 'legacyInferred'> {
  // The early-return only trusts well-formed semantic fields: port ids
  // must be members of the closed SemanticPortId union and the type a
  // member of SemanticRelationshipType. Corrupt values (manual edits,
  // stale JSON) are treated like missing ones: fall through to legacy
  // inference, which re-derives the legal fields from node identity
  // instead of letting a garbage string flow into the semantic graph.
  if (
    isLegalPortId(wire.fromPortId)
    && isLegalPortId(wire.toPortId)
    && RELATIONSHIP_TYPES.has(wire.relationshipType as SemanticRelationshipType)
  ) {
    return {
      fromPortId: wire.fromPortId,
      toPortId: wire.toPortId,
      relationshipType: wire.relationshipType!,
      legacyInferred: false,
    };
  }

  // Legacy geometric Store -> Workspace edges are migrated by node identity,
  // never by proximity or by the old right/left anchor names.
  if (fromNode?.kind === 'branch-location' && toNode?.kind === 'workspace') {
    return {
      fromPortId: 'location-out',
      toPortId: 'location-in',
      relationshipType: 'location',
      legacyInferred: true,
    };
  }

  // Older diagrams stored only geometry for workspace-to-workspace wires.
  // Preserve the Restaurant POS → KDS operational relationship from the
  // stable workspace type keys instead of folding it to legacy-out/legacy-in;
  // otherwise the KDS appears connected but still reports a missing
  // Operation In requirement after reload.
  if (
    fromNode?.kind === 'workspace'
    && fromNode.typeKey === 'restaurant-pos'
    && toNode?.kind === 'workspace'
    && toNode.typeKey === 'kds'
  ) {
    return {
      fromPortId: 'operation-out',
      toPortId: 'operation-in',
      relationshipType: 'generic',
      legacyInferred: true,
    };
  }

  if (fromNode?.kind === 'workspace' && toNode?.kind === 'warehouse') {
    return {
      // Same closed-union discipline: a truthy-but-corrupt port or type
      // folds to the identity-derived default, never the garbage value.
      fromPortId: isLegalPortId(wire.fromPortId) ? wire.fromPortId : 'stock-out',
      toPortId: isLegalPortId(wire.toPortId) ? wire.toPortId : 'stock-in',
      relationshipType: RELATIONSHIP_TYPES.has(wire.relationshipType as SemanticRelationshipType)
        ? wire.relationshipType!
        : 'stock-routing',
      legacyInferred: true,
    };
  }

  return {
    fromPortId: isLegalPortId(wire.fromPortId) ? wire.fromPortId : 'legacy-out',
    toPortId: isLegalPortId(wire.toPortId) ? wire.toPortId : 'legacy-in',
    // Same closed-union discipline as the early return: a truthy but
    // corrupt type still folds to the last-resort default.
    relationshipType: RELATIONSHIP_TYPES.has(wire.relationshipType as SemanticRelationshipType)
      ? wire.relationshipType!
      : 'generic',
    legacyInferred: true,
  };
}

/**
 * Convert the current editor model into the versioned semantic graph.
 *
 * Missing semantic fields are treated as legacy data and normalized from node
 * identity. Geometric `top/right/bottom/left` values are intentionally not
 * consulted for ownership meaning.
 */
export function normalizeTopologyGraph(
  nodes: TopologyNodeInput[],
  wires: TopologyWireInput[],
  schemaVersion = TOPOLOGY_SCHEMA_VERSION,
): SemanticTopologyGraph {
  const semanticNodes = nodes.map((node): SemanticTopologyNode => {
    const kind = nodeKind(node);
    const semanticNode: SemanticTopologyNode = {
      id: node.id,
      kind,
    };
    const storeProfileId = node.storeProfileId ?? metadataString(node, 'storeProfileId');
    const typeKey = metadataString(node, 'typeKey');
    const purposeKey = metadataString(node, 'purposeKey') ?? (kind === 'workspace' ? 'general' : undefined);
    if (storeProfileId !== undefined) semanticNode.storeProfileId = storeProfileId;
    if (typeKey !== undefined) semanticNode.typeKey = typeKey;
    if (purposeKey !== undefined) semanticNode.purposeKey = purposeKey;
    return semanticNode;
  });
  const nodeById = new Map(semanticNodes.map((node) => [node.id, node]));

  const semanticWires = wires.map((wire): SemanticTopologyWire => {
    const inferred = inferredWire(wire, nodeById.get(wire.fromNodeId), nodeById.get(wire.toNodeId));
    return {
      id: wire.id,
      fromNodeId: wire.fromNodeId,
      fromPortId: inferred.fromPortId,
      toNodeId: wire.toNodeId,
      toPortId: inferred.toPortId,
      relationshipType: inferred.relationshipType,
      // Direction is presentation-only, but the semantic graph is the
      // contract boundary: normalize corrupt/legacy values (undefined in
      // old persisted JSON, or garbage from manual edits) to a legal
      // value so consumers (renderer, validation) never see an unknown
      // state. one-way is the historical default.
      direction: normalizeWireDirection(wire.direction),
      legacyInferred: inferred.legacyInferred,
    };
  });

  return { schemaVersion, nodes: semanticNodes, wires: semanticWires };
}

/**
 * Fold a wire direction to a legal value. Direction is presentation-only,
 * but the closed union must hold everywhere the value crosses a boundary —
 * the semantic graph AND the editor load path (where a corrupt stored
 * value would otherwise render wrong markers and round-trip to the
 * backend on the next Apply). one-way is the historical default.
 */
export function normalizeWireDirection(value: string | undefined): 'one-way' | 'reverse' | 'two-way' {
  return value === 'two-way' || value === 'reverse' ? value : 'one-way';
}

/** Return only location-ownership wires for a normalized graph. */
function locationWires(graph: SemanticTopologyGraph): SemanticTopologyWire[] {
  return graph.wires.filter((wire) => wire.relationshipType === 'location');
}

/**
 * Validate the first vertical slice: one Branch Location owns ordinary
 * workspaces, while a KDS inherits its store scope through exactly one
 * Restaurant POS operation feed.
 *
 * This is pure and deterministic so the same contract can be mirrored by the
 * Rust Apply boundary. It deliberately does not resolve display names,
 * primary stores, or a `default` store.
 */
export function validateTopologyGraph(graph: SemanticTopologyGraph): TopologyValidationError[] {
  const errors: TopologyValidationError[] = [];
  if (graph.schemaVersion !== TOPOLOGY_SCHEMA_VERSION) {
    errors.push({
      code: 'unsupported-schema-version',
      messageId: 'topology-validation-unsupported-schema',
    });
    return errors;
  }

  const seenNodeIds = new Set<string>();
  for (const node of graph.nodes) {
    if (seenNodeIds.has(node.id)) {
      errors.push({
        code: 'duplicate-node',
        messageId: 'topology-validation-duplicate-node',
        nodeId: node.id,
      });
    }
    seenNodeIds.add(node.id);
  }

  // Wire ids must be unique across the WHOLE graph, not just the
  // location-ownership tuples the duplicate-wire 4-tuple check covers. Two
  // wires sharing an id (UUID collision, stale JSON merge) break the
  // editor's React keys, click-cycle-by-id, and delete-by-id even when
  // their endpoints differ — flag it here so it can never reach Apply.
  const seenWireIds = new Set<string>();
  for (const wire of graph.wires) {
    if (seenWireIds.has(wire.id)) {
      errors.push({
        code: 'duplicate-wire',
        messageId: 'topology-validation-duplicate-wire',
        wireId: wire.id,
      });
    }
    seenWireIds.add(wire.id);
  }

  // Every wire must reference nodes that exist in the graph. Location
  // wires are already guarded by invalid-location-connection, but a
  // NON-location wire (stock-routing, ticket-routing, generic) pointing
  // at a ghost id passed silently — inferredWire saw undefined nodes and
  // the wire round-tripped to Apply referencing nothing. Endpoint
  // resolution is structural integrity, independent of relationship type.
  // This guard deliberately runs BEFORE the ownership loop: a missing
  // node is more fundamental than a wrong connection, so a ghost-targeted
  // location wire surfaces unknown-wire-endpoint (the first error shown)
  // rather than invalid-location-connection.
  const nodeIds = new Set(graph.nodes.map((node) => node.id));
  for (const wire of graph.wires) {
    if (!nodeIds.has(wire.fromNodeId) || !nodeIds.has(wire.toNodeId)) {
      errors.push({
        code: 'unknown-wire-endpoint',
        messageId: 'topology-validation-unknown-wire-endpoint',
        wireId: wire.id,
      });
    }
  }

  const branches = graph.nodes.filter((node) => node.kind === 'branch-location');
  if (branches.length === 0) {
    errors.push({ code: 'missing-branch-location', messageId: 'topology-validation-missing-branch' });
  } else if (branches.length > 1) {
    errors.push({ code: 'multiple-branch-locations', messageId: 'topology-validation-multiple-branches' });
  }

  const branchIds = new Set(branches.map((node) => node.id));
  for (const branch of branches) {
    if (!branch.storeProfileId) {
      errors.push({
        code: 'branch-location-missing-identity',
        messageId: 'topology-validation-branch-identity',
        nodeId: branch.id,
      });
    }
  }
  const workspaceNodes = graph.nodes.filter((node) => node.kind === 'workspace');
  const workspaceIds = new Set(workspaceNodes.map((node) => node.id));
  for (const workspace of workspaceNodes) {
    const typeKey = workspace.typeKey ?? 'store-pos';
    const purposeKey = workspace.purposeKey ?? 'general';
    const definition = WORKSPACE_PURPOSES[purposeKey as keyof typeof WORKSPACE_PURPOSES];
    if (!definition || !(definition.typeKeys as readonly string[]).includes(typeKey)) {
      errors.push({
        code: 'invalid-purpose',
        messageId: 'topology-validation-invalid-purpose',
        nodeId: workspace.id,
      });
    }
  }
  const ownership = locationWires(graph);
  const seenOwnership = new Set<string>();

  for (const wire of ownership) {
    const key = `${wire.fromNodeId}|${wire.fromPortId}|${wire.toNodeId}|${wire.toPortId}`;
    if (seenOwnership.has(key)) {
      errors.push({
        code: 'duplicate-wire',
        messageId: 'topology-validation-duplicate-wire',
        wireId: wire.id,
      });
    }
    seenOwnership.add(key);

    if (
      !branchIds.has(wire.fromNodeId)
      || wire.fromPortId !== 'location-out'
      || !workspaceIds.has(wire.toNodeId)
      || wire.toPortId !== 'location-in'
    ) {
      errors.push({
        code: 'invalid-location-connection',
        messageId: 'topology-validation-invalid-location',
        wireId: wire.id,
      });
    }
  }

  for (const workspaceId of workspaceIds) {
    const workspace = graph.nodes.find((node) => node.id === workspaceId);
    const isKds = workspace?.typeKey === 'kds';
    if (isKds) {
      // KDS is operationally owned by the Restaurant POS feed. Its single
      // left socket is operation-in, so a valid operation wire satisfies the
      // KDS requirement; it must not also require a second Location wire on
      // the same socket.
      const operationInputs = graph.wires.filter(
        (wire) => wire.toNodeId === workspaceId
          && wire.toPortId === 'operation-in'
          && wire.relationshipType === 'generic',
      );
      if (operationInputs.length === 0) {
        errors.push({
          code: 'missing-operation-input',
          messageId: 'topology-validation-missing-operation',
          nodeId: workspaceId,
          portId: 'operation-in',
        });
      } else if (operationInputs.length > 1) {
        errors.push({
          code: 'multiple-operation-inputs',
          messageId: 'topology-validation-multiple-operation',
          nodeId: workspaceId,
          portId: 'operation-in',
        });
      } else {
        const operationInput = operationInputs[0]!;
        const source = graph.nodes.find((node) => node.id === operationInput.fromNodeId);
        if (operationInput.fromPortId !== 'operation-out' || source?.typeKey !== 'restaurant-pos') {
          errors.push({
            code: 'invalid-operation-source',
            messageId: 'topology-validation-invalid-operation-source',
            nodeId: workspaceId,
            wireId: operationInput.id,
            portId: 'operation-in',
          });
        }
      }
      continue;
    }

    const incoming = ownership.filter(
      (wire) => wire.toNodeId === workspaceId && wire.toPortId === 'location-in',
    );
    if (incoming.length === 0) {
      errors.push({
        code: 'missing-location-input',
        messageId: 'topology-validation-missing-location',
        nodeId: workspaceId,
        portId: 'location-in',
      });
    } else if (incoming.length > 1) {
      errors.push({
        code: 'multiple-location-inputs',
        messageId: 'topology-validation-multiple-location',
        nodeId: workspaceId,
        portId: 'location-in',
      });
    }
  }

  return errors;
}

/** Return a concise localized message descriptor for the first failure. */
export function firstTopologyValidationError(
  errors: TopologyValidationError[],
): TopologyValidationError | undefined {
  return errors[0];
}
