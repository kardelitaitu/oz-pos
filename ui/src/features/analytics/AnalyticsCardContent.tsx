//! Designed card visuals for the analytics grid.
//!
//! Every non-heatmap card renders a purpose-built layout — KPI + trend
//! chart, donut, stacked bar, ranked list, or alert list — fed by
//! deterministic demo data. The generators are stand-ins until the real
//! analytics IPC commands are wired; the layouts stay put when data lands.
//!
//! Charts use echarts (via echarts-for-react), matching the reports
//! DashboardScreen so the analytics page shares the same chart stack.

import { type ReactNode, useMemo } from 'react';
import { useLocalization } from '@fluent/react';
import ReactEChartsCore from 'echarts-for-react/lib/core';
import * as echarts from 'echarts/core';
import { BarChart as EBar, LineChart as ELine, PieChart as EPie } from 'echarts/charts';
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import { useCurrency } from '@/contexts/CurrencyContext';
import { minorUnitExponent } from '@/types/domain';
import { useAnalyticsQuery } from './useAnalyticsQuery';
import { cardQueryKey } from './analytics-cache';
import type { MenuEngineeringRow } from '@/api/reports';
import {
  CARD_LOADERS,
  seriesDelta,
  type AnalyticsQuery,
  type Bucket,
  type RankRow,
} from './analytics-data';
import type {
  BasketSizeRow,
  CategoryBreakdownRow,
  CustomerSplitRow,
  DiscountsSummaryRow,
  InventoryTrendRow,
  InventoryTurnoverRow,
  LowStockAlert,
  PaymentMethodRow,
  TopProductRow,
  VoidedItemRow,
  VoidedSummaryRow,
} from './analytics-data';
import type { StaffAnalyticsRow } from './analytics-data';
import type { Granularity, WorkspaceView } from './AnalyticsScreen';

