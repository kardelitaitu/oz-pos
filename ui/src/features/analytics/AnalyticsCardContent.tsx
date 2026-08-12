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

interface Bucket {
  label: string;
  value: number;
}

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

/** Deterministic per-row change vs. the previous period (−9% … +11%). */
function rowDeltas(key: string, count: number): number[] {
  return Array.from({ length: count }, (_, i) => {
    const rnd = seeded(`${key}-trend:${i}`);
    return Math.round((rnd() * 20 - 9) * 10) / 10;
  });
}

/** Deterministic percentage parts (rounded, sum ≈ 100). */
function shares(key: string, count: number): number[] {
  const rnd = seeded(`shares:${key}`);
  const raw = Array.from({ length: count }, () => 0.2 + rnd() * 0.8);
  const total = raw.reduce((s, v) => s + v, 0);
  return raw.map((v) => Math.round((v / total) * 100));
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

/** Wrapper that hosts the content + the "demo data" honesty chip. */
function Visual({ className, children }: { className?: string; children: ReactNode }) {
  const { l10n } = useLocalization();
  return (
    <div className={`analytics-card-visual${className ? ` ${className}` : ''}`}>
      {children}
      <span className="analytics-card-demo-chip">{l10n.getString('analytics-card-demo')}</span>
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

interface RankRow {
  name: string;
  value: number;
  display: string;
  /** Deterministic change vs the previous period (−9% … +11%); renders a trend arrow. */
  delta?: number;
}

/** Compact ranked list with proportional bars — no chart lib needed. */
function RankedList({ rows, ariaLabel }: { rows: RankRow[]; ariaLabel: string }) {
  const { l10n } = useLocalization();
  const max = Math.max(...rows.map((r) => r.value), 1);
  return (
    <ul className="analytics-rank-list" aria-label={ariaLabel}>
      {rows.map((r, i) => (
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
 * Per-card cached query — keyed by (card, workspace, granularity) so an
 * identical query revisits the TTL cache instead of recomputing. Sync
 * demo fetchers always resolve during render; `null` only occurs while
 * an async (real IPC) query is in flight.
 */
function useCardData<T>(cardKey: string, workspace: WorkspaceView, g: Granularity, fetchData: () => T): T {
  const { data } = useAnalyticsQuery(cardQueryKey(cardKey, workspace, g), fetchData);
  return data as T;
}

function RevenueCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { fmt, short } = useMoney();
  const data = useCardData('revenue', workspace, g, () => series('revenue', g, 18_500_000, 1.35));
  const total = data.reduce((s, d) => s + d.value, 0);
  const option = useMemo(() => ({
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
  }), [data, fmt, l10n]);
  return (
    <Visual className="analytics-card-visual--revenue">
      <div className="analytics-kpi-row">
        <Kpi value={short(total)} label={l10n.getString('analytics-card-total-revenue')} />
        <DeltaChip value={deltaFor('revenue')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 104 }} notMerge />
      </div>
    </Visual>
  );
}

function AovCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const data = useCardData('aov', workspace, g, () => series('aov', g, 187_000, 0.55));
  const avg = Math.round(data.reduce((s, d) => s + d.value, 0) / data.length);
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'bar' as const, data: data.map((d) => d.value),
      itemStyle: { color: '#4f46e5', borderRadius: [3, 3, 0, 0] }, barWidth: '55%',
    }],
  }), [data, fmt]);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={fmt(avg)} label={l10n.getString('analytics-card-aov')} />
        <DeltaChip value={deltaFor('aov')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 104 }} notMerge />
      </div>
    </Visual>
  );
}

function StaffCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { short } = useMoney();
  const names = ['Rina W.', 'Budi S.', 'Sari A.', 'Andi P.'];
  const rows = useCardData('staff', workspace, g, () => {
    const rnd = seeded(`staff:${g}`);
    const deltas = rowDeltas('staff', names.length);
    const values = names.map(() => Math.round(8_500_000 * (0.5 + rnd() * 1.1)));
    return names.map((name, i) => ({ name, value: values[i]!, display: short(values[i]!), delta: deltas[i]! }));
  });
  return (
    <Visual>
      <RankedList rows={rows} ariaLabel={title} />
    </Visual>
  );
}

function CustomersCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { newPct, total } = useCardData('customers', workspace, g, () => {
    const [newPct] = shares('customers', 2);
    const total = 1200 + Math.round(seeded(`customers-total:${g}`)() * 400);
    return { newPct: newPct!, total };
  });
  const newCount = Math.round((total * newPct) / 100);
  const retCount = total - newCount;
  const option = useMemo(() => ({
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
  }), [newCount, retCount, l10n]);
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-card-chart analytics-card-chart--donut" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 118 }} notMerge />
      </div>
      <Legend items={[
        { name: l10n.getString('analytics-card-customers-new'), value: String(newCount), color: '#4f46e5' },
        { name: l10n.getString('analytics-card-customers-returning'), value: String(retCount), color: '#c7d2fe' },
      ]} />
    </Visual>
  );
}

function PaymentsCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const segs = [
    { key: 'cash', name: l10n.getString('analytics-card-payments-cash') },
    { key: 'card', name: l10n.getString('analytics-card-payments-card') },
    { key: 'qris', name: l10n.getString('analytics-card-payments-qris') },
    { key: 'ewallet', name: l10n.getString('analytics-card-payments-ewallet') },
  ];
  const pcts = useCardData('payments', workspace, g, () => shares('payments', segs.length));
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 8, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, axisPointer: { type: 'shadow' as const } },
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
  }), [segs, pcts, l10n]);
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 84 }} notMerge />
      </div>
      <Legend items={segs.map((s, i) => ({ name: s.name, value: `${pcts[i]}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function DiscountsCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const names = ['WELCOME10', 'PROMO8.8', 'LOYALTY15', 'FREESHIP'];
  const rows = useCardData('discounts', workspace, g, () => {
    const rnd = seeded(`discounts:${g}`);
    const deltas = rowDeltas('discounts', names.length);
    const values = names.map(() => Math.round(140 * (0.4 + rnd() * 1.2)));
    return names.map((name, i) => ({ name, value: values[i]!, display: `${values[i]} ${l10n.getString('analytics-card-discounts-redeemed')}`, delta: deltas[i]! }));
  });
  // Share of sales from discounts (5–9%), derived deterministically per granularity.
  const discountShare = 5 + Math.round(seeded(`discount-share:${g}`)() * 4);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={`${discountShare}%`} label={l10n.getString('analytics-card-discounts-share')} />
        <DeltaChip value={deltaFor('discounts')} />
      </div>
      <RankedList rows={rows} ariaLabel={title} />
    </Visual>
  );
}

function RefundsCard({ g, workspace }: { g: Granularity; workspace: WorkspaceView }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const { data, count } = useCardData('refunds', workspace, g, () => ({
    data: series('refunds', g, 3_200_000, 1.1),
    count: Math.round(seeded(`refunds-count:${g}`)() * 28) + 6,
  }));
  const amount = data.reduce((s, d) => s + d.value, 0);
  const rate = Math.round((1 + seeded(`refunds-rate:${g}`)() * 15) * 10) / 10;
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 10, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [{
      type: 'bar' as const, data: data.map((d) => d.value),
      itemStyle: { color: '#ef4444', borderRadius: [3, 3, 0, 0] }, barWidth: '55%',
    }],
  }), [data, fmt]);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={`${rate.toFixed(1)}%`} label={l10n.getString('analytics-card-refunds-rate')} tone="bad" />
        <Kpi value={String(count)} label={l10n.getString('analytics-card-refunds-count')} tone="bad" />
        <Kpi value={fmt(amount)} label={l10n.getString('analytics-card-refunds-amount')} tone="bad" />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={l10n.getString('analytics-card-refunds')}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 64 }} notMerge />
      </div>
    </Visual>
  );
}

function TopItemsCard({ g, title, workspace }: { g: Granularity; title: string; workspace: WorkspaceView }) {
  const { short } = useMoney();
  const names = workspace === 'retail'
    ? ['CPU R7 7800X3D', 'RTX 4070 Ti S', 'DDR5 32GB', '990 PRO 2TB', 'B650-A ROG']
    : ['Caffè Latte', 'Chicken Sandwich', 'Butter Croissant', 'Matcha Latte', 'Espresso'];
  const rows = useCardData('top-items', workspace, g, () => {
    const rnd = seeded(`top-items:${g}:${workspace}`);
    const deltas = rowDeltas('top-items', names.length);
    const values = names.map(() => Math.round(6_500_000 * (0.5 + rnd() * 1.2)));
    return names.map((name, i) => ({ name, value: values[i]!, display: short(values[i]!), delta: deltas[i]! }));
  });
  return (
    <Visual>
      <RankedList rows={rows} ariaLabel={title} />
    </Visual>
  );
}

function CategoryCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { names, pcts } = useCardData('category', workspace, g, () => {
    const names = workspace === 'restaurant'
      ? ['Coffee', 'Pastry', 'Sandwiches', 'Beverages', 'Desserts']
      : ['CPU', 'GPU', 'RAM', 'Storage', 'Motherboard'];
    return { names, pcts: shares('category', names.length) };
  });
  const topName = names[pcts.indexOf(Math.max(...pcts))]!;
  const option = useMemo(() => ({
    tooltip: { trigger: 'item' as const },
    series: [{
      type: 'pie' as const, radius: ['58%', '82%'], center: ['50%', '50%'],
      itemStyle: { borderRadius: 4, borderColor: '#fff', borderWidth: 2 },
      label: { show: false }, emphasis: { scaleSize: 4 },
      data: names.map((n, i) => ({ value: pcts[i], name: n, itemStyle: { color: PALETTE[i % PALETTE.length] } })),
    }],
  }), [names, pcts]);
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        <Kpi value={topName} label={l10n.getString('analytics-card-category-top')} />
        <DeltaChip value={deltaFor('category')} />
      </div>
      <div className="analytics-card-chart analytics-card-chart--donut" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 118 }} notMerge />
      </div>
      <Legend items={names.map((n, i) => ({ name: n, value: `${pcts[i]}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function BasketCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const data = useCardData('basket', workspace, g, () => {
    const rnd = seeded(`basket:${g}`);
    return BUCKET_LABELS[g]!.map((label) => ({ label, value: Math.round((2.2 + rnd() * 1.3) * 10) / 10 }));
  });
  const avg = data.reduce((s, d) => s + d.value, 0) / data.length;
  const option = useMemo(() => ({
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
  }), [data]);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={`${avg.toFixed(1)}`} label={l10n.getString('analytics-card-basket-items')} />
        <DeltaChip value={deltaFor('basket')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 104 }} notMerge />
      </div>
    </Visual>
  );
}

function InventoryCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const data = useCardData('inventory', workspace, g, () => series('inventory', g, 42, 0.8));
  const option = useMemo(() => ({
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
  }), [data]);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value="4.2×" label={l10n.getString('analytics-card-inventory-turnover')} />
        <Kpi value="21d" label={l10n.getString('analytics-card-inventory-days')} />
        <Kpi value="486" label={l10n.getString('analytics-card-inventory-skus')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 80 }} notMerge />
      </div>
    </Visual>
  );
}

function LowStockCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const rows = useCardData('low-stock', workspace, g, () => {
    const rnd = seeded(`low-stock:${g}`);
    const items = [
      { name: 'RAM D4 16GB', base: 4, cost: 65_000 },
      { name: 'Thermal Paste MX-6', base: 7, cost: 18_000 },
      { name: 'PSU RM850x', base: 11, cost: 240_000 },
      { name: 'SSD P3 Plus', base: 14, cost: 190_000 },
    ];
    return items.map((it) => {
      const stock = Math.max(1, it.base + Math.round(rnd() * 4) - 2);
      return { ...it, stock, reorder: Math.max(1, 12 - stock) };
    });
  });
  const restockCost = rows.reduce((s, r) => s + r.reorder * r.cost, 0);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={fmt(restockCost)} label={l10n.getString('analytics-card-low-stock-restock')} tone="bad" />
        <Kpi value={String(rows.length)} label={l10n.getString('analytics-card-low-stock-items')} />
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

function TablesCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const data = useCardData('tables', workspace, g, () => series('tables', g, 46, 0.9));
  const avgTurn = Math.round(data.reduce((s, d) => s + d.value, 0) / data.length);
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const },
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
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={`${avgTurn}m`} label={l10n.getString('analytics-card-tables-turn')} />
        <DeltaChip value={deltaFor('tables')} />
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option} style={{ height: 104 }} notMerge />
      </div>
    </Visual>
  );
}

function OccupancyCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { rate, peak } = useCardData('occupancy', workspace, g, () => ({
    rate: 60 + Math.round(seeded(`occupancy:${g}`)() * 25),
    peak: 17 + Math.round(seeded(`occupancy-peak:${g}`)() * 4),
  }));
  const hourly = useMemo(() => {
    const r = seeded(`occupancy-hourly:${g}`);
    return OCCUPANCY_HOURS.map((hour, i) => ({
      hour,
      pct: Math.min(100, Math.round(OCCUPANCY_SHAPE[i]! * rate + r() * 6)),
    }));
  }, [g, rate]);
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 8, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => `${v}%` },
    xAxis: {
      type: 'category' as const, data: hourly.map((d) => d.hour),
      axisLabel: { fontSize: 9, color: CHART_TEXT, interval: 1 }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false, max: 100 },
    series: [{
      type: 'line' as const, data: hourly.map((d) => d.pct),
      smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#f59e0b' },
      areaStyle: { opacity: 0.12 }, itemStyle: { color: '#f59e0b' },
    }],
  }), [hourly]);
  return (
    <Visual>
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
          <ReactEChartsCore echarts={echarts} option={option} style={{ height: 64 }} notMerge />
        </div>
      </div>
    </Visual>
  );
}

function WaitstaffCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const names = ['Bima', 'Lina', 'Dedi', 'Yanti'];
  const rows = useCardData('waitstaff', workspace, g, () => {
    const rnd = seeded(`waitstaff:${g}`);
    const deltas = rowDeltas('waitstaff', names.length);
    const values = names.map(() => Math.round(85 * (0.5 + rnd() * 1.1)));
    return names.map((name, i) => ({ name, value: values[i]!, display: `${values[i]} ${l10n.getString('analytics-card-waitstaff-covers')}`, delta: deltas[i]! }));
  });
  const totalCovers = rows.reduce((s, r) => s + r.value, 0);
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={String(totalCovers)} label={l10n.getString('analytics-card-waitstaff-total')} />
        <DeltaChip value={deltaFor('waitstaff')} />
      </div>
      <RankedList rows={rows} ariaLabel={title} />
    </Visual>
  );
}

function VoidsCard({ g, workspace, title }: { g: Granularity; workspace: WorkspaceView; title: string }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const items = [
    { name: 'Caffè Latte', price: 38_000 },
    { name: 'Iced Coffee', price: 32_000 },
    { name: 'Avocado Toast', price: 65_000 },
    { name: 'Smoothie', price: 45_000 },
  ];
  const rows = useCardData('voids', workspace, g, () => {
    const rnd = seeded(`voids:${g}`);
    const deltas = rowDeltas('voids', items.length);
    return items.map((it, i) => {
      const count = Math.round(9 * (0.5 + rnd() * 1.3));
      return { name: it.name, value: count, price: it.price, display: `${count}×`, delta: deltas[i]! };
    });
  });
  const voidedValue = rows.reduce((s, r) => s + r.value * r.price, 0);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={String(rows.reduce((s, r) => s + r.value, 0))} label={l10n.getString('analytics-card-voids-count')} tone="bad" />
        <Kpi value={fmt(voidedValue)} label={l10n.getString('analytics-card-voids-value')} tone="bad" />
      </div>
      <RankedList rows={rows} ariaLabel={title} />
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
  /** Localized card title, used as the chart's accessible name. */
  title: string;
}

/** Renders the designed content for a non-heatmap analytics card. */
export function AnalyticsCardContent({ cardKey, granularity, workspaceView, title }: AnalyticsCardContentProps) {
  switch (cardKey) {
    case 'revenue': return <RevenueCard g={granularity} workspace={workspaceView} title={title} />;
    case 'aov': return <AovCard g={granularity} workspace={workspaceView} title={title} />;
    case 'staff': return <StaffCard g={granularity} workspace={workspaceView} title={title} />;
    case 'customers': return <CustomersCard g={granularity} workspace={workspaceView} title={title} />;
    case 'payments': return <PaymentsCard g={granularity} workspace={workspaceView} title={title} />;
    case 'discounts': return <DiscountsCard g={granularity} workspace={workspaceView} title={title} />;
    case 'refunds': return <RefundsCard g={granularity} workspace={workspaceView} />;
    case 'top-items': return <TopItemsCard g={granularity} workspace={workspaceView} title={title} />;
    case 'category': return <CategoryCard g={granularity} workspace={workspaceView} title={title} />;
    case 'basket': return <BasketCard g={granularity} workspace={workspaceView} title={title} />;
    case 'inventory': return <InventoryCard g={granularity} workspace={workspaceView} title={title} />;
    case 'low-stock': return <LowStockCard g={granularity} workspace={workspaceView} title={title} />;
    case 'tables': return <TablesCard g={granularity} workspace={workspaceView} title={title} />;
    case 'occupancy': return <OccupancyCard g={granularity} workspace={workspaceView} title={title} />;
    case 'waitstaff': return <WaitstaffCard g={granularity} workspace={workspaceView} title={title} />;
    case 'voids': return <VoidsCard g={granularity} workspace={workspaceView} title={title} />;
    default: return null;
  }
}
