import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { getCategoryBreakdown } from '@/api/reports';
import { Skeleton } from '@/components/Skeleton';
import CanvasPieChart from '@/components/charts/CanvasPieChart';
import type { PieSlice } from '@/components/charts/CanvasPieChart';

/** Canvas 2D category breakdown donut chart widget for the reporting dashboard. */
export default function CategoryPieChartWidget() {
  const [slices, setSlices] = useState<PieSlice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const end = new Date();
      const start = new Date();
      start.setDate(start.getDate() - 30);
      const rows = await getCategoryBreakdown(
        start.toISOString().slice(0, 10),
        end.toISOString().slice(0, 10),
      );
      setSlices(
        rows.map((r) => ({
          name: r.category_name || 'Uncategorized',
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
          <Skeleton width="8rem" height="0.875rem" />
        </div>
        <Skeleton variant="block" width="100%" height="200px" style={{ borderRadius: 'var(--radius-md)' }} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-header">
          <Localized id="sales-dashboard-category-title">
            <h3 className="reporting-widget-title">By Category</h3>
          </Localized>
        </div>
        <p className="reporting-widget-no-data">{error}</p>
      </div>
    );
  }

  if (slices.length === 0) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-header">
          <Localized id="sales-dashboard-category-title">
            <h3 className="reporting-widget-title">By Category</h3>
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
    <div className="reporting-widget reporting-widget--category" aria-label="Category breakdown">
      <div className="reporting-widget-header">
        <Localized id="sales-dashboard-category-title">
          <h3 className="reporting-widget-title">By Category</h3>
        </Localized>
      </div>
      <CanvasPieChart
        data={slices}
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