echarts.use([EBar, ELine, EPie, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

// ── Deterministic demo data ─────────────────────────────────────────
// The same (card, granularity) always yields the same numbers so tests,
// screenshots, and re-renders are stable, while values still vary
// plausibly per bucket.

/** FNV-1a-seeded PRNG — reproducible series per seed string. */
function seeded(seed: string): () => number {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h ^= h << 13;
    h ^= h >>> 17;
    h ^= h << 5;
    h |= 0;
    return ((h >>> 0) % 10000) / 10000;
  };
}

const BUCKET_LABELS: Record<Granularity, string[]> = {
  daily: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
  weekly: ['W1', 'W2', 'W3', 'W4'],
  monthly: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
  yearly: ['Q1', 'Q2', 'Q3', 'Q4'],
  custom: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
};

/** Service hours shown on the occupancy curve, with lunch ≈ 12:00 and dinner ≈ 19:00. */
const OCCUPANCY_HOURS = ['09', '10', '11', '12', '13', '14', '15', '16', '17', '18', '19', '20', '21'];
/** Relative occupancy weight per hour — twin-peak (lunch/dinner) service shape. */
const OCCUPANCY_SHAPE = [0.4, 0.5, 0.65, 0.95, 0.85, 0.55, 0.5, 0.55, 0.75, 0.9, 0.8, 0.55, 0.4];

/** Per-bucket values around `base`, deterministic per key + granularity. */
function series(key: string, g: Granularity, base: number, spread: number): Bucket[] {
  const rnd = seeded(`${key}:${g}`);
  return BUCKET_LABELS[g]!.map((label) => ({
    label,
    value: Math.round(base * (0.6 + rnd() * spread)),
  }));
}

/** Deterministic change vs. the previous period (−6% … +12%). */
function deltaFor(key: string): number {
  const rnd = seeded(`delta:${key}`);
  return Math.round((rnd() * 18 - 6) * 10) / 10;
}

// ── Money formatting (mirrors reports DashboardScreen) ───────────────

function useMoney() {
  const { currency } = useCurrency();
  const exp = minorUnitExponent(currency);
  const fmt = (minor: number) =>
    new Intl.NumberFormat('en', { style: 'currency', currency, maximumFractionDigits: exp }).format(minor / 10 ** exp);
  const short = (minor: number) =>
    new Intl.NumberFormat('en', {
      style: 'currency',
      currency,
      notation: 'compact',
      maximumFractionDigits: 1,
    }).format(minor / 10 ** exp);
  return { fmt, short };
}

const PALETTE = ['#4f46e5', '#3b82f6', '#06b6d4', '#22c55e', '#f59e0b', '#f97316', '#ef4444', '#8b5cf6'];

const CHART_TEXT = '#94a3b8';

// ── Shared building blocks ──────────────────────────────────────────

/**
 * Wrapper that hosts the content. `demo` marks cards still fed by
 * deterministic placeholder data — only those show the honesty chip;
 * real-data cards never advertise demo data.
 */
function Visual({ className, children, demo }: { className?: string; children: ReactNode; demo?: boolean }) {
  const { l10n } = useLocalization();
  return (
    <div className={`analytics-card-visual${className ? ` ${className}` : ''}`}>
      {children}
      {demo && <span className="analytics-card-demo-chip">{l10n.getString('analytics-card-demo')}</span>}
    </div>
  );
}

/** Shown while a real-data card's IPC query is still in flight. */
function CardLoading() {
  return (
    <div className="analytics-card-skeleton">
      <div className="skeleton-bar skeleton-bar--sm" />
      <div className="skeleton-bar skeleton-bar--lg" />
      <div className="skeleton-bar skeleton-bar--md" />
    </div>
  );
}

/** Big KPI number with a small caption underneath. */
function Kpi({ value, label, tone }: { value: string; label: string; tone?: 'good' | 'bad' }) {
  return (
    <div className="analytics-kpi">
      <span className={`analytics-kpi-value${tone ? ` analytics-kpi-value--${tone}` : ''}`}>{value}</span>
      <span className="analytics-kpi-label">{label}</span>
    </div>
  );
}

/** Small delta pill (▲/▼ % vs previous period). */
function DeltaChip({ value }: { value: number }) {
  const { l10n } = useLocalization();
  const up = value >= 0;
  return (
    <span className={`analytics-delta${up ? ' analytics-delta--up' : ' analytics-delta--down'}`}>
      {up ? '▲' : '▼'} {Math.abs(value).toFixed(1)}% {l10n.getString('analytics-card-vs-prev')}
    </span>
  );
}

/** Compact ranked list with proportional bars — no chart lib needed. */
function RankedList({ rows, ariaLabel, limit }: { rows: RankRow[]; ariaLabel: string; limit?: number | undefined }) {
  const { l10n } = useLocalization();
  const shown = limit !== undefined ? rows.slice(0, limit) : rows;
  const max = Math.max(...shown.map((r) => r.value), 1);
  return (
    <ul className="analytics-rank-list" aria-label={ariaLabel}>
      {shown.map((r, i) => (
        <li
          key={r.name}
          className="analytics-rank-row"
          aria-label={r.delta !== undefined
            ? l10n.getString('analytics-rank-delta-aria', {
                name: r.name,
                dir: l10n.getString(r.delta >= 0 ? 'analytics-rank-up' : 'analytics-rank-down'),
                pct: Math.abs(r.delta).toFixed(1),
              })
            : undefined}
        >
          <span className="analytics-rank-index">{i + 1}</span>
          <span className="analytics-rank-name">{r.name}</span>
          <span className="analytics-rank-bar-track">
            <span className="analytics-rank-bar" style={{ width: `${(r.value / max) * 100}%` }} />
          </span>
          {r.delta !== undefined && (
            <span
              className={`analytics-rank-delta${r.delta >= 0 ? ' analytics-rank-delta--up' : ' analytics-rank-delta--down'}`}
              aria-hidden="true"
            >
              {r.delta >= 0 ? '▲' : '▼'} {Math.abs(r.delta).toFixed(1)}%
            </span>
          )}
          <span className="analytics-rank-value">{r.display}</span>
        </li>
      ))}
    </ul>
  );
}

/** Color dot + name + value legend (used beside donuts and stacked bars). */
function Legend({ items }: { items: { name: string; value: string; color: string }[] }) {
  return (
    <ul className="analytics-legend">
      {items.map((it) => (
        <li key={it.name} className="analytics-legend-item">
          <span className="analytics-legend-dot" style={{ background: it.color }} />
          <span className="analytics-legend-name">{it.name}</span>
          <span className="analytics-legend-value">{it.value}</span>
        </li>
      ))}
    </ul>
  );
}

// ── Per-card layouts ────────────────────────────────────────────────

/**
 * Per-card cached query — keyed by (card, workspace, granularity, range)
 * so an identical query revisits the TTL cache instead of refetching.
 *
 * Real-data cards omit `fetchData` and load through `CARD_LOADERS`;
 * demo-only cards (tables, occupancy) pass their deterministic fetcher.
 * Returns `null` while an async query is in flight.
 */
function useCardData<T>(
  cardKey: string,
  q: AnalyticsQuery,
  fetchData?: () => T | Promise<T>,
): T | null {
  const { data } = useAnalyticsQuery(
    cardQueryKey(cardKey, q.workspace, q.granularity, q.from, q.to),
    () => {
      if (fetchData) return fetchData();
      const loader = CARD_LOADERS[cardKey] as ((query: AnalyticsQuery) => Promise<T>) | undefined;
      if (!loader) return null as T;
      return loader(q);
    },
  );
  return data as T | null;
}

function RevenueCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt, short } = useMoney();
  const data = useCardData<Bucket[]>('revenue', q);
  const total = data ? data.reduce((s, d) => s + d.value, 0) : 0;
  const peak = data && data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data && data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  const delta = data ? seriesDelta(data) : null;
  const option = useMemo(() => (data ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      name: l10n.getString('analytics-card-revenue'),
      type: 'line' as const, data: data.map((d) => d.value),
      smooth: true, symbol: 'circle', symbolSize: 4,
      itemStyle: { color: '#4f46e5' }, areaStyle: { opacity: 0.12 }, lineStyle: { width: 2 },
    }],
  }) : null), [data, fmt, l10n]);
  if (!data) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--revenue">
      <div className="analytics-kpi-row">
        <Kpi value={short(total)} label={l10n.getString('analytics-card-total-revenue')} />
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 240 : 104 }} notMerge />
      </div>
      {peak && <p className="analytics-card-insight">{l10n.getString('analytics-card-peak', { label: peak.label, value: short(peak.value) })}</p>}
      {low && <p className="analytics-card-insight">{l10n.getString('analytics-card-low', { label: low.label, value: short(low.value) })}</p>}
    </Visual>
  );
}

function AovCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const data = useCardData<Bucket[]>('aov', q);
  const avg = data && data.length ? Math.round(data.reduce((s, d) => s + d.value, 0) / data.length) : 0;
  const peak = data && data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data && data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  const delta = data ? seriesDelta(data) : null;
  const option = useMemo(() => (data ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'line' as const, data: data.map((d) => d.value),
      smooth: true, symbol: 'circle', symbolSize: 4,
      itemStyle: { color: '#4f46e5' }, areaStyle: { opacity: 0.12 }, lineStyle: { width: 2 },
    }],
  }) : null), [data, fmt]);
  if (!data) return <CardLoading />;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={fmt(avg)} label={l10n.getString('analytics-card-aov')} />
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 240 : 104 }} notMerge />
      </div>
      {peak && <p className="analytics-card-insight">{l10n.getString('analytics-card-peak', { label: peak.label, value: fmt(peak.value) })}</p>}
      {low && <p className="analytics-card-insight">{l10n.getString('analytics-card-low', { label: low.label, value: fmt(low.value) })}</p>}
    </Visual>
  );
}

function StaffCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { short } = useMoney();
  const staff = useCardData<StaffAnalyticsRow[]>('staff', q);
  if (!staff) return <CardLoading />;
  const rows: RankRow[] = staff
    .slice()
    .sort((a, b) => b.sale_total_minor - a.sale_total_minor)
    .map((r) => ({ name: r.display_name, value: r.sale_total_minor, display: short(r.sale_total_minor) }));
  const totalSales = rows.reduce((s, r) => s + r.value, 0);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={short(totalSales)} label={l10n.getString('analytics-card-staff-sales')} />
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function CustomersCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const split = useCardData<CustomerSplitRow>('customers', q);
  const newCount = split ? split.new_count : 0;
  const retCount = split ? split.returning_count : 0;
  const total = newCount + retCount;
  const newPct = total > 0 ? Math.round((newCount / total) * 100) : 0;
  const option = useMemo(() => (split ? ({
    tooltip: { trigger: 'item' as const },
    series: [{
      type: 'pie' as const, radius: ['58%', '82%'], center: ['50%', '50%'],
      itemStyle: { borderRadius: 4, borderColor: '#fff', borderWidth: 2 },
      label: { show: false }, emphasis: { scaleSize: 4 },
      data: [
        { value: newCount, name: l10n.getString('analytics-card-customers-new'), itemStyle: { color: '#4f46e5' } },
        { value: retCount, name: l10n.getString('analytics-card-customers-returning'), itemStyle: { color: '#c7d2fe' } },
      ],
    }],
  }) : null), [newCount, retCount, l10n]);
  if (!split) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        <Kpi value={String(total)} label={l10n.getString('analytics-card-customers-total')} />
      </div>
      <div className="analytics-card-chart analytics-card-chart--donut" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 210 : 118 }} notMerge />
      </div>
      <Legend items={[
        { name: l10n.getString('analytics-card-customers-new'), value: String(newCount), color: '#4f46e5' },
        { name: l10n.getString('analytics-card-customers-returning'), value: String(retCount), color: '#c7d2fe' },
      ]} />
      <p className="analytics-card-insight">
        {l10n.getString('analytics-card-customers-new-share', { pct: String(newPct) })}
      </p>
    </Visual>
  );
}

