import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useLocalization } from '@fluent/react';
import { listStores, createStore, updateStore, deleteStore, type StoreProfile } from '@/api/stores';
import {
  listWorkspacesScoped,
  updateWorkspaceInstanceScoped,
  type WorkspaceDto,
} from '@/api/workspaces';
import {
  applyTopologyDiff,
  type CreateInstanceRequest,
  type UpdateInstanceRequest,
} from '@/api/topology';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useToast } from '@/frontend/shared/Toast';
import { requiredLocalized } from '@/frontend/shared';
import { checkLicenseStatus } from '@/api/license';
import { plainErrorMessage } from '@/utils/app-error';
import SettingsSelect from '@/features/settings/SettingsSelect';
import { Button } from '@/components/Button';
import NodeTopologyEditor, {
  type TopologyNodeData,
  type TopologyWireData,
  type WorkspaceInstanceSeed,
  type BranchLocationSeed,
} from './NodeTopologyEditor';
import {
  normalizeTopologyGraph,
  validateTopologyGraph,
} from './topologyContract';

/**
 * Workspace instances that are physical nodes in the store topology.
 *
 * The 'admin' instance is a system workspace surfaced automatically for
 * owner/manager roles — it is NOT a store node. It must never seed the
 * topology canvas, and it must never reach the save diff's archive sweep
 * either: the sweep archives any instance missing from the canvas, so an
 * unseeded admin instance would be archived on every save.
 */
