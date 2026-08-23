import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { requiredLocalized, EmptyState } from '@/frontend/shared';
import { listWarehouseProductsAtLocation, adjustStockScoped, type ProductDto } from '@/api/products';

import { getLowStockAlertsAtLocation, type LowStockAlert } from '@/api/inventory';
import LocationPicker from '@/features/inventory/LocationPicker';
import { StockAlertPanel } from '@/features/inventory/StockAlertPanel';
import { l10nErrorMessage } from '@/utils/app-error';
import { formatMoney } from '@/types/domain';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import './WarehouseScreen.css';

type SortKey = 'name' | 'sku' | 'stock_qty' | 'cost';
type SortDir = 'asc' | 'desc';
type StockFilter = 'all' | 'in_stock' | 'out_of_stock' | 'low_stock';

const LOW_STOCK_THRESHOLD = 10;

/** Warehouse workspace — location-scoped stock view with search, sort, and adjust. */
export default function WarehouseScreen() {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken, activeInstance } = useWorkspace();
  const sessionToken = rawToken ?? '';
  const instanceId = activeInstance?.instance_id ?? '';

  const [locationId, setLocationId] = useState('');
  const [products, setProducts] = useState<ProductDto[]>([]);
  const [alerts, setAlerts] = useState<LowStockAlert[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  // Search / filter / sort state
  const [search, setSearch] = useState('');
  const searchRef = useRef<HTMLInputElement>(null);
  const [categoryFilter, setCategoryFilter] = useState<string>('');
  const [stockFilter, setStockFilter] = useState<StockFilter>('all');
  const [sortKey, setSortKey] = useState<SortKey>('name');
  const [sortDir, setSortDir] = useState<SortDir>('asc');

  // Adjust modal state
  const [adjustTarget, setAdjustTarget] = useState<ProductDto | null>(null);
  const [adjustDelta, setAdjustDelta] = useState('');
  const [adjustReason, setAdjustReason] = useState('');
  const [adjusting, setAdjusting] = useState(false);

  // Load products when location changes
  const loadProducts = useCallback(async () => {
    if (!sessionToken || !locationId) {
      setProducts([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const data = await listWarehouseProductsAtLocation(sessionToken, locationId);
      setProducts(data);
    } catch (err) {
      setError(l10nErrorMessage(err, l10n, 'warehouse-load-error'));
    } finally {
      setLoading(false);
    }
  }, [sessionToken, locationId, l10n]);

  // Categories are derived from products (no separate fetch needed)

  // Load low stock alerts
  const loadAlerts = useCallback(async () => {
    if (!sessionToken || !locationId) {
      setAlerts([]);
      return;
    }
    try {
      const data = await getLowStockAlertsAtLocation(sessionToken, locationId, LOW_STOCK_THRESHOLD);
      setAlerts(data);
    } catch {
      // Non-critical
    }
  }, [sessionToken, locationId]);

  useEffect(() => {
    loadProducts();
    loadAlerts();
  }, [loadProducts, loadAlerts, refreshKey]);

  const handleLocationChange = useCallback((id: string, _name: string) => {
    setLocationId(id);
    setSearch('');
    setCategoryFilter('');
    setStockFilter('all');
  }, []);

  // ── Filtered + sorted products ────────────────────────────────────
  const filtered = useMemo(() => {
    let list = products;

    // Text search
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(
        (p) =>
          p.sku.toLowerCase().includes(q) ||
          p.name.toLowerCase().includes(q),
      );
    }

    // Category filter
    if (categoryFilter) {
      list = list.filter((p) => p.category === categoryFilter);
    }

    // Stock status filter
    if (stockFilter === 'out_of_stock') {
      list = list.filter((p) => (p.stock_qty ?? 0) <= 0);
    } else if (stockFilter === 'in_stock') {
      list = list.filter((p) => (p.stock_qty ?? 0) > 0);
    } else if (stockFilter === 'low_stock') {
      list = list.filter((p) => (p.stock_qty ?? 0) > 0 && (p.stock_qty ?? 0) <= LOW_STOCK_THRESHOLD);
    }

    // Sort
    list = [...list].sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case 'name':
          cmp = a.name.localeCompare(b.name);
          break;
        case 'sku':
          cmp = a.sku.localeCompare(b.sku);
          break;
        case 'stock_qty':
          cmp = (a.stock_qty ?? 0) - (b.stock_qty ?? 0);
          break;
        case 'cost':
          cmp = (a.cost_minor ?? 0) - (b.cost_minor ?? 0);
          break;
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });

    return list;
  }, [products, search, categoryFilter, stockFilter, sortKey, sortDir]);

  // ── Summary stats ─────────────────────────────────────────────────
  const stats = useMemo(() => {
    const total = products.length;
    const outOfStock = products.filter((p) => (p.stock_qty ?? 0) <= 0).length;
    const lowStock = products.filter(
      (p) => (p.stock_qty ?? 0) > 0 && (p.stock_qty ?? 0) <= LOW_STOCK_THRESHOLD,
    ).length;
    const totalValue = products.reduce((sum, p) => sum + (p.cost_minor ?? 0) * (p.stock_qty ?? 0), 0);
    return { total, outOfStock, lowStock, totalValue };
  }, [products]);

  // ── Unique categories from products ───────────────────────────────
  const productCategories = useMemo(() => {
    const cats = new Set(products.map((p) => p.category).filter((c): c is string => Boolean(c)));
    return Array.from(cats).sort();
  }, [products]);

  // ── Sort handler ──────────────────────────────────────────────────
  const handleSort = useCallback(
    (key: SortKey) => {
      if (sortKey === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
      } else {
        setSortKey(key);
        setSortDir('asc');
      }
    },
    [sortKey],
  );

  // ── Adjust stock ──────────────────────────────────────────────────
  const handleAdjustClick = useCallback((product: ProductDto) => {
    setAdjustTarget(product);
    setAdjustDelta('');
    setAdjustReason('');
  }, []);

  const handleAdjustConfirm = useCallback(async () => {
    if (!sessionToken || !adjustTarget) return;
    const delta = parseInt(adjustDelta, 10);
    if (isNaN(delta) || delta === 0) return;

    setAdjusting(true);
    try {
      await adjustStockScoped(sessionToken, {
        sku: adjustTarget.sku,
        delta,
        reason: adjustReason || 'manual adjustment',
      });
      setAdjustTarget(null);
      setRefreshKey((k) => k + 1);
    } catch (err) {
      setError(l10nErrorMessage(err, l10n, 'warehouse-adjust-error'));
    } finally {
      setAdjusting(false);
    }
  }, [sessionToken, adjustTarget, adjustDelta, adjustReason, l10n]);

  // ── Keyboard shortcut: focus search on '/' ────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === '/' && !e.ctrlKey && !e.metaKey) {
        const target = e.target as HTMLElement;
        if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, []);

  const sortIcon = (key: SortKey) =>
    sortKey === key ? (sortDir === 'asc' ? ' ↑' : ' ↓') : '';

  return (
    <div className="warehouse-screen">
      {/* Header */}
      <div className="warehouse-header">
        <h1 className="warehouse-title">
          {requiredLocalized(l10n, 'warehouse-title')}
        </h1>
        {instanceId && (
          <LocationPicker
            value={locationId}
            onChange={handleLocationChange}
            refreshKey={refreshKey}
          />
        )}
      </div>

      {!locationId && (
        <EmptyState
          title={requiredLocalized(l10n, 'warehouse-no-location-title')}
          description={requiredLocalized(l10n, 'warehouse-no-location-desc')}
        />
      )}

      {locationId && (
        <>
          {/* Summary stats */}
          {products.length > 0 && (
            <div className="warehouse-stats">
              <div className="warehouse-stat">
                <span className="warehouse-stat-value">{stats.total}</span>
                <span className="warehouse-stat-label">{requiredLocalized(l10n, 'warehouse-stat-total')}</span>
              </div>
              <div className="warehouse-stat">
                <span className="warehouse-stat-value">{stats.outOfStock}</span>
                <span className="warehouse-stat-label">{requiredLocalized(l10n, 'warehouse-stat-out-of-stock')}</span>
              </div>
              <div className="warehouse-stat">
                <span className="warehouse-stat-value">{stats.lowStock}</span>
                <span className="warehouse-stat-label">{requiredLocalized(l10n, 'warehouse-stat-low-stock')}</span>
              </div>
            </div>
          )}

          {/* Search + filters */}
          {products.length > 0 && (
            <div className="warehouse-toolbar">
              <div className="warehouse-search-wrap">
                <input
                  ref={searchRef}
                  type="text"
                  className="warehouse-search"
                  placeholder={requiredLocalized(l10n, 'warehouse-search-placeholder')}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  aria-label={requiredLocalized(l10n, 'warehouse-search-aria')}
                />
                {search && (
                  <button
                    type="button"
                    className="warehouse-search-clear"
                    onClick={() => setSearch('')}
                    aria-label="Clear search"
                  >
                    ×
                  </button>
                )}
              </div>

              <div className="warehouse-filters">
                {/* Category filter */}
                {productCategories.length > 0 && (
                  <select
                    className="warehouse-select"
                    value={categoryFilter}
                    onChange={(e) => setCategoryFilter(e.target.value)}
                    aria-label={requiredLocalized(l10n, 'warehouse-filter-category')}
                  >
                    <option value="">{requiredLocalized(l10n, 'warehouse-all-categories')}</option>
                    {productCategories.map((cat) => (
                      <option key={cat} value={cat}>{cat}</option>
                    ))}
                  </select>
                )}

                {/* Stock status filter */}
                <select
                  className="warehouse-select"
                  value={stockFilter}
                  onChange={(e) => setStockFilter(e.target.value as StockFilter)}
                  aria-label={requiredLocalized(l10n, 'warehouse-filter-stock')}
                >
                  <option value="all">{requiredLocalized(l10n, 'warehouse-stock-all')}</option>
                  <option value="in_stock">{requiredLocalized(l10n, 'warehouse-stock-in')}</option>
                  <option value="out_of_stock">{requiredLocalized(l10n, 'warehouse-stock-out')}</option>
                  <option value="low_stock">{requiredLocalized(l10n, 'warehouse-stock-low')}</option>
                </select>
              </div>
            </div>
          )}

          {/* Loading */}
          {loading && (
            <div className="warehouse-loading">
              <div className="warehouse-skeleton" />
              <div className="warehouse-skeleton" />
              <div className="warehouse-skeleton" />
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="warehouse-error" role="alert">
              {error}
            </div>
          )}

          {/* Empty state */}
          {!loading && products.length === 0 && !error && (
            <EmptyState
              title={requiredLocalized(l10n, 'warehouse-empty-title')}
              description={requiredLocalized(l10n, 'warehouse-empty-desc')}
            />
          )}

          {/* No results after filter */}
          {!loading && products.length > 0 && filtered.length === 0 && (
            <div className="warehouse-no-results">
              {requiredLocalized(l10n, 'warehouse-no-results')}
            </div>
          )}

          {/* Product table */}
          {!loading && filtered.length > 0 && (
            <>
              {alerts.length > 0 && locationId && (
                <StockAlertPanel
                  locationId={locationId}
                />
              )}

              <div className="warehouse-table-wrap">
                <table className="warehouse-table" role="grid">
                  <thead>
                    <tr>
                      <th>
                        <button type="button" className="warehouse-th-btn" onClick={() => handleSort('sku')}>
                          {requiredLocalized(l10n, 'warehouse-col-sku')}{sortIcon('sku')}
                        </button>
                      </th>
                      <th>
                        <button type="button" className="warehouse-th-btn" onClick={() => handleSort('name')}>
                          {requiredLocalized(l10n, 'warehouse-col-name')}{sortIcon('name')}
                        </button>
                      </th>
                      <th>{requiredLocalized(l10n, 'warehouse-col-category')}</th>
                      <th className="warehouse-col-qty">
                        <button type="button" className="warehouse-th-btn" onClick={() => handleSort('stock_qty')}>
                          {requiredLocalized(l10n, 'warehouse-col-qty')}{sortIcon('stock_qty')}
                        </button>
                      </th>
                      <th>
                        <button type="button" className="warehouse-th-btn" onClick={() => handleSort('cost')}>
                          {requiredLocalized(l10n, 'warehouse-col-cost')}{sortIcon('cost')}
                        </button>
                      </th>
                      <th>{requiredLocalized(l10n, 'warehouse-col-actions')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filtered.map((p) => (
                      <tr key={p.sku} className={p.stock_qty != null && p.stock_qty <= 0 ? 'warehouse-row--out-of-stock' : ''}>
                        <td className="warehouse-cell-sku">{p.sku}</td>
                        <td>{p.name}</td>
                        <td>{p.category ?? '—'}</td>
                        <td className={`warehouse-cell-qty ${p.stock_qty != null && p.stock_qty <= 0 ? 'warehouse-qty--zero' : ''}`}>
                          {p.stock_qty ?? 0}
                        </td>
                        <td>{formatMoney({ minor_units: p.cost_minor ?? 0, currency: p.price.currency })}</td>
                        <td>
                          <button
                            type="button"
                            className="warehouse-btn warehouse-btn--adjust"
                            disabled={adjusting}
                            onClick={() => handleAdjustClick(p)}
                          >
                            {requiredLocalized(l10n, 'warehouse-btn-adjust')}
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="warehouse-summary">
                {filtered.length} / {products.length} {requiredLocalized(l10n, 'warehouse-products-count')}
                {alerts.length > 0 && (
                  <span className="warehouse-alert-count">
                    · {alerts.length} {requiredLocalized(l10n, 'warehouse-low-stock-alerts')}
                  </span>
                )}
              </div>
            </>
          )}
        </>
      )}

      {/* Adjust modal */}
      {adjustTarget && (
        <ConfirmDialog
          open={true}
          title={requiredLocalized(l10n, 'warehouse-adjust-title')}
          message={
            <div className="warehouse-adjust-body">
              <p className="warehouse-adjust-product">
                {adjustTarget.name} <span className="warehouse-adjust-sku">({adjustTarget.sku})</span>
              </p>
              <p className="warehouse-adjust-current">
                {requiredLocalized(l10n, 'warehouse-adjust-current')}: {adjustTarget.stock_qty ?? 0}
              </p>
              <div className="warehouse-adjust-field">
                <label htmlFor="warehouse-adjust-delta">
                  {requiredLocalized(l10n, 'warehouse-adjust-delta-label')}
                </label>
                <input
                  id="warehouse-adjust-delta"
                  type="number"
                  className="warehouse-adjust-input"
                  value={adjustDelta}
                  onChange={(e) => setAdjustDelta(e.target.value)}
                  placeholder="+5 or -3"
                  autoFocus
                />
              </div>
              <div className="warehouse-adjust-field">
                <label htmlFor="warehouse-adjust-reason">
                  {requiredLocalized(l10n, 'warehouse-adjust-reason-label')}
                </label>
                <input
                  id="warehouse-adjust-reason"
                  type="text"
                  className="warehouse-adjust-input"
                  value={adjustReason}
                  onChange={(e) => setAdjustReason(e.target.value)}
                  placeholder={requiredLocalized(l10n, 'warehouse-adjust-reason-placeholder')}
                />
              </div>
            </div>
          }
          confirmLabel={requiredLocalized(l10n, 'warehouse-adjust-confirm')}
          cancelLabel={requiredLocalized(l10n, 'warehouse-adjust-cancel')}
          onConfirm={handleAdjustConfirm}
          onCancel={() => setAdjustTarget(null)}
          disabled={adjusting || !adjustDelta || parseInt(adjustDelta, 10) === 0}
        />
      )}
    </div>
  );
}