const PAYMENT_NAMES: Record<string, string> = {
  cash: 'analytics-card-payments-cash',
  card: 'analytics-card-payments-card',
  qris: 'analytics-card-payments-qris',
  ewallet: 'analytics-card-payments-ewallet',
};

function PaymentsCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const rows = useCardData<PaymentMethodRow[]>('payments', q);
  const total = rows ? rows.reduce((s, r) => s + r.total_minor, 0) : 0;
  const segs = rows
    ? rows.map((r) => ({
        key: r.payment_method,
        name: PAYMENT_NAMES[r.payment_method] ? l10n.getString(PAYMENT_NAMES[r.payment_method]!) : r.payment_method,
        pct: total > 0 ? Math.round((r.total_minor / total) * 100) : 0,
      }))
    : [];
  const pcts = segs.map((s) => s.pct);
  const topPct = pcts.length ? Math.max(...pcts) : 0;
  const topSeg = segs[pcts.indexOf(topPct)];
  const option = useMemo(() => (segs.length ? ({
    grid: { left: 8, right: 8, top: 8, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, axisPointer: { type: 'shadow' as const }, valueFormatter: (v: unknown) => `${v}%` },
    xAxis: { type: 'value' as const, show: false },
    yAxis: { type: 'category' as const, data: [l10n.getString('analytics-card-payments')], show: false },
    series: segs.map((s, i) => ({
      name: s.name, type: 'bar' as const, stack: 'total', barWidth: 16,
      itemStyle: {
        color: PALETTE[i % PALETTE.length],
        borderRadius: i === segs.length - 1 ? [0, 4, 4, 0] : 0,
      },
      data: [pcts[i]],
    })),
  }) : null), [segs, pcts, l10n]);
  if (!rows) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        {topSeg && <Kpi value={`${topSeg.name} · ${topPct}%`} label={l10n.getString('analytics-card-payments-top')} />}
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 180 : 84 }} notMerge />
      </div>
      <Legend items={segs.map((s, i) => ({ name: s.name, value: `${s.pct}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function DiscountsCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const summary = useCardData<DiscountsSummaryRow>('discounts', q);
  if (!summary) return <CardLoading />;
  const rows: RankRow[] = summary.codes.map((c) => ({
    name: c.label,
    value: c.redeemed_count,
    display: `${c.redeemed_count} ${l10n.getString('analytics-card-discounts-redeemed')}`,
  }));
  const discountShare = summary.share_percent;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={`${discountShare.toFixed(1)}%`} label={l10n.getString('analytics-card-discounts-share')} />
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function RefundsCard({ q }: { q: AnalyticsQuery }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const summary = useCardData<VoidedSummaryRow>('refunds', q);
  if (!summary) return <CardLoading />;
  const count = summary.void_count;
  const amount = summary.void_total_minor;
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={String(count)} label={l10n.getString('analytics-card-refunds-count')} tone="bad" />
        <Kpi value={fmt(amount)} label={l10n.getString('analytics-card-refunds-amount')} tone="bad" />
      </div>
    </Visual>
  );
}

function TopItemsCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { short } = useMoney();
  const raw = useCardData<(TopProductRow | MenuEngineeringRow)[]>('top-items', q);
  if (!raw) return <CardLoading />;
  const rows: RankRow[] = raw.map((r) => {
    if ('total_qty' in r) {
      return { name: r.name, value: r.total_minor, display: `${short(r.total_minor)} · ${r.total_qty}×` };
    }
    return { name: r.name, value: r.total_revenue_minor, display: `${short(r.total_revenue_minor)} · ${r.total_volume}×` };
  });
  return (
    <Visual>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function CategoryCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const rows = useCardData<CategoryBreakdownRow[]>('category', q);
  const names = rows ? rows.map((r) => r.category_name).slice(0, 8) : [];
  const pcts = rows ? rows.map((r) => Math.round(r.percentage)).slice(0, 8) : [];
  const topName = pcts.length ? names[pcts.indexOf(Math.max(...pcts))] : '';
  const option = useMemo(() => (names.length ? ({
    tooltip: { trigger: 'item' as const },
    series: [{
      type: 'pie' as const, radius: ['58%', '82%'], center: ['50%', '50%'],
      itemStyle: { borderRadius: 4, borderColor: '#fff', borderWidth: 2 },
      label: { show: false }, emphasis: { scaleSize: 4 },
      data: names.map((n, i) => ({ value: pcts[i], name: n, itemStyle: { color: PALETTE[i % PALETTE.length] } })),
    }],
  }) : null), [names, pcts]);
  if (!rows) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        {topName && <Kpi value={topName} label={l10n.getString('analytics-card-category-top')} />}
      </div>
      <div className="analytics-card-chart analytics-card-chart--donut" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 210 : 118 }} notMerge />
      </div>
      <Legend items={names.map((n, i) => ({ name: n, value: `${pcts[i]}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function BasketCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { granularity: g } = q;
  const basket = useCardData<BasketSizeRow>('basket', q);
  const avg = basket ? basket.avg_line_count : 0;
  // Per-bucket basket size is not surfaced by the backend; show the range
  // average as a flat reference line across the period's buckets.
  const data: Bucket[] = basket ? BUCKET_LABELS[g]!.map((label) => ({ label, value: avg })) : [];
  const peak = data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  const option = useMemo(() => (data.length ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'bar' as const, data: data.map((d) => d.value),
      itemStyle: { color: '#06b6d4', borderRadius: [3, 3, 0, 0] }, barWidth: '55%',
    }],
  }) : null), [data]);
  if (!basket) return <CardLoading />;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={avg > 0 ? avg.toFixed(1) : '—'} label={l10n.getString('analytics-card-basket-items')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 240 : 104 }} notMerge />
      </div>
      {peak && <p className="analytics-card-insight">{l10n.getString('analytics-card-peak', { label: peak.label, value: peak.value.toFixed(1) })}</p>}
      {low && <p className="analytics-card-insight">{l10n.getString('analytics-card-low', { label: low.label, value: low.value.toFixed(1) })}</p>}
    </Visual>
  );
}

function InventoryCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const loaded = useCardData<[InventoryTurnoverRow, InventoryTrendRow[]]>('inventory', q);
  const [turnoverRow, trend]: [InventoryTurnoverRow | null, InventoryTrendRow[]] = loaded ?? [null, []];
  const turnover = turnoverRow && turnoverRow.stock_on_hand > 0 ? turnoverRow.units_sold / turnoverRow.stock_on_hand : 0;
  const data: Bucket[] = trend.map((t) => ({ label: t.date.slice(5), value: t.units_sold }));
  const skus = turnoverRow ? turnoverRow.sku_count : 0;
  const daysOfStock = turnoverRow && turnover > 0 ? Math.max(1, Math.round(turnoverRow.range_days / turnover)) : 0;
  const option = useMemo(() => (data.length ? ({
    grid: { left: 8, right: 8, top: 10, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'line' as const, data: data.map((d) => d.value),
      smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#22c55e' },
      areaStyle: { opacity: 0.12 }, itemStyle: { color: '#22c55e' },
    }],
  }) : null), [data]);
  if (!loaded) return <CardLoading />;
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={turnover > 0 ? `${turnover.toFixed(1)}×` : '—'} label={l10n.getString('analytics-card-inventory-turnover')} />
        <Kpi value={daysOfStock > 0 ? `${daysOfStock}d` : '—'} label={l10n.getString('analytics-card-inventory-days')} />
        <Kpi value={String(skus)} label={l10n.getString('analytics-card-inventory-skus')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 170 : 80 }} notMerge />
      </div>
    </Visual>
  );
}

