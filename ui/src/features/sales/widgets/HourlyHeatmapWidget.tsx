import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { getHourlyHeatmap } from '@/api/reports';
import { Skeleton } from '@/components/Skeleton';
import CanvasHeatmap from '@/components/charts/CanvasHeatmap';
import type { HeatmapCell } from '@/components/charts/CanvasHeatmap';

/** Canvas 2D hourly heatmap widget for the reporting dashboard. */
export default function HourlyHeatmapWidget() {
  const [cells, setCells] = useState<HeatmapCell[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const end = new Date();
      const start = new Date();
      start.setDate(start.getDate() - 7);
      const rows = await getHourlyHeatmap(
        start.toISOString().slice(0, 10),
        end.toISOString().slice(0, 10),
      );
      setCells(
        rows.map((r) => ({
          dayOfWeek: r.day_of_week,
          hour: r.hour,
          value: r.total_minor,
        })),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  if (loading) {
    return (
      <div className="reporting-widget" aria-hidden="true">
        <div className="reporting-widget-header">
          <Skeleton width="7rem" height="0.875rem" />
        </div>
        <Skeleton variant="block" width="100%" height="140px" style={{ borderRadius: 'var(--radius-md)' }} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-header">
          <Localized id="sales-dashboard-heatmap-title">
            <h3 className="reporting-widget-title">Busiest Hours</h3>
          </Localized>
        </div>
        <p className="reporting-widget-no-data">{error}</p>
      </div>
    );
  }

  if (cells.length === 0) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-header">
          <Localized id="sales-dashboard-heatmap-title">
            <h3 className="reporting-widget-title">Busiest Hours</h3>
          </Localized>
        </div>
        <p className="reporting-widget-no-data">
          <Localized id="sales-dashboard-no-data">
            <span>No data for this period</span>
          </Localized>
        </p>
      </div>
    );
  }

  return (
    <div className="reporting-widget reporting-widget--heatmap" aria-label="Hourly sales heatmap">
      <div className="reporting-widget-header">
        <Localized id="sales-dashboard-heatmap-title">
          <h3 className="reporting-widget-title">Busiest Hours</h3>
        </Localized>
      </div>
      <CanvasHeatmap
        data={cells}
        formatValue={(v) =>
          new Intl.NumberFormat('en', {
            style: 'currency',
            currency: 'USD',
            minimumFractionDigits: 0,
          }).format(v / 100)
        }
        minHeight="140px"
      />
    </div>
  );
}
