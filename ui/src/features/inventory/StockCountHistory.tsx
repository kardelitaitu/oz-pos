import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listStockCounts,
  listStockAdjustments,
  getCountLines,
  type StockCountDto,
  type StockCountLineDto,
  type StockAdjustmentDto,
} from '@/api/inventoryCounts';
import { Card } from '@/components/Card';
import './StockCountHistory.css';

export default function StockCountHistory() {
  const [counts, setCounts] = useState<StockCountDto[]>([]);
  const [adjustments, setAdjustments] = useState<StockAdjustmentDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedCount, setSelectedCount] = useState<string | null>(null);
  const [selectedLines, setSelectedLines] = useState<StockCountLineDto[]>([]);

  const { l10n } = useLocalization();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [c, a] = await Promise.all([
        listStockCounts(),
        listStockAdjustments(),
      ]);
      setCounts(c.filter((cnt) => cnt.status === 'completed' || cnt.status === 'cancelled'));
      setAdjustments(a);
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleSelectCount = useCallback(async (id: string) => {
    setSelectedCount(id);
    try {
      const lines = await getCountLines(id);
      setSelectedLines(lines);
    } catch {
      setSelectedLines([]);
    }
  }, []);

  const countAdjustments = useMemo(() => {
    if (!selectedCount) return [];
    return adjustments.filter((a) => a.count_id === selectedCount);
  }, [adjustments, selectedCount]);

  if (loading) {
    return <p className="sc-hist-loading"><Localized id="sc-loading"><span>Loading…</span></Localized></p>;
  }

  return (
    <div className="sc-hist-screen">
      <div className="sc-hist-header">
        <h1 className="sc-title">
          <Localized id="sc-hist-title">
            <span>Stock Count History</span>
          </Localized>
        </h1>
      </div>

      {counts.length === 0 ? (
        <p className="sc-hist-empty">
          <Localized id="sc-hist-empty">
            <span>No completed or cancelled counts yet.</span>
          </Localized>
        </p>
      ) : (
        <div className="sc-hist-layout">
          <div className="sc-hist-list">
            {counts.map((c) => (
              <button
                key={c.id}
                type="button"
                className={`sc-hist-item ${selectedCount === c.id ? 'sc-hist-item--sel' : ''}`}
                onClick={() => handleSelectCount(c.id)}
              >
                <span className="sc-hist-item-number">{c.count_number}</span>
                <span className={`sc-badge sc-badge--${c.status}`}>
                  {l10n.getString(`sc-status-${c.status}`) ?? c.status}
                </span>
                <span className="sc-hist-item-date">{new Date(c.created_at).toLocaleDateString()}</span>
              </button>
            ))}
          </div>

          {selectedCount && (
            <div className="sc-hist-detail">
              <h2>
                <Localized id="sc-hist-reconciliation">
                  <span>Reconciliation Report</span>
                </Localized>
              </h2>

              {selectedLines.length > 0 && (
                <div className="sc-hist-lines">
                  <h3><Localized id="sc-hist-lines-title"><span>Count Lines</span></Localized></h3>
                  <div className="sc-hist-table">
                    <div className="sc-hist-tr sc-hist-th">
                      <span>SKU</span><span>Product</span><span>Expected</span><span>Counted</span><span>Diff</span>
                    </div>
                    {selectedLines.map((l) => (
                      <div key={l.id} className="sc-hist-tr">
                        <span>{l.sku}</span>
                        <span>{l.product_name}</span>
                        <span>{l.expected_qty}</span>
                        <span>{l.counted_qty ?? '—'}</span>
                        <span className={l.difference < 0 ? 'sc-diff-neg' : l.difference > 0 ? 'sc-diff-pos' : ''}>
                          {l.counted_qty != null ? (l.difference > 0 ? '+' : '') + l.difference : '—'}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {countAdjustments.length > 0 && (
                <div className="sc-hist-adjustments">
                  <h3><Localized id="sc-hist-adjust-title"><span>Adjustments Applied</span></Localized></h3>
                  <div className="sc-hist-table">
                    <div className="sc-hist-tr sc-hist-th">
                      <span>SKU</span><span>Product</span><span>Previous</span><span>New</span><span>Reason</span>
                    </div>
                    {countAdjustments.map((a) => (
                      <div key={a.id} className="sc-hist-tr">
                        <span>{a.sku}</span>
                        <span>{a.product_name}</span>
                        <span>{a.previous_qty}</span>
                        <span>{a.adjusted_qty}</span>
                        <span>{a.reason}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {selectedLines.length === 0 && countAdjustments.length === 0 && (
                <p><Localized id="sc-hist-no-data"><span>No data available for this count.</span></Localized></p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
