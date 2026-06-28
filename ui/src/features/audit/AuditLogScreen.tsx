import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { listAuditLog, type AuditEntryDto } from '@/api/pos';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './AuditLogScreen.css';

// ── Helpers ─────────────────────────────────────────────────────────

const ACTION_LABELS: Record<string, string> = {
  'sale.void': 'Void Sale',
  'sale.complete': 'Complete Sale',
  'sale.refund': 'Refund',
  'login': 'Staff Login',
  'login.failed': 'Login Failed',
  'user.create': 'Staff Created',
  'user.update': 'Staff Updated',
  'product.create': 'Product Created',
  'product.update': 'Product Updated',
  'product.delete': 'Product Deleted',
  'stock.adjust': 'Stock Adjusted',
  'setting.change': 'Setting Changed',
  'system.backup': 'Backup Created',
  'system.restore': 'Restore',
  'system.export': 'Data Export',
  'system.import': 'Data Import',
};

function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

function outcomeBadgeClass(outcome: string): string {
  switch (outcome) {
    case 'success': return 'audit-badge--success';
    case 'failure': return 'audit-badge--failure';
    default: return 'audit-badge--info';
  }
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

// ── Component ───────────────────────────────────────────────────────

type OutcomeFilter = 'all' | 'success' | 'failure';

export default function AuditLogScreen() {
  const [entries, setEntries] = useState<AuditEntryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const limit = 50;
  const cancelledRef = useRef(false);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [outcomeFilter, setOutcomeFilter] = useState<OutcomeFilter>('all');

  // ── Load ──────────────────────────────────────────────────────────

  const load = useCallback(async (newOffset: number, append: boolean = false) => {
    setLoading(true);
    setError(null);
    try {
      const data = await listAuditLog(limit, newOffset);
      if (!cancelledRef.current) {
        if (append) {
          setEntries((prev) => [...prev, ...data]);
        } else {
          setEntries(data);
        }
        setHasMore(data.length >= limit);
        setOffset(newOffset);
      }
    } catch (err) {
      if (!cancelledRef.current) {
        setError(err instanceof Error ? err.message : 'Failed to load audit log');
      }
    } finally {
      if (!cancelledRef.current) {
        setLoading(false);
      }
    }
  }, [limit]);

  useEffect(() => {
    cancelledRef.current = false;
    load(0);
    return () => { cancelledRef.current = true; };
  }, [load]);

  // ── Filtered entries ──────────────────────────────────────────────

  const filteredEntries = useMemo(() => {
    let items = entries;

    if (outcomeFilter !== 'all') {
      items = items.filter((e) => e.outcome === outcomeFilter);
    }

    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      items = items.filter(
        (e) =>
          e.action.toLowerCase().includes(q) ||
          actionLabel(e.action).toLowerCase().includes(q) ||
          (e.target_type ?? '').toLowerCase().includes(q) ||
          (e.target_id ?? '').toLowerCase().includes(q) ||
          e.user_id.toLowerCase().includes(q),
      );
    }

    return items;
  }, [entries, outcomeFilter, searchQuery]);

  const handleLoadMore = useCallback(() => {
    load(offset + limit, true);
  }, [load, offset, limit]);

  // ── Render ────────────────────────────────────────────────────────

  return (
    <div className="audit-log">
      <div className="audit-log-header">
        <h1 className="audit-log-title">Audit Log</h1>
        <Button variant="secondary" onClick={() => load(0)} loading={loading}>
          Refresh
        </Button>
      </div>

      {/* Filters */}
      <div className="audit-log-filters">
        <div className="audit-log-search-wrap">
          <svg className="audit-log-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            type="search"
            className="audit-log-search"
            placeholder="Search actions, targets, or users…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search audit log"
          />
        </div>

        <div className="audit-log-outcome-filters" role="radiogroup" aria-label="Filter by outcome">
          {(['all', 'success', 'failure'] as OutcomeFilter[]).map((outcome) => (
            <button
              key={outcome}
              type="button"
              className={`audit-log-chip ${outcomeFilter === outcome ? 'audit-log-chip--active' : ''}`}
              onClick={() => setOutcomeFilter(outcome)}
              role="radio"
              aria-checked={outcomeFilter === outcome}
            >
              {outcome === 'all' ? 'All' : outcome === 'success' ? 'Success' : 'Failure'}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      {loading && entries.length === 0 ? (
        <div className="audit-log-loading">Loading audit log…</div>
      ) : error && entries.length === 0 ? (
        <Card shadow="sm">
          <div className="audit-log-error">
            <p>{error}</p>
            <Button variant="secondary" onClick={() => load(0)}>Retry</Button>
          </div>
        </Card>
      ) : filteredEntries.length === 0 && !loading ? (
        <Card shadow="sm">
          <div className="audit-log-empty">
            {searchQuery || outcomeFilter !== 'all'
              ? 'No audit entries match the current filters.'
              : 'No audit entries recorded yet. Entries appear when sales are completed, voided, or staff actions occur.'}
          </div>
        </Card>
      ) : (
        <div className="audit-log-table-wrap">
          <table className="audit-log-table" aria-label="Audit log entries">
            <thead>
              <tr>
                <th>Date</th>
                <th>Action</th>
                <th>Target</th>
                <th>User ID</th>
                <th>Outcome</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              {filteredEntries.map((entry) => (
                <tr key={entry.id}>
                  <td className="audit-log-cell-date">{formatDate(entry.created_at)}</td>
                  <td>
                    <span className="audit-log-action-label">{actionLabel(entry.action)}</span>
                    <span className="audit-log-action-key">{entry.action}</span>
                  </td>
                  <td>
                    {entry.target_type ? (
                      <span className="audit-log-target">
                        <span className="audit-log-target-type">{entry.target_type}</span>
                        {entry.target_id && (
                          <span className="audit-log-target-id">{entry.target_id.slice(0, 8)}</span>
                        )}
                      </span>
                    ) : (
                      <span className="audit-log-target-none">&mdash;</span>
                    )}
                  </td>
                  <td className="audit-log-cell-mono">{entry.user_id ? entry.user_id.slice(0, 8) : 'system'}</td>
                  <td>
                    <span className={`audit-log-badge ${outcomeBadgeClass(entry.outcome)}`}>
                      {entry.outcome}
                    </span>
                  </td>
                  <td className="audit-log-cell-details">
                    {entry.details && entry.details !== '{}' ? (
                      <span className="audit-log-details-preview">
                        {entry.details.slice(0, 60)}{entry.details.length > 60 ? '…' : ''}
                      </span>
                    ) : (
                      <span className="audit-log-details-none">&mdash;</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {hasMore && (
            <div className="audit-log-load-more-wrap">
              <button
                type="button"
                className="audit-log-load-more"
                onClick={handleLoadMore}
                disabled={loading}
              >
                {loading ? 'Loading…' : 'Load More'}
              </button>
            </div>
          )}
          <div className="audit-log-footer">
            <span className="audit-log-count">
              {filteredEntries.length} entr{filteredEntries.length === 1 ? 'y' : 'ies'}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
