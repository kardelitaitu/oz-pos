import { Localized, useLocalization } from '@fluent/react';
import { requiredLocalized } from '@/frontend/shared';
import { getWidgets } from '@/platform/ui/widget-registry';
import { useFeatures } from '@/hooks/useFeatures';
import { Card } from '@/components/Card';
import { LazyBoundary } from '@/components/LazyBoundary';
import './widgets/widgets.css';
import './SalesDashboardScreen.css';

/**
 * Sales Dashboard page — renders all registered reporting widgets
 * from the WidgetRegistry on a responsive grid.
 *
 * Widgets are filtered by enabled features so feature-gated widgets
 * only appear when their feature is turned on.
 */
export default function SalesDashboardScreen() {
  const { l10n } = useLocalization();
  const { enabled } = useFeatures();
  const widgets = getWidgets(enabled);

  return (
    <div className="reporting-dashboard" role="region" aria-label={requiredLocalized(l10n, 'sales-dashboard-region-aria')}>
      <Localized id="sales-dashboard-title">
        <h1 className="reporting-dashboard-title">Sales Dashboard</h1>
      </Localized>

      {widgets.length === 0 ? (
        <div className="reporting-dashboard-empty">
          <Localized id="sales-dashboard-no-data">
            <p>No widgets registered. Enable features to see reporting data.</p>
          </Localized>
        </div>
      ) : (
        <div className="reporting-dashboard-grid" role="list" aria-label={requiredLocalized(l10n, 'sales-dashboard-grid-aria')}>
          {widgets.map((w) => {
            const WidgetComponent = w.component;
            const widthClass = w.width === 2 ? 'widget-width-2' : 'widget-width-1';
            return (
              <div key={w.id} role="listitem" aria-label={w.title}>
                <Card shadow="sm" className={widthClass}>
                  {/* PERF-01: lazy widgets need a Suspense boundary per card */}
                  <LazyBoundary>
                    <WidgetComponent />
                  </LazyBoundary>
                </Card>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
