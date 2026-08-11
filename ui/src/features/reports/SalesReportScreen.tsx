import { useContext, useEffect, useState, useCallback, useMemo } from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { WorkspaceContext } from '@/contexts/WorkspaceContext';
import { Localized, useLocalization } from '@fluent/react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Legend,
} from 'recharts';
import { printSalesReceipt } from '@/api/sales';
import {
  getDailyRevenue,
  getWeeklyRevenue,
  getMonthlyRevenue,
  getTopProducts,
  getHourlyHeatmap,
  getCategoryBreakdown,
  getCategoryPopularity,
  type DailyRevenueRow,
  type WeeklyRevenueRow,
  type MonthlyRevenueRow,
  type TopProductRow,
  type HourlyHeatmapRow,
  type CategoryBreakdownRow,
  type CategoryPopularityRow,
} from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import { minorUnitExponent } from '@/types/domain';
import { sumGrossProfitByCurrency, sumRevenueByCurrency } from './revenueTotals';
import './SalesReportScreen.css';

const PIE_COLORS = [
  '#4f46e5', '#06b6d4', '#10b981', '#f59e0b', '#ef4444',
  '#8b5cf6', '#ec4899', '#14b8a6', '#f97316', '#6366f1',
];

const HEATMAP_COLORS = [
  '#f0fdf4', '#bbf7d0', '#86efac', '#4ade80',
  '#22c55e', '#16a34a', '#15803d', '#166534',
];

type ViewMode = 'daily' | 'weekly' | 'monthly';

const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

type RevenueRow = DailyRevenueRow | WeeklyRevenueRow | MonthlyRevenueRow;

function fmtCurrency(minor: number, currency: string): string {
  // Exponent-driven: IDR/JPY = 0 decimals, KWD = 3, USD/EUR = 2 — the
  // shared minorUnitExponent map is the single source of truth (mirrors
  // the Rust Currency::minor_unit_exponent). No hardcoded /100 math.
  const exp = minorUnitExponent(currency);
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
    minimumFractionDigits: exp,
    maximumFractionDigits: exp,
  }).format(minor / 10 ** exp);
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function monthAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().slice(0, 10);
}

