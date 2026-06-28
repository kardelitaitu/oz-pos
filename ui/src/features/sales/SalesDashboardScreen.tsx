import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import {
  exportDailySummary,
  exportSalesByHour,
  type DailySummaryRow,
  type SalesByHourRow,
} from '@/api/pos';
import { formatMoney, type Money } from '@/types/domain';
import { Card } from '@/components/Card';
import './SalesDashboardScreen.css';

export default function SalesDashboardScreen() {
  const [summary, setSummary] = useState<DailySummaryRow[]>([]);
  const [hourly, setHourly] = useState<SalesByHourRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [s, h] = await Promise.all([
        exportDailySummary(),
        exportSalesByHour(),
      ]);
      setSummary(s);
      setHourly(h);
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
  const peakHour = Math.max(...hourly.map((h) => h.total_minor), 0);

  return (
    <div className="sales-dashboard">
      <Localized id="sales-dashboard-title">
        <h1 className="sales-dashboard-title">Sales Dashboard</h1>
      </Localized>

      {loading ? (
        <Localized id="sales-dashboard-loading">
          <p className="sales-dashboard-loading">Loading&hellip;</p>
        </Localized>
      ) : (
        <>
          <div className="sales-dashboard-kpi-row">
            <Card shadow="sm">
              <div className="sales-dashboard-kpi">
                <Localized id="sales-dashboard-daily-total">
                  <span className="sales-dashboard-kpi-label">Daily Total</span>
                </Localized>
                <span className="sales-dashboard-kpi-value">
                  {formatMoney({ minor_units: dailyTotal, currency } as Money)}
                </span>
              </div>
            </Card>
            <Card shadow="sm">
              <div className="sales-dashboard-kpi">
                <Localized id="sales-dashboard-total-sales">
                  <span className="sales-dashboard-kpi-label">Total Sales</span>
                </Localized>
                <span className="sales-dashboard-kpi-value">{totalSales}</span>
              </div>
            </Card>
            <Card shadow="sm">
              <div className="sales-dashboard-kpi">
                <Localized id="sales-dashboard-total-items">
                  <span className="sales-dashboard-kpi-label">Total Items</span>
                </Localized>
                <span className="sales-dashboard-kpi-value">{totalItems}</span>
              </div>
            </Card>
          </div>

          <div className="sales-dashboard-section">
            <Localized id="sales-dashboard-hourly-title">
              <h2 className="sales-dashboard-section-title">Sales by Hour</h2>
            </Localized>
            <div className="sales-dashboard-hourly-chart" aria-label="Sales by hour bar chart">
              {hourly.map((h) => {
                const barPct = peakHour > 0 ? Math.round((h.total_minor / peakHour) * 100) : 0;
                return (
                  <div key={h.hour} className="sales-dashboard-hour-bar-row"
                       aria-label={`${String(h.hour).padStart(2, '0')}:00 — ${h.sale_count} sales, ${formatMoney({ minor_units: h.total_minor, currency } as Money)}`}>
                    <span className="sales-dashboard-hour-label">
                      {String(h.hour).padStart(2, '0')}
                    </span>
                    <div className="sales-dashboard-hour-bar-track">
                      <div className={`sales-dashboard-hour-bar ${barPct > 0 ? 'sales-dashboard-hour-bar--active' : ''}`}
                           style={{ width: `${Math.max(barPct, h.total_minor > 0 ? 4 : 0)}%` }} />
                    </div>
                    <span className="sales-dashboard-hour-value">
                      {formatMoney({ minor_units: h.total_minor, currency } as Money)}
                    </span>
                  </div>
                );
              })}
              {hourly.length === 0 && (
                <Localized id="sales-dashboard-no-data">
                  <p className="sales-dashboard-no-data">No data for today</p>
                </Localized>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
