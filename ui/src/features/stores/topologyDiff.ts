// ── Topology save diff ────────────────────────────────────────────
//
// Pure computation of the workspace-instance diff TopologyScreen builds
// when the editor applies: given the canvas model and the backend's
// loaded workspace instances, produce the create/update/archive vectors
// and the type-change remap (Critical #1). Kept free of React and screen
// state so the workspace-instance semantics are unit-testable directly
// (round 149) — TopologyScreen's handleTopologySave delegates here
// instead of embedding this block in the callback.

import { normalizeTopologyGraph } from './topologyContract';
import type { CreateInstanceRequest, UpdateInstanceRequest } from '@/api/topology';
import type { WorkspaceDto } from '@/api/workspaces';
import type { TopologyNodeData, TopologyWireData } from './NodeTopologyEditor';

export interface TopologyDiffInput {
  /** Canvas nodes (the diff's "after" side). */
  nodes: TopologyNodeData[];
  wires: TopologyWireData[];
  /** Loaded backend workspace instances (the diff's "before" side). */
  workspaceInstances: WorkspaceDto[];
  /** Known store profiles — needed for legacy store-node id resolution. */
  stores: Array<{ id: string }>;
  /** UUID generator for type-change recreates; injectable for tests. */
  makeId?: () => string;
}

export interface TopologyDiffResult {
  creations: CreateInstanceRequest[];
  updates: UpdateInstanceRequest[];
  archives: string[];
  /** Node id → replacement identity for type-changed workspaces. */
  typeChanges: Map<string, { newId: string; newTypeKey: string }>;
  /** oldId → newId remap the editor applies to its canvas state. */
  idMap: Record<string, string>;
}

export function computeTopologyDiff(input: TopologyDiffInput): TopologyDiffResult {
  const {
    nodes,
    wires,
    workspaceInstances,
    stores,
    makeId = () => `ws-${crypto.randomUUID()}`,
  } = input;

  const idMap: Record<string, string> = {};
  const semanticGraph = normalizeTopologyGraph(nodes, wires);

  const wsNodes = nodes.filter((n) => n.type === 'workspace');
  const loadedById = new Map(workspaceInstances.map((w) => [w.instance_id, w]));
  const canvasIds = new Set(wsNodes.map((n) => n.id));

  // ── Semantic store_id resolution ────────────────────────────────
  // The validator has already established one Branch Location parent
  // for ordinary workspaces, and a Restaurant POS operation source for
  // KDS. Resolve from the stable node reference or canonical
  // store_profile_id; never use names, primary, or default.
  const semanticNodes = new Map(semanticGraph.nodes.map((node) => [node.id, node]));
  const resolveStoreId = (node: TopologyNodeData): string => {
    const semanticWire = semanticGraph.wires.find(
      (wire) => wire.toNodeId === node.id
        && (wire.relationshipType === 'location' || wire.legacyInferred)
        && semanticNodes.get(wire.fromNodeId)?.kind === 'branch-location',
    );
    const branchNode = semanticWire ? semanticNodes.get(semanticWire.fromNodeId) : undefined;
    const storeId = branchNode?.storeProfileId
      ?? node.storeProfileId
      ?? (node.metadata?.['storeProfileId'] as string | undefined);
    if (storeId) return storeId;

    // KDS nodes are operational children of a POS node rather than
    // direct Branch Location children. Follow the Operation feed back
    // to its source so the KDS inherits the same store scope without
    // inventing a second Location wire on its single input socket.
    const operationSource = semanticGraph.wires
      .find((wire) => wire.toNodeId === node.id
        && wire.toPortId === 'operation-in'
        && wire.relationshipType === 'generic');
    if (operationSource && operationSource.fromNodeId !== node.id) {
      const sourceNode = nodes.find((candidate) => candidate.id === operationSource.fromNodeId);
      if (sourceNode) return resolveStoreId(sourceNode);
    }

    // Compatibility boundary for legacy CRUD-only calls. Resolve a
    // legacy Store node by its stable node ID when it is a real store
    // profile ID; never match by display name.
    const connectedLegacyStoreId = wires
      .map((wire) => {
        const otherId = wire.toNodeId === node.id
          ? wire.fromNodeId
          : wire.fromNodeId === node.id
            ? wire.toNodeId
            : undefined;
        return otherId && nodes.some((candidate) => candidate.id === otherId && candidate.type === 'store')
          ? otherId
          : undefined;
      })
      .find((storeId): storeId is string => storeId !== undefined);
    if (connectedLegacyStoreId && stores.some((store) => store.id === connectedLegacyStoreId)) {
      return connectedLegacyStoreId;
    }
    // Strict validation above guarantees this path is unreachable for
    // real topology saves. Keep an explicit error rather than silently
    // reintroducing primary/default ownership inference.
    throw new Error('workspace has no semantic Branch Location ownership');
  };

  // ── Type-change detection (Critical #1) ──────────────────────────
  //
  // Walk persisted workspace nodes. For each one where the inspector's
  // typeKey differs from the backend's type_key, schedule an archive +
  // recreate. Generate new UUIDs so the recreated instance gets a fresh
  // primary key and the topology diagram stays consistent.
  const typeChanges = new Map<
    string,
    { newId: string; newTypeKey: string }
  >();
  for (const node of wsNodes) {
    const existing = loadedById.get(node.id);
    if (!existing) continue;
    const newTypeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';
    if (existing.type_key !== newTypeKey) {
      const newId = makeId();
      typeChanges.set(node.id, { newId, newTypeKey });
      idMap[node.id] = newId;
    }
  }

  // ── Build diff vectors ───────────────────────────────────────────

  const creations: CreateInstanceRequest[] = [];
  const updates: UpdateInstanceRequest[] = [];
  const archives: string[] = [];

  for (const node of wsNodes) {
    const change = typeChanges.get(node.id);
    if (change) {
      // Archive old instance, create replacement with new typeKey.
      archives.push(node.id);
      creations.push({
        id: change.newId,
        type_key: change.newTypeKey,
        purpose_key: (node.metadata?.['purposeKey'] as string) ?? 'general',
        store_id: resolveStoreId(node),
        name: node.name,
      });
      continue;
    }

    const existing = loadedById.get(node.id);
    if (!existing) {
      creations.push({
        id: node.id,
        type_key: (node.metadata?.['typeKey'] as string) ?? 'store-pos',
        purpose_key: (node.metadata?.['purposeKey'] as string) ?? 'general',
        store_id: resolveStoreId(node),
        name: node.name,
      });
    } else {
      const nextPurposeKey = (node.metadata?.['purposeKey'] as string) ?? existing.purpose_key ?? 'general';
      if (existing.name !== node.name || existing.purpose_key !== nextPurposeKey) {
        updates.push({ id: node.id, name: node.name, purpose_key: nextPurposeKey });
      }
    }
  }

  // Archive instances removed from the canvas.
  for (const inst of workspaceInstances) {
    if (!canvasIds.has(inst.instance_id)) {
      archives.push(inst.instance_id);
    }
  }

  return { creations, updates, archives, typeChanges, idMap };
}
