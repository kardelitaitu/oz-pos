import { useState, useCallback, useEffect, useRef } from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { Localized, useLocalization } from '@fluent/react';
import {
  listAuditLogScoped,
  getAuditReviewStatusScoped,
  markAuditReviewedScoped,
  type AuditEntryDto,
} from '@/api/audit';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import {
  ACTION_FLUENT_IDS,
  ACTION_FALLBACK_ID,
  OUTCOME_FLUENT_IDS,
  OUTCOME_FALLBACK_ID,
  CRITICAL_ACTIONS,
} from './auditCatalog';
import './AuditLogScreen.css';

// ── Helpers ─────────────────────────────────────────────────────────

/**
 * Resolve the active application locale from the Fluent localization
 * context (AUD-07). `ReactLocalization` exposes its bundles as an
 * iterable; the first bundle's locale is the negotiated language.
 */
function activeLocale(l10n: ReturnType<typeof useLocalization>['l10n']): string {
  for (const bundle of l10n.bundles) {
    const locales = bundle.locales;
    const primary = locales && locales.length > 0 ? locales[0] : undefined;
    if (primary) return primary;
  }
  return 'en';
}

/**
 * Format an ISO timestamp for display using the application locale
 * (AUD-07). Falls back to the raw ISO string when the date is invalid.
 */
