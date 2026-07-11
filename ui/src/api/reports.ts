import { invoke } from '@tauri-apps/api/core';

/** Daily revenue aggregate for a date range. */
export interface DailyRevenueRow {
  date: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

/** Weekly revenue aggregate for a date range. */
export interface WeeklyRevenueRow {
  week_start: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

/** Monthly revenue aggregate for a date range. */
export interface MonthlyRevenueRow {
  month: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

/** Top-selling product within a date range. */
export interface TopProductRow {
  product_id: string;
  sku: string;
  name: string;
  total_qty: number;
  total_minor: number;
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
}

/** Sales breakdown by product category. */
export interface CategoryBreakdownRow {
  category_id: string | null;
  category_name: string;
  total_minor: number;
  sale_count: number;
  percentage: number;
}

/** Get daily revenue aggregates for a date range. */
export const getDailyRevenue = (startDate: string, endDate: string): Promise<DailyRevenueRow[]> =>
  invoke<DailyRevenueRow[]>('get_daily_revenue', { startDate, endDate });

/** Get weekly revenue aggregates for a date range. */
export const getWeeklyRevenue = (startDate: string, endDate: string): Promise<WeeklyRevenueRow[]> =>
  invoke<WeeklyRevenueRow[]>('get_weekly_revenue', { startDate, endDate });

/** Get monthly revenue aggregates for a date range. */
export const getMonthlyRevenue = (startDate: string, endDate: string): Promise<MonthlyRevenueRow[]> =>
  invoke<MonthlyRevenueRow[]>('get_monthly_revenue', { startDate, endDate });

/** Get top-selling products for a date range, limited to a given count. */
export const getTopProducts = (startDate: string, endDate: string, limit: number): Promise<TopProductRow[]> =>
  invoke<TopProductRow[]>('get_top_products', { startDate, endDate, limit });

/** Get hourly sales heatmap data for a date range. */
export const getHourlyHeatmap = (startDate: string, endDate: string): Promise<HourlyHeatmapRow[]> =>
  invoke<HourlyHeatmapRow[]>('get_hourly_heatmap', { startDate, endDate });

/** Get products with stock levels below a given threshold. */
export const getLowStockAlerts = (threshold: number): Promise<LowStockAlert[]> =>
  invoke<LowStockAlert[]>('get_low_stock_alerts', { threshold });

/** Get sales breakdown by product category for a date range. */
export const getCategoryBreakdown = (startDate: string, endDate: string): Promise<CategoryBreakdownRow[]> =>
  invoke<CategoryBreakdownRow[]>('get_category_breakdown', { startDate, endDate });

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

/** Get menu engineering data (volume and margin) for a date range. */
export const getMenuEngineering = (
  startDate: string,
  endDate: string,
): Promise<MenuEngineeringResult> =>
  invoke<MenuEngineeringResult>('get_menu_engineering', {
    startDate,
    endDate,
  });
