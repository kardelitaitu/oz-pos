/**
 * Sales Widgets — register reporting dashboard widgets with the
 * WidgetRegistry so they can be rendered dynamically on the
 * reporting dashboard page.
 */
import { registerWidget } from '@/platform/ui/widget-registry';
import DailyTotalWidget from './DailyTotalWidget';
import SalesByHourWidget from './SalesByHourWidget';

export { DailyTotalWidget, SalesByHourWidget };

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
}