/** Sales report screen — daily/weekly/monthly revenue charts, top products, hourly heatmap, and category breakdown with CSV export. */
export default function SalesReportScreen() {
  const { l10n } = useLocalization();
  const sessionToken = useContext(WorkspaceContext)?.sessionToken ?? '';
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ViewMode>('daily');
  const [startDate, setStartDate] = useState(monthAgo());
  const [endDate, setEndDate] = useState(today());

  const [revenueData, setRevenueData] = useState<RevenueRow[]>([]);
  const [topProducts, setTopProducts] = useState<TopProductRow[]>([]);
  // Rank the top-products table by revenue (default) or gross profit.
  // Boolean flag keeps className interpolations quote-free (see
  // screenExtraction: quoted strings inside `${...}` are read as classes).
  const [rankByProfit, setRankByProfit] = useState(false);
  const [heatmap, setHeatmap] = useState<HourlyHeatmapRow[]>([]);
  const [categoryBreakdown, setCategoryBreakdown] = useState<
    CategoryBreakdownRow[]
  >([]);
  const [categoryPopularity, setCategoryPopularity] = useState<
    CategoryPopularityRow[]
  >([]);

  // P9-3: Period comparison
  const [comparePeriod, setComparePeriod] = useState(false);
  const [prevRevenueData, setPrevRevenueData] = useState<RevenueRow[]>([]);

  const revenueTotals = sumRevenueByCurrency(revenueData);
  const multiCurrencyPeriod = revenueTotals.length > 1;
  // Single-currency periods keep the exact pre-existing display; the chart
  // tooltip still uses the first-seen currency (multi-currency chart
  // semantics are a tracked follow-up, not this fix).
  const currency: string = revenueTotals[0]?.currency ?? 'USD';
  const totalRevenue = revenueTotals[0]?.total_minor ?? 0;

  const fetchData = useCallback(() => {
    setLoading(true);
    setError(null);

    let revenuePromise: Promise<RevenueRow[]>;
    switch (view) {
      case 'daily':
        revenuePromise = getDailyRevenue(startDate, endDate, sessionToken);
        break;
      case 'weekly':
        revenuePromise = getWeeklyRevenue(startDate, endDate, sessionToken);
        break;
      case 'monthly':
        revenuePromise = getMonthlyRevenue(startDate, endDate, sessionToken);
        break;
    }

    Promise.all([
      revenuePromise,
      getTopProducts(startDate, endDate, 10, sessionToken, rankByProfit ? 'profit' : 'revenue'),
      getHourlyHeatmap(startDate, endDate, sessionToken),
      getCategoryBreakdown(startDate, endDate, sessionToken),
      getCategoryPopularity(sessionToken, 3),
    ])
      .then(([rev, top, heat, cat, catPop]) => {
        setRevenueData(rev);
        setTopProducts(top);
        setHeatmap(heat);
        setCategoryBreakdown(cat);
        setCategoryPopularity(catPop);
      })
      .catch((e) => {
        setError(e.message ?? String(e));
      })
      .finally(() => {
        setLoading(false);
      });
  }, [view, startDate, endDate, sessionToken, rankByProfit]);

  // P9-3: Fetch previous period data when comparison is enabled
  const calcPrevRange = useCallback(() => {
    const start = new Date(startDate);
    const end = new Date(endDate);
    const periodMs = end.getTime() - start.getTime();
    const prevEnd = new Date(start.getTime() - 1);
    const prevStart = new Date(prevEnd.getTime() - periodMs);
    return {
      prevStart: prevStart.toISOString().slice(0, 10),
      prevEnd: prevEnd.toISOString().slice(0, 10),
    };
  }, [startDate, endDate]);

  const fetchPrevData = useCallback(() => {
    if (!comparePeriod) {
      setPrevRevenueData([]);
      return;
    }

    const { prevStart, prevEnd } = calcPrevRange();

    let revenuePromise: Promise<RevenueRow[]>;
    switch (view) {
      case 'daily':
        revenuePromise = getDailyRevenue(prevStart, prevEnd, sessionToken);
        break;
      case 'weekly':
        revenuePromise = getWeeklyRevenue(prevStart, prevEnd, sessionToken);
        break;
      case 'monthly':
        revenuePromise = getMonthlyRevenue(prevStart, prevEnd, sessionToken);
        break;
    }

    revenuePromise
      .then(setPrevRevenueData)
      .catch(() => { /* period comparison is best-effort; silently clear on failure */ setPrevRevenueData([]); })

  }, [comparePeriod, view, calcPrevRange, sessionToken]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    fetchPrevData();
  }, [fetchPrevData]);

  const heatmapGrid: number[][] = Array.from({ length: 7 }, () =>
    Array(24).fill(0),
  );
  for (const row of heatmap) {
    if (
      row.day_of_week >= 0 &&
      row.day_of_week < 7 &&
      row.hour >= 0 &&
      row.hour < 24
    ) {
      heatmapGrid[row.day_of_week]![row.hour] = row.total_minor;
    }
  }
  const heatmapMax = Math.max(...heatmapGrid.flat(), 1);

  const exportCsv = () => {
    const headers = ['Period', 'Revenue', 'Currency', 'Orders'];
    const rows = revenueData.map((r) => {
      const period = 'date' in r ? r.date : 'week_start' in r ? r.week_start : r.month;
      return [
        period,
        (r.total_minor / 10 ** minorUnitExponent(r.currency)).toFixed(minorUnitExponent(r.currency)),
        r.currency,
        r.sale_count,
      ].join(',');
    });
    const bom = '\uFEFF';
    const csv = [headers.join(','), ...rows].join('\n');
    const blob = new Blob([bom + csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sales-report-${startDate}-${endDate}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const printReport = async () => {
    const totalMinor = revenueData.reduce(
      (s: number, r) => s + r.total_minor,
      0,
    );

    await printSalesReceipt({
      date: new Date().toISOString().slice(0, 10),
      receiptNumber: `RPT-${Date.now()}`,
      items: topProducts.map((p) => ({
        name: p.name,
        quantity: p.total_qty,
        unitPrice: { minorUnits: 0, currency },
        totalPrice: { minorUnits: p.total_minor, currency },
      })),
      subtotal: { minorUnits: totalMinor, currency },
      total: { minorUnits: totalMinor, currency },
      payments: [{ method: 'Report', amount: { minorUnits: totalMinor, currency }, change: null }],
    });
  };

  // P9-3: Calculate deltas — per-currency, never across a collapsed sum.
  const prevRevenueTotals = useMemo(
    () => sumRevenueByCurrency(prevRevenueData),
    [prevRevenueData],
  );
  const prevTotalRevenue = prevRevenueTotals[0]?.total_minor ?? 0;
  const prevMultiCurrencyPeriod = prevRevenueTotals.length > 1;
  const prevTotalOrders = useMemo(
    () => prevRevenueData.reduce((s: number, r) => r.sale_count + s, 0),
    [prevRevenueData],
  );

  if (loading) {
    return (
      <div className="sales-report-loading-skeleton" aria-hidden="true">
        {/* Header: title + controls */}
        <div className="sales-report-header">
          <Skeleton width="10rem" height="1.75rem" />
          <div className="sales-report-controls">
            <Skeleton width="5rem" height="2rem" />
            <Skeleton width="5rem" height="2rem" />
            <Skeleton width="8rem" height="2rem" />
            <Skeleton width="4rem" height="2rem" />
            <Skeleton width="6rem" height="2rem" />
          </div>
        </div>
        {/* Revenue chart card */}
        <Card shadow="sm" className="sales-report-chart-card">
          <Skeleton width="5rem" height="1.25rem" />
          <Skeleton variant="block" width="100%" height="300px" style={{ borderRadius: 'var(--radius-lg)', marginTop: 'var(--space-3)' }} />
          <div className="sales-report-totals" style={{ marginTop: 'var(--space-3)' }}>
            <Skeleton width="6rem" height="1rem" />
            <Skeleton width="4rem" height="1rem" />
          </div>
        </Card>
        {/* Two-column layout */}
        <div className="sales-report-columns">
          <Card shadow="sm" className="sales-report-chart-card">
            <Skeleton width="6rem" height="1.25rem" />
            <Skeleton variant="block" width="100%" height="250px" style={{ borderRadius: 'var(--radius-lg)', marginTop: 'var(--space-3)' }} />
          </Card>
          <Card shadow="sm" className="sales-report-chart-card">
            <Skeleton width="6rem" height="1.25rem" />
            {/* 4-column table header + 4 skeleton rows */}
            <div className="sales-report-top-header">
              <Skeleton width="1rem" height="0.75rem" />
              <Skeleton width="3rem" height="0.75rem" />
              <Skeleton width="2rem" height="0.75rem" />
              <Skeleton width="3rem" height="0.75rem" />
            </div>
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="sales-report-top-row">
                <Skeleton width="1rem" height="0.875rem" />
                <Skeleton width="5rem" height="0.875rem" />
                <Skeleton width="2rem" height="0.875rem" />
                <Skeleton width="4rem" height="0.875rem" />
              </div>
            ))}
          </Card>
        </div>
        {/* Heatmap card */}
        <Card shadow="sm" className="sales-report-chart-card">
          <Skeleton width="6rem" height="1.25rem" />
        </Card>
      </div>
    );
  }

  const revenueKey =
    view === 'daily' ? 'date' : view === 'weekly' ? 'week_start' : 'month';
  const totalOrders = revenueData.reduce(
    (s: number, r) => r.sale_count + s,
    0,
  );

  const canCompareRevenue = comparePeriod
    && !multiCurrencyPeriod
    && !prevMultiCurrencyPeriod
    && prevTotalRevenue > 0;
  const revenueDelta = canCompareRevenue
    ? ((totalRevenue - prevTotalRevenue) / prevTotalRevenue) * 100
    : null;
  const ordersDelta = comparePeriod && prevTotalOrders > 0
    ? ((totalOrders - prevTotalOrders) / prevTotalOrders) * 100
    : null;

  return (
    <div className="sales-report" role="region" aria-label={requiredLocalized(l10n, 'sales-report-region-aria')}>
      <div className="sales-report-header">
        <Localized id="sales-report-title">
          <h1 className="sales-report-title">Sales Report</h1>
        </Localized>

        <div className="sales-report-controls">
          <label htmlFor="start-date" className="sales-report-label">
            <Localized id="sales-report-start-date">Start</Localized>
          </label>
          <input
            id="start-date"
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="sales-report-input"
            aria-label={requiredLocalized(l10n, 'sales-report-start-aria')}
          />

          <label htmlFor="end-date" className="sales-report-label">
            <Localized id="sales-report-end-date">End</Localized>
          </label>
          <input
            id="end-date"
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="sales-report-input"
            aria-label={requiredLocalized(l10n, 'sales-report-end-aria')}
          />

          <div
            className="sales-report-view-toggle"
            role="radiogroup"
            aria-label={requiredLocalized(l10n, 'sales-report-view-aria')}
          >
            {(['daily', 'weekly', 'monthly'] as ViewMode[]).map((mode) => (
              <button
                key={mode}
                className={`sales-report-view-btn ${view === mode ? 'active' : ''}`}
                onClick={() => setView(mode)}
                role="radio"
                aria-checked={view === mode}
                aria-label={mode}
              >
                <Localized id={`sales-report-${mode}`}>
                  {mode.charAt(0).toUpperCase() + mode.slice(1)}
                </Localized>
              </button>
            ))}
          </div>

          <Button
            variant={comparePeriod ? 'primary' : 'secondary'}
            onClick={() => setComparePeriod((p) => !p)}
            aria-label={comparePeriod ? (requiredLocalized(l10n, 'sales-report-compare-off-aria')) : (requiredLocalized(l10n, 'sales-report-compare-on-aria'))}
            aria-pressed={comparePeriod}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true" style={{ marginRight: 'var(--space-1)' }}>
              <line x1="12" y1="20" x2="12" y2="10" />
              <line x1="18" y1="20" x2="18" y2="4" />
              <line x1="6" y1="20" x2="6" y2="16" />
            </svg>
            <Localized id="sales-report-compare">Compare</Localized>
          </Button>
          <Button
            variant="secondary"
            onClick={printReport}
            aria-label={requiredLocalized(l10n, 'sales-report-print-aria')}
          >
            <Localized id="print">Print</Localized>
          </Button>
          <Button
            variant="secondary"
            onClick={exportCsv}
            aria-label={requiredLocalized(l10n, 'sales-report-export-aria')}
          >
            <Localized id="sales-report-export-csv">Export CSV</Localized>
          </Button>
        </div>
      </div>

      {error && (
        <p className="sales-report-error">
          <Localized id="error-occurred">
            <span>An error occurred</span>
          </Localized>
        </p>
      )}

      <Card shadow="sm" className="sales-report-chart-card">
        <Localized id="sales-report-revenue-chart">
          <h2 className="sales-report-section-title">Revenue</h2>
        </Localized>
        <ResponsiveContainer width="100%" height={300}>
          <BarChart data={revenueData as unknown as Record<string, unknown>[]}>
            <XAxis
              dataKey={revenueKey}
              tick={{ fontSize: 12 }}
            />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip
                  formatter={(value: unknown) => fmtCurrency(Number(value), currency)}
            />
            <Bar
              dataKey="total_minor"
              fill="var(--color-accent, #4f46e5)"
              radius={[4, 4, 0, 0]}
              aria-label={l10n.getString('sales-report-revenue-label')}
            />
          </BarChart>
        </ResponsiveContainer>
        <div className="sales-report-totals">
          <span>
            <Localized id="sales-report-total-revenue">Total</Localized>:{' '}
            {multiCurrencyPeriod
              ? revenueTotals.map((t) => fmtCurrency(t.total_minor, t.currency)).join(' · ')
              : fmtCurrency(totalRevenue, currency)}
            {revenueDelta !== null && (
              <span className={`comparison-delta ${revenueDelta >= 0 ? 'comparison-delta--positive' : 'comparison-delta--negative'}`}>
                <span>{revenueDelta >= 0 ? '▲' : '▼'}</span>
                <span>{Math.abs(revenueDelta).toFixed(1)}%</span>
                <span style={{ fontWeight: 400, opacity: 0.6, fontSize: '0.85em' }}>
                  vs {fmtCurrency(prevTotalRevenue, currency)}
                </span>
              </span>
            )}
          </span>
          <span>
            <Localized id="sales-report-total-orders">Orders</Localized>:{' '}
            {totalOrders}
            {ordersDelta !== null && (
              <span className={`comparison-delta ${ordersDelta >= 0 ? 'comparison-delta--positive' : 'comparison-delta--negative'}`}>
                <span>{ordersDelta >= 0 ? '▲' : '▼'}</span>
                <span>{Math.abs(ordersDelta).toFixed(1)}%</span>
                <span style={{ fontWeight: 400, opacity: 0.6, fontSize: '0.85em' }}>
                  vs {prevTotalOrders}
                </span>
              </span>
            )}
          </span>
          {/* HPP exposure: gross profit per period (daily, weekly, monthly) */}
          {revenueData.length > 0 && (() => {
            const profitTotals = sumGrossProfitByCurrency(revenueData);
            return (
              <span>
                <Localized id="sales-report-total-gross-profit">Gross Profit</Localized>:{' '}
                {profitTotals.map((t) => fmtCurrency(t.gross_profit_minor, t.currency)).join(' · ')}
                <span className={`comparison-delta ${profitTotals.every((t) => t.gross_profit_minor >= 0) ? 'comparison-delta--positive' : 'comparison-delta--negative'}`}>
                  <span>
                    {profitTotals.length === 1 && totalRevenue > 0
                      ? `(${((profitTotals[0]!.gross_profit_minor / totalRevenue) * 100).toFixed(1)}%)`
                      : ''}
                  </span>
                </span>
              </span>
            );
          })()}
        </div>
      </Card>

      <div className="sales-report-columns">
        <Card shadow="sm" className="sales-report-chart-card">
          <Localized id="sales-report-category-breakdown">
            <h2 className="sales-report-section-title">By Category</h2>
          </Localized>
          {categoryBreakdown.length === 0 ? (
            <p className="sales-report-no-data">
              <Localized id="no-results">
                <span>No results</span>
              </Localized>
            </p>
          ) : (
            <ResponsiveContainer width="100%" height={250}>
              <PieChart>
                <Pie
                  data={categoryBreakdown}
                  dataKey="total_minor"
                  nameKey="category_name"
                  cx="50%"
                  cy="50%"
                  outerRadius={80}
                  label={(...args: unknown[]) =>
                    `${String((args[0] as Record<string, unknown>)['category_name'] ?? '')} ${Number((args[0] as Record<string, unknown>)['percentage']).toFixed(0)}%`
                  }
                >
                  {categoryBreakdown.map((_, i) => (
                    <Cell
                      key={i}
                      fill={PIE_COLORS[i % PIE_COLORS.length]!}
                    />
                  ))}
                </Pie>
                <Tooltip
              formatter={(value: unknown) => fmtCurrency(Number(value), currency)}
                />
                <Legend />
              </PieChart>
            </ResponsiveContainer>
          )}
        </Card>

        <Card shadow="sm" className="sales-report-chart-card">
          <div className="sales-report-top-heading">
            <Localized id="sales-report-top-products">
              <h2 className="sales-report-section-title">Top Products</h2>
            </Localized>
            <div
              className="sales-report-view-toggle"
              role="radiogroup"
              aria-label={requiredLocalized(l10n, 'sales-report-top-rank-aria')}
            >
              <button
                className={`sales-report-view-btn ${rankByProfit ? 'active' : ''}`}
                onClick={() => setRankByProfit(false)}
                role="radio"
                aria-checked={!rankByProfit}
                aria-label={requiredLocalized(l10n, 'sales-report-top-rank-revenue-aria')}
              >
                <Localized id="top-products-revenue">Revenue</Localized>
              </button>
              <button
                className={`sales-report-view-btn ${rankByProfit ? 'active' : ''}`}
                onClick={() => setRankByProfit(true)}
                role="radio"
                aria-checked={rankByProfit}
                aria-label={requiredLocalized(l10n, 'sales-report-top-rank-profit-aria')}
              >
                <Localized id="top-products-gross-profit">Gross Profit</Localized>
              </button>
            </div>
          </div>
          {topProducts.length === 0 ? (
            <p className="sales-report-no-data">
              <Localized id="no-results">
                <span>No results</span>
              </Localized>
            </p>
          ) : (
            <div className="sales-report-top-table">
              <div className="sales-report-top-header">
                <span><Localized id="sales-report-rank">#</Localized></span>
                <span>
                  <Localized id="top-products-name">Name</Localized>
                </span>
                <span>
                  <Localized id="top-products-quantity">Qty</Localized>
                </span>
                <span>
                  <Localized id="top-products-revenue">Revenue</Localized>
                </span>
                <span>
                  <Localized id="top-products-gross-profit">Gross Profit</Localized>
                </span>
                <span>
                  <Localized id="top-products-margin">Margin</Localized>
                </span>
              </div>
              {topProducts.map((p, i) => (
                <div key={p.product_id} className="sales-report-top-row">
                  <span>{i + 1}</span>
                  <span>{p.name}</span>
                  <span>{p.total_qty}</span>
                  <span>{fmtCurrency(p.total_minor, currency)}</span>
                  <span className={p.gross_profit_minor < 0 ? 'sales-report-top-negative' : undefined}>
                    {fmtCurrency(p.gross_profit_minor, currency)}
                  </span>
                  <span>{`${p.gross_margin_percent.toFixed(1)}%`}</span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      <Card shadow="sm" className="sales-report-chart-card">
        <Localized id="sales-report-category-popularity">
          <h2 className="sales-report-section-title">Category Popularity</h2>
        </Localized>
        {categoryPopularity.length === 0 ? (
          <p className="sales-report-no-data">
            <Localized id="no-results">
              <span>No results</span>
            </Localized>
          </p>
        ) : (
          <div className="sales-report-top-table">
            <div className="sales-report-top-header">
              <span>
                <Localized id="sales-report-category-popularity-category">Category</Localized>
              </span>
              <span>
                <Localized id="sales-report-category-popularity-products">Products</Localized>
              </span>
              <span>
                <Localized id="sales-report-category-popularity-mean">Popularity</Localized>
              </span>
              <span>
                <Localized id="sales-report-category-popularity-top">Top Products</Localized>
              </span>
            </div>
            {categoryPopularity.map((cat) => (
              <div key={cat.category_id || 'uncategorized'} className="sales-report-top-row">
                <span>
                  {cat.category_name ??
                    requiredLocalized(l10n, 'sales-report-category-popularity-uncategorized')}
                </span>
                <span>{cat.product_count}</span>
                <span
                  title={requiredLocalized(l10n, 'sales-report-category-popularity-mean-tip')}
                >
                  {cat.catalog_ratio > 0
                    ? `${cat.catalog_ratio.toFixed(1)}×`
                    : '—'}
                </span>
                <span>
                  {cat.top_products
                    .map((t) => `${t.rank}. ${t.name}`)
                    .join(' · ')}
                </span>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card shadow="sm" className="sales-report-chart-card">
        <Localized id="heatmap-title">
          <h2 className="sales-report-section-title">Busiest Hours</h2>
        </Localized>
        {heatmap.length === 0 ? (
          <p className="sales-report-no-data">
            <Localized id="heatmap-no-data">
              <span>No data</span>
            </Localized>
          </p>
        ) : (
          <div
            className="sales-report-heatmap"
            role="grid"
            aria-label={requiredLocalized(l10n, 'sales-report-heatmap-aria')}
          >
            <div className="sales-report-heatmap-header">
              <div className="sales-report-heatmap-corner" />
              {Array.from({ length: 24 }, (_, h) => (
                <div key={h} className="sales-report-heatmap-col-header">
                  {h}
                </div>
              ))}
            </div>
            {heatmapGrid.map((row, day) => (
              <div key={day} className="sales-report-heatmap-row" role="row">
                <div className="sales-report-heatmap-row-label">
                  {DAY_NAMES[day]}
                </div>
                {row.map((val, hour) => (
                  <div
                    key={hour}
                    className="sales-report-heatmap-cell"
                    style={{
                      backgroundColor:
                        val > 0
                          ? HEATMAP_COLORS[
                              Math.min(
                                Math.floor(
                                  (val / heatmapMax) *
                                    HEATMAP_COLORS.length,
                                ),
                                HEATMAP_COLORS.length - 1,
                              )
                            ]
                          : 'var(--color-bg-hover, #f3f4f6)',
                    }}
                    role="gridcell"
                    aria-label={`${DAY_NAMES[day]} ${hour}:00 - ${fmtCurrency(val, currency)}`}
                    title={`${DAY_NAMES[day]} ${hour}:00 - ${fmtCurrency(val, currency)}`}
                  />
                ))}
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
