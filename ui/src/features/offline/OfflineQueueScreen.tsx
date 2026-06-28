import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listAllOffline,
  pendingOfflineCount,
  retryOfflineSync,
  deleteOfflineItem,
  type OfflineQueueItemDto,
  type SyncResult,
} from '@/api/pos';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
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

export default function OfflineQueueScreen() {
  const { l10n } = useLocalization();
  const [items, setItems] = useState<OfflineQueueItemDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingCount, setPendingCount] = useState<number>(0);
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncResult | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // ── Load data ──────────────────────────────────────────────────

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [data, count] = await Promise.all([
        listAllOffline(),
        pendingOfflineCount(),
      ]);
      setItems(data);
      setPendingCount(count);
    } catch {
      setError('Failed to load queue');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // Poll pending count every 10 seconds.
  useEffect(() => {
    pollRef.current = setInterval(async () => {
      try {
        const count = await pendingOfflineCount();
        setPendingCount(count);
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
      setError('Sync failed');
    } finally {
      setSyncing(false);
    }
  }, [load]);

  // ── Delete item ────────────────────────────────────────────────

  const handleDelete = useCallback(async (id: string) => {
    setDeleteError(null);
    try {
      await deleteOfflineItem(id);
      await load();
    } catch {
      setDeleteError('Failed to delete item');
    }
  }, [load]);

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
              <span className="offline-queue-badge" aria-label={`${pendingCount} pending`}>
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
          aria-label="Sync all pending offline items"
        >
          <Localized id={syncing ? 'offline-queue-syncing' : 'offline-queue-sync-all'}>
            <span>{syncing ? 'Syncing…' : 'Sync All'}</span>
          </Localized>
        </Button>
      </div>

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

      {loading ? (
        <Localized id="offline-queue-loading">
          <p className="offline-queue-loading">Loading queue…</p>
        </Localized>
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
          <div className="offline-queue-empty">
            <Localized id="offline-queue-empty">
              <p>All transactions synced. No pending items.</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="offline-queue-table-wrap">
          <table className="offline-queue-table" aria-label="Offline queue items">
            <thead>
              <tr>
                <Localized id="offline-queue-action"><th>Action</th></Localized>
                <Localized id="offline-queue-status"><th>Status</th></Localized>
                <Localized id="offline-queue-retries"><th>Retries</th></Localized>
                <Localized id="offline-queue-last-error"><th>Last Error</th></Localized>
                <Localized id="offline-queue-created"><th>Created</th></Localized>
                <Localized id="offline-queue-synced-at"><th>Synced At</th></Localized>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <tr key={item.id}>
                  <td className="offline-queue-cell-action">{item.action}</td>
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
                  <td className="offline-queue-cell-actions">
                    <button
                      type="button"
                      className="offline-queue-action-btn offline-queue-action-btn--danger"
                      onClick={() => handleDelete(item.id)}
                      aria-label={l10n.getString('offline-queue-delete')}
                    >
                      <Localized id="offline-queue-delete"><span>Delete</span></Localized>
                    </button>
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