function formatDate(iso: string, locale: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(locale, {
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

function outcomeBadgeClass(outcome: string): string {
  switch (outcome) {
    case 'success': return 'audit-badge--success';
    case 'failure': return 'audit-badge--failure';
    default: return 'audit-badge--info';
  }
}

/** High-water-mark cursor: newest entry a page boundary references. */
interface Cursor {
  created_at: string;
  id: string;
}

// ── Component ───────────────────────────────────────────────────────

type OutcomeFilter = 'all' | 'success' | 'failure';

/** Audit log screen — view filtered action history with date range, action type, and outcome filters for compliance monitoring. */
export default function AuditLogScreen() {
  const { l10n } = useLocalization();
  const locale = activeLocale(l10n);
  const { isManager } = useAuth();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';

  const [entries, setEntries] = useState<AuditEntryDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const limit = 50;
  const loadSeqRef = useRef(0);
  const cursorRef = useRef<Cursor | null>(null);

  // Server-side review checkpoint (AUD-04): review time for display and a
  // server-computed unreviewed count over the FULL table (AUD-02), not just
  // the loaded page.
  const [reviewedAt, setReviewedAt] = useState<string | null>(null);
  const [unreviewedCount, setUnreviewedCount] = useState(0);
  const [markingReviewed, setMarkingReviewed] = useState(false);

  // Filters (server-side). Search is debounced to avoid an IPC per keystroke.
  const [searchInput, setSearchInput] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [outcomeFilter, setOutcomeFilter] = useState<OutcomeFilter>('all');

  // ── Load (server-filtered + keyset paginated, AUD-01/02/03) ─────

  const load = useCallback(
    async (opts: { reset?: boolean } = {}) => {
      const { reset = false } = opts;
      if (!sessionToken) return;
      const seq = ++loadSeqRef.current;
      setLoading(true);
      setError(null);
      try {
        const page = await listAuditLogScoped(sessionToken, {
          limit,
          ...(outcomeFilter !== 'all' ? { outcome: outcomeFilter } : {}),
          ...(searchQuery ? { query: searchQuery } : {}),
          ...(!reset && cursorRef.current
            ? {
                beforeCreatedAt: cursorRef.current.created_at,
                beforeId: cursorRef.current.id,
              }
            : {}),
        });
        if (seq !== loadSeqRef.current) return;
        if (reset) {
          setEntries(page.items);
        } else {
          // Deduplicate by entry id (AUD-03 defense in depth).
          setEntries((prev) => {
            const seen = new Set(prev.map((e) => e.id));
            return [...prev, ...page.items.filter((e) => !seen.has(e.id))];
          });
        }
        setTotal(page.total);
        setHasMore(page.has_more);
        const last = page.items[page.items.length - 1];
        if (last) {
          cursorRef.current = { created_at: last.created_at, id: last.id };
        }
      } catch (err) {
        if (seq !== loadSeqRef.current) return;
        setError(err instanceof Error ? err.message : requiredLocalized(l10n, 'audit-log-error-load'));
      } finally {
        if (seq === loadSeqRef.current) setLoading(false);
      }
    },
    [sessionToken, limit, outcomeFilter, searchQuery, l10n],
  );

  // Debounce the free-text search, then reload from the first page.
  useEffect(() => {
    const t = window.setTimeout(() => setSearchQuery(searchInput.trim()), 250);
    return () => window.clearTimeout(t);
  }, [searchInput]);

  // Initial load + reload whenever a server-side filter changes.
  useEffect(() => {
    cursorRef.current = null;
    void load({ reset: true });
  }, [load]);

  // ── Review checkpoint (AUD-04) ───────────────────────────────────

  const loadReviewStatus = useCallback(async () => {
    if (!sessionToken) return;
    try {
      const status = await getAuditReviewStatusScoped(sessionToken);
      setReviewedAt(status.checkpoint?.reviewed_at ?? null);
      setUnreviewedCount(status.unreviewed_count);
    } catch {
      // Server is authoritative; a transient failure keeps the previous state.
    }
  }, [sessionToken]);

  useEffect(() => {
    void loadReviewStatus();
  }, [loadReviewStatus]);

  const handleMarkReviewed = useCallback(async () => {
    if (!sessionToken) return;
    setMarkingReviewed(true);
    try {
      // High-water mark = the newest entry the reviewer has seen (page 1 is
      // newest-first, so entries[0] is the globally newest row).
      const newest = entries[0];
      await markAuditReviewedScoped(sessionToken, {
        reviewedThroughCreatedAt: newest?.created_at ?? new Date().toISOString(),
        reviewedThroughId: newest?.id ?? '',
      });
      await loadReviewStatus();
      // The audit.review event just landed — refresh the first page.
      await load({ reset: true });
    } catch {
      await loadReviewStatus();
    } finally {
      setMarkingReviewed(false);
    }
  }, [sessionToken, entries, load, loadReviewStatus]);

  const handleLoadMore = useCallback(() => {
    void load({ reset: false });
  }, [load]);

  // ── Render ────────────────────────────────────────────────────────

  // With server-side filtering the page only ever holds matching rows, so the
  // empty-filtered state is distinguishable by whether a filter is active.
  const filtersActive = outcomeFilter !== 'all' || searchQuery.length > 0;

  return (
    <div className="audit-log" data-testid="audit-log-table">
      <div className="audit-log-header">
        <div className="audit-log-header-left">
          <Localized id="audit-log-title">
            <h1 className="audit-log-title"><span>Audit Log</span></h1>
          </Localized>
          {unreviewedCount > 0 && (
            <span className="audit-log-unreviewed-badge" title={l10n.getString('audit-log-unreviewed-title', { count: String(unreviewedCount) }, `${unreviewedCount} unreviewed events since last review`)}>
              {unreviewedCount} new
            </span>
          )}
          {reviewedAt && (
            <span className="audit-log-reviewed-at">
              <time dateTime={reviewedAt} title={reviewedAt}>
                <Localized id="audit-log-reviewed-at" vars={{ date: formatDate(reviewedAt, locale) }}><span>Reviewed: {formatDate(reviewedAt, locale)}</span></Localized>
              </time>
            </span>
          )}
        </div>
        <div className="audit-log-header-right">
          {isManager && unreviewedCount > 0 && (
            <Button variant="secondary" onClick={() => void handleMarkReviewed()} loading={markingReviewed} size="sm">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true" style={{ marginRight: 4 }}>
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <Localized id="audit-log-mark-reviewed"><span>Mark Reviewed</span></Localized>
            </Button>
          )}
          <Button variant="secondary" onClick={() => void load({ reset: true })} loading={loading}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 102.13-9.36L1 10" />
            </svg>
            <Localized id="audit-log-refresh">
              <span>Refresh</span>
            </Localized>
          </Button>
        </div>
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
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
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
          <div className="audit-log-skeleton-filters">
            <Skeleton variant="block" width="100%" height="2.25rem" />
            <Skeleton variant="block" width="10rem" height="2rem" />
          </div>
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
              <tbody>{Array.from({ length: 6 }).map((_, i) => (
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
              <Button variant="secondary" onClick={() => void load({ reset: true })}><span>Retry</span></Button>
            </Localized>
          </div>
        </Card>
      ) : entries.length === 0 && !loading ? (
        <Card shadow="sm">
          <div className="audit-log-empty">
            {filtersActive ? (
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
        <div className="audit-log-table-wrap" aria-live="polite" aria-relevant="additions text">
          <table className="audit-log-table" aria-label={l10n.getString('audit-log-table-label')}>
            <thead>
              <tr>
                <th style={{ width: '4px', padding: 0 }} />
                <Localized id="audit-log-col-date"><th><span>Date</span></th></Localized>
                <Localized id="audit-log-col-action"><th><span>Action</span></th></Localized>
                <Localized id="audit-log-col-target"><th><span>Target</span></th></Localized>
                <Localized id="audit-log-col-user"><th><span>User ID</span></th></Localized>
                <Localized id="audit-log-col-outcome"><th><span>Outcome</span></th></Localized>
                <Localized id="audit-log-col-details"><th><span>Details</span></th></Localized>
              </tr>
            </thead>
            <tbody>{entries.map((entry) => {
                const isCritical = CRITICAL_ACTIONS.has(entry.action) || entry.outcome === 'failure';
                return (
                  <tr key={entry.id} className={isCritical ? 'audit-log-row--critical' : ''}>
                    <td className="audit-log-critical-indicator" style={{ width: '4px', padding: 0 }}>
                      {isCritical && <div className="audit-log-critical-bar" />}
                    </td>
                    <td className="audit-log-cell-date">
                      <time dateTime={entry.created_at} title={entry.created_at}>{formatDate(entry.created_at, locale)}</time>
                    </td>
                    <td>
                      <Localized id={ACTION_FLUENT_IDS[entry.action] ?? ACTION_FALLBACK_ID}>
                        <span className="audit-log-action-label"><span>{entry.action}</span></span>
                      </Localized>
                      <span className="audit-log-action-key" title={entry.action}>{entry.action}</span>
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
                    <td className="audit-log-cell-mono">{entry.user_id ? entry.user_id.slice(0, 8) : requiredLocalized(l10n, 'audit-log-user-system')}</td>
                    <td>
                      <span className={`audit-log-badge ${outcomeBadgeClass(entry.outcome)}`} title={entry.outcome}>
                        <Localized id={OUTCOME_FLUENT_IDS[entry.outcome] ?? OUTCOME_FALLBACK_ID}>
                          <span>{entry.outcome}</span>
                        </Localized>
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
                );
              })}
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
              <Localized id="audit-log-count-of" vars={{ shown: entries.length, total }}>
                <span>{entries.length} of {total} entries</span>
              </Localized>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
