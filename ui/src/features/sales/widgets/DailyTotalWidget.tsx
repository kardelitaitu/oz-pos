import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { exportDailySummary, type DailySummaryRow } from '@/api/sales';
import { formatMoney, type Money } from '@/types/domain';
import { Skeleton } from '@/components/Skeleton';
/**
 * Daily Total Widget — shows revenue, sales count, and item count
 * for the current day. Registered with the WidgetRegistry so it
 * can be rendered on any dashboard page.
 *
 * This widget is designed to be rendered inside a container Card
 * provided by the host dashboard page.
 */
export default function DailyTotalWidget() {
  const [summary, setSummary] = useState<DailySummaryRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const s = await exportDailySummary();
      setSummary(s);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const dailyTotal = summary.reduce((acc, r) => acc + r.total_minor, 0);
  const totalSales = summary.length;
  const totalItems = summary.reduce((acc, r) => acc + r.line_count, 0);
  const currency = summary[0]?.currency ?? 'USD';

  if (loading) {
    return (
      <div className="reporting-widget" aria-hidden="true">
        <div className="reporting-widget-kpi-row">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="reporting-widget-kpi">
              <Skeleton width="4rem" height="0.75rem" />
              <Skeleton width="5rem" height="1.5rem" style={{ marginTop: '4px' }} />
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="reporting-widget reporting-widget--daily-total" aria-label="Daily sales summary">
      <div className="reporting-widget-header">
        <Localized id="sales-dashboard-daily-total">
          <h3 className="reporting-widget-title">Daily Summary</h3>
        </Localized>
      </div>
      <div className="reporting-widget-kpi-row">
        <div className="reporting-widget-kpi">
          <Localized id="sales-dashboard-daily-total">
            <span className="reporting-widget-kpi-label">Daily Total</span>
          </Localized>
          <span className="reporting-widget-kpi-value reporting-widget-kpi-value--primary">
            {formatMoney({ minor_units: dailyTotal, currency } as Money)}
          </span>
        </div>
        <div className="reporting-widget-kpi">
          <Localized id="sales-dashboard-total-sales">
            <span className="reporting-widget-kpi-label">Sales</span>
          </Localized>
          <span className="reporting-widget-kpi-value">{totalSales}</span>
        </div>
        <div className="reporting-widget-kpi">
          <Localized id="sales-dashboard-total-items">
            <span className="reporting-widget-kpi-label">Items</span>
          </Localized>
          <span className="reporting-widget-kpi-value">{totalItems}</span>
        </div>
      </div>
    </div>
  );
}
