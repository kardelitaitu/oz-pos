import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { usePullToRefresh } from '@/hooks/usePullToRefresh';
import {
  listAllOffline,
  pendingOfflineCount,
  retryOfflineSync,
  deleteOfflineItem,
  getOfflineQueueStatusSummary,
  type OfflineQueueItemDto,
  type SyncResult,
} from '@/api/offline';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './OfflineQueueScreen.css';

// ── Helpers ─────────────────────────────────────────────────────────

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function statusClass(status: string): string {
  switch (status) {
    case 'pending':
      return 'status-pending';
    case 'synced':
      return 'status-synced';
    case 'failed':
      return 'status-failed';
    default:
      return '';
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'pending':
      return 'offline-queue-status-pending';
    case 'synced':
      return 'offline-queue-status-synced';
    case 'failed':
      return 'offline-queue-status-failed';
    default:
      return 'offline-queue-status-pending';
  }
}

// ── Component ───────────────────────────────────────────────────────

/** Offline queue screen — view pending, synced, and failed offline operations with retry and delete capabilities. */
export default function OfflineQueueScreen() {
  const { l10n } = useLocalization();
  const [items, setItems] = useState<OfflineQueueItemDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingCount, setPendingCount] = useState<number>(0);
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncResult | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [conflictCount, setConflictCount] = useState<number>(0);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [data, count, summary] = await Promise.all([
        listAllOffline(),
        pendingOfflineCount(),
        getOfflineQueueStatusSummary().catch(() => null),
      ]);
      setItems(data);
      setPendingCount(count);
      if (summary) setConflictCount(summary.conflictCount);
    } catch {
      setError(l10n.getString('offline-queue-error'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

  // P7-3: Pull-to-refresh gesture (defined after load so it's hoist-safe)
  const { containerProps: pullRefreshProps, state: pullState, pullDistance } = usePullToRefresh({
    onRefresh: load,
  });

  // ── Load data on mount ─────────────────────────────────────────

  useEffect(() => { load(); }, [load]);

  // Poll pending count and conflict count every 10 seconds (P1-3).
  useEffect(() => {
    pollRef.current = setInterval(async () => {
      try {
        const [count, summary] = await Promise.all([
          pendingOfflineCount(),
          getOfflineQueueStatusSummary().catch(() => null),
        ]);
        setPendingCount(count);
        if (summary) setConflictCount(summary.conflictCount);
      } catch {
        // Silently ignore poll errors.
      }
    }, 10_000);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, []);

  // ── Sync all ──────────────────────────────────────────────────

  const handleSyncAll = useCallback(async () => {
    setSyncing(true);
    setSyncResult(null);
    try {
      const result = await retryOfflineSync();
      setSyncResult(result);
      await load();
    } catch {
      setError(l10n.getString('offline-queue-sync-error'));
    } finally {
      setSyncing(false);
    }
  }, [load, l10n]);

  // ── Delete item ────────────────────────────────────────────────

  const handleDelete = useCallback(async (id: string) => {
    setDeleteError(null);
    try {
      await deleteOfflineItem(id);
      await load();
    } catch {
      setDeleteError(l10n.getString('offline-queue-delete-error'));
    }
  }, [load, l10n]);

  // ── Render ─────────────────────────────────────────────────────

  return (
    <div className="offline-queue-screen">
      <div className="offline-queue-header">
        <div className="offline-queue-title-row">
          <Localized id="offline-queue-title">
            <h1 className="offline-queue-title">Offline Queue</h1>
          </Localized>
          {pendingCount > 0 && (
            <Localized id="offline-queue-pending-count" vars={{ count: String(pendingCount) }}>
              <span className="offline-queue-badge" aria-label={`${pendingCount} pending`} aria-live="polite">
                {pendingCount} pending
              </span>
            </Localized>
          )}
        </div>
        <Button
          variant="primary"
          loading={syncing}
          disabled={pendingCount === 0 || syncing}
          onClick={handleSyncAll}
          aria-label={l10n.getString('offline-queue-sync-all-label')}
        >
          <Localized id={syncing ? 'offline-queue-syncing' : 'offline-queue-sync-all'}>
            <span>{syncing ? 'Syncing…' : 'Sync All'}</span>
          </Localized>
        </Button>
      </div>

      {conflictCount > 0 && (
        <div className="offline-queue-sync-result" role="alert" style={{ borderColor: 'var(--color-warning-border, #ffc107)' }}>
          <Localized id="offline-queue-conflict-count" vars={{ count: String(conflictCount) }}>
            <span>{conflictCount} item(s) resolved via sync conflict.</span>
          </Localized>
        </div>
      )}

      {syncResult && (
        <div className="offline-queue-sync-result" role="status">
          <Localized
            id="offline-queue-sync-success"
            vars={{ synced: String(syncResult.synced), failed: String(syncResult.failed) }}
          >
            <span>
              Synced {syncResult.synced} items, {syncResult.failed} failed.
            </span>
          </Localized>
        </div>
      )}

      {deleteError && (
        <div className="offline-queue-error" role="alert">
          <span>{deleteError}</span>
        </div>
      )}

      {/* P7-3: Pull-to-refresh indicator */}
      {pullState !== 'idle' && (
        <div
          className="offline-queue-pull-indicator"
          style={{
            transform: `translateY(${pullDistance}px)`,
            opacity: Math.min(1, pullDistance / 60),
          }}
        >
          {pullState === 'pulling' && (
            <span>{l10n.getString('offline-queue-pull-to-refresh') || 'Pull to refresh'}</span>
          )}
          {pullState === 'ready' && (
            <span>{l10n.getString('offline-queue-release-to-refresh') || 'Release to refresh'}</span>
          )}
          {pullState === 'loading' && <span className="offline-queue-refresh-spinner" />}
        </div>
      )}

      {loading ? (
        <div className="offline-queue-loading-skeleton" {...pullRefreshProps}>
          {/* Header skeleton */}
          <div className="offline-queue-skeleton-header">
            <Skeleton variant="block" width="12rem" height="1.75rem" />
            <Skeleton variant="block" width="7rem" height="2.25rem" />
          </div>
          {/* Table skeleton */}
          <div className="offline-queue-table-wrap">
            <table className="offline-queue-table" aria-hidden="true">
              <thead>
                <tr>
                  <th>Action</th>
                  <th>Status</th>
                  <th>Retries</th>
                  <th>Last Error</th>
                  <th>Created</th>
                  <th>Synced At</th>
                  <th aria-label="Actions"> </th>
                </tr>
              </thead>
              <tbody>
                {Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i}>
                    <td><Skeleton variant="text" width="6rem" /></td>
                    <td><Skeleton variant="block" width="4.5rem" height="1.25rem" /></td>
                    <td style={{ textAlign: 'center' }}><Skeleton variant="text" width="2rem" /></td>
                    <td><Skeleton variant="text" width="8rem" /></td>
                    <td><Skeleton variant="text" width="7rem" /></td>
                    <td><Skeleton variant="text" width="7rem" /></td>
                    <td><Skeleton variant="block" width="3rem" height="1.5rem" /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : error ? (
        <Card shadow="sm">
          <div className="offline-queue-empty">
            <Localized id="offline-queue-error">
              <p>Failed to load queue. Please try again.</p>
            </Localized>
            <Localized id="offline-queue-retry">
              <Button variant="secondary" onClick={load}>Retry</Button>
            </Localized>
          </div>
        </Card>
      ) : items.length === 0 ? (
        <Card shadow="sm">
          <div className="offline-queue-empty" {...pullRefreshProps}>
            <Localized id="offline-queue-empty">
              <p>All transactions synced. No pending items.</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="offline-queue-table-wrap" {...pullRefreshProps}>
          {/* P7-3: Pull-to-refresh indicator */}
          {pullState !== 'idle' && (
            <div
              className="offline-queue-pull-indicator"
              style={{
                transform: `translateY(${pullDistance}px)`,
                opacity: Math.min(1, pullDistance / 60),
              }}
            >
              {pullState === 'pulling' && (
                <span>{l10n.getString('offline-queue-pull-to-refresh') || 'Pull down to refresh'}</span>
              )}
              {pullState === 'ready' && (
                <span>{l10n.getString('offline-queue-release-to-refresh') || 'Release to refresh'}</span>
              )}
              {pullState === 'loading' && <span className="offline-queue-refresh-spinner" />}
            </div>
          )}
          <table className="offline-queue-table" aria-label="Offline queue items">
            <thead>
              <tr>
                <Localized id="offline-queue-action"><th>Action</th></Localized>
                <Localized id="offline-queue-status"><th>Status</th></Localized>
                <Localized id="offline-queue-retries"><th>Retries</th></Localized>
                <Localized id="offline-queue-last-error"><th>Last Error</th></Localized>
                <Localized id="offline-queue-created"><th>Created</th></Localized>
                <Localized id="offline-queue-synced-at"><th>Synced At</th></Localized>
                <th aria-label={l10n.getString('offline-queue-table-actions')}> </th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <tr key={item.id}>
                  <td className="offline-queue-cell-action">{item.action}</td>
                  {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label provided via l10n.getString */}
                  <td>
                    <Localized id={statusLabel(item.status)}>
                      <span className={`offline-queue-status-badge ${statusClass(item.status)}`}>
                        {item.status.charAt(0).toUpperCase() + item.status.slice(1)}
                      </span>
                    </Localized>
                  </td>
                  <td className="offline-queue-cell-retries">{item.retryCount}</td>
                  <td className="offline-queue-cell-error">
                    {item.lastError ? (
                      <span title={item.lastError}>{item.lastError}</span>
                    ) : (
                      <Localized id="offline-queue-none">
                        <span className="offline-queue-cell-none">—</span>
                      </Localized>
                    )}
                  </td>
                  <td className="offline-queue-cell-created">{formatDate(item.createdAt)}</td>
                  <td className="offline-queue-cell-synced">
                    {item.syncedAt ? formatDate(item.syncedAt) : (
                      <Localized id="offline-queue-none">
                        <span className="offline-queue-cell-none">—</span>
                      </Localized>
                    )}
                  </td>
                  <td>
                    <div className="offline-queue-cell-actions">
                    <button
                      type="button"
                      className="offline-queue-action-btn offline-queue-action-btn--danger"
                      onClick={() => handleDelete(item.id)}
                      aria-label={l10n.getString('offline-queue-delete')}
                    >
                      <Localized id="offline-queue-delete"><span>Delete</span></Localized>
                    </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
