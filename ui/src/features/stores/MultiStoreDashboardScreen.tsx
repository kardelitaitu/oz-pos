import { useState, useEffect, useCallback, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { listStores, setPrimaryStore, deleteStore, type StoreProfile } from '@/api/stores';
import { listTerminals, type TerminalDto } from '@/api/terminals';
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
import { Button } from '@/components/Button';
import { Card } from '@/components/Card';
import { Skeleton } from '@/components/Skeleton';
import TerminalStatusPanel from './TerminalStatusPanel';
import NodeTopologyEditor, {
  type TopologyNodeData,
  type TopologyWireData,
  type WorkspaceInstanceSeed,
} from './NodeTopologyEditor';
import { checkLicenseStatus } from '@/api/license';
import './MultiStoreDashboardScreen.css';

const ONLINE_THRESHOLD_MS = 5 * 60 * 1000;

function isOnline(lastSeenAt: string | null): boolean {
  if (!lastSeenAt) return false;
  return Date.now() - new Date(lastSeenAt).getTime() < ONLINE_THRESHOLD_MS;
}

/** Multi-store dashboard — overview of all store profiles with terminal status, primary store designation, and node topology builder. */
export default function MultiStoreDashboardScreen() {
  const { l10n } = useLocalization();
  const { sessionToken } = useWorkspace();
  const { addToast } = useToast();
  const [stores, setStores] = useState<StoreProfile[]>([]);
  const [terminals, setTerminals] = useState<TerminalDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'cards' | 'topology'>('cards');
  const [licenseTier, setLicenseTier] = useState('standard');
  /** Real workspace instances loaded from the backend, used to seed the topology editor. */
  const [workspaceInstances, setWorkspaceInstances] = useState<WorkspaceDto[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [storeData, termData, licStatus] = await Promise.all([
        listStores(),
        listTerminals(),
        checkLicenseStatus(),
      ]);
      setStores(storeData);
      setTerminals(termData);
      setLicenseTier(licStatus.tier.toLowerCase());
      // Load workspace instances separately — requires a session token and
      // must not block the core dashboard if unavailable.
      if (sessionToken) {
        try {
          setWorkspaceInstances(await listWorkspacesScoped(sessionToken));
        } catch {
          setWorkspaceInstances([]);
        }
      }
    } catch {
      setError(l10n.getString('multi-store-error-load'));
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => { load(); }, [load]);

  const handleSetPrimary = useCallback(async (id: string) => {
    try {
      await setPrimaryStore(id);
      setStores((prev) =>
        prev.map((s) => ({ ...s, is_primary: s.id === id })),
      );
    } catch {
      // silently fail
    }
  }, []);

  const handleDelete = useCallback(async (id: string) => {
    setDeletingId(id);
    try {
      await deleteStore(id);
      setStores((prev) => prev.filter((s) => s.id !== id));
    } catch {
      // silently fail
    } finally {
      setDeletingId(null);
    }
  }, []);

  const activeTerminals = terminals.filter((t) => t.isActive).length;
  const onlineTerminals = terminals.filter((t) => isOnline(t.lastSeenAt)).length;

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

  const getTerminalCount = useCallback(
    (_storeId: string) => {
      return terminals.length;
    },
    [terminals],
  );

  return (
    <div className="multi-store-dashboard">
      <div className="multi-store-dashboard-header">
        <Localized id="multi-store-dashboard-title">
          <h1 className="multi-store-dashboard-title">Multi-Store Dashboard</h1>
        </Localized>

        <div className="multi-store-view-toggle">
          <Button
            variant={viewMode === 'cards' ? 'primary' : 'secondary'}
            onClick={() => setViewMode('cards')}
          >
            📋 Store Cards
          </Button>
          <Button
            variant={viewMode === 'topology' ? 'primary' : 'secondary'}
            onClick={() => setViewMode('topology')}
          >
            🗺️ Node Topology Builder
          </Button>
        </div>
      </div>

      {viewMode === 'topology' ? (
        <div className="multi-store-dashboard-topology-view" style={{ flex: 1, minHeight: '600px' }}>
          <NodeTopologyEditor
            currentTier={licenseTier as 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise'}
            workspaceInstances={workspaceSeed}
            onSave={handleTopologySave}
          />
        </div>
      ) : loading ? (
        <div className="multi-store-dashboard-loading-skeleton">
          <div className="multi-store-stat-grid">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="multi-store-stat-card">
                <Skeleton variant="block" width="3rem" height="2.5rem" />
                <Skeleton variant="text" width="6rem" />
              </div>
            ))}
          </div>
          <Skeleton variant="block" width="8rem" height="1.25rem" style={{ marginBottom: 'var(--space-4)' }} />
          <div className="multi-store-card-grid">
            {Array.from({ length: 3 }).map((_, i) => (
              <Card key={i} shadow="sm" padding="md" className="multi-store-card">
                <div className="multi-store-card-header">
                  <Skeleton variant="text" width="8rem" />
                </div>
                <div className="multi-store-card-body">
                  {Array.from({ length: 4 }).map((_, j) => (
                    <div key={j} className="multi-store-card-row">
                      <Skeleton variant="text" width="4rem" />
                      <Skeleton variant="text" width="5rem" />
                    </div>
                  ))}
                </div>
              </Card>
            ))}
          </div>
        </div>
      ) : error ? (
        <Card shadow="sm">
          <div className="multi-store-dashboard-error">
            <p>{error}</p>
            <Button variant="secondary" onClick={load}><Localized id="retry">Retry</Localized></Button>
          </div>
        </Card>
      ) : (
        <>
          {/* ── Stat cards ────────────────────────────────────── */}
          <div className="multi-store-stat-grid">
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{stores.length}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-total-stores">Total Stores</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{activeTerminals}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-active-terminals">Active Terminals</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{onlineTerminals}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-online-terminals">Online Terminals</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{terminals.length}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-total-terminals">Total Terminals</Localized></span>
            </div>
          </div>

          {/* ── Store cards ───────────────────────────────────── */}
          <section aria-label={l10n.getString('multi-store-section-stores-overview')}>
            <h2 className="multi-store-section-title"><Localized id="multi-store-section-stores">Stores</Localized></h2>
            <div className="multi-store-card-grid">
              {stores.map((store) => {
                const tc = getTerminalCount(store.id);
                return (
                  <Card
                    key={store.id}
                    shadow={store.is_primary ? 'md' : 'sm'}
                    padding="md"
                    className={`multi-store-card ${store.is_primary ? 'multi-store-card--primary' : ''}`}
                    header={
                      <div className="multi-store-card-header">
                        <span className="multi-store-card-name">{store.name}</span>
                        {store.is_primary && (
                          <span className="multi-store-card-badge"><Localized id="multi-store-badge-primary">Primary</Localized></span>
                        )}
                      </div>
                    }
                    footer={
                      <div className="multi-store-card-actions">
                        {!store.is_primary && (
                          <>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => handleSetPrimary(store.id)}
                              aria-label={l10n.getString('multi-store-btn-set-primary-label', { name: store.name })}
                            >
                              <Localized id="multi-store-btn-set-primary">Set as Primary</Localized>
                            </Button>
                            <Button
                              variant="danger"
                              size="sm"
                              loading={deletingId === store.id}
                              onClick={() => handleDelete(store.id)}
                              aria-label={l10n.getString('multi-store-btn-delete-label', { name: store.name })}
                            >
                              <Localized id="multi-store-btn-delete">Delete</Localized>
                            </Button>
                          </>
                        )}
                      </div>
                    }
                  >
                    <div className="multi-store-card-body">
                      {store.address && (
                        <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-address">Address</Localized></span>
                          <span className="multi-store-card-value">{store.address}</span>
                        </div>
                      )}
                      {store.tax_id && (
                        <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-tax-id">Tax ID</Localized></span>
                          <span className="multi-store-card-value">{store.tax_id}</span>
                        </div>
                      )}
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-currency">Currency</Localized></span>
                        <span className="multi-store-card-value">{store.currency}</span>
                      </div>
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-timezone">Timezone</Localized></span>
                        <span className="multi-store-card-value">{store.timezone}</span>
                      </div>
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-terminals">Terminals</Localized></span>
                        <span className="multi-store-card-value">{tc}</span>
                      </div>
                    </div>
                  </Card>
                );
              })}
            </div>
          </section>

          {/* ── Terminal Status Panel ─────────────────────────── */}
          <section aria-label={l10n.getString('multi-store-section-terminal-status')} className="multi-store-terminal-section">
            <TerminalStatusPanel refreshTrigger={0} />
          </section>
        </>
      )}
    </div>
  );
}
