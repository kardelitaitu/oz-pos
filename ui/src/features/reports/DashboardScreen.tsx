import { useEffect, useState } from 'react';
import { Localized } from '@fluent/react';
import {
  getDailyRevenue,
  getTopProducts,
  getLowStockAlerts,
  type DailyRevenueRow,
  type TopProductRow,
  type LowStockAlert,
} from '@/api/reports';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import './DashboardScreen.css';

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function weekAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 6);
  return d.toISOString().slice(0, 10);
}

function fmtCurrency(minor: number, currency: string): string {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
  }).format(minor / 100);
}

export default function DashboardScreen() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [revenue, setRevenue] = useState<DailyRevenueRow[]>([]);
  const [weeklyRevenue, setWeeklyRevenue] = useState<DailyRevenueRow[]>([]);
  const [topProduct, setTopProduct] = useState<TopProductRow | null>(null);
  const [lowStock, setLowStock] = useState<LowStockAlert[]>([]);

  useEffect(() => {
    const t = today();
    const w = weekAgo();
    Promise.all([
      getDailyRevenue(t, t),
      getDailyRevenue(w, t),
      getTopProducts(t, t, 1),
      getLowStockAlerts(10),
    ])
      .then(([rev, weekRev, top, stock]) => {
        setRevenue(rev);
        setWeeklyRevenue(weekRev);
        setTopProduct(top[0] ?? null);
        setLowStock(stock);
      })
      .catch((e) => {
        setError(e.message ?? String(e));
      })
      .finally(() => {
        setLoading(false);
      });
  }, []);

  if (loading) {
    return (
      <div className="dashboard">
        <Spinner aria-label="Loading dashboard" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="dashboard">
        <Localized id="error-occurred">
          <p>An error occurred</p>
        </Localized>
      </div>
    );
  }

  const todayRevenue = revenue.reduce((s, r) => s + r.total_minor, 0);
  const todayOrders = revenue.reduce((s, r) => s + r.sale_count, 0);
  const todayCurrency = revenue.length > 0 ? revenue[0]!.currency : 'USD';
  const maxWeekly = weeklyRevenue.length > 0
    ? Math.max(...weeklyRevenue.map((r) => r.total_minor), 1)
    : 1;

  return (
    <div className="dashboard" role="region" aria-label="Dashboard">
      <Localized id="dashboard-title">
        <h1 className="dashboard-title">Dashboard</h1>
      </Localized>

      <div className="dashboard-kpi-row">
        <Card shadow="sm" className="dashboard-kpi">
          <Localized id="dashboard-today-revenue">
            <span className="dashboard-kpi-label">Today&apos;s Revenue</span>
          </Localized>
          <span className="dashboard-kpi-value">
            {fmtCurrency(todayRevenue, todayCurrency)}
          </span>
        </Card>
        <Card shadow="sm" className="dashboard-kpi">
          <Localized id="dashboard-orders-today">
            <span className="dashboard-kpi-label">Orders Today</span>
          </Localized>
          <span className="dashboard-kpi-value">{todayOrders}</span>
        </Card>
        <Card shadow="sm" className="dashboard-kpi">
          <Localized id="dashboard-top-product">
            <span className="dashboard-kpi-label">Top Product</span>
          </Localized>
          <span className="dashboard-kpi-value">{topProduct?.name ?? '-'}</span>
        </Card>
      </div>

      <Card shadow="sm" className="dashboard-section">
        <Localized id="sales-report-title">
          <h2 className="dashboard-section-title">Revenue This Week</h2>
        </Localized>
        <div className="dashboard-weekly-chart">
          {weeklyRevenue.map((row) => (
            <div key={row.date} className="dashboard-weekly-bar-row">
              <span className="dashboard-weekly-bar-label">
                {row.date.slice(5)}
              </span>
              <div className="dashboard-weekly-bar-track">
                <div
                  className="dashboard-weekly-bar"
                  style={{
                    width: `${Math.max(5, (row.total_minor / maxWeekly) * 100)}%`,
                  }}
                  role="progressbar"
                  aria-valuenow={row.total_minor}
                  aria-valuemax={maxWeekly}
                />
              </div>
              <span className="dashboard-weekly-bar-value">
                {fmtCurrency(row.total_minor, row.currency)}
              </span>
            </div>
          ))}
        </div>
      </Card>

      <Card shadow="sm" className="dashboard-section">
        <Localized id="dashboard-low-stock-alerts">
          <h2 className="dashboard-section-title">Low Stock Alerts</h2>
        </Localized>
        {lowStock.length === 0 ? (
          <p className="dashboard-no-data">
            <Localized id="dashboard-no-data">
              <span>No sales data yet today</span>
            </Localized>
          </p>
        ) : (
          <ul
            className="dashboard-low-stock-list"
            aria-label="Low stock alerts"
          >
            {lowStock.map((item) => (
              <li key={item.product_id} className="dashboard-low-stock-item">
                <span className="dashboard-low-stock-name">{item.name}</span>
                <span className="dashboard-low-stock-qty">
                  {item.current_qty} left
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