function LowStockCard({ q, title }: { q: AnalyticsQuery; title: string }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const alerts = useCardData<LowStockAlert[]>('low-stock', q);
  if (!alerts) return <CardLoading />;
  const rows = alerts.map((a) => ({
    name: a.name,
    stock: a.current_qty,
    reorder: Math.max(0, a.threshold - a.current_qty),
    cost: a.cost_minor,
  }));
  const restockCost = rows.reduce((s, r) => s + r.reorder * r.cost, 0);
  const criticalCount = rows.filter((r) => r.stock <= 5).length;
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={fmt(restockCost)} label={l10n.getString('analytics-card-low-stock-restock')} tone="bad" />
        <Kpi value={String(rows.length)} label={l10n.getString('analytics-card-low-stock-items')} />
        <Kpi value={String(criticalCount)} label={l10n.getString('analytics-card-low-stock-critical')} tone="bad" />
      </div>
      <ul className="analytics-alert-list" aria-label={title}>
        {rows.map((r) => {
          const critical = r.stock <= 5;
          return (
            <li key={r.name} className="analytics-alert-row">
              <span className={`analytics-alert-dot${critical ? ' analytics-alert-dot--critical' : ' analytics-alert-dot--warn'}`} />
              <span className="analytics-alert-name">{r.name}</span>
              <span className="analytics-alert-count">{r.stock} {l10n.getString('analytics-card-low-stock-left')}</span>
              <span className="analytics-alert-reorder">{l10n.getString('analytics-card-low-stock-order', { n: r.reorder })}</span>
            </li>
          );
        })}
      </ul>
    </Visual>
  );
}

function TablesCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { granularity: g } = q;
  const { l10n } = useLocalization();
  const data = useCardData<Bucket[]>('tables', q, () => series('tables', g, 46, 0.9)) ?? [];
  const avgTurn = Math.round(data.reduce((s, d) => s + d.value, 0) / data.length);
  const peak = data.reduce((a, b) => (b.value > a.value ? b : a));
  const low = data.reduce((a, b) => (b.value < a.value ? b : a));
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => `${v}m` },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'bar' as const, data: data.map((d) => d.value),
      itemStyle: { color: '#f59e0b', borderRadius: [3, 3, 0, 0] }, barWidth: '55%',
    }],
  }), [data]);
  return (
    <Visual demo>
      <div className="analytics-kpi-row">
        <Kpi value={`${avgTurn}m`} label={l10n.getString('analytics-card-tables-turn')} />
        <DeltaChip value={deltaFor('tables')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: expanded ? 240 : 104 }} notMerge />
      </div>
      <p className="analytics-card-insight">{l10n.getString('analytics-card-peak', { label: peak.label, value: `${peak.value}m` })}</p>
      <p className="analytics-card-insight">{l10n.getString('analytics-card-low', { label: low.label, value: `${low.value}m` })}</p>
    </Visual>
  );
}

function OccupancyCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { granularity: g } = q;
  const { l10n } = useLocalization();
  const occ = useCardData<{ rate: number; peak: number; hourly: { hour: string; pct: number }[] }>(
    'occupancy',
    q,
    () => {
      const rate = 60 + Math.round(seeded(`occupancy:${g}`)() * 25);
      const peak = 17 + Math.round(seeded(`occupancy-peak:${g}`)() * 4);
      const r = seeded(`occupancy-hourly:${g}`);
      const hourly = OCCUPANCY_HOURS.map((hour, i) => ({
        hour,
        pct: Math.min(100, Math.round(OCCUPANCY_SHAPE[i]! * rate + r() * 6)),
      }));
      return { rate, peak, hourly };
    },
  );
  const rate = occ ? occ.rate : 0;
  const peak = occ ? occ.peak : 0;
  const option = useMemo(() => (occ ? ({
    grid: { left: 8, right: 8, top: 8, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => `${v}%` },
    xAxis: {
      type: 'category' as const, data: occ.hourly.map((d) => d.hour),
      axisLabel: { fontSize: 9, color: CHART_TEXT, interval: 1 }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false, max: 100 },
    series: [{
      type: 'line' as const, data: occ.hourly.map((d) => d.pct),
      smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#f59e0b' },
      areaStyle: { opacity: 0.12 }, itemStyle: { color: '#f59e0b' },
    }],
  }) : null), [occ]);
  if (!occ) return <CardLoading />;
  return (
    <Visual demo>
      <div className="analytics-occupancy">
        <div className="analytics-occupancy-head">
          <span className="analytics-occupancy-value">{rate}%</span>
          <span className="analytics-occupancy-label">{l10n.getString('analytics-card-occupancy-occupied')}</span>
        </div>
        <div className="analytics-occupancy-track" role="img" aria-label={title}>
          <span className="analytics-occupancy-fill" style={{ width: `${rate}%` }} />
        </div>
        <div className="analytics-occupancy-meta">
          <span>{l10n.getString('analytics-card-occupancy-peak')} · {String(peak).padStart(2, '0')}:00</span>
        </div>
        <div className="analytics-card-chart" role="img" aria-label={l10n.getString('analytics-card-occupancy-hourly')}>
          <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 150 : 64 }} notMerge />
        </div>
      </div>
    </Visual>
  );
}

function WaitstaffCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { short } = useMoney();
  const staff = useCardData<StaffAnalyticsRow[]>('waitstaff', q);
  if (!staff) return <CardLoading />;
  const rows: RankRow[] = staff
    .slice()
    .sort((a, b) => b.sale_total_minor - a.sale_total_minor)
    .map((r) => ({ name: r.display_name, value: r.sale_total_minor, display: short(r.sale_total_minor) }));
  const totalSales = rows.reduce((s, r) => s + r.value, 0);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={short(totalSales)} label={l10n.getString('analytics-card-waitstaff-total')} />
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function VoidsCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const loaded = useCardData<[VoidedSummaryRow, VoidedItemRow[]]>('voids', q);
  if (!loaded) return <CardLoading />;
  const items = loaded[1];
  const rows: RankRow[] = items.map((it) => ({ name: it.name, value: it.qty, display: `${it.qty}×` }));
  const totalQty = rows.reduce((s, r) => s + r.value, 0);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={String(totalQty)} label={l10n.getString('analytics-card-voids-count')} tone="bad" />
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

// ── Dispatcher ──────────────────────────────────────────────────────

export interface AnalyticsCardContentProps {
  /** Card key from ANALYTICS_CARDS (e.g. 'revenue', 'top-items'). */
  cardKey: string;
  granularity: Granularity;
  /** Workspace view — differentiates retail vs restaurant card data. */
  workspaceView: WorkspaceView;
  /** Inclusive date range backing the query (derived from granularity). */
  from: string;
  to: string;
  /** Session token for the scoped reporting commands. */
  sessionToken: string;
  /** Localized card title, used as the chart's accessible name. */
  title: string;
  /** When true the card fills the main area — charts grow and lists uncap. */
  expanded?: boolean | undefined;
}

/** Renders the designed content for a non-heatmap analytics card. */
export function AnalyticsCardContent({
  cardKey,
  granularity,
  workspaceView,
  from,
  to,
  sessionToken,
  title,
  expanded,
}: AnalyticsCardContentProps) {
  const q: AnalyticsQuery = { workspace: workspaceView, granularity, from, to, sessionToken };
  switch (cardKey) {
    case 'revenue': return <RevenueCard q={q} title={title} expanded={expanded} />;
    case 'aov': return <AovCard q={q} title={title} expanded={expanded} />;
    case 'staff': return <StaffCard q={q} title={title} expanded={expanded} />;
    case 'customers': return <CustomersCard q={q} title={title} expanded={expanded} />;
    case 'payments': return <PaymentsCard q={q} title={title} expanded={expanded} />;
    case 'discounts': return <DiscountsCard q={q} title={title} expanded={expanded} />;
    case 'refunds': return <RefundsCard q={q} />;
    case 'top-items': return <TopItemsCard q={q} title={title} expanded={expanded} />;
    case 'category': return <CategoryCard q={q} title={title} expanded={expanded} />;
    case 'basket': return <BasketCard q={q} title={title} expanded={expanded} />;
    case 'inventory': return <InventoryCard q={q} title={title} expanded={expanded} />;
    case 'low-stock': return <LowStockCard q={q} title={title} />;
    case 'tables': return <TablesCard q={q} title={title} expanded={expanded} />;
    case 'occupancy': return <OccupancyCard q={q} title={title} expanded={expanded} />;
    case 'waitstaff': return <WaitstaffCard q={q} title={title} expanded={expanded} />;
    case 'voids': return <VoidsCard q={q} title={title} expanded={expanded} />;
    default: return null;
  }
}
