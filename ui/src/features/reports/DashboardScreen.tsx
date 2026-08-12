//! Reports Dashboard — owner/admin landing page.
//!
//! Date range picker → granularity toggle → KPI bar (with deltas) →
//! revenue trend area chart + category donut → sales heatmap +
//! top products bar → low stock alerts.

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useCurrency } from '@/contexts/CurrencyContext';
import { requiredLocalized } from '@/frontend/shared';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import { minorUnitExponent } from '@/types/domain';
import ReactEChartsCore from 'echarts-for-react/lib/core';
import * as echarts from 'echarts/core';
import { BarChart, LineChart, PieChart, HeatmapChart } from 'echarts/charts';
import {
  GridComponent, TooltipComponent, LegendComponent, VisualMapComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import {
  getDailyRevenue, getWeeklyRevenue, getMonthlyRevenue,
  getTopProducts, getLowStockAlerts, getCategoryBreakdown, getHourlyHeatmap,
  type DailyRevenueRow, type WeeklyRevenueRow, type MonthlyRevenueRow,
  type TopProductRow, type LowStockAlert, type CategoryBreakdownRow,
  type HourlyHeatmapRow,
} from '@/api/reports';
import './DashboardScreen.css';

echarts.use([
  BarChart, LineChart, PieChart, HeatmapChart,
  GridComponent, TooltipComponent, LegendComponent, VisualMapComponent,
  CanvasRenderer,
]);

// ── Date helpers ───────────────────────────────────────────────────

function isoDay(d: Date): string { return d.toISOString().slice(0, 10); }
function today(): string { return isoDay(new Date()); }
function daysAgo(n: number): string { const d = new Date(); d.setDate(d.getDate() - n); return isoDay(d); }

type Granularity = 'daily' | 'weekly' | 'monthly';

// ── Currency formatting ────────────────────────────────────────────

function fmtCurrency(minor: number, currency: string): string {
  const exp = minorUnitExponent(currency);
  return new Intl.NumberFormat('en', { style: 'currency', currency,
    minimumFractionDigits: exp, maximumFractionDigits: exp,
  }).format(minor / 10 ** exp);
}

function fmtShort(minor: number, currency: string): string {
  const exp = minorUnitExponent(currency);
  const val = minor / 10 ** exp;
  if (Math.abs(val) >= 1_000_000) return `${val < 0 ? '-' : ''}${(Math.abs(val) / 1_000_000).toFixed(1)}M`;
  if (Math.abs(val) >= 1_000) return `${val < 0 ? '-' : ''}${(Math.abs(val) / 1_000).toFixed(1)}K`;
  return fmtCurrency(minor, currency);
}

function fmtDelta(current: number, previous: number): string {
  if (previous === 0) return current > 0 ? '+∞' : '−';
  const pct = ((current - previous) / previous) * 100;
  const sign = pct >= 0 ? '+' : '';
  return `${sign}${pct.toFixed(1)}%`;
}

// ── Component ───────────────────────────────────────────────────────

export default function DashboardScreen() {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const { currency } = useCurrency();

  const [granularity, setGranularity] = useState<Granularity>('daily');
  const [fromDraft, setFromDraft] = useState(daysAgo(29));
  const [toDraft, setToDraft] = useState(today());
  const [from, setFrom] = useState(daysAgo(29));
  const [to, setTo] = useState(today());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Current period
  const [dailyRevenue, setDailyRevenue] = useState<DailyRevenueRow[]>([]);
  const [weeklyRevenue, setWeeklyRevenue] = useState<WeeklyRevenueRow[]>([]);
  const [monthlyRevenue, setMonthlyRevenue] = useState<MonthlyRevenueRow[]>([]);
  const [topProducts, setTopProducts] = useState<TopProductRow[]>([]);
  const [lowStock, setLowStock] = useState<LowStockAlert[]>([]);
  const [categoryBreakdown, setCategoryBreakdown] = useState<CategoryBreakdownRow[]>([]);
  const [heatmap, setHeatmap] = useState<HourlyHeatmapRow[]>([]);

  // Previous period for deltas
  const [prevDaily, setPrevDaily] = useState<DailyRevenueRow[]>([]);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const days = (new Date(to).getTime() - new Date(from).getTime()) / 86400000 + 1;
      const prevFromDate = new Date(from);
      prevFromDate.setDate(prevFromDate.getDate() - days);
      const prevToDate = new Date(from);
      prevToDate.setDate(prevToDate.getDate() - 1);
      const prevFrom = isoDay(prevFromDate);
      const prevTo = isoDay(prevToDate);

      const [daily, weekly, monthly, top, stock, cats, heat, prev] = await Promise.all([
        getDailyRevenue(from, to, sessionToken),
        getWeeklyRevenue(from, to, sessionToken),
        getMonthlyRevenue(from, to, sessionToken),
        getTopProducts(from, to, 10, sessionToken, 'revenue'),
        getLowStockAlerts(10, sessionToken),
        getCategoryBreakdown(from, to, sessionToken),
        getHourlyHeatmap(from, to, sessionToken),
        getDailyRevenue(prevFrom, prevTo, sessionToken),
      ]);
      setDailyRevenue(daily);
      setWeeklyRevenue(weekly);
      setMonthlyRevenue(monthly);
      setTopProducts(top);
      setLowStock(stock);
      setCategoryBreakdown(cats);
      setHeatmap(heat);
      setPrevDaily(prev);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [sessionToken, from, to]);

  useEffect(() => { loadData(); }, [loadData]);

  // ── Revenue series for chart ─────────────────────────────────────

  const revenueSeries = useMemo(() => {
    switch (granularity) {
      case 'daily': return dailyRevenue.map((r) => ({ date: r.date, total: r.total_minor, profit: r.gross_profit_minor, count: r.sale_count }));
      case 'weekly': return weeklyRevenue.map((r) => ({ date: r.week_start, total: r.total_minor, profit: r.gross_profit_minor, count: r.sale_count }));
      case 'monthly': return monthlyRevenue.map((r) => ({ date: r.month, total: r.total_minor, profit: r.gross_profit_minor, count: r.sale_count }));
    }
  }, [granularity, dailyRevenue, weeklyRevenue, monthlyRevenue]);

  // ── KPI computed values with deltas ──────────────────────────────

  const todayKPIs = useMemo(() => {
    const todayRev = dailyRevenue.reduce((s, r) => s + r.total_minor, 0);
    const todayProfit = dailyRevenue.reduce((s, r) => s + r.gross_profit_minor, 0);
    const todayOrders = dailyRevenue.reduce((s, r) => s + r.sale_count, 0);
    const prevRev = prevDaily.reduce((s, r) => s + r.total_minor, 0);
    const prevProfit = prevDaily.reduce((s, r) => s + r.gross_profit_minor, 0);
    const prevOrders = prevDaily.reduce((s, r) => s + r.sale_count, 0);
    const top = topProducts[0];
    return {
      todayRev, todayProfit, todayOrders, top,
      prevRev, prevProfit, prevOrders,
      currency: dailyRevenue[0]?.currency ?? currency,
    };
  }, [dailyRevenue, prevDaily, topProducts, currency]);

  // ── ECharts options (same as before, trimmed for space) ──────────

  const revenueChartOption = useMemo(() => {
    const dates = revenueSeries.map((r) => r.date);
    return {
      tooltip: { trigger: 'axis' as const, valueFormatter: (val: unknown) => fmtCurrency(Number(val), currency) },
      legend: { data: [l10n.getString('dashboard-chart-revenue'), l10n.getString('dashboard-chart-profit')], top: 0 },
      grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
      xAxis: { type: 'category' as const, data: dates, axisLabel: { rotate: granularity === 'monthly' ? 45 : 0 } },
      yAxis: { type: 'value' as const, axisLabel: { formatter: (v: number) => fmtShort(v, currency) } },
      series: [
        { name: l10n.getString('dashboard-chart-revenue'), type: 'line' as const, data: revenueSeries.map((r) => r.total), areaStyle: { opacity: 0.15 }, smooth: true, itemStyle: { color: '#5470c6' }, symbol: 'circle', symbolSize: 4 },
        { name: l10n.getString('dashboard-chart-profit'), type: 'line' as const, data: revenueSeries.map((r) => r.profit), areaStyle: { opacity: 0.08 }, smooth: true, itemStyle: { color: '#91cc75' }, symbol: 'circle', symbolSize: 4 },
      ],
    };
  }, [revenueSeries, currency, granularity, l10n]);

  const categoryDonutOption = useMemo(() => {
    if (categoryBreakdown.length === 0) return null;
    const total = categoryBreakdown.reduce((s, c) => s + c.total_minor, 0);
    return {
      tooltip: { trigger: 'item' as const, valueFormatter: (val: unknown) => fmtCurrency(Number(val), currency) },
      legend: { orient: 'vertical' as const, right: 0, top: 'middle', textStyle: { fontSize: 11 } },
      series: [{
        name: l10n.getString('dashboard-chart-category'), type: 'pie' as const,
        radius: ['45%', '75%'], center: ['35%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 4, borderColor: 'var(--color-bg)', borderWidth: 2 },
        label: { show: false },
        emphasis: { label: { show: true, fontWeight: 'bold' } },
        data: categoryBreakdown.map((c) => ({ value: c.total_minor, name: c.category_name })),
      }],
      graphic: total > 0 ? [{ type: 'text' as const, left: '24%', top: 'middle',
        style: { text: fmtShort(total, currency), textAlign: 'center' as const, fill: 'var(--color-fg)', fontSize: 14, fontWeight: 'bold' } }] : undefined,
    };
  }, [categoryBreakdown, currency, l10n]);

  const heatmapOption = useMemo(() => {
    if (heatmap.length === 0) return null;
    const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const hours = Array.from({ length: 24 }, (_, i) => `${i}:00`);
    const maxVal = Math.max(...heatmap.map((h) => h.sale_count), 1);
    return {
      tooltip: { position: 'top' as const, formatter: (params: { value: [number, number, number] }) => `${dayNames[params.value[1]]} ${hours[params.value[0]]}: ${params.value[2]} orders` },
      grid: { left: 60, right: 20, bottom: 40, top: 10 },
      xAxis: { type: 'category' as const, data: hours, splitArea: { show: true }, axisLabel: { fontSize: 9, interval: 3 } },
      yAxis: { type: 'category' as const, data: dayNames, splitArea: { show: true } },
      visualMap: { min: 0, max: maxVal, calculable: true, orient: 'horizontal' as const, left: 'center', bottom: 0, inRange: { color: ['#ebedf0', '#9be9a8', '#40c463', '#30a14e', '#216e39'] }, itemWidth: 10, itemHeight: 80, textStyle: { fontSize: 9 } },
      series: [{ name: l10n.getString('dashboard-chart-heatmap'), type: 'heatmap' as const, data: heatmap.map((h) => [h.hour, h.day_of_week, h.sale_count]), label: { show: false }, emphasis: { itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0,0,0,0.5)' } } }],
    };
  }, [heatmap, l10n]);

  const topProductsOption = useMemo(() => {
    if (topProducts.length === 0) return null;
    const names = topProducts.map((p) => p.name);
    const values = topProducts.map((p) => p.total_minor);
    return {
      tooltip: { trigger: 'axis' as const, axisPointer: { type: 'shadow' as const }, valueFormatter: (val: unknown) => fmtCurrency(Number(val), currency) },
      grid: { left: '3%', right: '4%', bottom: '3%', top: 0, containLabel: true },
      xAxis: { type: 'value' as const, axisLabel: { formatter: (v: number) => fmtShort(v, currency) } },
      yAxis: { type: 'category' as const, data: names.reverse(), axisLabel: { width: 120, overflow: 'truncate', fontSize: 10 } },
      series: [{ name: l10n.getString('dashboard-chart-top-products'), type: 'bar' as const, barMaxWidth: 28,
        data: values.reverse().map((v) => ({ value: v, itemStyle: { color: new echarts.graphic.LinearGradient(0, 0, 1, 0, [{ offset: 0, color: '#5470c6' }, { offset: 1, color: '#91cc75' }]) } })),
      }],
    };
  }, [topProducts, currency, l10n]);

  // ── Render ───────────────────────────────────────────────────────

  if (loading) return <div className="dashboard"><Spinner aria-label={l10n.getString('spinner-label')} /></div>;
  if (error) return <div className="dashboard"><p className="dashboard-error">{error}</p></div>;

  return (
    <div className="dashboard" role="region" aria-label={requiredLocalized(l10n, 'dashboard-region-aria')}>
      {/* Header: title + date range + granularity */}
      <div className="dashboard-header">
        <Localized id="dashboard-title"><h1 className="dashboard-title">Dashboard</h1></Localized>
        <div className="dashboard-controls">
          <div className="dashboard-date-row">
            <input type="date" className="dashboard-date-input" value={fromDraft} max={toDraft}
              onChange={(e) => setFromDraft(e.target.value)} aria-label={l10n.getString('dashboard-filter-from')} />
            <span className="dashboard-date-sep">—</span>
            <input type="date" className="dashboard-date-input" value={toDraft} min={fromDraft}
              onChange={(e) => setToDraft(e.target.value)} aria-label={l10n.getString('dashboard-filter-to')} />
            <button type="button" className="dashboard-apply-btn" onClick={() => { setFrom(fromDraft); setTo(toDraft); }}
              aria-label={l10n.getString('dashboard-btn-apply')}>
              <Localized id="dashboard-btn-apply"><span>Apply</span></Localized>
            </button>
          </div>
          <div className="dashboard-granularity" role="radiogroup" aria-label={l10n.getString('dashboard-granularity-aria')}>
            {(['daily', 'weekly', 'monthly'] as Granularity[]).map((g) => (
              <button key={g} type="button" className={`dashboard-granularity-btn${granularity === g ? ' dashboard-granularity-btn--active' : ''}`}
                onClick={() => setGranularity(g)} role="radio" aria-checked={granularity === g}>
                <Localized id={`dashboard-granularity-${g}`}><span>{g}</span></Localized>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* ── KPI Row ──────────────────────────────────── */}
      <div className="dashboard-kpi-row">
        <Card shadow="sm" className="dashboard-kpi">
          <span className="dashboard-kpi-label"><Localized id="dashboard-today-revenue"><span>Revenue</span></Localized></span>
          <span className="dashboard-kpi-value">{fmtCurrency(todayKPIs.todayRev, todayKPIs.currency)}</span>
          <span className={`dashboard-kpi-delta${todayKPIs.todayRev >= todayKPIs.prevRev ? '' : ' dashboard-kpi-delta--down'}`}>
            {todayKPIs.prevRev > 0 ? fmtDelta(todayKPIs.todayRev, todayKPIs.prevRev) : ''}
          </span>
        </Card>
        <Card shadow="sm" className="dashboard-kpi">
          <span className="dashboard-kpi-label"><Localized id="dashboard-gross-profit"><span>Gross Profit</span></Localized></span>
          <span className={`dashboard-kpi-value${todayKPIs.todayProfit < 0 ? ' dashboard-kpi-negative' : ''}`}>
            {fmtCurrency(todayKPIs.todayProfit, todayKPIs.currency)}
          </span>
          <span className={`dashboard-kpi-delta${todayKPIs.todayProfit >= todayKPIs.prevProfit ? '' : ' dashboard-kpi-delta--down'}`}>
            {todayKPIs.prevProfit > 0 ? fmtDelta(todayKPIs.todayProfit, todayKPIs.prevProfit) : ''}
          </span>
        </Card>
        <Card shadow="sm" className="dashboard-kpi">
          <span className="dashboard-kpi-label"><Localized id="dashboard-orders-today"><span>Orders</span></Localized></span>
          <span className="dashboard-kpi-value">{todayKPIs.todayOrders}</span>
          <span className={`dashboard-kpi-delta${todayKPIs.todayOrders >= todayKPIs.prevOrders ? '' : ' dashboard-kpi-delta--down'}`}>
            {todayKPIs.prevOrders > 0 ? fmtDelta(todayKPIs.todayOrders, todayKPIs.prevOrders) : ''}
          </span>
        </Card>
        <Card shadow="sm" className="dashboard-kpi">
          <span className="dashboard-kpi-label"><Localized id="dashboard-top-product"><span>Top Product</span></Localized></span>
          <span className="dashboard-kpi-value dashboard-kpi-value--name">{todayKPIs.top?.name ?? '-'}</span>
        </Card>
      </div>

      {/* ── Revenue trend + Category donut ────────────── */}
      <div className="dashboard-chart-row">
        <Card shadow="sm" className="dashboard-chart-card">
          <Localized id="dashboard-chart-revenue"><h2 className="dashboard-section-title">Revenue Trend</h2></Localized>
          {revenueSeries.length > 0 ? (
            <ReactEChartsCore echarts={echarts} option={revenueChartOption} style={{ height: 320 }} notMerge aria-label={l10n.getString('dashboard-chart-revenue-aria')} />
          ) : <p className="dashboard-no-data"><Localized id="dashboard-no-data"><span>No data yet</span></Localized></p>}
        </Card>
        <Card shadow="sm" className="dashboard-chart-card">
          <Localized id="dashboard-chart-category-breakdown"><h2 className="dashboard-section-title">Category Breakdown</h2></Localized>
          {categoryDonutOption ? (
            <ReactEChartsCore echarts={echarts} option={categoryDonutOption} style={{ height: 320 }} notMerge aria-label={l10n.getString('dashboard-chart-category-aria')} />
          ) : <p className="dashboard-no-data"><Localized id="dashboard-no-data"><span>No data yet</span></Localized></p>}
        </Card>
      </div>

      {/* ── Heatmap + Top Products ─────────────────────── */}
      <div className="dashboard-chart-row">
        <Card shadow="sm" className="dashboard-chart-card">
          <Localized id="dashboard-chart-heatmap"><h2 className="dashboard-section-title">Sales Heatmap</h2></Localized>
          {heatmapOption ? (
            <ReactEChartsCore echarts={echarts} option={heatmapOption} style={{ height: 280 }} notMerge aria-label={l10n.getString('dashboard-chart-heatmap-aria')} />
          ) : <p className="dashboard-no-data"><Localized id="dashboard-heatmap-empty"><span>No heatmap data yet</span></Localized></p>}
        </Card>
        <Card shadow="sm" className="dashboard-chart-card">
          <Localized id="dashboard-chart-top-products"><h2 className="dashboard-section-title">Top 10 Products</h2></Localized>
          {topProductsOption ? (
            <ReactEChartsCore echarts={echarts} option={topProductsOption} style={{ height: 280 }} notMerge aria-label={l10n.getString('dashboard-chart-top-products-aria')} />
          ) : <p className="dashboard-no-data"><Localized id="dashboard-no-data"><span>No data yet</span></Localized></p>}
        </Card>
      </div>

      {/* ── Low Stock Alerts ──────────────────────────── */}
      <Card shadow="sm" className="dashboard-section">
        <Localized id="dashboard-low-stock-alerts"><h2 className="dashboard-section-title">Low Stock Alerts</h2></Localized>
        {lowStock.length === 0 ? (
          <p className="dashboard-no-data"><Localized id="dashboard-stock-ok"><span>All stock levels are healthy.</span></Localized></p>
        ) : (
          <ul className="dashboard-low-stock-list" aria-label={requiredLocalized(l10n, 'dashboard-stock-alerts-aria')}>
            {lowStock.map((item) => (
              <li key={item.product_id} className="dashboard-low-stock-item">
                <span className="dashboard-low-stock-name">{item.name}</span>
                <span className="dashboard-low-stock-qty">{item.current_qty} {l10n.getString('dashboard-stock-left')}</span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
