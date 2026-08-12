//! Real-data loaders and pure mapping helpers for the analytics cards.
//!
//! Each card's data slice is fetched through the shared TTL cache via
//! `useAnalyticsQuery`, keyed by card + workspace + granularity + range.
//! This module owns *what* to fetch and how to derive the per-bucket
//! shapes the layouts consume; the cards keep their visual structure and
//! only swap their data source. Raw API rows stay unformatted here —
//! money display formatting lives in the UI layer (the cards' `useMoney`).

import {
  getBasketSize,
  getCategoryBreakdown,
  getCustomerSplit,
  getDailyRevenue,
  getDiscountsSummary,
  getHourlyHeatmap,
  getInventoryTrend,
  getInventoryTurnover,
  getLowStockAlerts,
  getMenuEngineering,
  getMonthlyRevenue,
  getPaymentMethodBreakdown,
  getTopProducts,
  getVoidedItems,
  getVoidedSalesSummary,
  getWeeklyRevenue,
} from '@/api/reports';
import type {
  BasketSizeRow,
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

// ── Heatmap intensity builders ──────────────────────────────────────
//
// Cell keys identify the heatmap's fixed grid per granularity:
//   daily/custom: weekday index (0 = Mon … 6 = Sun)
//   weekly:       `${dayIdx}:${hour}` (dayIdx 0 = Mon, hour 0–23)
//   monthly:      day-of-month number (1…31)
//   yearly:       `${monthIdx}:${weekIdx}` (month 0–11, week 0–3)
//
// Intensities are normalized 0–4 against the strongest cell in the set.

/** Map `[key, value]` entries to 0–4 levels, max-normalized. */
export function normalizeIntensities(entries: [string, number][]): Map<string, number> {
  const max = Math.max(1, ...entries.map(([, v]) => v));
  const map = new Map<string, number>();
  for (const [key, v] of entries) {
    map.set(key, v <= 0 ? 0 : Math.min(4, Math.floor((v / max) * 5)));
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
  refunds: (q) => getVoidedSalesSummary(q.from, q.to, q.sessionToken),
  'top-items': loadTopItems,
  category: (q) => getCategoryBreakdown(q.from, q.to, q.sessionToken),
  basket: (q) => getBasketSize(q.from, q.to, q.sessionToken),
  inventory: (q) =>
    Promise.all([
      getInventoryTurnover(q.from, q.to, q.sessionToken, 'default'),
      getInventoryTrend(q.from, q.to, q.sessionToken),
    ]),
  'low-stock': (q) => getLowStockAlerts(10, q.sessionToken),
  waitstaff: loadStaff,
  voids: (q) =>
    Promise.all([
      getVoidedSalesSummary(q.from, q.to, q.sessionToken),
      getVoidedItems(q.from, q.to, q.sessionToken, 5),
    ]),
};

/** Cards that keep deterministic demo data (no backend query yet). */
export const DEMO_CARDS = new Set(['tables', 'occupancy']);

// ── Re-export the raw row types for card-side mapping ───────────────

export type {
  BasketSizeRow,
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
