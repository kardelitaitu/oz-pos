import { useContext, useState, useCallback, useEffect, useMemo } from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { WorkspaceContext } from '@/contexts/WorkspaceContext';
import { Localized, useLocalization } from '@fluent/react';
import { getDailyRevenue } from '@/api/reports';
import { l10nErrorMessage } from '@/utils/app-error';
import { Skeleton } from '@/components/Skeleton';
import CanvasLineChart from '@/components/charts/CanvasLineChart';
import type { LineChartPoint } from '@/components/charts/CanvasLineChart';

/** Canvas 2D revenue line chart widget for the reporting dashboard. */
export default function RevenueLineChartWidget() {
  const { l10n } = useLocalization();
  const sessionToken = useContext(WorkspaceContext)?.sessionToken ?? '';
  const [data, setData] = useState<LineChartPoint[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const end = new Date();
      const start = new Date();
      start.setDate(start.getDate() - 13); // 14-day window
      const rows = await getDailyRevenue(
        start.toISOString().slice(0, 10),
        end.toISOString().slice(0, 10),
        sessionToken,
      );
      // Convert to chart points — show MM/DD labels
      const points: LineChartPoint[] = rows.map((r) => ({
        label: r.date.slice(5), // "MM-DD"
        value: r.total_minor,
      }));
      setData(points);
    } catch (e) {
      // ERR-05: never render raw backend messages — map to user-safe copy.
      setError(l10nErrorMessage(e, l10n, 'app-error-generic'));
    } finally {
      setLoading(false);
    }
  }, [sessionToken, l10n]);

  useEffect(() => { load(); }, [load]);

  const totalRevenue = useMemo(
    () => data.reduce((s, d) => s + d.value, 0),
    [data],
  );

  if (loading) {
    return (
      <div className="reporting-widget" aria-hidden="true">
        <div className="reporting-widget-header">
          <Skeleton width="7rem" height="0.875rem" />
        </div>
        <Skeleton variant="block" width="100%" height="200px" style={{ borderRadius: 'var(--radius-md)' }} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-header">
          <Localized id="sales-dashboard-revenue-title">
            <h3 className="reporting-widget-title">Revenue (14d)</h3>
          </Localized>
        </div>
        <p className="reporting-widget-no-data">{error}</p>
      </div>
    );
  }

  return (
    <div className="reporting-widget reporting-widget--revenue" aria-label={requiredLocalized(l10n, 'sales-dashboard-revenue-aria')}>
      <div className="reporting-widget-header">
        <Localized id="sales-dashboard-revenue-title">
          <h3 className="reporting-widget-title">Revenue (14d)</h3>
        </Localized>
        <span className="reporting-widget-kpi-value reporting-widget-kpi-value--primary" style={{ fontSize: 'var(--text-base)', marginTop: 'var(--space-1)' }}>
          {new Intl.NumberFormat('en', {
            style: 'currency',
            currency: 'USD',
            minimumFractionDigits: 2,
          }).format(totalRevenue / 100)}
        </span>
      </div>
      <CanvasLineChart
        data={data}
        label={requiredLocalized(l10n, 'sales-dashboard-revenue-aria')}
        summary={requiredLocalized(l10n, 'sales-dashboard-revenue-summary', {
          total: new Intl.NumberFormat('en', {
            style: 'currency',
            currency: 'USD',
            minimumFractionDigits: 2,
          }).format(totalRevenue / 100),
          days: String(data.length),
        })}
        formatValue={(v) =>
          new Intl.NumberFormat('en', {
            style: 'currency',
            currency: 'USD',
            minimumFractionDigits: 0,
          }).format(v / 100)
        }
        minHeight="200px"
      />
    </div>
  );
}
