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
   * 1. Resolve store_id for each workspace node from topology wires.
   * 2. Detect typeKey changes on persisted nodes and implement archive +
   *    recreate (Critical #1) — type_key is immutable by backend contract.
   * 3. Diff workspace nodes against loaded instances, send creates,
   *    updates, and archives as a single atomic `apply_topology_diff` call.
   *
   * Returns an `oldId -> newId` map so the editor can remap the canvas
   * state when archive+recreate assigns new UUIDs.
   */
  const handleTopologySave = useCallback(
    async (
      nodes: TopologyNodeData[],
      wires: TopologyWireData[],
    ): Promise<Record<string, string>> => {
      const idMap: Record<string, string> = {};

      if (!sessionToken) {
        addToast({ message: l10n.getString('topology-toast-no-session'), type: 'error' });
        return idMap;
      }

      const wsNodes = nodes.filter((n) => n.type === 'workspace');
      const loadedById = new Map(workspaceInstances.map((w) => [w.instance_id, w]));
      const canvasIds = new Set(wsNodes.map((n) => n.id));

      // ── Wire-based store_id resolution (Critical #5) ─────────────────
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
          const storeNode = nodes.find((n) => n.id === storeNodeId);
          if (storeNode) {
            const matched = stores.find((s) => s.name === storeNode.name);
            if (matched) return matched.id;
          }
        }
        return stores.find((s) => s.is_primary)?.id ?? 'default';
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
          const newId = `ws-${crypto.randomUUID()}`;
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
            store_id: resolveStoreId(node),
            name: node.name,
          });
        } else if (existing.name !== node.name) {
          updates.push({ id: node.id, name: node.name });
        }
      }

      // Archive instances removed from the canvas.
      for (const inst of workspaceInstances) {
        if (!canvasIds.has(inst.instance_id)) {
          archives.push(inst.instance_id);
        }
      }

      // ── Remap diagram for type-changed nodes ─────────────────────────
      //
      // Replace old node IDs with new UUIDs in both the node and wire
      // payloads so the saved topology diagram stays consistent with the
      // recreated workspace instances.

      type DiagramNodePayload = Parameters<typeof applyTopologyDiff>[4][number];
      type DiagramWirePayload = Parameters<typeof applyTopologyDiff>[5][number];

      const diagramNodes: DiagramNodePayload[] = nodes.map((n) => {
        const changedId = typeChanges.get(n.id)?.newId ?? n.id;
        const payload: DiagramNodePayload = {
          id: changedId,
          type: n.type,
          name: n.name,
          x: n.x,
          y: n.y,
        };
        if (n.subtitle !== undefined) payload.subtitle = n.subtitle;
        if (n.tierRequirement !== undefined) payload.tier_requirement = n.tierRequirement;
        if (n.telemetryBadge !== undefined) payload.telemetry_badge = n.telemetryBadge;
        if (n.telemetryStatus !== undefined) payload.telemetry_status = n.telemetryStatus;
        if (n.metadata !== undefined) {
          // For type-changed nodes, reflect the new backend state
          // (the recreated instance will be persisted).
          const change = typeChanges.get(n.id);
          payload.metadata = change
            ? { ...n.metadata, persisted: true }
            : n.metadata;
        }
        return payload;
      });

      const diagramWires: DiagramWirePayload[] = wires.map((w) => {
        const fromId = typeChanges.get(w.fromNodeId)?.newId ?? w.fromNodeId;
        const toId = typeChanges.get(w.toNodeId)?.newId ?? w.toNodeId;
        const payload: DiagramWirePayload = {
          id: w.id,
          from_node_id: fromId,
          to_node_id: toId,
          direction: w.direction,
        };
        if (w.label !== undefined) payload.label = w.label;
        if (w.fromPort !== undefined) payload.from_port = w.fromPort;
        if (w.toPort !== undefined) payload.to_port = w.toPort;
        return payload;
      });

      // ── Atomic apply ─────────────────────────────────────────────────

      try {
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
        const typeChangeCount = typeChanges.size;
        const parts = [
          `${created} created`,
          `${updated} updated`,
          `${archived} archived`,
        ];
        if (typeChangeCount > 0) {
          parts.push(`${typeChangeCount} type-changed`);
        }
        addToast({
          message: l10n.getString('topology-toast-saved', { detail: parts.join(', ') }),
          type: 'success',
        });

        // Refresh loaded instances so subsequent saves diff against truth.
        try {
          setWorkspaceInstances(await listWorkspacesScoped(sessionToken));
        } catch {
          /* non-fatal */
        }

        return idMap;
      } catch (err) {
        addToast({
          message: `${l10n.getString('topology-toast-save-error')}: ${err instanceof Error ? err.message : String(err)}`,
          type: 'error',
        });
        return {};
      }
    },
    [sessionToken, workspaceInstances, stores, addToast, l10n],
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
