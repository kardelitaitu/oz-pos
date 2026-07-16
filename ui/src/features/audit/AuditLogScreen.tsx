import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { listAuditLog, type AuditEntryDto } from '@/api/audit';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './AuditLogScreen.css';

// ── Helpers ─────────────────────────────────────────────────────────

const ACTION_FLUENT_IDS: Record<string, string> = {
  'sale.void': 'audit-action-sale-void',
  'sale.complete': 'audit-action-sale-complete',
  'sale.refund': 'audit-action-sale-refund',
  'login': 'audit-action-login',
  'login.failed': 'audit-action-login-failed',
  'user.create': 'audit-action-user-create',
  'user.update': 'audit-action-user-update',
  'product.create': 'audit-action-product-create',
  'product.update': 'audit-action-product-update',
  'product.delete': 'audit-action-product-delete',
  'stock.adjust': 'audit-action-stock-adjust',
  'setting.change': 'audit-action-setting-change',
  'system.backup': 'audit-action-system-backup',
  'system.restore': 'audit-action-system-restore',
  'system.export': 'audit-action-system-export',
  'system.import': 'audit-action-system-import',
};

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

/** Audit log screen — view filtered action history with date range, action type, and outcome filters for compliance monitoring. */
export default function AuditLogScreen() {
  const { l10n } = useLocalization();
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
          (ACTION_FLUENT_IDS[e.action] ?? '').toLowerCase().includes(q) ||
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
        <Localized id="audit-log-title">
          <h1 className="audit-log-title"><span>Audit Log</span></h1>
        </Localized>
        <Button variant="secondary" onClick={() => load(0)} loading={loading}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
            <polyline points="1 4 1 10 7 10" />
            <path d="M3.51 15a9 9 0 102.13-9.36L1 10" />
          </svg>
          <Localized id="audit-log-refresh">
            <span>Refresh</span>
          </Localized>
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
            id="audit-log-search"
            name="audit-log-search"
            placeholder={l10n.getString('audit-log-search-placeholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label={l10n.getString('audit-log-search-label')}
          />
        </div>

        <div className="audit-log-outcome-filters" role="radiogroup" aria-label={l10n.getString('audit-log-filter-label')}>
          {(['all', 'success', 'failure'] as OutcomeFilter[]).map((outcome) => {
            const outcomeIds: Record<string, string> = {
              'all': 'audit-log-filter-all',
              'success': 'audit-log-filter-success',
              'failure': 'audit-log-filter-failure',
            };
            return (
              <Localized id={outcomeIds[outcome] ?? outcome} key={outcome}>
                <button
                  type="button"
                  className={`audit-log-chip ${outcomeFilter === outcome ? 'audit-log-chip--active' : ''}`}
                  onClick={() => setOutcomeFilter(outcome)}
                  role="radio"
                  aria-checked={outcomeFilter === outcome}
                >
                  <span>{outcome === 'all' ? 'All' : outcome === 'success' ? 'Success' : 'Failure'}</span>
                </button>
              </Localized>
            );
            })}
        </div>
      </div>

      {/* Content */}
      {loading && entries.length === 0 ? (
        <div className="audit-log-loading-skeleton">
          {/* Filters skeleton */}
          <div className="audit-log-skeleton-filters">
            <Skeleton variant="block" width="100%" height="2.25rem" />
            <Skeleton variant="block" width="10rem" height="2rem" />
          </div>
          {/* Table skeleton */}
          <div className="audit-log-table-wrap">
            <table className="audit-log-table" aria-hidden="true">
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
                {Array.from({ length: 6 }).map((_, i) => (
                  <tr key={i}>
                    <td><Skeleton variant="text" width="7rem" /></td>
                    <td><Skeleton variant="text" width="9rem" /></td>
                    <td><Skeleton variant="text" width="6rem" /></td>
                    <td><Skeleton variant="text" width="4rem" /></td>
                    <td><Skeleton variant="block" width="4rem" height="1.25rem" /></td>
                    <td><Skeleton variant="text" width="8rem" /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : error && entries.length === 0 ? (
        <Card shadow="sm">
          <div className="audit-log-error">
            <p>{error}</p>
            <Localized id="audit-log-retry">
              <Button variant="secondary" onClick={() => load(0)}><span>Retry</span></Button>
            </Localized>
          </div>
        </Card>
      ) : filteredEntries.length === 0 && !loading ? (
        <Card shadow="sm">
          <div className="audit-log-empty">
            {searchQuery || outcomeFilter !== 'all' ? (
              <Localized id="audit-log-empty-filtered">
                <span>No audit entries match the current filters.</span>
              </Localized>
            ) : (
              <Localized id="audit-log-empty-none">
                <span>No audit entries recorded yet. Entries appear when sales are completed, voided, or staff actions occur.</span>
              </Localized>
            )}
          </div>
        </Card>
      ) : (
        <div className="audit-log-table-wrap">
          <table className="audit-log-table" aria-label={l10n.getString('audit-log-table-label')}>
            <thead>
              <tr>
                <Localized id="audit-log-col-date"><th><span>Date</span></th></Localized>
                <Localized id="audit-log-col-action"><th><span>Action</span></th></Localized>
                <Localized id="audit-log-col-target"><th><span>Target</span></th></Localized>
                <Localized id="audit-log-col-user"><th><span>User ID</span></th></Localized>
                <Localized id="audit-log-col-outcome"><th><span>Outcome</span></th></Localized>
                <Localized id="audit-log-col-details"><th><span>Details</span></th></Localized>
              </tr>
            </thead>
            <tbody>
              {filteredEntries.map((entry) => (
                <tr key={entry.id}>
                  <td className="audit-log-cell-date">{formatDate(entry.created_at)}</td>
                  <td>
                    <Localized id={ACTION_FLUENT_IDS[entry.action] ?? entry.action}>
                      <span className="audit-log-action-label"><span>{entry.action}</span></span>
                    </Localized>
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
              <Localized id={loading ? 'shared-loading' : 'audit-log-load-more'}>
                <button
                  type="button"
                  className="audit-log-load-more"
                  onClick={handleLoadMore}
                  disabled={loading}
                >
                  <span>{loading ? 'Loading…' : 'Load More'}</span>
                </button>
              </Localized>
            </div>
          )}
          <div className="audit-log-footer">
            <span className="audit-log-count">
              <Localized id="audit-log-count" vars={{ count: filteredEntries.length }}>
                <span>{filteredEntries.length} entr{filteredEntries.length === 1 ? 'y' : 'ies'}</span>
              </Localized>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
