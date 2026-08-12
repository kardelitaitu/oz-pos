import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Real-data IPC mocks ─────────────────────────────────────────────
// The loaders call through the scoped reporting commands; jsdom has no
// Tauri backend, so mock the API modules with deterministic rows.

vi.mock('@/api/reports', () => ({
  getDailyRevenue: vi.fn(() => Promise.resolve([
    { date: '2026-07-27', total_minor: 1250000, currency: 'USD', sale_count: 12, cogs_minor: 500000, gross_profit_minor: 750000, gross_margin_percent: 60 },
  ])),
  getWeeklyRevenue: vi.fn(() => Promise.resolve([
    { week_start: '2026-07-21', total_minor: 8500000, currency: 'USD', sale_count: 65, cogs_minor: 3400000, gross_profit_minor: 5100000, gross_margin_percent: 60 },
  ])),
  getMonthlyRevenue: vi.fn(() => Promise.resolve([
    { month: '2026-07', total_minor: 35000000, currency: 'USD', sale_count: 280, cogs_minor: 14000000, gross_profit_minor: 21000000, gross_margin_percent: 60 },
  ])),
  getHourlyHeatmap: vi.fn(() => Promise.resolve([
    { day_of_week: 1, hour: 10, total_minor: 350000, sale_count: 3 },
    { day_of_week: 2, hour: 11, total_minor: 400000, sale_count: 4 },
  ])),
  getTopProducts: vi.fn(() => Promise.resolve([])),
  getLowStockAlerts: vi.fn(() => Promise.resolve([])),
  getCategoryBreakdown: vi.fn(() => Promise.resolve([])),
  getMenuEngineering: vi.fn(() => Promise.resolve({ rows: [], median_volume: 0, median_margin: 0 })),
  getBasketSize: vi.fn(() => Promise.resolve({ sale_count: 0, avg_line_count: 0 })),
  getBasketSizeTrend: vi.fn(() => Promise.resolve([])),
  getCustomerSplit: vi.fn(() => Promise.resolve({ new_count: 0, returning_count: 0 })),
  getPaymentMethodBreakdown: vi.fn(() => Promise.resolve([])),
  getDiscountsSummary: vi.fn(() => Promise.resolve({ sale_count: 0, discounted_sale_count: 0, share_percent: 0, codes: [] })),
  getVoidedSalesSummary: vi.fn(() => Promise.resolve({ void_count: 0, void_total_minor: 0 })),
  getVoidedItems: vi.fn(() => Promise.resolve([])),
  getInventoryTurnover: vi.fn(() => Promise.resolve({ units_sold: 0, stock_on_hand: 0, sku_count: 0, range_days: 0 })),
  getInventoryTrend: vi.fn(() => Promise.resolve([])),
  getTableTurnover: vi.fn(() => Promise.resolve([])),
  getHourlyOccupancy: vi.fn(() => Promise.resolve([
    { hour: 11, table_orders: 20 },
    { hour: 12, table_orders: 40 },
    { hour: 19, table_orders: 50 },
    { hour: 20, table_orders: 25 },
  ])),
}));

vi.mock('@/api/analytics', () => ({
  getStaffAnalyticsScoped: vi.fn(() => Promise.resolve([])),
}));

vi.mock('@/api/tables', () => ({
  listTablesScoped: vi.fn(() => Promise.resolve([
    { id: 'table-01', name: 'Table 1', capacity: 4, pos_x: 10, pos_y: 10, shape: 'circle', width: 8, height: 8, status: 'occupied', active_sale_id: 'sale-1', section: 'Indoor', active: true, sort_order: 1 },
    { id: 'table-02', name: 'Table 2', capacity: 4, pos_x: 30, pos_y: 10, shape: 'circle', width: 8, height: 8, status: 'occupied', active_sale_id: 'sale-2', section: 'Indoor', active: true, sort_order: 2 },
    { id: 'table-03', name: 'Table 3', capacity: 2, pos_x: 50, pos_y: 10, shape: 'circle', width: 8, height: 8, status: 'available', active_sale_id: null, section: 'Indoor', active: true, sort_order: 3 },
    { id: 'table-04', name: 'Table 4', capacity: 6, pos_x: 70, pos_y: 10, shape: 'circle', width: 8, height: 8, status: 'available', active_sale_id: null, section: 'Patio', active: true, sort_order: 4 },
    { id: 'table-05', name: 'Table 5', capacity: 2, pos_x: 20, pos_y: 40, shape: 'circle', width: 8, height: 8, status: 'cleaning', active_sale_id: null, section: 'Indoor', active: false, sort_order: 5 },
  ])),
}));

