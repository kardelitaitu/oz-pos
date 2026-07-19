import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized } from '@fluent/react';
import { getDailyRevenue } from '@/api/reports';
import { Skeleton } from '@/components/Skeleton';
import CanvasLineChart from '@/components/charts/CanvasLineChart';
import type { LineChartPoint } from '@/components/charts/CanvasLineChart';

/** Canvas 2D revenue line chart widget for the reporting dashboard. */
export default function RevenueLineChartWidget() {
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
      );
      // Convert to chart points — show MM/DD labels
      const points: LineChartPoint[] = rows.map((r) => ({
        label: r.date.slice(5), // "MM-DD"
        value: r.total_minor,
      }));
      setData(points);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

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
    <div className="reporting-widget reporting-widget--revenue" aria-label="14-day revenue chart">
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