const isTopologyInstance = (w: Pick<WorkspaceDto, 'type_key'>) => w.type_key !== 'admin';

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
  const { sessionToken, resolvedStoreId } = useWorkspace();
  const { addToast } = useToast();
  const { l10n } = useLocalization();
  const [licenseTier, setLicenseTier] = useState('standard');
  /** Real workspace instances loaded from the backend, used to seed the editor. */
  const [workspaceInstances, setWorkspaceInstances] = useState<WorkspaceDto[]>([]);
  const [stores, setStores] = useState<StoreProfile[]>([]);
  /** Branch (store profile) whose topology graph is on canvas. */
  const [selectedBranchId, setSelectedBranchId] = useState<string | null>(null);
  const [addingBranch, setAddingBranch] = useState(false);
  const [newBranchName, setNewBranchName] = useState('');
  /** Two-step branch deletion: armed state + in-flight guard. The target
   *  id is captured at arm time so a mid-confirm branch switch can neither
   *  change what the confirm message names nor what the button deletes. */
  const [deletingBranch, setDeletingBranch] = useState(false);
  const [deleteBranchSaving, setDeleteBranchSaving] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [licStatus, storeData] = await Promise.all([
        checkLicenseStatus(),
        listStores(),
      ]);
      setLicenseTier(licStatus.tier.toLowerCase());
      setStores(storeData);
    } catch {
      /* non-fatal — the editor still renders with the preset fallback */
    }
  }, []);

  /** Fetch the workspace instances for the selected branch. Runs on mount
   *  AND whenever the branch selector changes: each branch owns its own
   *  topology graph, so switching branches must load that branch's
   *  instances (and, via the editor's workspaceInstances effect, its saved
   *  diagram) instead of showing the previous branch's canvas. The default
   *  null→first-branch transition is NOT a user switch — it is the initial
   *  resolution, and the mount effect already loaded the instances. */
  const loadWorkspaceInstances = useCallback(async () => {
    if (!sessionToken) {
      setWorkspaceInstances([]);
      return;
    }
    try {
      setWorkspaceInstances((await listWorkspacesScoped(sessionToken)).filter(isTopologyInstance));
    } catch {
      setWorkspaceInstances([]);
    }
  }, [sessionToken]);

  useEffect(() => { void load(); }, [load]);
  // Mount: load the default branch's instances once.
  useEffect(() => { void loadWorkspaceInstances(); }, [loadWorkspaceInstances]);
  // Branch switch: reload that branch's graph. The ref ignores the initial
  // null→default resolution (already loaded on mount) so a genuine change
  // is the only thing that triggers a refetch.
  useEffect(() => {
    if (selectedBranchId === null || selectedBranchId === lastBranchRef.current) return;
    lastBranchRef.current = selectedBranchId;
    void loadWorkspaceInstances();
  }, [selectedBranchId, loadWorkspaceInstances]);

  /** The branch whose graph is currently loaded on canvas. Lets the
   *  branch-switch refetch effect below distinguish a genuine user switch
   *  from the initial null→default resolution (whose instances were already
   *  loaded on mount). Initialized by the defaulting effect below. */
  const lastBranchRef = useRef<string | null>(null);

  /** Default the selector to the session's resolved store when available.
   *  The default branch is resolved ONCE — record it so the branch-switch
   *  refetch effect skips the initial null→default transition (the mount
   *  effect already loaded those instances). */
  useEffect(() => {
    setSelectedBranchId((prev) => {
      if (prev) {
        if (lastBranchRef.current === null) lastBranchRef.current = prev;
        return prev;
      }
      const next = resolvedStoreId && stores.some((s) => s.id === resolvedStoreId)
        ? resolvedStoreId
        : stores[0]?.id ?? null;
      if (next !== null) lastBranchRef.current = next;
      return next;
    });
  }, [resolvedStoreId, stores]);

  /** Name of the branch armed for deletion, for the delete-confirm message. */
  const deleteTargetName = stores.find((s) => s.id === deleteTargetId)?.name ?? '';

  /** Seed the topology editor with real workspace instances for the selected branch. */
  const branchLocationSeed: BranchLocationSeed[] = useMemo(
    () => stores
      // A topology is branch-scoped: exactly one Branch Location root per
      // graph. The selector picks which branch's graph is on canvas; without
      // a selected branch the graph stays visibly unowned and is blocked by
      // semantic validation rather than guessing a fallback.
      .filter((store) => selectedBranchId === null || store.id === selectedBranchId)
      .map((store) => ({ id: store.id, name: store.name })),
    [stores, selectedBranchId],
  );

  const workspaceSeed: WorkspaceInstanceSeed[] = useMemo(
    () => workspaceInstances
      .filter((w) => selectedBranchId === null || w.store_id === selectedBranchId)
      .map((w) => {
        const seed: WorkspaceInstanceSeed = {
          instanceId: w.instance_id,
          typeKey: w.type_key,
          purposeKey: w.purpose_key,
          storeId: w.store_id,
          storeName: w.store_name,
          name: w.name,
        };
        if (w.description) seed.subtitle = w.description;
        if (w.colour) seed.colour = w.colour;
        return seed;
      }),
    [workspaceInstances, selectedBranchId],
  );

  const handleAddBranch = async () => {
    const name = newBranchName.trim();
    if (!name) return;
    try {
      const created = await createStore({ id: `store-${crypto.randomUUID()}`, name });
      setStores((prev) => [...prev, created]);
      setSelectedBranchId(created.id);
      setAddingBranch(false);
      setNewBranchName('');
    } catch (err) {
      addToast({
        message: `${l10n.getString('topology-branch-add-error')}: ${plainErrorMessage(err)}`,
        type: 'error',
      });
    }
  };

  /** Delete the selected store profile. Its card, wires, and selector
   *  option leave the canvas cleanly: the stores-state update drops the
   *  selector option and the branchLocations seed, the editor's merge/
   *  rebuild drops the card + wires, and the selection moves to the next
   *  branch (or clears the canvas when none remain). */
  const handleDeleteBranch = async () => {
    if (!deleteTargetId) return;
    const id = deleteTargetId;
    const remaining = stores.filter((s) => s.id !== id);
    setDeleteBranchSaving(true);
    try {
      await deleteStore(id);
      setStores(remaining);
      setSelectedBranchId(remaining[0]?.id ?? null);
      // No branches left: nothing owns the graph — clear the instances so
      // the remounted editor lands on a clean, unowned canvas.
      if (remaining.length === 0) setWorkspaceInstances([]);
      setDeleteTargetId(null);
      setDeletingBranch(false);
    } catch (err) {
      addToast({
        message: `${l10n.getString('topology-branch-delete-error')}: ${plainErrorMessage(err)}`,
        type: 'error',
      });
    } finally {
      setDeleteBranchSaving(false);
    }
  };

  /** Persist a Branch Location rename (store profile) from the editor's
   *  card. Returns true on success so the card can close its inline form;
   *  false keeps the draft open for a retry. */
  const handleRenameBranch = useCallback(async (id: string, name: string): Promise<boolean> => {
    const store = stores.find((s) => s.id === id);
    if (!store) return false;
    const trimmed = name.trim();
    if (!trimmed || trimmed === store.name) return false;
    try {
      const updated = await updateStore({
        id: store.id,
        name: trimmed,
        address: store.address,
        tax_id: store.tax_id,
        currency: store.currency,
        timezone: store.timezone,
      });
      setStores((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
      return true;
    } catch (err) {
      addToast({
        message: `${l10n.getString('topology-branch-rename-error')}: ${plainErrorMessage(err)}`,
        type: 'error',
      });
      return false;
    }
  }, [stores, addToast, l10n]);

  /** Persist a workspace instance rename (the live row, not just the canvas
   *  label) from the editor's card. Same contract as handleRenameBranch. */
  const handleRenameWorkspace = useCallback(async (instanceId: string, name: string): Promise<boolean> => {
    const ws = workspaceInstances.find((w) => w.instance_id === instanceId);
    if (!ws || !sessionToken) return false;
    const trimmed = name.trim();
    if (!trimmed || trimmed === ws.name) return false;
    try {
      // The wrapper nulls omitted description/colour — pass the existing
      // values through so a rename never wipes the card subtitle/colour.
      await updateWorkspaceInstanceScoped(sessionToken, instanceId, {
        name: trimmed,
        description: ws.description,
        ...(ws.colour ? { colour: ws.colour } : {}),
      });
      setWorkspaceInstances((prev) => prev.map((w) => (w.instance_id === instanceId ? { ...w, name: trimmed } : w)));
      return true;
    } catch (err) {
      addToast({
        message: `${l10n.getString('topology-workspace-rename-error')}: ${plainErrorMessage(err)}`,
        type: 'error',
      });
      return false;
    }
  }, [workspaceInstances, sessionToken, addToast, l10n]);

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

      const semanticGraph = normalizeTopologyGraph(nodes, wires);
      // TopologyScreen is the real, strict boundary. Do not permit the
      // legacy primary/default fallback to survive into workspace mutation.
      const validationErrors = validateTopologyGraph(semanticGraph);
      if (validationErrors.length > 0) {
        const firstError = validationErrors[0]!;
        addToast({
          message: l10n.getString(firstError.messageId),
          type: 'error',
        });
        return idMap;
      }

      const wsNodes = nodes.filter((n) => n.type === 'workspace');
      const loadedById = new Map(workspaceInstances.map((w) => [w.instance_id, w]));
      const canvasIds = new Set(wsNodes.map((n) => n.id));

      // ── Semantic store_id resolution ────────────────────────────────
      // The validator has already established exactly one Branch Location
      // parent per workspace. Resolve from the stable node reference or the
      // canonical store_profile_id; never use names, primary, or default.
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
        if (n.storeProfileId !== undefined) payload.store_profile_id = n.storeProfileId;
        if (n.subtitle !== undefined) payload.subtitle = n.subtitle;
        if (n.tierRequirement !== undefined) payload.tier_requirement = n.tierRequirement;
        if (n.telemetryBadge !== undefined) payload.telemetry_badge = n.telemetryBadge;
        if (n.telemetryStatus !== undefined) payload.telemetry_status = n.telemetryStatus;
        if (n.metadata !== undefined || n.storeProfileId !== undefined) {
          // Keep the identity in metadata as a compatibility bridge for
          // backend versions that predate the explicit store_profile_id field.
          // The semantic compiler still reads the stable identity, never the
          // display name.
          const change = typeChanges.get(n.id);
          payload.metadata = {
            ...(n.metadata ?? {}),
            ...(n.storeProfileId !== undefined ? { storeProfileId: n.storeProfileId } : {}),
            ...(change ? { persisted: true } : {}),
          };
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
        if (w.fromPortId !== undefined) payload.from_port_id = w.fromPortId;
        if (w.toPortId !== undefined) payload.to_port_id = w.toPortId;
        if (w.relationshipType !== undefined) payload.relationship_type = w.relationshipType;
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
          setWorkspaceInstances((await listWorkspacesScoped(sessionToken)).filter(isTopologyInstance));
        } catch {
          /* non-fatal */
        }

        return idMap;
      } catch (err) {
        addToast({
          message: `${l10n.getString('topology-toast-save-error')}: ${plainErrorMessage(err)}`,
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
      aria-label={requiredLocalized(l10n, 'settings-nav-topology')}
    >
      {/* Keying by branch makes each branch's topology a fresh editor
          session: switching branches remounts the canvas and loads that
          branch's saved diagram instead of leaking the previous branch's
          nodes onto the new graph. */}
      <NodeTopologyEditor
        key={selectedBranchId ?? 'unassigned'}
        currentTier={licenseTier as 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise'}
        workspaceInstances={workspaceSeed}
        branchLocations={branchLocationSeed}
        onRenameBranch={handleRenameBranch}
        onRenameWorkspace={handleRenameWorkspace}
        allowLegacyApply={false}
        onSave={handleTopologySave}
        branchToolbar={(
          /* ── Branch (graph) selector toolbar, merged into the editor header ── */
          <div className="topology-branch-toolbar">
            <div className="topology-branch-selector">
              <label className="topology-branch-label" htmlFor="topology-branch-select">
                {l10n.getString('topology-branch-selector-label')}
              </label>
              <SettingsSelect
                id="topology-branch-select"
                value={selectedBranchId ?? ''}
                onChange={setSelectedBranchId}
                options={stores.map((s) => ({ value: s.id, label: s.name }))}
                ariaLabel={l10n.getString('topology-branch-selector-aria')}
                placeholder={l10n.getString('topology-branch-selector-label')}
                disabled={deletingBranch}
              />
            </div>
            {deletingBranch ? null : addingBranch ? (
              <div className="topology-branch-add-form">
                <input
                  className="topology-branch-add-input"
                  value={newBranchName}
                  onChange={(e) => setNewBranchName(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Enter') void handleAddBranch(); if (e.key === 'Escape') { setAddingBranch(false); setNewBranchName(''); } }}
                  aria-label={l10n.getString('topology-branch-add-name-placeholder')}
                  placeholder={l10n.getString('topology-branch-add-name-placeholder')}
                />
                <Button variant="primary" onClick={() => void handleAddBranch()} disabled={!newBranchName.trim()}>
                  {l10n.getString('topology-branch-add-confirm')}
                </Button>
                <Button variant="secondary" onClick={() => { setAddingBranch(false); setNewBranchName(''); }}>
                  {l10n.getString('topology-branch-add-cancel')}
                </Button>
              </div>
            ) : (
              <Button variant="secondary" onClick={() => { setDeleteTargetId(null); setDeletingBranch(false); setAddingBranch(true); }}>
                {l10n.getString('topology-branch-add')}
              </Button>
            )}
            {deletingBranch ? (
              <div className="topology-branch-delete-form">
                <span className="topology-branch-delete-msg">
                  {l10n.getString('topology-branch-delete-confirm', { name: deleteTargetName })}
                </span>
                <Button variant="danger" onClick={() => void handleDeleteBranch()} disabled={deleteBranchSaving}>
                  {l10n.getString('topology-branch-delete-confirm-btn')}
                </Button>
                <Button variant="secondary" onClick={() => { setDeleteTargetId(null); setDeletingBranch(false); }}>
                  {l10n.getString('topology-branch-add-cancel')}
                </Button>
              </div>
            ) : !addingBranch ? (
              <Button
                variant="secondary"
                onClick={() => { setAddingBranch(false); setDeleteTargetId(selectedBranchId); setDeletingBranch(true); }}
                disabled={!selectedBranchId}
              >
                {l10n.getString('topology-branch-delete')}
              </Button>
            ) : null}
          </div>
        )}
      />
    </div>
  );
}
