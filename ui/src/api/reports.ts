import { invoke } from '@tauri-apps/api/core';

export interface DailyRevenueRow {
  date: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

export interface WeeklyRevenueRow {
  week_start: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

export interface MonthlyRevenueRow {
  month: string;
  total_minor: number;
  currency: string;
  sale_count: number;
}

export interface TopProductRow {
  product_id: string;
  sku: string;
  name: string;
  total_qty: number;
  total_minor: number;
}

export interface HourlyHeatmapRow {
  day_of_week: number;
  hour: number;
  total_minor: number;
  sale_count: number;
}

export interface LowStockAlert {
  product_id: string;
  sku: string;
  name: string;
  current_qty: number;
  threshold: number;
}

export interface CategoryBreakdownRow {
  category_id: string | null;
  category_name: string;
  total_minor: number;
  sale_count: number;
  percentage: number;
}

export const getDailyRevenue = (startDate: string, endDate: string): Promise<DailyRevenueRow[]> =>
  invoke<DailyRevenueRow[]>('get_daily_revenue', { startDate, endDate });

export const getWeeklyRevenue = (startDate: string, endDate: string): Promise<WeeklyRevenueRow[]> =>
  invoke<WeeklyRevenueRow[]>('get_weekly_revenue', { startDate, endDate });

export const getMonthlyRevenue = (startDate: string, endDate: string): Promise<MonthlyRevenueRow[]> =>
  invoke<MonthlyRevenueRow[]>('get_monthly_revenue', { startDate, endDate });

export const getTopProducts = (startDate: string, endDate: string, limit: number): Promise<TopProductRow[]> =>
  invoke<TopProductRow[]>('get_top_products', { startDate, endDate, limit });

export const getHourlyHeatmap = (startDate: string, endDate: string): Promise<HourlyHeatmapRow[]> =>
  invoke<HourlyHeatmapRow[]>('get_hourly_heatmap', { startDate, endDate });

export const getLowStockAlerts = (threshold: number): Promise<LowStockAlert[]> =>
  invoke<LowStockAlert[]>('get_low_stock_alerts', { threshold });

export const getCategoryBreakdown = (startDate: string, endDate: string): Promise<CategoryBreakdownRow[]> =>
  invoke<CategoryBreakdownRow[]>('get_category_breakdown', { startDate, endDate });

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

export type MenuQuadrant = 'Star' | 'Plowhorse' | 'Puzzle' | 'Dog';

export interface MenuEngineeringResult {
  rows: MenuEngineeringRow[];
  median_volume: number;
  median_margin: number;
}

export const getMenuEngineering = (
  startDate: string,
  endDate: string,
): Promise<MenuEngineeringResult> =>
  invoke<MenuEngineeringResult>('get_menu_engineering', {
    startDate,
    endDate,
  });
