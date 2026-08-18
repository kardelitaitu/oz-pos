import { useEffect, useState, useCallback, useRef } from 'react';
import { requiredLocalized, LoadingStatus } from '@/frontend/shared';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listKdsOrdersScoped, type KdsOrder, type KdsStatus } from '@/api/kds';

const STATUS_ORDER: { key: KdsStatus; label: string }[] = [
  { key: 'served', label: 'kds-served' },
  { key: 'cancelled', label: 'kds-cancelled' },
];

/** Displays served and cancelled orders (recall/history view for KDS). */
export function KdsHistoryPanel() {
  const { l10n } = useLocalization();
  const numLocale = [...l10n.bundles][0]?.locales[0] ?? 'en-US';
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<KdsStatus>('served');
  // LOAD-07: request-generation guard — a slow response from an earlier
  // filter/session must never overwrite a newer one.
  const loadSeqRef = useRef(0);
  const hasLoadedOnceRef = useRef(false);
  // Whether the last-known list is non-empty. Kept in a ref so the
  // load/refresh decision does not depend on `orders.length` — that would
  // re-create fetchHistory on every successful load and re-fire the effect,
  // issuing a duplicate request on mount, filter change, and retry.
  const hasOrdersRef = useRef(false);

  const fetchHistory = useCallback(async () => {
    const seq = ++loadSeqRef.current;
    setError(null);
    // LOAD-04: only the first load (or a load with nothing on screen)
    // shows the full loading state — a filter change over existing orders
    // preserves the last-known list and marks the region refreshing.
    if (!hasLoadedOnceRef.current || !hasOrdersRef.current) {
      setLoading(true);
      setRefreshing(false);
    } else {
      setRefreshing(true);
    }
    try {
      const result = await listKdsOrdersScoped(sessionToken, statusFilter);
      if (seq !== loadSeqRef.current) return;
      setOrders(result);
      hasLoadedOnceRef.current = true;
      hasOrdersRef.current = result.length > 0;
    } catch {
      // LOAD-08: a raw String(e) leaks implementation details; surface
      // the localized failure and let the user Retry.
      if (seq !== loadSeqRef.current) return;
      setError(requiredLocalized(l10n, 'kds-history-error'));
    } finally {
      if (seq === loadSeqRef.current) {
        setLoading(false);
        setRefreshing(false);
      }
    }
  }, [sessionToken, statusFilter, l10n]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  return (
    <div className="kds-history">
      {/* Filter tabs */}
      <div className="kds-history-tabs" role="tablist" aria-label={requiredLocalized(l10n, 'kds-history-filter-aria')}>
        {STATUS_ORDER.map(({ key, label }) => (
          <button
            key={key}
            className={`kds-history-tab${statusFilter === key ? ' kds-history-tab--active' : ''}`}
            onClick={() => setStatusFilter(key)}
            role="tab"
            aria-selected={statusFilter === key}
          >
            <Localized id={label}>{key}</Localized>
          </button>
        ))}
      </div>

      {/* Error — LOAD-08: localized message + Retry, never raw String(e) */}
      {error && (
        <div className="kds-history-error" role="alert">
          <span>{error}</span>
          <button
            type="button"
            className="kds-history-retry"
            onClick={() => fetchHistory()}
          >
            {requiredLocalized(l10n, 'retry')}
          </button>
        </div>
      )}

      {/* Loading — LOAD-05: status wrapper + localized label */}
      {loading && (
        <LoadingStatus
          className="kds-history-loading"
          label={requiredLocalized(l10n, 'kds-history-loading')}
        >
          <span className="kds-refresh-spinner" />
        </LoadingStatus>
      )}

      {/* LOAD-04: filter change over existing orders — keep the list
          visible and announce the in-place refresh. */}
      {refreshing && (
        <div className="kds-history-refreshing" role="status" aria-live="polite">
          <span className="kds-refresh-spinner" />
          <Localized id="kds-history-loading">Loading history...</Localized>
        </div>
      )}

      {/* Empty state */}
      {!loading && !error && orders.length === 0 && (
        <div className="kds-history-empty">
          <Localized id="kds-history-empty">No completed orders yet</Localized>
        </div>
      )}

      {/* Order list */}
      {!loading && orders.length > 0 && (
        <div className="kds-history-list">
          {orders.map((order) => (
            <div key={order.id} className="kds-history-card">
              <div className="kds-history-card-header">
                <span className="kds-history-card-number">#{order.display_number}</span>
                {order.table_number && (
                  <span className="kds-ticket-table">{order.table_number}</span>
                )}
                <span className={`kds-history-card-status kds-history-card-status--${order.status}`}>
                  <Localized id={`kds-${order.status}`}>{order.status}</Localized>
                </span>
              </div>
              <span className="kds-ticket-items">{order.items_summary}</span>
              <div className="kds-history-card-meta">
                <span className="kds-history-card-time">
                  <Localized id="kds-history-received">Received</Localized>: {new Date(order.received_at).toLocaleString(numLocale)}
                </span>
                {order.served_at && (
                  <span className="kds-history-card-time">
                    <Localized id="kds-history-served">Served</Localized>: {new Date(order.served_at).toLocaleString(numLocale)}
                  </span>
                )}
                {order.notes && <span className="kds-ticket-notes">{order.notes}</span>}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
