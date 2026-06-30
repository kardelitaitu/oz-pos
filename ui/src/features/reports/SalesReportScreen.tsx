import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
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
  type DailyRevenueRow,
  type WeeklyRevenueRow,
  type MonthlyRevenueRow,
  type TopProductRow,
  type HourlyHeatmapRow,
  type CategoryBreakdownRow,
} from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Spinner } from '@/components/Spinner';
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

function fmtCurrency(minor: number, currency: string): string {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
  }).format(minor / 100);
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function monthAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().slice(0, 10);
}

export default function SalesReportScreen() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ViewMode>('daily');
  const [startDate, setStartDate] = useState(monthAgo());
  const [endDate, setEndDate] = useState(today());

  const [revenueData, setRevenueData] = useState<
    DailyRevenueRow[] | WeeklyRevenueRow[] | MonthlyRevenueRow[]
  >([]);
  const [topProducts, setTopProducts] = useState<TopProductRow[]>([]);
  const [heatmap, setHeatmap] = useState<HourlyHeatmapRow[]>([]);
  const [categoryBreakdown, setCategoryBreakdown] = useState<
    CategoryBreakdownRow[]
  >([]);

  const currency =
    revenueData.length > 0
      ? ((revenueData[0] as any).currency ?? 'USD')
      : 'USD';

  const fetchData = useCallback(() => {
    setLoading(true);
    setError(null);

    let revenuePromise: Promise<any>;
    switch (view) {
      case 'daily':
        revenuePromise = getDailyRevenue(startDate, endDate);
        break;
      case 'weekly':
        revenuePromise = getWeeklyRevenue(startDate, endDate);
        break;
      case 'monthly':
        revenuePromise = getMonthlyRevenue(startDate, endDate);
        break;
    }

    Promise.all([
      revenuePromise,
      getTopProducts(startDate, endDate, 10),
      getHourlyHeatmap(startDate, endDate),
      getCategoryBreakdown(startDate, endDate),
    ])
      .then(([rev, top, heat, cat]) => {
        setRevenueData(rev);
        setTopProducts(top);
        setHeatmap(heat);
        setCategoryBreakdown(cat);
      })
      .catch((e) => {
        setError(e.message ?? String(e));
      })
      .finally(() => {
        setLoading(false);
      });
  }, [view, startDate, endDate]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

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
    const rows = revenueData.map((r: any) =>
      [
        r.date ?? r.week_start ?? r.month,
        ((r.total_minor ?? 0) / 100).toFixed(2),
        r.currency ?? 'USD',
        r.sale_count ?? 0,
      ].join(','),
    );
    const csv = [headers.join(','), ...rows].join('\n');
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sales-report-${startDate}-${endDate}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const printReport = async () => {
    const totalMinor = (revenueData as any[]).reduce(
      (s: number, r: any) => s + (r.total_minor ?? 0),
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

  if (loading) {
    return (
      <div className="sales-report">
        <Spinner aria-label="Loading sales report" />
      </div>
    );
  }

  const revenueKey =
    view === 'daily' ? 'date' : view === 'weekly' ? 'week_start' : 'month';
  const totalRevenue = (revenueData as any[]).reduce(
    (s: number, r: any) => s + (r.total_minor ?? 0),
    0,
  );
  const totalOrders = (revenueData as any[]).reduce(
    (s: number, r: any) => s + (r.sale_count ?? 0),
    0,
  );

  return (
    <div className="sales-report" role="region" aria-label="Sales Report">
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
            aria-label="Start date"
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
            aria-label="End date"
          />

          <div
            className="sales-report-view-toggle"
            role="radiogroup"
            aria-label="View mode"
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
            variant="secondary"
            onClick={printReport}
            aria-label="Print report"
          >
            <Localized id="print">Print</Localized>
          </Button>
          <Button
            variant="secondary"
            onClick={exportCsv}
            aria-label="Export CSV"
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
          <BarChart data={revenueData as any}>
            <XAxis
              dataKey={revenueKey}
              tick={{ fontSize: 12 }}
            />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip
              formatter={(value: any) => fmtCurrency(Number(value), currency)}
            />
            <Bar
              dataKey="total_minor"
              fill="var(--color-accent, #4f46e5)"
              radius={[4, 4, 0, 0]}
              aria-label="Revenue"
            />
          </BarChart>
        </ResponsiveContainer>
        <div className="sales-report-totals">
          <span>
            <Localized id="sales-report-total-revenue">Total</Localized>:{' '}
            {fmtCurrency(totalRevenue, currency)}
          </span>
          <span>
            <Localized id="sales-report-total-orders">Orders</Localized>:{' '}
            {totalOrders}
          </span>
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
                  label={({ category_name, percentage }: any) =>
                    `${category_name} ${Number(percentage).toFixed(0)}%`
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
                  formatter={(value: any) => fmtCurrency(Number(value), currency)}
                />
                <Legend />
              </PieChart>
            </ResponsiveContainer>
          )}
        </Card>

        <Card shadow="sm" className="sales-report-chart-card">
          <Localized id="sales-report-top-products">
            <h2 className="sales-report-section-title">Top Products</h2>
          </Localized>
          {topProducts.length === 0 ? (
            <p className="sales-report-no-data">
              <Localized id="no-results">
                <span>No results</span>
              </Localized>
            </p>
          ) : (
            <div className="sales-report-top-table">
              <div className="sales-report-top-header">
                <span>#</span>
                <span>
                  <Localized id="top-products-name">Name</Localized>
                </span>
                <span>
                  <Localized id="top-products-quantity">Qty</Localized>
                </span>
                <span>
                  <Localized id="top-products-revenue">Revenue</Localized>
                </span>
              </div>
              {topProducts.map((p, i) => (
                <div key={p.product_id} className="sales-report-top-row">
                  <span>{i + 1}</span>
                  <span>{p.name}</span>
                  <span>{p.total_qty}</span>
                  <span>{fmtCurrency(p.total_minor, currency)}</span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

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
            aria-label="Hourly heatmap"
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
