import { loggedInvoke } from '@/utils/logged-invoke';

/** Daily revenue aggregate for a date range. */
export interface DailyRevenueRow {
  date: string;
  total_minor: number;
  currency: string;
  sale_count: number;
  /** Cost of goods sold in minor units (HPP × qty over completed lines). */
  cogs_minor: number;
  /** Gross profit in minor units: revenue − COGS. */
  gross_profit_minor: number;
  /** Gross margin as a percentage of revenue. */
  gross_margin_percent: number;
}

/** Weekly revenue aggregate for a date range. */
export interface WeeklyRevenueRow {
  week_start: string;
  total_minor: number;
  currency: string;
  sale_count: number;
  /** Cost of goods sold in minor units (HPP × qty over completed lines). */
  cogs_minor: number;
  /** Gross profit in minor units: revenue − COGS. */
  gross_profit_minor: number;
  /** Gross margin as a percentage of revenue. */
  gross_margin_percent: number;
}

/** Monthly revenue aggregate for a date range. */
export interface MonthlyRevenueRow {
  month: string;
  total_minor: number;
  currency: string;
  sale_count: number;
  /** Cost of goods sold in minor units (HPP × qty over completed lines). */
  cogs_minor: number;
  /** Gross profit in minor units: revenue − COGS. */
  gross_profit_minor: number;
  /** Gross margin as a percentage of revenue. */
  gross_margin_percent: number;
}

/** Top-selling product within a date range. */
export interface TopProductRow {
  product_id: string;
  sku: string;
  name: string;
  total_qty: number;
  total_minor: number;
  cogs_minor: number;
  gross_profit_minor: number;
  gross_margin_percent: number;
}

/** Sales volume by day-of-week and hour for heatmap visualisation. */
export interface HourlyHeatmapRow {
  day_of_week: number;
  hour: number;
  total_minor: number;
  sale_count: number;
}

/** A product whose stock has fallen below the configured threshold. */
export interface LowStockAlert {
  product_id: string;
  sku: string;
  name: string;
  current_qty: number;
  threshold: number;
  currency: string;
  /** Selling price per unit in minor units. */
  price_minor: number;
  /** Cost (HPP) per unit in minor units. */
  cost_minor: number;
}

/** Sales breakdown by product category. */
export interface CategoryBreakdownRow {
  category_id: string | null;
  category_name: string;
  total_minor: number;
  sale_count: number;
  percentage: number;
}

/** One product inside a category's popularity leaderboard. */
export interface CategoryTopProduct {
  sku: string;
  name: string;
  /** Materialized popularity score (category-smoothed). */
  popularity_score: number;
  /** 1-based rank within the category by score. */
  rank: number;
  /** Category-relative standing: 1.0 = most popular, 0.0 = least. */
  percentile: number;
}

/** Per-category popularity standings (ADR #37 per-category evolution). */
export interface CategoryPopularityRow {
  /** Category id; empty string for uncategorized products. */
  category_id: string;
  /** Category name; `null` for uncategorized. */
  category_name: string | null;
  product_count: number;
  /** Mean popularity score across the category's products. */
  mean_score: number;
  /** `mean_score` relative to the catalog mean (1.0 = average). */
  catalog_ratio: number;
  /** The category's most popular products, ranked by score. */
  top_products: CategoryTopProduct[];
}

