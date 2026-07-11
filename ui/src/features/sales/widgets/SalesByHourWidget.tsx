import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { exportSalesByHour, type SalesByHourRow } from '@/api/sales';
import { formatMoney, type Money } from '@/types/domain';
/**
 * Sales by Hour Widget — shows a bar chart of sales broken down
 * by hour of the day. Registered with the WidgetRegistry.
 *
 * This widget is designed to be rendered inside a container Card
 * provided by the host dashboard page.
 */
export default function SalesByHourWidget() {
  const [hourly, setHourly] = useState<SalesByHourRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const h = await exportSalesByHour();
      setHourly(h);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const peakHour = Math.max(...hourly.map((h) => h.total_minor), 0);
  const currency = 'USD';

  if (loading) {
    return (
      <div className="reporting-widget">
        <div className="reporting-widget-loading">
          <Localized id="sales-dashboard-loading">
            <span>Loading&hellip;</span>
          </Localized>
        </div>
      </div>
    );
  }

  return (
    <div className="reporting-widget reporting-widget--hourly" aria-label="Sales by hour">
      <div className="reporting-widget-header">
        <Localized id="sales-dashboard-hourly-title">
          <h3 className="reporting-widget-title">Sales by Hour</h3>
        </Localized>
      </div>
      <div className="reporting-widget-hourly-chart" role="list" aria-label="Hourly sales bars">
        {hourly.map((h) => {
          const barPct = peakHour > 0 ? Math.round((h.total_minor / peakHour) * 100) : 0;
          return (
            <div key={h.hour} className="reporting-widget-hour-bar-row"
                 role="listitem"
                 aria-label={`${String(h.hour).padStart(2, '0')}:00 — ${h.sale_count} sales, ${formatMoney({ minor_units: h.total_minor, currency } as Money)}`}>
              <span className="reporting-widget-hour-label">
                {String(h.hour).padStart(2, '0')}
              </span>
              <div className="reporting-widget-hour-bar-track">
                <div
                  className={`reporting-widget-hour-bar ${barPct > 0 ? 'reporting-widget-hour-bar--active' : ''}`}
                  style={{ width: `${Math.max(barPct, h.total_minor > 0 ? 4 : 0)}%` }}
                />
              </div>
              <span className="reporting-widget-hour-value">
                {formatMoney({ minor_units: h.total_minor, currency } as Money)}
              </span>
            </div>
          );
        })}
        {hourly.length === 0 && (
          <Localized id="sales-dashboard-no-data">
            <p className="reporting-widget-no-data">No data for today</p>
          </Localized>
        )}
      </div>
    </div>
  );
}
