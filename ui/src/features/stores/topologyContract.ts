import type { TopologyNodeData, TopologyWireData } from './NodeTopologyEditor';

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
  direction: 'one-way' | 'two-way';
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
    | 'multiple-branch-locations'
    | 'missing-branch-location'
    | 'branch-location-missing-identity'
    | 'duplicate-node'
    | 'invalid-purpose'
    | 'missing-location-input'
    | 'multiple-location-inputs'
    | 'invalid-location-connection'
    | 'duplicate-wire';
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

/** Closed first-slice registry. Unknown node kinds are not accepted. */
export const SEMANTIC_NODE_DEFINITIONS: Readonly<Record<SemanticNodeKind, SemanticNodeDefinition>> = {
  'branch-location': BRANCH_LOCATION_DEFINITION,
  workspace: WORKSPACE_DEFINITION,
  warehouse: WAREHOUSE_DEFINITION,
  hardware: HARDWARE_DEFINITION,
};

function nodeKind(node: TopologyNodeInput): SemanticNodeKind {
  // `store` is deliberately accepted only as a serialized compatibility alias.
  if (node.type === 'store') return 'branch-location';
  return node.type;
}

function metadataString(node: TopologyNodeInput, key: string): string | undefined {
  const value = node.metadata?.[key];
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function inferredWire(
  wire: TopologyWireInput,
  fromNode: SemanticTopologyNode | undefined,
  toNode: SemanticTopologyNode | undefined,
): Pick<SemanticTopologyWire, 'fromPortId' | 'toPortId' | 'relationshipType' | 'legacyInferred'> {
  if (wire.fromPortId && wire.toPortId && wire.relationshipType) {
    return {
      fromPortId: wire.fromPortId,
      toPortId: wire.toPortId,
      relationshipType: wire.relationshipType,
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

  if (fromNode?.kind === 'workspace' && toNode?.kind === 'warehouse') {
    return {
      fromPortId: wire.fromPortId ?? 'stock-out',
      toPortId: wire.toPortId ?? 'stock-in',
      relationshipType: wire.relationshipType ?? 'stock-routing',
      legacyInferred: true,
    };
  }

  return {
    fromPortId: wire.fromPortId ?? 'legacy-out',
    toPortId: wire.toPortId ?? 'legacy-in',
    relationshipType: wire.relationshipType ?? 'generic',
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
      direction: wire.direction,
      legacyInferred: inferred.legacyInferred,
    };
  });

  return { schemaVersion, nodes: semanticNodes, wires: semanticWires };
}

/** Return only location-ownership wires for a normalized graph. */
function locationWires(graph: SemanticTopologyGraph): SemanticTopologyWire[] {
  return graph.wires.filter((wire) => wire.relationshipType === 'location');
}

/**
 * Validate the first vertical slice: one Branch Location owns every workspace.
 *
 * This is pure and deterministic so the same contract can be mirrored by the
 * Rust Apply boundary later. It deliberately does not resolve display names,
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
      || wire.direction !== 'one-way'
    ) {
      errors.push({
        code: 'invalid-location-connection',
        messageId: 'topology-validation-invalid-location',
        wireId: wire.id,
      });
    }
  }

  for (const workspaceId of workspaceIds) {
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
