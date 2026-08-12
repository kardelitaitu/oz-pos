//! Designed card visuals for the analytics grid.
//!
//! Every non-heatmap card renders a purpose-built layout — KPI + trend
//! chart, donut, stacked bar, ranked list, or alert list — fed by the
//! real analytics loaders (CARD_LOADERS). Comparison mode overlays the
//! previous equal-length period per card (delta chips + chart overlays).
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
import { l10nErrorMessage } from '@/utils/app-error';
import { useAnalyticsQuery } from './useAnalyticsQuery';
import { cardQueryKey } from './analytics-cache';
import type { MenuEngineeringRow } from '@/api/reports';
import {
  CARD_LOADERS,
  periodDelta,
  previousRange,
  seriesDelta,
  turnDelta,
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
import type { StaffAnalyticsRow, TableOccupancy } from './analytics-data';
import type { Granularity, WorkspaceView } from './AnalyticsScreen';

echarts.use([EBar, ELine, EPie, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

// ── Deterministic demo data ─────────────────────────────────────────
// The same (card, granularity) always yields the same numbers so tests,
// screenshots, and re-renders are stable, while values still vary
// plausibly per bucket.

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
 * Wrapper that hosts the card content. Every card runs on real backend
 * data — there is no demo-data path left.
 */
function Visual({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div className={`analytics-card-visual${className ? ` ${className}` : ''}`}>
      {children}
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

/**
 * Shown when a card's IPC query failed.
 *
 * The query layer records the failure and does NOT re-invoke the fetcher
 * on re-render, so this is a stable state — the screen's refresh action
 * clears the recorded failure and retries. Only the localized user-safe
 * copy is rendered (ERR-05), never the raw backend message.
 */
function CardError({ error }: { error: unknown }) {
  const { l10n } = useLocalization();
  const message = l10nErrorMessage(error, l10n, 'analytics-card-error-load');
  return (
    <div className="analytics-card-error" role="alert">
      <span className="analytics-card-error-icon" aria-hidden="true">⚠</span>
      <span className="analytics-card-error-text">{message}</span>
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
function DeltaChip({ value, tone }: { value: number; tone?: 'good' | 'bad' }) {
  const { l10n } = useLocalization();
  const up = value >= 0;
  // For metrics where up is bad (voids, refunds, restock cost, turn time)
  // the pill's colour follows the *semantic* direction, not the sign.
  const good = tone === 'bad' ? !up : up;
  return (
    <span className={`analytics-delta${good ? ' analytics-delta--up' : ' analytics-delta--down'}`}>
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
 * Every card loads through `CARD_LOADERS` (real backend data).
 * Returns `null` while an async query is in flight.
 *
 * `enabled=false` (the comparison baseline while compare mode is off)
 * never fetches and always yields `data: null`.
 */
function useCardData<T>(
  cardKey: string,
  q: AnalyticsQuery,
  enabled = true,
): { data: T | null; error: unknown } {
  const result = useAnalyticsQuery(
    cardQueryKey(cardKey, q.workspace, q.granularity, q.from, q.to),
    () => {
      const loader = CARD_LOADERS[cardKey] as ((query: AnalyticsQuery) => Promise<T>) | undefined;
      if (!loader) return null as T;
      return loader(q);
    },
    enabled,
  );
  return { data: result.data as T | null, error: result.error };
}

/**
 * Period-over-period variant: the current query plus the previous
 * equal-length window, both through the shared TTL cache. The baseline
 * only fetches while `compare` is on, so compare mode costs nothing when
 * it is off; a failing baseline just yields `prev: null` (no chip).
 */
function useCardDataCompare<T>(
  cardKey: string,
  q: AnalyticsQuery,
  compare: boolean,
): { data: T | null; prev: T | null; error: unknown } {
  const cur = useCardData<T>(cardKey, q);
  const prevQ = useMemo(
    () => previousRange(q),
    [q.workspace, q.granularity, q.from, q.to, q.sessionToken],
  );
  const prev = useCardData<T>(cardKey, prevQ, compare);
  return { data: cur.data, prev: prev.data, error: cur.error };
}

/**
 * Attach per-row deltas to a ranked list by matching row names against
 * the previous period. Rows absent from the baseline keep no chip.
 */
function rowDeltas(cur: RankRow[], prev: RankRow[] | null | undefined): RankRow[] {
  if (!prev) return cur;
  const prevByName = new Map(prev.map((r) => [r.name, r.value]));
  return cur.map((r) => {
    const pv = prevByName.get(r.name);
    const d = pv !== undefined ? periodDelta(r.value, pv) : null;
    return d !== null ? { ...r, delta: d } : r;
  });
}

function RevenueCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt, short } = useMoney();
  const { data, prev, error } = useCardDataCompare<Bucket[]>('revenue', q, compare ?? false);
  const prevData = prev ?? [];
  const total = data ? data.reduce((s, d) => s + d.value, 0) : 0;
  const prevTotal = prevData.reduce((s, d) => s + d.value, 0);
  const peak = data && data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data && data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  // Compare mode replaces the in-period trend with the true period-over-
  // period change; off-mode keeps the existing series delta.
  const delta = compare ? periodDelta(total, prevTotal) : data ? seriesDelta(data) : null;
  const option = useMemo(() => (data ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [
      {
        name: l10n.getString('analytics-card-revenue'),
        type: 'line' as const, data: data.map((d) => d.value),
        smooth: true, symbol: 'circle', symbolSize: 4,
        itemStyle: { color: '#4f46e5' }, areaStyle: { opacity: 0.12 }, lineStyle: { width: 2 },
      },
      ...(compare && prevData.length ? [{
        name: l10n.getString('analytics-card-prev'),
        type: 'line' as const, data: prevData.map((d) => d.value),
        smooth: true, symbol: 'none',
        itemStyle: { color: '#94a3b8' }, lineStyle: { width: 1.5, color: '#94a3b8', type: 'dashed' as const },
      }] : []),
    ],
  }) : null), [data, prevData, compare, fmt, l10n]);
  if (error) return <CardError error={error} />;
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

function AovCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const { data, prev, error } = useCardDataCompare<Bucket[]>('aov', q, compare ?? false);
  const prevData = prev ?? [];
  const avg = data && data.length ? Math.round(data.reduce((s, d) => s + d.value, 0) / data.length) : 0;
  const prevAvg = prevData.length ? Math.round(prevData.reduce((s, d) => s + d.value, 0) / prevData.length) : 0;
  const peak = data && data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data && data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  const delta = compare ? periodDelta(avg, prevAvg) : data ? seriesDelta(data) : null;
  const option = useMemo(() => (data ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => fmt(Number(v)) },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [
      {
        type: 'line' as const, data: data.map((d) => d.value),
        smooth: true, symbol: 'circle', symbolSize: 4,
        itemStyle: { color: '#4f46e5' }, areaStyle: { opacity: 0.12 }, lineStyle: { width: 2 },
      },
      ...(compare && prevData.length ? [{
        name: l10n.getString('analytics-card-prev'),
        type: 'line' as const, data: prevData.map((d) => d.value),
        smooth: true, symbol: 'none',
        itemStyle: { color: '#94a3b8' }, lineStyle: { width: 1.5, color: '#94a3b8', type: 'dashed' as const },
      }] : []),
    ],
  }) : null), [data, prevData, compare, fmt, l10n]);
  if (error) return <CardError error={error} />;
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

function StaffCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { short } = useMoney();
  const { data: staff, prev: prevStaff, error } = useCardDataCompare<StaffAnalyticsRow[]>('staff', q, compare ?? false);
  if (error) return <CardError error={error} />;
  if (!staff) return <CardLoading />;
  const buildRows = (rows: StaffAnalyticsRow[]): RankRow[] => rows
    .slice()
    .sort((a, b) => b.sale_total_minor - a.sale_total_minor)
    .map((r) => ({ name: r.display_name, value: r.sale_total_minor, display: short(r.sale_total_minor) }));
  const rows = rowDeltas(buildRows(staff), prevStaff ? buildRows(prevStaff) : null);
  const totalSales = rows.reduce((s, r) => s + r.value, 0);
  const prevTotal = prevStaff ? prevStaff.reduce((s, r) => s + r.sale_total_minor, 0) : 0;
  const delta = compare ? periodDelta(totalSales, prevTotal) : null;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={short(totalSales)} label={l10n.getString('analytics-card-staff-sales')} />
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function CustomersCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: split, prev: prevSplit, error } = useCardDataCompare<CustomerSplitRow>('customers', q, compare ?? false);
  const newCount = split ? split.new_count : 0;
  const retCount = split ? split.returning_count : 0;
  const total = newCount + retCount;
  const prevTotal = prevSplit ? prevSplit.new_count + prevSplit.returning_count : 0;
  const newPct = total > 0 ? Math.round((newCount / total) * 100) : 0;
  const delta = compare ? periodDelta(total, prevTotal) : null;
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
  if (error) return <CardError error={error} />;
  if (!split) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        <Kpi value={String(total)} label={l10n.getString('analytics-card-customers-total')} />
        {delta !== null && <DeltaChip value={delta} />}
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

function PaymentsCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: rows, prev: prevRows, error } = useCardDataCompare<PaymentMethodRow[]>('payments', q, compare ?? false);
  const total = rows ? rows.reduce((s, r) => s + r.total_minor, 0) : 0;
  const prevTotal = prevRows ? prevRows.reduce((s, r) => s + r.total_minor, 0) : 0;
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
  const delta = compare ? periodDelta(total, prevTotal) : null;
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
  if (error) return <CardError error={error} />;
  if (!rows) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        {topSeg && <Kpi value={`${topSeg.name} · ${topPct}%`} label={l10n.getString('analytics-card-payments-top')} />}
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 180 : 84 }} notMerge />
      </div>
      <Legend items={segs.map((s, i) => ({ name: s.name, value: `${s.pct}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function DiscountsCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: summary, prev: prevSummary, error } = useCardDataCompare<DiscountsSummaryRow>('discounts', q, compare ?? false);
  if (error) return <CardError error={error} />;
  if (!summary) return <CardLoading />;
  const rows: RankRow[] = summary.codes.map((c) => ({
    name: c.label,
    value: c.redeemed_count,
    display: `${c.redeemed_count} ${l10n.getString('analytics-card-discounts-redeemed')}`,
  }));
  const discountShare = summary.share_percent;
  const redeemed = summary.codes.reduce((s, c) => s + c.redeemed_count, 0);
  const prevRedeemed = prevSummary ? prevSummary.codes.reduce((s, c) => s + c.redeemed_count, 0) : 0;
  const delta = compare ? periodDelta(redeemed, prevRedeemed) : null;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={`${discountShare.toFixed(1)}%`} label={l10n.getString('analytics-card-discounts-share')} />
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function RefundsCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const { data: loaded, prev: prevLoaded, error } = useCardDataCompare<[VoidedSummaryRow, VoidedItemRow[]]>('refunds', q, compare ?? false);
  if (error) return <CardError error={error} />;
  if (!loaded) return <CardLoading />;
  const [summary, items] = loaded;
  const rows = rowDeltas(
    items.map((it) => ({ name: it.name, value: it.qty, display: `${it.qty}×` })),
    prevLoaded ? prevLoaded[1].map((it) => ({ name: it.name, value: it.qty, display: '' })) : null,
  );
  const delta = compare && prevLoaded ? periodDelta(summary.void_count, prevLoaded[0].void_count) : null;
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={String(summary.void_count)} label={l10n.getString('analytics-card-refunds-count')} tone="bad" />
        <Kpi value={fmt(summary.void_total_minor)} label={l10n.getString('analytics-card-refunds-amount')} tone="bad" />
      </div>
      {delta !== null && <p className="analytics-card-insight"><DeltaChip value={delta} tone="bad" /></p>}
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function TopItemsCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { short } = useMoney();
  const { data: raw, prev: prevRaw, error } = useCardDataCompare<(TopProductRow | MenuEngineeringRow)[]>('top-items', q, compare ?? false);
  if (error) return <CardError error={error} />;
  if (!raw) return <CardLoading />;
  const buildRows = (list: (TopProductRow | MenuEngineeringRow)[]): RankRow[] => list.map((r) => {
    if ('total_qty' in r) {
      return { name: r.name, value: r.total_minor, display: `${short(r.total_minor)} · ${r.total_qty}×` };
    }
    return { name: r.name, value: r.total_revenue_minor, display: `${short(r.total_revenue_minor)} · ${r.total_volume}×` };
  });
  const rows = rowDeltas(buildRows(raw), prevRaw ? buildRows(prevRaw) : null);
  const total = rows.reduce((s, r) => s + r.value, 0);
  const prevTotal = prevRaw ? prevRaw.reduce((s, r) => s + ('total_qty' in r ? r.total_minor : r.total_revenue_minor), 0) : 0;
  const topName = rows[0]?.name;
  const delta = compare ? periodDelta(total, prevTotal) : null;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        {topName && <Kpi value={topName} label={l10n.getString('analytics-card-top-product')} />}
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <RankedList rows={rows} ariaLabel={title} limit={expanded ? undefined : 5} />
    </Visual>
  );
}

function CategoryCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: rows, prev: prevRows, error } = useCardDataCompare<CategoryBreakdownRow[]>('category', q, compare ?? false);
  const names = rows ? rows.map((r) => r.category_name).slice(0, 8) : [];
  const pcts = rows ? rows.map((r) => Math.round(r.percentage)).slice(0, 8) : [];
  const topName = pcts.length ? names[pcts.indexOf(Math.max(...pcts))] : '';
  const total = rows ? rows.reduce((s, r) => s + r.total_minor, 0) : 0;
  const prevTotal = prevRows ? prevRows.reduce((s, r) => s + r.total_minor, 0) : 0;
  const delta = compare ? periodDelta(total, prevTotal) : null;
  const option = useMemo(() => (names.length ? ({
    tooltip: { trigger: 'item' as const },
    series: [{
      type: 'pie' as const, radius: ['58%', '82%'], center: ['50%', '50%'],
      itemStyle: { borderRadius: 4, borderColor: '#fff', borderWidth: 2 },
      label: { show: false }, emphasis: { scaleSize: 4 },
      data: names.map((n, i) => ({ value: pcts[i], name: n, itemStyle: { color: PALETTE[i % PALETTE.length] } })),
    }],
  }) : null), [names, pcts]);
  if (error) return <CardError error={error} />;
  if (!rows) return <CardLoading />;
  return (
    <Visual className="analytics-card-visual--split">
      <div className="analytics-kpi-row">
        {topName && <Kpi value={topName} label={l10n.getString('analytics-card-category-top')} />}
        {delta !== null && <DeltaChip value={delta} />}
      </div>
      <div className="analytics-card-chart analytics-card-chart--donut" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 210 : 118 }} notMerge />
      </div>
      <Legend items={names.map((n, i) => ({ name: n, value: `${pcts[i]}%`, color: PALETTE[i % PALETTE.length]! }))} />
    </Visual>
  );
}

function BasketCard({ q, expanded, compare }: { q: AnalyticsQuery; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: basket, prev: prevBasket, error } = useCardDataCompare<BasketSizeRow>('basket', q, compare ?? false);
  const avg = basket ? basket.avg_line_count : 0;
  const orders = basket ? basket.sale_count : 0;
  const prevAvg = prevBasket ? prevBasket.avg_line_count : 0;
  const delta = compare ? periodDelta(avg, prevAvg) : null;
  if (error) return <CardError error={error} />;
  if (!basket) return <CardLoading />;
  // The backend only surfaces the range average, so a per-bucket chart or
  // peak/low insight would be fabricated. Present the honest aggregate:
  // average items per order plus the order volume behind that number.
  return (
    <Visual>
      <div className={`analytics-kpi-tiles${expanded ? ' analytics-kpi-tiles--expanded' : ''}`}>
        <Kpi value={avg > 0 ? avg.toFixed(1) : '—'} label={l10n.getString('analytics-card-basket-items')} />
        <Kpi value={orders > 0 ? String(orders) : '—'} label={l10n.getString('analytics-card-basket-orders')} />
      </div>
      {delta !== null && <p className="analytics-card-insight"><DeltaChip value={delta} /></p>}
      <p className="analytics-card-insight">{l10n.getString('analytics-card-basket-range')}</p>
    </Visual>
  );
}

function InventoryCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: loaded, prev: prevLoaded, error } = useCardDataCompare<[InventoryTurnoverRow, InventoryTrendRow[]]>('inventory', q, compare ?? false);
  const [turnoverRow, trend]: [InventoryTurnoverRow | null, InventoryTrendRow[]] = loaded ?? [null, []];
  const [prevRow, prevTrend]: [InventoryTurnoverRow | null, InventoryTrendRow[]] = prevLoaded ?? [null, []];
  const turnover = turnoverRow && turnoverRow.stock_on_hand > 0 ? turnoverRow.units_sold / turnoverRow.stock_on_hand : 0;
  const prevTurnover = prevRow && prevRow.stock_on_hand > 0 ? prevRow.units_sold / prevRow.stock_on_hand : 0;
  const data: Bucket[] = trend.map((t) => ({ label: t.date.slice(5), value: t.units_sold }));
  const prevData: Bucket[] = prevTrend.map((t) => ({ label: t.date.slice(5), value: t.units_sold }));
  const skus = turnoverRow ? turnoverRow.sku_count : 0;
  const daysOfStock = turnoverRow && turnover > 0 ? Math.max(1, Math.round(turnoverRow.range_days / turnover)) : 0;
  const delta = compare ? periodDelta(turnover, prevTurnover) : null;
  const option = useMemo(() => (data.length ? ({
    grid: { left: 8, right: 8, top: 10, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [
      {
        type: 'line' as const, data: data.map((d) => d.value),
        smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#22c55e' },
        areaStyle: { opacity: 0.12 }, itemStyle: { color: '#22c55e' },
      },
      ...(compare && prevData.length ? [{
        name: l10n.getString('analytics-card-prev'),
        type: 'line' as const, data: prevData.map((d) => d.value),
        smooth: true, symbol: 'none',
        itemStyle: { color: '#94a3b8' }, lineStyle: { width: 1.5, color: '#94a3b8', type: 'dashed' as const },
      }] : []),
    ],
  }) : null), [data, prevData, compare, l10n]);
  if (error) return <CardError error={error} />;
  if (!loaded) return <CardLoading />;
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={turnover > 0 ? `${turnover.toFixed(1)}×` : '—'} label={l10n.getString('analytics-card-inventory-turnover')} />
        <Kpi value={daysOfStock > 0 ? `${daysOfStock}d` : '—'} label={l10n.getString('analytics-card-inventory-days')} />
        <Kpi value={String(skus)} label={l10n.getString('analytics-card-inventory-skus')} />
      </div>
      {delta !== null && <p className="analytics-card-insight"><DeltaChip value={delta} /></p>}
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 170 : 80 }} notMerge />
      </div>
    </Visual>
  );
}

function LowStockCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { fmt } = useMoney();
  const { data: alerts, prev: prevAlerts, error } = useCardDataCompare<LowStockAlert[]>('low-stock', q, compare ?? false);
  if (error) return <CardError error={error} />;
  if (!alerts) return <CardLoading />;
  const build = (list: LowStockAlert[]) => list.map((a) => ({
    name: a.name,
    stock: a.current_qty,
    reorder: Math.max(0, a.threshold - a.current_qty),
    cost: a.cost_minor,
  }));
  const rows = build(alerts);
  const restockCost = rows.reduce((s, r) => s + r.reorder * r.cost, 0);
  const prevCost = prevAlerts ? build(prevAlerts).reduce((s, r) => s + r.reorder * r.cost, 0) : 0;
  const criticalCount = rows.filter((r) => r.stock <= 5).length;
  const delta = compare ? periodDelta(restockCost, prevCost) : null;
  // Collapsed cards cap the alert list; expanding reveals every alert.
  const shown = expanded ? rows : rows.slice(0, 5);
  return (
    <Visual>
      <div className="analytics-kpi-tiles">
        <Kpi value={fmt(restockCost)} label={l10n.getString('analytics-card-low-stock-restock')} tone="bad" />
        <Kpi value={String(rows.length)} label={l10n.getString('analytics-card-low-stock-items')} />
        <Kpi value={String(criticalCount)} label={l10n.getString('analytics-card-low-stock-critical')} tone="bad" />
      </div>
      {delta !== null && <p className="analytics-card-insight"><DeltaChip value={delta} tone="bad" /></p>}
      <ul className="analytics-alert-list" aria-label={title}>
        {shown.map((r) => {
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

function TablesCard({ q, title, expanded, compare }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined; compare?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { data: raw, prev: prevRaw, error } = useCardDataCompare<Bucket[]>('tables', q, compare ?? false);
  const data = raw ?? [];
  const prevData = prevRaw ?? [];
  const avgTurn = data.length ? Math.round(data.reduce((s, d) => s + d.value, 0) / data.length) : 0;
  const prevAvgTurn = prevData.length ? Math.round(prevData.reduce((s, d) => s + d.value, 0) / prevData.length) : 0;
  const peak = data.length ? data.reduce((a, b) => (b.value > a.value ? b : a)) : null;
  const low = data.length ? data.reduce((a, b) => (b.value < a.value ? b : a)) : null;
  // Compare mode shows the period-over-period change; off-mode keeps the
  // in-series turn-time delta (faster turns = shorter minutes).
  const delta = compare ? periodDelta(avgTurn, prevAvgTurn) : turnDelta(data);
  const option = useMemo(() => (data.length ? ({
    grid: { left: 8, right: 8, top: 12, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => `${v}m` },
    xAxis: {
      type: 'category' as const, data: data.map((d) => d.label),
      axisLabel: { fontSize: 9, color: CHART_TEXT }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false },
    series: [
      {
        type: 'bar' as const, data: data.map((d) => d.value),
        itemStyle: { color: '#f59e0b', borderRadius: [3, 3, 0, 0] }, barWidth: '55%',
      },
      ...(compare && prevData.length ? [{
        name: l10n.getString('analytics-card-prev'),
        type: 'line' as const, data: prevData.map((d) => d.value),
        smooth: true, symbol: 'none',
        itemStyle: { color: '#94a3b8' }, lineStyle: { width: 1.5, color: '#94a3b8', type: 'dashed' as const },
      }] : []),
    ],
  }) : null), [data, prevData, compare, l10n]);
  if (error) return <CardError error={error} />;
  if (!raw) return <CardLoading />;
  return (
    <Visual>
      <div className="analytics-kpi-row">
        <Kpi value={avgTurn > 0 ? `${avgTurn}m` : '—'} label={l10n.getString('analytics-card-tables-turn')} />
        {delta !== null && <DeltaChip value={delta} tone={compare ? 'bad' : undefined} />}
      </div>
      <div className="analytics-card-chart" role="img" aria-label={title}>
        <ReactEChartsCore echarts={echarts} option={option!} style={{ height: expanded ? 240 : 104 }} notMerge />
      </div>
      {peak && <p className="analytics-card-insight">{l10n.getString('analytics-card-peak', { label: peak.label, value: `${peak.value}m` })}</p>}
      {low && <p className="analytics-card-insight">{l10n.getString('analytics-card-low', { label: low.label, value: `${low.value}m` })}</p>}
    </Visual>
  );
}

function OccupancyCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  // Real rate from the live tables snapshot + real per-hour completed table
  // orders from the backend — nothing is demo-shaped anymore.
  const { data: occ, error } = useCardData<TableOccupancy>('occupancy', q);
  const rate = occ ? occ.rate : 0;
  const hourly = occ ? occ.hourly : [];
  const peak = occ ? occ.peak_hour : null;
  // The peak-hour bucket carries the raw order count for the meta line;
  // pct/level already share the heatmap's intensity scale.
  const peakBucket = peak !== null ? hourly.find((h) => h.hour === peak) : null;
  const option = useMemo(() => ({
    grid: { left: 8, right: 8, top: 8, bottom: 0, containLabel: true },
    tooltip: { trigger: 'axis' as const, valueFormatter: (v: unknown) => `${v}%` },
    xAxis: {
      type: 'category' as const, data: hourly.map((d) => String(d.hour).padStart(2, '0')),
      axisLabel: { fontSize: 9, color: CHART_TEXT, interval: 1 }, axisLine: { show: false }, axisTick: { show: false },
    },
    yAxis: { type: 'value' as const, show: false, max: 100 },
    series: [{
      type: 'line' as const, data: hourly.map((d) => d.pct),
      smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#f59e0b' },
      areaStyle: { opacity: 0.12 }, itemStyle: { color: '#f59e0b' },
    }],
  }), [hourly]);
  if (error) return <CardError error={error} />;
  if (!occ) return <CardLoading />;
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
          {peak !== null && (
            <span>
              {l10n.getString('analytics-card-occupancy-peak')} · {String(peak).padStart(2, '0')}:00
              {peakBucket && ` · ${peakBucket.table_orders} ${l10n.getString('analytics-card-occupancy-orders')}`}
            </span>
          )}
        </div>
        <div className="analytics-card-chart" role="img" aria-label={l10n.getString('analytics-card-occupancy-hourly')}>
          <ReactEChartsCore echarts={echarts} option={option} style={{ height: expanded ? 150 : 64 }} notMerge />
        </div>
      </div>
    </Visual>
  );
}

function WaitstaffCard({ q, title, expanded }: { q: AnalyticsQuery; title: string; expanded?: boolean | undefined }) {
  const { l10n } = useLocalization();
  const { short } = useMoney();
  const { data: staff, error } = useCardData<StaffAnalyticsRow[]>('waitstaff', q);
  if (error) return <CardError error={error} />;
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
  const { data: loaded, error } = useCardData<[VoidedSummaryRow, VoidedItemRow[]]>('voids', q);
  if (error) return <CardError error={error} />;
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
    case 'refunds': return <RefundsCard q={q} title={title} expanded={expanded} />;
    case 'top-items': return <TopItemsCard q={q} title={title} expanded={expanded} />;
    case 'category': return <CategoryCard q={q} title={title} expanded={expanded} />;
    case 'basket': return <BasketCard q={q} expanded={expanded} />;
    case 'inventory': return <InventoryCard q={q} title={title} expanded={expanded} />;
    case 'low-stock': return <LowStockCard q={q} title={title} expanded={expanded} />;
    case 'tables': return <TablesCard q={q} title={title} expanded={expanded} />;
    case 'occupancy': return <OccupancyCard q={q} title={title} expanded={expanded} />;
    case 'waitstaff': return <WaitstaffCard q={q} title={title} expanded={expanded} />;
    case 'voids': return <VoidsCard q={q} title={title} expanded={expanded} />;
    default: return null;
  }
}
