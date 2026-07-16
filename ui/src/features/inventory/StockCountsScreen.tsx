import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listStockCounts,
  type StockCountDto,
} from '@/api/inventoryCounts';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './StockCountsScreen.css';

/** Stock counts list screen — displays all stock counts with status filters and links to create new counts or view details. */
export default function StockCountsScreen() {
  const [counts, setCounts] = useState<StockCountDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string>('all');

  const { l10n } = useLocalization();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listStockCounts();
      setCounts(data);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (filter === 'all') return counts;
    return counts.filter((c) => c.status === filter);
  }, [counts, filter]);

  const statusBadge = (status: string) => {
    const cls = `sc-badge sc-badge--${status}`;
    return <span className={cls}>{l10n.getString(`sc-status-${status}`) ?? status}</span>;
  };

  const typeLabel = (t: string) => l10n.getString(`sc-type-${t}`) ?? t;

  return (
    <div className="sc-screen">
      <div className="sc-header">
        <h1 className="sc-title">
          <Localized id="sc-title">
            <span>Stock Counts</span>
          </Localized>
        </h1>
        <Button variant="primary" onClick={() => { window.location.hash = '#stock-count-new'; }}>
          <Localized id="sc-new-count">
            <span>New Count</span>
          </Localized>
        </Button>
      </div>

      <div className="sc-filters">
        {['all', 'draft', 'in_progress', 'completed', 'cancelled'].map((f) => (
          {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- visible text inside Localized */}
          <button
            key={f}
            type="button"
            className={`sc-filter-btn ${filter === f ? 'sc-filter-btn--active' : ''}`}
            onClick={() => setFilter(f)}
            aria-pressed={filter === f}
          >
            <Localized id={`sc-filter-${f}`}>
              <span>{f.charAt(0).toUpperCase() + f.slice(1).replace('_', ' ')}</span>
            </Localized>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="sc-loading-skeleton" aria-hidden="true">
          <div className="sc-header">
            <Skeleton variant="block" width="10rem" height="1.75rem" />
            <Skeleton variant="block" width="7rem" height="2.25rem" />
          </div>
          <div className="sc-filters">
            {[0, 1, 2, 3, 4].map((i) => (
              <Skeleton key={i} variant="block" width="5rem" height="1.75rem" />
            ))}
          </div>
          <div className="sc-list">
            {[0, 1, 2, 3].map((i) => (
              <Card key={i} shadow="sm" className="sc-card">
                <div className="sc-card-row">
                  <Skeleton variant="text" width="5rem" height="1rem" />
                  <Skeleton variant="block" width="4rem" height="1.125rem" style={{ borderRadius: 'var(--radius-sm)' }} />
                </div>
                <div className="sc-card-meta">
                  <Skeleton variant="text" width="4rem" height="0.75rem" />
                  <Skeleton variant="text" width="6rem" height="0.75rem" />
                </div>
                <div className="sc-card-actions">
                  <Skeleton variant="text" width="3rem" height="0.875rem" />
                </div>
              </Card>
            ))}
          </div>
        </div>
      ) : filtered.length === 0 ? (
        <p className="sc-empty">
          <Localized id="sc-empty-list">
            <span>No stock counts found.</span>
          </Localized>
        </p>
      ) : (
        <div className="sc-list">
          {filtered.map((c) => (
            <Card key={c.id} shadow="sm" className="sc-card">
              <div className="sc-card-row">
                <span className="sc-card-number">{c.count_number}</span>
                {statusBadge(c.status)}
              </div>
              <div className="sc-card-meta">
                <span className="sc-card-type">{typeLabel(c.count_type)}</span>
                <span className="sc-card-date">{new Date(c.created_at).toLocaleDateString()}</span>
              </div>
              {c.notes && <p className="sc-card-notes">{c.notes}</p>}
              <div className="sc-card-actions">
                <button
                  type="button"
                  className="sc-card-action"
                  onClick={() => { window.location.hash = `#stock-count-${c.id}`; }}
                  aria-label={l10n.getString('sc-view-aria', { id: c.count_number })}
                >
                  <Localized id="sc-view">
                    <span>View</span>
                  </Localized>
                </button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
