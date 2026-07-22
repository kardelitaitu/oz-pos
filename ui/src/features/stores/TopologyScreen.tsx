import { useState, useEffect, useCallback, useMemo } from 'react';
import { useLocalization } from '@fluent/react';
import { listStores, type StoreProfile } from '@/api/stores';
import {
  listWorkspacesScoped,
  createWorkspaceInstanceScoped,
  updateWorkspaceInstanceScoped,
  archiveWorkspaceInstanceScoped,
  type WorkspaceDto,
} from '@/api/workspaces';
import { saveTopology } from '@/api/topology';
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
   * Persist topology edits: diff the canvas workspace nodes against the
   * loaded workspace_instances and create / update / archive accordingly,
   * then save the diagram (positions + wires) for layout restoration.
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

      let created = 0;
      let updated = 0;
      let archived = 0;

      try {
        // Create + update workspace nodes.
        //
        // NOTE: only `name` is diffed/persisted. The instance `description`
        // column is NOT round-trippable through WorkspaceDto (which returns
        // the workspace *type* description, not the instance's), so a node's
        // subtitle is treated as read-only cosmetic — persisting it would
        // produce phantom "changed" diffs on every reload.
        for (const node of wsNodes) {
          const existing = loadedById.get(node.id);
          const typeKey = (node.metadata?.['typeKey'] as string) ?? 'store-pos';
          if (!existing) {
            // New node — not yet backed by a workspace_instances row.
            await createWorkspaceInstanceScoped(sessionToken, {
              id: node.id,
              type_key: typeKey,
              store_id: loadedById.size > 0
                ? [...loadedById.values()][0]!.store_id
                : (stores.find((s) => s.is_primary)?.id ?? 'default'),
              name: node.name,
            });
            created += 1;
          } else if (existing.name !== node.name) {
            await updateWorkspaceInstanceScoped(sessionToken, node.id, {
              name: node.name,
            });
            updated += 1;
          }
        }

        // Archive instances that were removed from the canvas.
        for (const inst of workspaceInstances) {
          if (!canvasIds.has(inst.instance_id)) {
            await archiveWorkspaceInstanceScoped(sessionToken, inst.instance_id);
            archived += 1;
          }
        }

        // Persist the visual diagram (node positions + wires). Map the
        // editor's camelCase shapes to the backend's snake_case payloads.
        await saveTopology(
          nodes.map((n) => {
            const payload = {
              id: n.id,
              type: n.type,
              name: n.name,
              x: n.x,
              y: n.y,
            } as Parameters<typeof saveTopology>[0][number];
            if (n.subtitle !== undefined) payload.subtitle = n.subtitle;
            if (n.tierRequirement !== undefined) payload.tier_requirement = n.tierRequirement;
            if (n.telemetryBadge !== undefined) payload.telemetry_badge = n.telemetryBadge;
            if (n.telemetryStatus !== undefined) payload.telemetry_status = n.telemetryStatus;
            if (n.metadata !== undefined) payload.metadata = n.metadata;
            return payload;
          }),
          wires.map((w) => {
            const payload = {
              id: w.id,
              from_node_id: w.fromNodeId,
              to_node_id: w.toNodeId,
              direction: w.direction,
            } as Parameters<typeof saveTopology>[1][number];
            if (w.label !== undefined) payload.label = w.label;
            if (w.fromPort !== undefined) payload.from_port = w.fromPort;
            if (w.toPort !== undefined) payload.to_port = w.toPort;
            return payload;
          }),
        );

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
