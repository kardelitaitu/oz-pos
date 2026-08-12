//! Real-data loaders and pure mapping helpers for the analytics cards.
//!
//! Each card's data slice is fetched through the shared TTL cache via
//! `useAnalyticsQuery`, keyed by card + workspace + granularity + range.
//! This module owns *what* to fetch and how to derive the per-bucket
//! shapes the layouts consume; the cards keep their visual structure and
//! only swap their data source. Raw API rows stay unformatted here —
//! money display formatting lives in the UI layer (the cards' `useMoney`).

import {
  getBasketSizeTrend,
  getCategoryBreakdown,
  getCustomerSplit,
  getDailyRevenue,
  getDiscountsSummary,
  getHourlyHeatmap,
  getHourlyOccupancy,
  getInventoryTrend,
  getInventoryTurnover,
  getLowStockAlerts,
  getMenuEngineering,
  getMonthlyRevenue,
  getPaymentMethodBreakdown,
  getTableTurnover,
  getTopProducts,
  getVoidedItems,
  getVoidedSalesSummary,
  getWeeklyRevenue,
} from '@/api/reports';
import { listTablesScoped } from '@/api/tables';
import type {
  BasketSizeRow,
  BasketTrendRow,
  CategoryBreakdownRow,
  CustomerSplitRow,
  DailyRevenueRow,
  DiscountsSummaryRow,
  HourlyHeatmapRow,
  InventoryTrendRow,
  InventoryTurnoverRow,
  LowStockAlert,
  MonthlyRevenueRow,
  PaymentMethodRow,
  TopProductRow,
  VoidedItemRow,
  VoidedSummaryRow,
  WeeklyRevenueRow,
} from '@/api/reports';
import { getStaffAnalyticsScoped } from '@/api/analytics';
import type { StaffAnalyticsRow } from '@/api/analytics';
import type { Granularity, WorkspaceView } from './AnalyticsScreen';

// ── Shared layout shapes ────────────────────────────────────────────

/** One time-bucket of a chart series. */
export interface Bucket {
  label: string;
  value: number;
}

/** One row of a ranked list (top products, staff, discount codes, …). */
export interface RankRow {
  name: string;
  value: number;
  display: string;
  /** Optional % change vs previous period; renders a trend arrow. */
  delta?: number;
}

/** Everything a card needs to load its slice: scope + range. */
export interface AnalyticsQuery {
  workspace: WorkspaceView;
  granularity: Granularity;
  from: string;
  to: string;
  sessionToken: string;
}

// ── Date-range helper ───────────────────────────────────────────────

