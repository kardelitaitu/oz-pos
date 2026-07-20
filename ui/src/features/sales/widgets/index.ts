/**
 * Sales Widgets — register reporting dashboard widgets with the
 * WidgetRegistry so they can be rendered dynamically on the
 * reporting dashboard page.
 */
import { registerWidget } from '@/platform/ui/widget-registry';
import DailyTotalWidget from './DailyTotalWidget';
import SalesByHourWidget from './SalesByHourWidget';
import RevenueLineChartWidget from './RevenueLineChartWidget';
import CategoryPieChartWidget from './CategoryPieChartWidget';
import HourlyHeatmapWidget from './HourlyHeatmapWidget';

export {
  DailyTotalWidget,
  SalesByHourWidget,
  RevenueLineChartWidget,
  CategoryPieChartWidget,
  HourlyHeatmapWidget,
};

/**
 * Register all sales reporting widgets with the platform.
 * Called once from App.tsx during app initialisation.
 */
export function registerSalesWidgets(): void {
  registerWidget({
    id: 'daily-total',
    component: DailyTotalWidget,
    title: 'Daily Summary',
    feature: 'simple-retail',
    width: 2,
  });

  registerWidget({
    id: 'sales-by-hour',
    component: SalesByHourWidget,
    title: 'Sales by Hour',
    feature: 'simple-retail',
    width: 2,
    height: 2,
  });

  registerWidget({
    id: 'revenue-line-chart',
    component: RevenueLineChartWidget,
    title: 'Revenue (14d)',
    feature: 'simple-retail',
    width: 2,
  });

  registerWidget({
    id: 'category-pie-chart',
    component: CategoryPieChartWidget,
    title: 'By Category',
    feature: 'simple-retail',
    width: 1,
  });

  registerWidget({
    id: 'hourly-heatmap',
    component: HourlyHeatmapWidget,
    title: 'Busiest Hours',
    feature: 'simple-retail',
    width: 1,
  });
}