/** Get daily revenue aggregates for a date range in the active store. */
export const getDailyRevenue = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<DailyRevenueRow[]> =>
  loggedInvoke<DailyRevenueRow[]>('get_daily_revenue_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

/** Get weekly revenue aggregates for a date range in the active store. */
export const getWeeklyRevenue = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<WeeklyRevenueRow[]> =>
  loggedInvoke<WeeklyRevenueRow[]>('get_weekly_revenue_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

/** Get monthly revenue aggregates for a date range in the active store. */
export const getMonthlyRevenue = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<MonthlyRevenueRow[]> =>
  loggedInvoke<MonthlyRevenueRow[]>('get_monthly_revenue_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

/** Get top-selling products for a date range in the active store, ranked by
 * `orderBy` — `'revenue'` (default) or `'profit'` (gross profit). */
export const getTopProducts = (
  startDate: string,
  endDate: string,
  limit: number,
  sessionToken: string,
  orderBy: 'revenue' | 'profit' = 'revenue',
): Promise<TopProductRow[]> =>
  loggedInvoke<TopProductRow[]>('get_top_products_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
    limit,
    orderBy,
  });

/** Get per-category popularity standings for the active store: each
 * category's mean score, its ratio to the catalog average, and its top
 * `topPerCategory` products ranked by popularity. */
export const getCategoryPopularity = (
  sessionToken: string,
  topPerCategory = 3,
): Promise<CategoryPopularityRow[]> =>
  loggedInvoke<CategoryPopularityRow[]>('get_category_popularity_scoped', {
    sessionToken: sessionToken ?? '',
    topPerCategory,
  });

/** Get hourly sales heatmap data for a date range in the active store. */
export const getHourlyHeatmap = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<HourlyHeatmapRow[]> =>
  loggedInvoke<HourlyHeatmapRow[]>('get_hourly_heatmap_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

/** Get products with stock levels below a given threshold in the active store. */
export const getLowStockAlerts = (
  threshold: number,
  sessionToken: string,
): Promise<LowStockAlert[]> =>
  loggedInvoke<LowStockAlert[]>('get_low_stock_alerts_scoped', {
    sessionToken: sessionToken ?? '',
    threshold,
  });

/** Get sales breakdown by product category for a date range in the active store. */
export const getCategoryBreakdown = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<CategoryBreakdownRow[]> =>
  loggedInvoke<CategoryBreakdownRow[]>('get_category_breakdown_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

/** A single product row in the menu engineering report. */
export interface MenuEngineeringRow {
  product_id: string;
  sku: string;
  name: string;
  total_volume: number;
  unit_price_minor: number;
  unit_cost_minor: number;
  margin_per_unit: number;
  total_margin_minor: number;
  total_revenue_minor: number;
}

/** Menu engineering quadrant classification based on volume and margin. */
export type MenuQuadrant = 'Star' | 'Plowhorse' | 'Puzzle' | 'Dog';

/** Menu engineering result with rows and median values for quadrant classification. */
export interface MenuEngineeringResult {
  rows: MenuEngineeringRow[];
  median_volume: number;
  median_margin: number;
}

/** Get menu engineering data for the active store and date range. */
export const getMenuEngineering = (
  startDate: string,
  endDate: string,
  sessionToken: string,
): Promise<MenuEngineeringResult> =>
  loggedInvoke<MenuEngineeringResult>('get_menu_engineering_scoped', {
    sessionToken: sessionToken ?? '',
    startDate,
    endDate,
  });

// ── Per-sale-line margin (HPP exposure) ────────────────────────────

/** One enriched sale line with cost and margin figures (HPP). */
export interface SaleLineMarginDto {
  sale_line_id: string;
  sku: string;
  name: string;
  qty: number;
  unit_price_minor: number;
  line_total_minor: number;
  unit_cost_minor: number;
  margin_minor: number;
  margin_percent: number;
}

/** Get per-line cost and margin for a single sale (ADR #36 HPP exposure). */
export const getSaleLineMarginsScoped = (
  sessionToken: string,
  saleId: string,
): Promise<SaleLineMarginDto[]> =>
  loggedInvoke<SaleLineMarginDto[]>('get_sale_line_margins_scoped', {
    sessionToken: sessionToken ?? '',
    saleId,
  });

// ── Custom Report Builder (P24) ──────────────────────────────────

/** Request payload for the custom report builder. */
export interface CustomReportRequest {
  dataset: string;
  columns: string[];
  start_date: string | null;
  end_date: string | null;
}

/** Response from the custom report builder — generic grid for table/CSV. */
export interface CustomReportResponse {
  columns: string[];
  rows: string[][];
}

/** Build a custom report for the active store. */
export const buildCustomReport = (
  request: CustomReportRequest,
  sessionToken: string,
): Promise<CustomReportResponse> =>
  loggedInvoke<CustomReportResponse>('build_custom_report_scoped', {
    sessionToken: sessionToken ?? '',
    request,
  });