import {
  buildHeatmapIntensities,
  alignPrevHourly,
  intensityFromPct,
  loadAov,
  loadBasketSize,
  loadHeatmapRows,
  loadRevenue,
  loadTableOccupancy,
  normalizeIntensities,
  pctOfPeak,
  periodDelta,
  previousRange,
  rangeForGranularity,
  seriesDelta,
  type AnalyticsQuery,
  weekdayIntensities,
  weeklyHourlyIntensities,
  monthDayIntensities,
  yearlyWeekIntensities,
} from '@/features/analytics/analytics-data';

describe('rangeForGranularity — inclusive date windows', () => {
  it('daily is anchored to today', () => {
    const r = rangeForGranularity('daily', '2026-08-01', '2026-08-31');
    expect(r.from).toBe(r.to);
    expect(r.from).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('weekly starts on Monday (Monday-first)', () => {
    const r = rangeForGranularity('weekly', 'ignored', 'ignored');
    const start = new Date(`${r.from}T00:00:00`);
    // Monday is ISO weekday 1; getDay() 1 = Monday.
    expect(start.getDay()).toBe(1);
    expect(r.to >= r.from).toBe(true);
  });

  it('monthly starts on the 1st', () => {
    const r = rangeForGranularity('monthly', 'ignored', 'ignored');
    expect(r.from.endsWith('-01')).toBe(true);
    expect(r.to >= r.from).toBe(true);
  });

  it('yearly starts on Jan 1', () => {
    const r = rangeForGranularity('yearly', 'ignored', 'ignored');
    expect(r.from.endsWith('-01-01')).toBe(true);
  });

  it('custom uses the picked range verbatim', () => {
    expect(rangeForGranularity('custom', '2026-08-03', '2026-08-17')).toEqual({
      from: '2026-08-03',
      to: '2026-08-17',
    });
  });
});

describe('seriesDelta — % change first→last bucket', () => {
  it('returns null with fewer than two buckets', () => {
    expect(seriesDelta([{ label: 'A', value: 100 }])).toBeNull();
    expect(seriesDelta([])).toBeNull();
  });

  it('computes the percentage change across buckets', () => {
    const delta = seriesDelta([
      { label: 'A', value: 100 },
      { label: 'B', value: 150 },
    ]);
    expect(delta).toBe(50);
  });

  it('returns 0 when the first bucket is zero', () => {
    expect(seriesDelta([
      { label: 'A', value: 0 },
      { label: 'B', value: 150 },
    ])).toBe(0);
  });
});

describe('previousRange — equal-length preceding window', () => {
  it('a single day compares against the day before', () => {
    const q: AnalyticsQuery = {
      workspace: 'retail', granularity: 'daily', from: '2026-08-10', to: '2026-08-10', sessionToken: 's',
    };
    expect(previousRange(q)).toMatchObject({ from: '2026-08-09', to: '2026-08-09' });
  });

  it('a multi-day range shifts back by its own span', () => {
    const q: AnalyticsQuery = {
      workspace: 'retail', granularity: 'custom', from: '2026-08-05', to: '2026-08-11', sessionToken: 's',
    };
    expect(previousRange(q)).toMatchObject({ from: '2026-07-29', to: '2026-08-04' });
  });

  it('preserves the workspace/granularity/session of the source query', () => {
    const q: AnalyticsQuery = {
      workspace: 'restaurant', granularity: 'weekly', from: '2026-08-03', to: '2026-08-09', sessionToken: 'tok',
    };
    const prev = previousRange(q);
    expect(prev.workspace).toBe('restaurant');
    expect(prev.granularity).toBe('weekly');
    expect(prev.sessionToken).toBe('tok');
  });
});

describe('alignPrevHourly — hour-aligned compare overlay', () => {
  it('maps previous pct onto the current hour set, filling missing hours with 0', () => {
    // The backend returns only hours with orders, so the two periods can
    // have different active hours — index alignment would misplot the
    // previous curve. Alignment must be by hour, not position.
    const current = [
      { hour: 9, pct: 40 },
      { hour: 10, pct: 60 },
      { hour: 11, pct: 100 },
    ];
    const previous = [
      { hour: 8, pct: 50 }, // absent from current → 0
      { hour: 10, pct: 80 }, // present → mapped by hour
      { hour: 12, pct: 90 }, // absent from current → 0
    ];
    expect(alignPrevHourly(current, previous)).toEqual([0, 80, 0]);
  });

  it('keeps the exact match for identical hour sets regardless of order', () => {
    const current = [
      { hour: 9, pct: 50 },
      { hour: 10, pct: 100 },
    ];
    const previous = [
      { hour: 10, pct: 100 },
      { hour: 9, pct: 50 },
    ];
    expect(alignPrevHourly(current, previous)).toEqual([50, 100]);
  });
});

describe('periodDelta — % change vs the previous period', () => {
  it('computes a one-decimal percent change', () => {
    expect(periodDelta(150, 100)).toBe(50);
    expect(periodDelta(90, 100)).toBe(-10);
  });

  it('returns null without a usable baseline', () => {
    expect(periodDelta(100, 0)).toBeNull();
    expect(periodDelta(Number.NaN, 100)).toBeNull();
  });
});

describe('shared intensity scale — pctOfPeak + intensityFromPct', () => {
  it('pctOfPeak is percent of the strongest value with a floor of 1', () => {
    expect(pctOfPeak([50, 25, 0])).toEqual([100, 50, 0]);
    // All-zero input stays flat at 0 (no NaN from a 0 denominator)
    expect(pctOfPeak([0, 0])).toEqual([0, 0]);
  });

  it('intensityFromPct bins percent-of-peak into the 0–4 heat levels', () => {
    // level = ⌊pct / 20⌋: 0–19% → 0, 20–39% → 1, … 80–100% → 4
    expect(intensityFromPct(0)).toBe(0);
    expect(intensityFromPct(19)).toBe(0);
    expect(intensityFromPct(20)).toBe(1);
    expect(intensityFromPct(39)).toBe(1);
    expect(intensityFromPct(60)).toBe(3);
    expect(intensityFromPct(80)).toBe(4);
    expect(intensityFromPct(100)).toBe(4);
  });

  it('normalizeIntensities and intensityFromPct agree on the same scale', () => {
    // The heatmap cell levels are exactly the binned occupancy pcts — a
    // cell at level 3 is the same intensity as an occupancy curve at 60%.
    const map = normalizeIntensities([
      ['a', 100],
      ['b', 50],
      ['c', 20],
      ['d', 0],
    ]);
    expect(map.get('a')).toBe(4);
    expect(map.get('b')).toBe(2);
    expect(map.get('c')).toBe(1);
    expect(map.get('d')).toBe(0);
  });

  it('handles empty input without NaN', () => {
    const map = normalizeIntensities([]);
    expect(map.size).toBe(0);
  });
});

describe('heatmap intensity builders', () => {
  it('weekdayIntensities aggregates hourly rows by Monday-first day', () => {
    // day_of_week 1 = Monday (JS getDay), day_of_week 7 = Sunday.
    const map = weekdayIntensities([
      { day_of_week: 1, hour: 9, total_minor: 100, sale_count: 1 },
      { day_of_week: 1, hour: 12, total_minor: 300, sale_count: 2 },
      { day_of_week: 7, hour: 19, total_minor: 200, sale_count: 2 },
    ]);
    // Monday aggregates 100 + 300 = 400 → strongest; Sunday 200 → 2.
    expect(map.get('0')).toBe(4);
    expect(map.get('6')).toBe(2);
  });

  it('weeklyHourlyIntensities keys are dayIdx:hour', () => {
    const map = weeklyHourlyIntensities([
      { day_of_week: 1, hour: 10, total_minor: 100, sale_count: 1 },
    ]);
    expect(map.get('0:10')).toBe(4);
  });

  it('monthDayIntensities keys are day-of-month', () => {
    const map = monthDayIntensities([
      { date: '2026-07-27', total_minor: 100, currency: 'USD', sale_count: 1, cogs_minor: 0, gross_profit_minor: 100, gross_margin_percent: 100 },
    ]);
    expect(map.get('27')).toBe(4);
  });

  it('yearlyWeekIntensities keys are month:week with week capped at 3', () => {
    const map = yearlyWeekIntensities([
      { week_start: '2026-07-21', total_minor: 100, currency: 'USD', sale_count: 1, cogs_minor: 0, gross_profit_minor: 100, gross_margin_percent: 100 },
    ]);
    // 2026-07-21 is day 21 → (21−1)/7 = 2 → week index 2 (0-based).
    expect(map.get('6:2')).toBe(4);
  });

  it('buildHeatmapIntensities dispatches by granularity', () => {
    const hourly = [{ day_of_week: 1, hour: 10, total_minor: 100, sale_count: 1 }];
    const daily = [{ date: '2026-07-27', total_minor: 100, currency: 'USD', sale_count: 1, cogs_minor: 0, gross_profit_minor: 100, gross_margin_percent: 100 }];
    const weekly = [{ week_start: '2026-07-21', total_minor: 100, currency: 'USD', sale_count: 1, cogs_minor: 0, gross_profit_minor: 100, gross_margin_percent: 100 }];

    expect(buildHeatmapIntensities('weekly', { hourly }).has('0:10')).toBe(true);
    expect(buildHeatmapIntensities('monthly', { daily }).has('27')).toBe(true);
    expect(buildHeatmapIntensities('yearly', { weekly }).has('6:2')).toBe(true);
    // daily + custom fall back to the weekday view from hourly rows.
    expect(buildHeatmapIntensities('daily', { hourly }).has('0')).toBe(true);
    expect(buildHeatmapIntensities('custom', { hourly }).has('0')).toBe(true);
  });
});

describe('loaders — raw rows mapped to card shapes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loadRevenue maps daily rows to labeled buckets', async () => {
    const buckets = await loadRevenue({
      workspace: 'retail', granularity: 'daily', from: '2026-07-27', to: '2026-07-27', sessionToken: 's',
    });
    expect(buckets.length).toBeGreaterThan(0);
    expect(buckets[0]?.label).toBe('07-27');
    expect(buckets[0]?.value).toBe(1250000);
  });

  it('loadAov divides revenue by sale count', async () => {
    const buckets = await loadAov({
      workspace: 'retail', granularity: 'daily', from: '2026-07-27', to: '2026-07-27', sessionToken: 's',
    });
    expect(buckets[0]?.value).toBe(Math.round(1250000 / 12));
  });

  it('loadBasketSize buckets daily rows with weighted averages', async () => {
    const { getBasketSizeTrend } = await import('@/api/reports');
    (getBasketSizeTrend as ReturnType<typeof vi.fn>).mockResolvedValue([
      { date: '2026-08-03', sale_count: 10, avg_line_count: 2 },
      { date: '2026-08-04', sale_count: 20, avg_line_count: 3 },
    ]);
    const trend = await loadBasketSize({
      workspace: 'retail', granularity: 'daily', from: '2026-08-03', to: '2026-08-04', sessionToken: 's',
    });
    // One bucket per day, labeled MM-DD
    expect(trend.buckets).toEqual([
      { label: '08-03', value: 2 },
      { label: '08-04', value: 3 },
    ]);
    expect(trend.sale_count).toBe(30);
    // Weighted mean: (10×2 + 20×3) / 30 = 2.666…
    expect(trend.avg_line_count).toBeCloseTo(2.667, 2);
  });

  it('loadHeatmapRows fetches the granularity-relevant sets', async () => {
    const { from, to, sessionToken } = { from: '2026-07-27', to: '2026-07-27', sessionToken: 's' };

    const hourly = await loadHeatmapRows({ workspace: 'retail', granularity: 'daily', from, to, sessionToken });
    expect(hourly.hourly.length).toBe(2);

    const weekly = await loadHeatmapRows({ workspace: 'retail', granularity: 'weekly', from, to, sessionToken });
    expect(weekly.hourly.length).toBe(2);

    const monthly = await loadHeatmapRows({ workspace: 'retail', granularity: 'monthly', from, to, sessionToken });
    expect(monthly.daily.length).toBeGreaterThan(0);
    expect(monthly.hourly.length).toBe(0);

    const yearly = await loadHeatmapRows({ workspace: 'retail', granularity: 'yearly', from, to, sessionToken });
    expect(yearly.weekly.length).toBeGreaterThan(0);
    expect(yearly.hourly.length).toBe(0);
  });

  it('loadTableOccupancy derives the live rate from the tables snapshot', async () => {
    const occ = await loadTableOccupancy({
      workspace: 'restaurant', granularity: 'daily', from: '2026-07-27', to: '2026-07-27', sessionToken: 's',
    });
    // 4 active tables, 2 occupied (the inactive 'cleaning' row is excluded)
    expect(occ.total).toBe(4);
    expect(occ.occupied).toBe(2);
    expect(occ.rate).toBe(50);
    // Seats: occupied 4 + 4 = 8 of 4 + 4 + 2 + 6 = 16 total
    expect(occ.seats_used).toBe(8);
    expect(occ.seats_total).toBe(16);
    // Hourly curve: completed table orders normalized to % of the peak
    // hour (19:00 with 50 orders) on the heatmap's shared 0–4 level scale,
    // sorted ascending by hour
    expect(occ.hourly).toEqual([
      { hour: 11, table_orders: 20, pct: 40, level: 2 },
      { hour: 12, table_orders: 40, pct: 80, level: 4 },
      { hour: 19, table_orders: 50, pct: 100, level: 4 },
      { hour: 20, table_orders: 25, pct: 50, level: 2 },
    ]);
    expect(occ.peak_hour).toBe(19);
  });

  it('loadTableOccupancy yields 0 when no active tables exist', async () => {
    const { listTablesScoped } = await import('@/api/tables');
    (listTablesScoped as ReturnType<typeof vi.fn>).mockResolvedValueOnce([]);
    const occ = await loadTableOccupancy({
      workspace: 'restaurant', granularity: 'daily', from: '2026-07-27', to: '2026-07-27', sessionToken: 's',
    });
    expect(occ.total).toBe(0);
    expect(occ.rate).toBe(0);
    expect(occ.seats_used).toBe(0);
    expect(occ.seats_total).toBe(0);
    // The hourly activity query is independent of the floor plan snapshot
    expect(occ.peak_hour).toBe(19);
  });
});
