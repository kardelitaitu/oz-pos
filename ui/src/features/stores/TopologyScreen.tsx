import { useState, useEffect, useCallback, useMemo } from 'react';
import { useLocalization } from '@fluent/react';
import { listStores, type StoreProfile } from '@/api/stores';
import {
  listWorkspacesScoped,
  type WorkspaceDto,
} from '@/api/workspaces';
import {
  applyTopologyDiff,
  type CreateInstanceRequest,
  type UpdateInstanceRequest,
} from '@/api/topology';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useToast } from '@/frontend/shared/Toast';
import { checkLicenseStatus } from '@/api/license';
import NodeTopologyEditor, {
  type TopologyNodeData,
  type TopologyWireData,
  type WorkspaceInstanceSeed,
} from './NodeTopologyEditor';

/**
 * Dedicated topology screen — the single home for the node-based store
 * topology builder. Owns loading of real workspace instances, license tier,
 * seeding the editor, and the create/update/archive bridge to
 * `workspace_instances` on save.
 *
 * This is intentionally separate from the Stores dashboard: "Stores" manages
 * store profiles only, while topology is its own concern (ADR #7 IA cleanup).
 */
export default function TopologyScreen() {
  const { sessionToken } = useWorkspace();
  const { addToast } = useToast();
  const { l10n } = useLocalization();
  const [licenseTier, setLicenseTier] = useState('standard');
  /** Real workspace instances loaded from the backend, used to seed the editor. */
  const [workspaceInstances, setWorkspaceInstances] = useState<WorkspaceDto[]>([]);
  const [stores, setStores] = useState<StoreProfile[]>([]);

  const load = useCallback(async () => {
    try {
      const [licStatus, storeData] = await Promise.all([
        checkLicenseStatus(),
        listStores(),
      ]);
      setLicenseTier(licStatus.tier.toLowerCase());
      setStores(storeData);
      if (sessionToken) {
        try {
          setWorkspaceInstances(await listWorkspacesScoped(sessionToken));
        } catch {
          setWorkspaceInstances([]);
        }
      }
    } catch {
      /* non-fatal — the editor still renders with the preset fallback */
    }
  }, [sessionToken]);

  useEffect(() => { load(); }, [load]);

  /** Seed the topology editor with real workspace instances. */
  const workspaceSeed: WorkspaceInstanceSeed[] = useMemo(
    () => workspaceInstances.map((w) => {
      const seed: WorkspaceInstanceSeed = {
        instanceId: w.instance_id,
        typeKey: w.type_key,
        name: w.name,
      };
      if (w.description) seed.subtitle = w.description;
      if (w.colour) seed.colour = w.colour;
      return seed;
    }),
    [workspaceInstances],
  );

  /**
   * Persist topology edits atomically (Critical #4 + #5):
   *
   * 1. Resolve store_id for each workspace node from topology wires
   *    (Store→Workspace edges determine which store a register belongs to,
   *    fixing the #5 bug where all registers bound to the first loaded
   *    instance's store_id).
   * 2. Diff workspace nodes against loaded instances and send creates,
   *    updates, and archives as a single `apply_topology_diff` call.
   *    The backend executes all workspace CRUD in one SQLite transaction,
   *    then saves the diagram on the global DB.
   */
  const handleTopologySave = useCallback(
    async (nodes: TopologyNodeData[], wires: TopologyWireData[]) => {
      if (!sessionToken) {
        addToast({ message: 'No active session — cannot save workspaces.', type: 'error' });
        return;
      }

      const wsNodes = nodes.filter((n) => n.type === 'workspace');
      const loadedById = new Map(workspaceInstances.map((w) => [w.instance_id, w]));
      const canvasIds = new Set(wsNodes.map((n) => n.id));

      // ── Wire-based store_id resolution (Critical #5) ─────────────────
      //
      // Build the set of store-node IDs on the canvas, then walk every
      // wire to find which store node each workspace node is connected to.
      // The resolved store_id flows into the workspace's creation request.
      const storeNodeIds = new Set(nodes.filter((n) => n.type === 'store').map((n) => n.id));
      const wsToStoreNode = new Map<string, string>();
      for (const wire of wires) {
        if (storeNodeIds.has(wire.fromNodeId) && canvasIds.has(wire.toNodeId)) {
          wsToStoreNode.set(wire.toNodeId, wire.fromNodeId);
        }
        if (storeNodeIds.has(wire.toNodeId) && canvasIds.has(wire.fromNodeId)) {
          wsToStoreNode.set(wire.fromNodeId, wire.toNodeId);
        }
      }

      const resolveStoreId = (node: TopologyNodeData): string => {
        const storeNodeId = wsToStoreNode.get(node.id);
        if (storeNodeId) {
          // Heuristic: match by node name against loaded store profiles.
          // If the topology store node's name matches a real store_profile
          // name, use that profile's id. Falls back to the primary store
          // when no match is found (same behaviour as single-store setups).
          const storeNode = nodes.find((n) => n.id === storeNodeId);
          if (storeNode) {
            const matched = stores.find((s) => s.name === storeNode.name);
            if (matched) return matched.id;
          }
        }
        // Fallback: primary store (same behaviour as before for single-store setups).
        return stores.find((s) => s.is_primary)?.id ?? 'default';
      };

      // ── Build diff vectors ───────────────────────────────────────────

      const creations: CreateInstanceRequest[] = [];
      const updates: UpdateInstanceRequest[] = [];
      const archives: string[] = [];

      for (const node of wsNodes) {
        const existing = loadedById.get(node.id);
        const typeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';

        if (!existing) {
          // New workspace node — needs a creation.
          creations.push({
            id: node.id,
            type_key: typeKey,
            store_id: resolveStoreId(node),
            name: node.name,
          });
        } else if (existing.name !== node.name) {
          // Existing node with a changed name.
          updates.push({ id: node.id, name: node.name });
        }
      }

      // Archive instances that were removed from the canvas.
      for (const inst of workspaceInstances) {
        if (!canvasIds.has(inst.instance_id)) {
          archives.push(inst.instance_id);
        }
      }

      // ── Atomic apply ─────────────────────────────────────────────────

      try {
        // Map the editor's camelCase shapes to the backend's snake_case payloads.
        // applyTopologyDiff param order: (sessionToken, creations, updates, archives, diagramNodes, diagramWires).
        // Use Parameters<typeof applyTopologyDiff>[4] for the diagramNodes and [5] for diagramWires.
        type DiagramNodePayload = Parameters<typeof applyTopologyDiff>[4][number];
        type DiagramWirePayload = Parameters<typeof applyTopologyDiff>[5][number];

        const diagramNodes: DiagramNodePayload[] = nodes.map((n) => {
          const payload: DiagramNodePayload = {
            id: n.id,
            type: n.type,
            name: n.name,
            x: n.x,
            y: n.y,
          };
          if (n.subtitle !== undefined) payload.subtitle = n.subtitle;
          if (n.tierRequirement !== undefined) payload.tier_requirement = n.tierRequirement;
          if (n.telemetryBadge !== undefined) payload.telemetry_badge = n.telemetryBadge;
          if (n.telemetryStatus !== undefined) payload.telemetry_status = n.telemetryStatus;
          if (n.metadata !== undefined) payload.metadata = n.metadata;
          return payload;
        });

        const diagramWires: DiagramWirePayload[] = wires.map((w) => {
          const payload: DiagramWirePayload = {
            id: w.id,
            from_node_id: w.fromNodeId,
            to_node_id: w.toNodeId,
            direction: w.direction,
          };
          if (w.label !== undefined) payload.label = w.label;
          if (w.fromPort !== undefined) payload.from_port = w.fromPort;
          if (w.toPort !== undefined) payload.to_port = w.toPort;
          return payload;
        });

        await applyTopologyDiff(
          sessionToken,
          creations,
          updates,
          archives,
          diagramNodes,
          diagramWires,
        );

        const created = creations.length;
        const updated = updates.length;
        const archived = archives.length;
        addToast({
          message: `Topology saved: ${created} created, ${updated} updated, ${archived} archived.`,
          type: 'success',
        });

        // Refresh loaded instances so subsequent saves diff against truth.
        try {
          setWorkspaceInstances(await listWorkspacesScoped(sessionToken));
        } catch {
          /* non-fatal */
        }
      } catch (err) {
        addToast({
          message: `Failed to save topology: ${err instanceof Error ? err.message : String(err)}`,
          type: 'error',
        });
      }
    },
    [sessionToken, workspaceInstances, stores, addToast],
  );

  return (
    <div
      className="settings-topology-container"
      aria-label={l10n.getString('settings-nav-topology') || 'Topology'}
    >
      <NodeTopologyEditor
        currentTier={licenseTier as 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise'}
        workspaceInstances={workspaceSeed}
        onSave={handleTopologySave}
      />
    </div>
  );
}
