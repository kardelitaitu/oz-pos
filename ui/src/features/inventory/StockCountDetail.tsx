import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  getStockCount,
  addCountLine,
  updateCountLine,
  removeCountLine,
  completeStockCount,
  updateStockCountStatus,
  type StockCountDto,
  type StockCountLineDto,
} from '@/api/inventoryCounts';
import { type ProductDto } from '@/api/products';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './StockCountDetail.css';

interface Props {
  countId: string;
  onBack: () => void;
}

export default function StockCountDetail({ countId, onBack }: Props) {
  const [count, setCount] = useState<StockCountDto | null>(null);
  const [lines, setLines] = useState<StockCountLineDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  // Product search
  const [products, setProducts] = useState<ProductDto[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSku, setSelectedSku] = useState('');
  const [selectedName, setSelectedName] = useState('');
  const [expectedQty, setExpectedQty] = useState('');

  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const { l10n } = useLocalization();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const c = await getStockCount(countId);
      setCount(c);
      if (c) {
        const { getCountLines } = await import('@/api/inventoryCounts');
        setLines(await getCountLines(countId));
      }
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, [countId]);

  useEffect(() => {
    load();
    listProducts().then(setProducts).catch(() => {});
  }, [load]);

  const isEditable = count?.status === 'draft' || count?.status === 'in_progress';

  const filteredProducts = useMemo(() => {
    if (!searchQuery.trim()) return [];
    const q = searchQuery.trim().toLowerCase();
    return products.filter(
      (p) =>
        p.sku.toLowerCase().includes(q) ||
        p.name.toLowerCase().includes(q) ||
        (p.barcode ?? '').includes(q),
    );
  }, [products, searchQuery]);

  const handleAddLine = useCallback(async () => {
    if (!selectedSku || !expectedQty) return;
    setSaving(true);
    setError(null);
    try {
      await addCountLine({
        countId,
        sku: selectedSku,
        productName: selectedName,
        expectedQty: parseInt(expectedQty, 10),
      });
      setSelectedSku('');
      setSelectedName('');
      setExpectedQty('');
      setSearchQuery('');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add line');
    } finally {
      setSaving(false);
    }
  }, [countId, selectedSku, selectedName, expectedQty, load]);

  const handleRecordCount = useCallback(async (lineId: string, countedQty: number) => {
    try {
      await updateCountLine({ lineId, countedQty, notes: '' });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update');
    }
  }, [load]);

  const handleRemoveLine = useCallback(async (lineId: string) => {
    try {
      await removeCountLine({ lineId });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to remove');
    }
  }, [load]);

  const handleStartCounting = useCallback(async () => {
    try {
      await updateStockCountStatus(countId, 'in_progress');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start count');
    }
  }, [countId, load]);

  const handleComplete = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const adjustments = await completeStockCount({ countId });
      setSuccessMsg(
        l10n.getString('sc-complete-success', { count: adjustments.length }),
      );
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to complete');
    } finally {
      setSaving(false);
    }
  }, [countId, load, l10n]);

  const totalExpected = lines.reduce((s, l) => s + l.expected_qty, 0);
  const totalCounted = lines.reduce((s, l) => s + (l.counted_qty ?? 0), 0);
  const totalDiff = lines.reduce((s, l) => s + l.difference, 0);

  if (loading) {
    return <p className="sc-detail-loading"><Localized id="sc-loading"><span>Loading…</span></Localized></p>;
  }

  if (!count) {
    return <p className="sc-detail-error"><Localized id="sc-not-found"><span>Count not found.</span></Localized></p>;
  }

  return (
    <div className="sc-detail-screen">
      <div className="sc-detail-header">
        <button type="button" className="sc-detail-back" onClick={onBack}>
          &larr; <Localized id="sc-back"><span>Back</span></Localized>
        </button>
        <h1 className="sc-title">{count.count_number}</h1>
      </div>

      <div className="sc-detail-meta">
        <span className={`sc-badge sc-badge--${count.status}`}>
          {l10n.getString(`sc-status-${count.status}`) ?? count.status}
        </span>
        <span>{l10n.getString(`sc-type-${count.count_type}`) ?? count.count_type}</span>
        <span>{new Date(count.created_at).toLocaleDateString()}</span>
      </div>

      {count.notes && <p className="sc-detail-notes">{count.notes}</p>}

      {error && <div className="sc-detail-err" role="alert">{error}</div>}
      {successMsg && <div className="sc-detail-success" role="status">{successMsg}</div>}

      {/* Actions */}
      <div className="sc-detail-actions">
        {count.status === 'draft' && (
          <Button variant="primary" onClick={handleStartCounting}>
            <Localized id="sc-start-counting"><span>Start Counting</span></Localized>
          </Button>
        )}
        {isEditable && lines.length > 0 && (
          <Button variant="primary" onClick={handleComplete} loading={saving}>
            <Localized id="sc-complete-count"><span>Complete Count</span></Localized>
          </Button>
        )}
      </div>

      {/* Add line */}
      {isEditable && (
        <Card shadow="sm" className="sc-detail-add-line">
          <h3><Localized id="sc-add-line"><span>Add Product to Count</span></Localized></h3>
          <div className="sc-add-line-search">
            <input
              type="search"
              placeholder={l10n.getString('sc-search-placeholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              aria-label={l10n.getString('sc-search-aria')}
            />
          </div>
          {searchQuery && filteredProducts.length > 0 && (
            <div className="sc-add-line-results">
              {filteredProducts.slice(0, 8).map((p) => (
                <button
                  key={p.sku}
                  type="button"
                  className={`sc-add-line-item ${selectedSku === p.sku ? 'sc-add-line-item--sel' : ''}`}
                  onClick={() => {
                    setSelectedSku(p.sku);
                    setSelectedName(p.name);
                    setExpectedQty(String(p.stock_qty ?? 0));
                    setSearchQuery('');
                  }}
                >
                  <span className="sc-add-line-name">{p.name}</span>
                  <span className="sc-add-line-sku">{p.sku}</span>
                  <span className="sc-add-line-stock">{p.stock_qty ?? '—'}</span>
                </button>
              ))}
            </div>
          )}
          {selectedSku && (
            <div className="sc-add-line-form">
              <span className="sc-add-line-selected">{selectedName} ({selectedSku})</span>
              <div>
                <Localized id="sc-expected-qty"><span>Expected Qty</span></Localized>
                <input
                  type="number"
                  value={expectedQty}
                  onChange={(e) => setExpectedQty(e.target.value)}
                  min="0"
                  aria-label={l10n.getString('sc-expected-qty')}
                />
              </div>
              <Button variant="primary" onClick={handleAddLine} loading={saving} disabled={!expectedQty}>
                <Localized id="sc-add"><span>Add</span></Localized>
              </Button>
            </div>
          )}
        </Card>
      )}

      {/* Lines table */}
      {lines.length > 0 ? (
        <div className="sc-detail-lines">
          <div className="sc-lines-header">
            <span className="sc-lines-col-sku"><Localized id="sc-col-sku"><span>SKU</span></Localized></span>
            <span className="sc-lines-col-name"><Localized id="sc-col-name"><span>Product</span></Localized></span>
            <span className="sc-lines-col-expected"><Localized id="sc-col-expected"><span>Expected</span></Localized></span>
            <span className="sc-lines-col-counted"><Localized id="sc-col-counted"><span>Counted</span></Localized></span>
            <span className="sc-lines-col-diff"><Localized id="sc-col-diff"><span>Diff</span></Localized></span>
            {isEditable && <span className="sc-lines-col-actions"></span>}
          </div>
          {lines.map((line) => (
            <div key={line.id} className="sc-lines-row">
              <span className="sc-lines-col-sku">{line.sku}</span>
              <span className="sc-lines-col-name">{line.product_name}</span>
              <span className="sc-lines-col-expected">{line.expected_qty}</span>
              <span className="sc-lines-col-counted">
                {isEditable ? (
                  <input
                    type="number"
                    className="sc-counted-input"
                    value={line.counted_qty ?? ''}
                    onChange={(e) => handleRecordCount(line.id, parseInt(e.target.value) || 0)}
                    min="0"
                    aria-label={l10n.getString('sc-counted-aria', { sku: line.sku })}
                  />
                ) : (
                  line.counted_qty ?? '—'
                )}
              </span>
              <span className={`sc-lines-col-diff ${line.difference < 0 ? 'sc-diff-neg' : line.difference > 0 ? 'sc-diff-pos' : ''}`}>
                {line.counted_qty != null ? (line.difference > 0 ? '+' : '') + line.difference : '—'}
              </span>
              {isEditable && (
                <span className="sc-lines-col-actions">
                  <button
                    type="button"
                    className="sc-remove-btn"
                    onClick={() => handleRemoveLine(line.id)}
                    aria-label={l10n.getString('sc-remove-aria', { sku: line.sku })}
                  >
                    &times;
                  </button>
                </span>
              )}
            </div>
          ))}
          <div className="sc-lines-total">
            <span className="sc-lines-col-sku"></span>
            <span className="sc-lines-col-name"><strong><Localized id="sc-total"><span>Total</span></Localized></strong></span>
            <span className="sc-lines-col-expected"><strong>{totalExpected}</strong></span>
            <span className="sc-lines-col-counted"><strong>{totalCounted}</strong></span>
            <span className={`sc-lines-col-diff ${totalDiff < 0 ? 'sc-diff-neg' : totalDiff > 0 ? 'sc-diff-pos' : ''}`}>
              <strong>{totalDiff > 0 ? '+' : ''}{totalDiff}</strong>
            </span>
          </div>
        </div>
      ) : (
        <p className="sc-detail-empty">
          <Localized id="sc-no-lines">
            <span>No products added yet. Search and add products above.</span>
          </Localized>
        </p>
      )}
    </div>
  );
}