function isoDay(d: Date): string {
  // Local calendar date — `toISOString()` is UTC and can shift a day for
  // late-evening local times, which would query the wrong range.
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/**
 * The inclusive `[from, to]` window for a granularity. Daily/Weekly/
 * Monthly/Yearly are anchored at "now"; Custom uses the picked range.
 */
export function rangeForGranularity(
  g: Granularity,
  customFrom: string,
  customTo: string,
): { from: string; to: string } {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  switch (g) {
    case 'daily':
      return { from: isoDay(now), to: isoDay(now) };
    case 'weekly': {
      // Monday-first week start.
      const dow = (now.getDay() + 6) % 7;
      const start = new Date(y, m, now.getDate() - dow);
      return { from: isoDay(start), to: isoDay(now) };
    }
    case 'monthly':
      return { from: isoDay(new Date(y, m, 1)), to: isoDay(now) };
    case 'yearly':
      return { from: `${y}-01-01`, to: isoDay(now) };
    case 'custom':
      return { from: customFrom, to: customTo };
  }
}

// ── Pure mapping helpers ────────────────────────────────────────────

/** Short bucket label for a revenue row by granularity. */
export function revenueLabel(g: Granularity, raw: string): string {
  // daily/weekly raw = "YYYY-MM-DD" → "MM-DD"; monthly = "YYYY-MM" → "MM".
  return g === 'monthly' ? raw.slice(5) : raw.slice(5);
}

/** % change from the first bucket to the last (null when < 2 buckets). */
export function seriesDelta(buckets: Bucket[]): number | null {
  if (buckets.length < 2) return null;
  const first = buckets[0]!.value;
  const last = buckets[buckets.length - 1]!.value;
  if (first === 0) return 0;
  return Math.round(((last - first) / first) * 1000) / 10;
}

/**
 * The equal-length window immediately preceding `q` — the comparison
 * baseline for period-over-period mode (same span, ending the day before
 * the current window starts).
 */
export function previousRange(q: AnalyticsQuery): AnalyticsQuery {
  // All arithmetic is UTC-anchored (`...Z` + toISOString), so the window
  // shifts by whole calendar days in every timezone — never by hours.
  const fromMs = Date.parse(`${q.from}T00:00:00Z`);
  const toMs = Date.parse(`${q.to}T00:00:00Z`);
  const spanDays = Math.max(1, Math.round((toMs - fromMs) / 86_400_000) + 1);
  const prevToMs = fromMs - 86_400_000;
  const prevFromMs = prevToMs - (spanDays - 1) * 86_400_000;
  const iso = (ms: number) => new Date(ms).toISOString().slice(0, 10);
  return { ...q, from: iso(prevFromMs), to: iso(prevToMs) };
}

/**
 * % change of `current` vs `previous` (one decimal). Returns `null` when
 * there is no previous baseline (missing or zero) — callers then omit the
 * comparison chip instead of showing a misleading ±∞.
 */
export function periodDelta(current: number, previous: number): number | null {
  if (previous === 0 || !Number.isFinite(current) || !Number.isFinite(previous)) return null;
  return Math.round(((current - previous) / previous) * 1000) / 10;
}

/**
 * Map the previous period's per-hour `pct` onto the current period's hour
 * set for the compare overlay. The backend returns only hours with orders,
 * so the two periods can have different active hours — alignment must be
 * by hour, never by array position. Current hours absent from the previous
 * period read as 0 (no activity there last period).
 */
export function alignPrevHourly(
  current: { hour: number; pct: number }[],
  previous: { hour: number; pct: number }[],
): number[] {
  const byHour = new Map(previous.map((h) => [h.hour, h.pct]));
  return current.map((h) => byHour.get(h.hour) ?? 0);
}

// ── Heatmap intensity builders ──────────────────────────────────────
//
// Cell keys identify the heatmap's fixed grid per granularity:
//   daily/custom: weekday index (0 = Mon … 6 = Sun)
//   weekly:       `${dayIdx}:${hour}` (dayIdx 0 = Mon, hour 0–23)
//   monthly:      day-of-month number (1…31)
//   yearly:       `${monthIdx}:${weekIdx}` (month 0–11, week 0–3)
//
// Intensities are normalized 0–4 against the strongest cell in the set.

/**
 * Percent-of-peak (0–100) with a shared `max` baseline — the single
 * normalization rule behind every intensity scale (heatmap cells and the
 * occupancy curve), so the same level of business always renders as the
 * same relative intensity.
 */
export function pctOfPeak(values: number[]): number[] {
  const max = Math.max(1, ...values);
  return values.map((v) => Math.round((v / max) * 100));
}

/**
 * 0–4 heat level for a percent-of-peak value — the binning the heatmap
 * renders: level = ⌊pct / 20⌋, so 0–19% → 0, 20–39% → 1, … 80–100% → 4.
 * An occupancy curve at 60% and a heatmap cell at level 3 are the same
 * intensity.
 */
export function intensityFromPct(pct: number): number {
  return pct <= 0 ? 0 : Math.min(4, Math.floor((pct / 100) * 5));
}

/** Map `[key, value]` entries to 0–4 levels, max-normalized. */
export function normalizeIntensities(entries: [string, number][]): Map<string, number> {
  const max = Math.max(1, ...entries.map(([, v]) => v));
  const map = new Map<string, number>();
  for (const [key, v] of entries) {
    // Same scale as the occupancy curve: level = ⌊percent-of-peak / 20⌋.
    map.set(key, intensityFromPct(Math.round((v / max) * 100)));
  }
  return map;
}

/** Monday-first index from a `Date.getDay()` (0 = Sunday). */
function mondayFirst(jsDay: number): number {
  return (jsDay + 6) % 7;
}

/** Hourly rows aggregated by day-of-week (daily/custom view). */
export function weekdayIntensities(rows: HourlyHeatmapRow[]): Map<string, number> {
  const byDay = new Map<string, number>();
  for (const r of rows) {
    const key = String(mondayFirst(r.day_of_week));
    byDay.set(key, (byDay.get(key) ?? 0) + r.total_minor);
  }
  return normalizeIntensities([...byDay.entries()]);
}

/** Hourly rows mapped to the 7×24 weekly grid. */
export function weeklyHourlyIntensities(rows: HourlyHeatmapRow[]): Map<string, number> {
  return normalizeIntensities(
    rows.map((r) => [`${mondayFirst(r.day_of_week)}:${r.hour}`, r.total_minor] as [string, number]),
  );
}

/** Daily revenue mapped to calendar days of the current month. */
export function monthDayIntensities(rows: DailyRevenueRow[]): Map<string, number> {
  return normalizeIntensities(
    rows.map((r) => [String(new Date(`${r.date}T00:00:00`).getDate()), r.total_minor] as [string, number]),
  );
}

/** Weekly revenue mapped to (month, week-of-month) for the 12×4 yearly grid. */
export function yearlyWeekIntensities(rows: WeeklyRevenueRow[]): Map<string, number> {
  return normalizeIntensities(
    rows.map((r) => {
      const d = new Date(`${r.week_start}T00:00:00`);
      const month = d.getMonth();
      const week = Math.min(3, Math.floor((d.getDate() - 1) / 7));
      return [`${month}:${week}`, r.total_minor] as [string, number];
    }),
  );
}

/** Per-cell intensities for the heatmap card at the given granularity. */
export function buildHeatmapIntensities(
  g: Granularity,
  data: { daily?: DailyRevenueRow[]; hourly?: HourlyHeatmapRow[]; weekly?: WeeklyRevenueRow[] },
): Map<string, number> {
  switch (g) {
    case 'weekly':
      return weeklyHourlyIntensities(data.hourly ?? []);
    case 'monthly':
      return monthDayIntensities(data.daily ?? []);
    case 'yearly':
      return yearlyWeekIntensities(data.weekly ?? []);
    default:
      // daily + custom fall back to the 7-day weekday view, aggregated
      // from the hourly heatmap when available.
      return weekdayIntensities(data.hourly ?? []);
  }
}

// ── Per-card loaders (raw API rows, no formatting) ──────────────────

/** Raw revenue rows for the granularity's bucket size. */
async function revenueRows(q: AnalyticsQuery): Promise<
  DailyRevenueRow[] | WeeklyRevenueRow[] | MonthlyRevenueRow[]
> {
  switch (q.granularity) {
    case 'weekly':
      return getWeeklyRevenue(q.from, q.to, q.sessionToken);
    case 'monthly':
    case 'yearly':
      return getMonthlyRevenue(q.from, q.to, q.sessionToken);
    default:
      return getDailyRevenue(q.from, q.to, q.sessionToken);
  }
}

/** Per-bucket label for a raw revenue row. */
function rowLabel(g: Granularity, r: DailyRevenueRow | WeeklyRevenueRow | MonthlyRevenueRow): string {
  const raw = 'date' in r ? r.date : 'week_start' in r ? r.week_start : r.month;
  return revenueLabel(g, raw);
}

/** Revenue per bucket (Revenue Overview card). */
export async function loadRevenue(q: AnalyticsQuery): Promise<Bucket[]> {
  const rows = await revenueRows(q);
  return rows.map((r) => ({ label: rowLabel(q.granularity, r), value: r.total_minor }));
}

/** Average order value per bucket (AOV card). */
export async function loadAov(q: AnalyticsQuery): Promise<Bucket[]> {
  const rows = await revenueRows(q);
  return rows.map((r) => ({
    label: rowLabel(q.granularity, r),
    value: r.sale_count > 0 ? Math.round(r.total_minor / r.sale_count) : 0,
  }));
}

/** Staff analytics (shared Staff Performance + restaurant Top Waitstaff). */
export function loadStaff(q: AnalyticsQuery): Promise<StaffAnalyticsRow[]> {
  return getStaffAnalyticsScoped(q.sessionToken, q.from, q.to);
}

/** Top products (retail) / menu engineering (restaurant). */
export function loadTopItems(q: AnalyticsQuery): Promise<TopProductRow[] | unknown[]> {
  if (q.workspace === 'retail') {
    return getTopProducts(q.from, q.to, 10, q.sessionToken, 'revenue');
  }
  return getMenuEngineering(q.from, q.to, q.sessionToken).then((r) => r.rows);
}

/** Raw rows the heatmap needs at the given granularity. */
export async function loadHeatmapRows(q: AnalyticsQuery): Promise<{
  daily: DailyRevenueRow[];
  hourly: HourlyHeatmapRow[];
  weekly: WeeklyRevenueRow[];
}> {
  const { from, to, sessionToken } = q;
  // Weekly needs the 7×24 hourly grid; monthly needs per-day revenue;
  // yearly needs per-week revenue. Daily/custom use the weekday view
  // aggregated from the hourly heatmap. Fetch the two needed sets only.
  switch (q.granularity) {
    case 'monthly':
      return { daily: await getDailyRevenue(from, to, sessionToken), hourly: [], weekly: [] };
    case 'yearly':
      return { daily: [], hourly: [], weekly: await getWeeklyRevenue(from, to, sessionToken) };
    default:
      return { daily: [], hourly: await getHourlyHeatmap(from, to, sessionToken), weekly: [] };
  }
}

/** Completed table-bound orders per day → per-bucket turn minutes. */
function weekStartKey(iso: string): string {
  const d = new Date(`${iso}T00:00:00`);
  const dow = (d.getDay() + 6) % 7; // Monday-first
  d.setDate(d.getDate() - dow);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function monthDays(ym: string): number {
  const [y, m] = ym.split('-').map(Number);
  return new Date(y!, m!, 0).getDate();
}

/**
 * Average table-turn minutes per granularity bucket. Table service is
 * recorded as completed KDS orders carrying a table number; each bucket
 * sums its turns and divides the bucket's service minutes by them
 * (fewer turns per hour ⇒ longer average turn).
 */
export async function loadTables(q: AnalyticsQuery): Promise<Bucket[]> {
  const rows = await getTableTurnover(q.from, q.to, q.sessionToken);
  if (rows.length === 0) return [];
  // Group consecutive days into the granularity's buckets.
  const groups: { key: string; orders: number; days: number }[] = [];
  for (const r of rows) {
    const key =
      q.granularity === 'weekly'
        ? weekStartKey(r.date)
        : q.granularity === 'monthly'
          ? r.date.slice(0, 7)
          : q.granularity === 'yearly'
            ? r.date.slice(0, 4)
            : r.date;
    const last = groups[groups.length - 1];
    if (last && last.key === key) {
      last.orders += r.table_orders;
      last.days += 1;
    } else {
      groups.push({ key, orders: r.table_orders, days: 1 });
    }
  }
  return groups.map((grp) => {
    const bucketMinutes =
      q.granularity === 'yearly'
        ? 365 * 1440
        : q.granularity === 'monthly'
          ? monthDays(grp.key) * 1440
          : q.granularity === 'weekly'
            ? 7 * 1440
            : 1440;
    return {
      label: q.granularity === 'yearly' ? grp.key : grp.key.slice(5),
      value: grp.orders > 0 ? Math.round(bucketMinutes / grp.orders) : 0,
    };
  });
}

/** Table-turn delta: positive when turns got *faster* (minutes dropped). */
export function turnDelta(buckets: Bucket[]): number | null {
  const d = seriesDelta(buckets);
  if (d === null) return null;
  return Math.round(-d * 10) / 10;
}

/** Basket-size shape for the trend card: per-bucket averages + range total. */
export interface BasketTrend {
  /** Per-bucket mean items/order (one bucket per granularity cell). */
  buckets: Bucket[];
  /** Completed sales across the range. */
  sale_count: number;
  /** Weighted mean line count across the range (matches the KPI tile). */
  avg_line_count: number;
}

/**
 * Basket size per granularity bucket — weighted average of items/order
 * across each bucket's days, plus the range totals. The daily rows come
 * from the backend; weekly/monthly/yearly group them like `loadTables`.
 */
export async function loadBasketSize(q: AnalyticsQuery): Promise<BasketTrend> {
  const rows = await getBasketSizeTrend(q.from, q.to, q.sessionToken);
  if (rows.length === 0) return { buckets: [], sale_count: 0, avg_line_count: 0 };
  // Group consecutive days into the granularity's buckets.
  const groups: { key: string; sales: number; lines: number }[] = [];
  for (const r of rows) {
    const key =
      q.granularity === 'weekly'
        ? weekStartKey(r.date)
        : q.granularity === 'monthly'
          ? r.date.slice(0, 7)
          : q.granularity === 'yearly'
            ? r.date.slice(0, 4)
            : r.date;
    const last = groups[groups.length - 1];
    const lines = r.sale_count * r.avg_line_count;
    if (last && last.key === key) {
      last.sales += r.sale_count;
      last.lines += lines;
    } else {
      groups.push({ key, sales: r.sale_count, lines });
    }
  }
  const buckets: Bucket[] = groups.map((g) => ({
    label: q.granularity === 'yearly' ? g.key : g.key.slice(5),
    value: g.sales > 0 ? Math.round((g.lines / g.sales) * 10) / 10 : 0,
  }));
  const saleCount = rows.reduce((s, r) => s + r.sale_count, 0);
  const lineTotal = rows.reduce((s, r) => s + r.sale_count * r.avg_line_count, 0);
  return {
    buckets,
    sale_count: saleCount,
    avg_line_count: saleCount > 0 ? lineTotal / saleCount : 0,
  };
}

/** Live floor-plan occupancy derived from the `tables` snapshot. */
export interface TableOccupancy {
  /** Total active tables on the floor plan. */
  total: number;
  /** Tables currently `occupied` (linked to an active sale). */
  occupied: number;
  /** Percentage of tables currently occupied (0–100). */
  rate: number;
  /** Seats in use on occupied tables vs total capacity. */
  seats_used: number;
  seats_total: number;
  /**
   * Completed table orders per hour (0–23). `pct` is percent-of-peak on the
   * same scale as the heatmap (`pctOfPeak`) and `level` is the matching
   * 0–4 heat level (`intensityFromPct`) — the curve and the heatmap now
   * share one intensity scale.
   */
  hourly: { hour: number; table_orders: number; pct: number; level: number }[];
  /** Hour (0–23) with the most completed table orders, or null when empty. */
  peak_hour: number | null;
}

/**
 * Real occupancy: the live rate comes from the current `tables` snapshot,
 * and the hourly curve + peak hour come from completed table orders per
 * hour of day in the selected range (`hourly_table_activity`).
 */
export async function loadTableOccupancy(q: AnalyticsQuery): Promise<TableOccupancy> {
  const [tables, hourlyRows] = await Promise.all([
    listTablesScoped(q.sessionToken),
    getHourlyOccupancy(q.from, q.to, q.sessionToken),
  ]);
  const active = tables.filter((t) => t.active);
  const occupied = active.filter((t) => t.status === 'occupied').length;
  const seatsUsed = active
    .filter((t) => t.status === 'occupied')
    .reduce((sum, t) => sum + t.capacity, 0);
  const seatsTotal = active.reduce((sum, t) => sum + t.capacity, 0);
  // Shared normalization with the heatmap: percent-of-peak plus the 0–4
  // heat level, so the curve and the heatmap cells speak the same scale.
  const pcts = pctOfPeak(hourlyRows.map((r) => r.table_orders));
  const hourly = hourlyRows
    .map((r, i) => ({
      hour: r.hour,
      table_orders: r.table_orders,
      pct: pcts[i]!,
      level: intensityFromPct(pcts[i]!),
    }))
    .sort((a, b) => a.hour - b.hour);
  let peak: number | null = null;
  let peakCount = 0;
  for (const r of hourlyRows) {
    if (r.table_orders > peakCount) {
      peak = r.hour;
      peakCount = r.table_orders;
    }
  }
  return {
    total: active.length,
    occupied,
    rate: active.length > 0 ? Math.round((occupied / active.length) * 100) : 0,
    seats_used: seatsUsed,
    seats_total: seatsTotal,
    hourly,
    peak_hour: peak,
  };
}

/** Everything the dashboard cards need, keyed by card key. */
export type CardLoaderResult = unknown;

export const CARD_LOADERS: Record<string, (q: AnalyticsQuery) => Promise<CardLoaderResult>> = {
  heatmap: loadHeatmapRows,
  revenue: loadRevenue,
  aov: loadAov,
  staff: loadStaff,
  customers: (q) => getCustomerSplit(q.from, q.to, q.sessionToken),
  payments: (q) => getPaymentMethodBreakdown(q.from, q.to, q.sessionToken),
  discounts: (q) => getDiscountsSummary(q.from, q.to, q.sessionToken),
  refunds: (q) =>
    Promise.all([
      getVoidedSalesSummary(q.from, q.to, q.sessionToken),
      // Higher limit so the expanded card can reveal the full list.
      getVoidedItems(q.from, q.to, q.sessionToken, 25),
    ]),
  'top-items': loadTopItems,
  category: (q) => getCategoryBreakdown(q.from, q.to, q.sessionToken),
  basket: loadBasketSize,
  inventory: (q) =>
    Promise.all([
      getInventoryTurnover(q.from, q.to, q.sessionToken, 'default'),
      getInventoryTrend(q.from, q.to, q.sessionToken),
    ]),
  'low-stock': (q) => getLowStockAlerts(10, q.sessionToken),
  waitstaff: loadStaff,
  tables: loadTables,
  voids: (q) =>
    Promise.all([
      getVoidedSalesSummary(q.from, q.to, q.sessionToken),
      getVoidedItems(q.from, q.to, q.sessionToken, 5),
    ]),
  occupancy: loadTableOccupancy,
};

// ── Re-export the raw row types for card-side mapping ───────────────

export type {
  BasketSizeRow,
  BasketTrendRow,
  CategoryBreakdownRow,
  CustomerSplitRow,
  DailyRevenueRow,
  DiscountsSummaryRow,
  HourlyHeatmapRow,
  InventoryTrendRow,
  InventoryTurnoverRow,
  LowStockAlert,
  MonthlyRevenueRow,
  PaymentMethodRow,
  TopProductRow,
  VoidedItemRow,
  VoidedSummaryRow,
  WeeklyRevenueRow,
};
export type { StaffAnalyticsRow };
