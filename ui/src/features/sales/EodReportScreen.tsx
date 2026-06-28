import { useState, useCallback, useEffect } from 'react';
import {
  exportEodReport,
  type EodReport,
} from '@/api/pos';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import './EodReportScreen.css';

/**
 * EOD (End-of-Day) Report screen.
 *
 * Displays a comprehensive summary of today's sales activity including
 * revenue KPIs, payment method breakdown, void/discount statistics,
 * and an hourly sales chart.
 */
export default function EodReportScreen() {
  const [report, setReport] = useState<EodReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await exportEodReport();
      setReport(data);
      setLastRefresh(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load EOD report');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const currency = report?.currency ?? 'USD';
  const money = (minor: number) => formatMoney({ minor_units: minor, currency });

  // Find the peak hour for the hourly bar chart.
  const peakHourSales = report
    ? Math.max(...report.hourly_breakdown.map((h) => h.total_minor), 1)
    : 1;

  return (
    <div className="eod-report">
      <div className="eod-report-header">
        <h1 className="eod-report-title">End-of-Day Report</h1>
        <div className="eod-report-header-right">
          <span className="eod-report-date">
            {lastRefresh.toLocaleDateString('en-US', {
              weekday: 'long', year: 'numeric', month: 'long', day: 'numeric',
            })}
          </span>
          <button
            type="button"
            className="eod-report-refresh-btn"
            onClick={load}
            disabled={loading}
            aria-label="Refresh report"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
              <polyline points="23 4 23 10 17 10" />
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
            </svg>
            Refresh
          </button>
        </div>
      </div>

      {loading && !report ? (
        <div className="eod-report-loading">
          <div className="eod-report-spinner" />
          <span>Loading report…</span>
        </div>
      ) : error ? (
        <Card shadow="sm">
          <div className="eod-report-error">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="24" height="24" aria-hidden="true">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <p>{error}</p>
            <button type="button" className="eod-report-retry-btn" onClick={load}>Retry</button>
          </div>
        </Card>
      ) : !report ? (
        <Card shadow="sm">
          <div className="eod-report-empty">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="32" height="32" aria-hidden="true">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="12" y1="18" x2="12" y2="12" />
              <line x1="9" y1="15" x2="15" y2="15" />
            </svg>
            <p>No sales data available for today.</p>
            <p className="eod-report-empty-sub">Sales will appear here once transactions are completed.</p>
          </div>
        </Card>
      ) : (
        <>
          {/* ── KPI Cards ─────────────────────────────── */}
          <div className="eod-report-kpi-row">
            <Card shadow="sm" className="eod-report-kpi-card">
              <div className="eod-report-kpi">
                <span className="eod-report-kpi-label">Total Revenue</span>
                <span className="eod-report-kpi-value eod-report-kpi-value--primary">
                  {money(report.total_revenue)}
                </span>
                <span className="eod-report-kpi-sub">{report.total_sales} completed sales</span>
              </div>
            </Card>
            <Card shadow="sm" className="eod-report-kpi-card">
              <div className="eod-report-kpi">
                <span className="eod-report-kpi-label">Average Sale</span>
                <span className="eod-report-kpi-value">
                  {report.total_sales > 0
                    ? money(Math.round(report.total_revenue / report.total_sales))
                    : money(0)}
                </span>
                <span className="eod-report-kpi-sub">per transaction</span>
              </div>
            </Card>
            <Card shadow="sm" className="eod-report-kpi-card">
              <div className="eod-report-kpi">
                <span className="eod-report-kpi-label">Voids</span>
                <span className="eod-report-kpi-value eod-report-kpi-value--danger">
                  {report.void_count}
                </span>
                <span className="eod-report-kpi-sub">{money(report.void_total)} voided</span>
              </div>
            </Card>
            <Card shadow="sm" className="eod-report-kpi-card">
              <div className="eod-report-kpi">
                <span className="eod-report-kpi-label">Discounts Applied</span>
                <span className="eod-report-kpi-value eod-report-kpi-value--warning">
                  {report.discount_count}
                </span>
                <span className="eod-report-kpi-sub">
                  {report.discount_count > 0 ? `${report.discount_count} sales with discount` : 'No discounts applied'}
                </span>
              </div>
            </Card>
          </div>

          {/* ── Two-column layout ─────────────────────── */}
          <div className="eod-report-columns">
            {/* Left: Payment Breakdown */}
            <Card shadow="sm" className="eod-report-section-card">
              <h2 className="eod-report-section-title">Payment Breakdown</h2>
              {report.payment_breakdown.length === 0 ? (
                <p className="eod-report-no-data">No payment data</p>
              ) : (
                <div className="eod-report-payment-list">
                  {report.payment_breakdown.map((pmt) => {
                    const pct = report.total_revenue > 0
                      ? Math.round((pmt.total / report.total_revenue) * 100)
                      : 0;
                    return (
                      <div key={pmt.method} className="eod-report-payment-row">
                        <div className="eod-report-payment-info">
                          <span className="eod-report-payment-method">
                            {pmt.method.charAt(0).toUpperCase() + pmt.method.slice(1)}
                          </span>
                          <span className="eod-report-payment-count">{pmt.count} transactions</span>
                        </div>
                        <div className="eod-report-payment-bar-wrap">
                          <div
                            className="eod-report-payment-bar"
                            style={{ width: `${pct}%` }}
                            role="progressbar"
                            aria-valuenow={pct}
                            aria-valuemin={0}
                            aria-valuemax={100}
                            aria-label={`${pmt.method}: ${pct}% of revenue`}
                          />
                        </div>
                        <div className="eod-report-payment-amount">
                          <span className="eod-report-payment-total">{money(pmt.total)}</span>
                          <span className="eod-report-payment-pct">{pct}%</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>

            {/* Right: Hourly Sales (bar chart) */}
            <Card shadow="sm" className="eod-report-section-card">
              <h2 className="eod-report-section-title">Sales by Hour</h2>
              {report.hourly_breakdown.length === 0 ? (
                <p className="eod-report-no-data">No hourly data</p>
              ) : (
                <div className="eod-report-hourly-chart" aria-label="Hourly sales bar chart">
                  {Array.from({ length: 24 }, (_, hour) => {
                    const h = report.hourly_breakdown.find((r) => r.hour === hour);
                    const barPct = h ? Math.round((h.total_minor / peakHourSales) * 100) : 0;
                    return (
                      <div key={hour} className="eod-report-hour-bar-row" aria-label={`${String(hour).padStart(2, '0')}:00 — ${h ? `${h.sale_count} sales, ${money(h.total_minor)}` : 'No sales'}`}>
                        <span className="eod-report-hour-label">{String(hour).padStart(2, '0')}</span>
                        <div className="eod-report-hour-bar-track">
                          <div
                            className={`eod-report-hour-bar ${barPct > 0 ? 'eod-report-hour-bar--active' : ''}`}
                            style={{ width: `${Math.max(barPct, h ? 4 : 0)}%` }}
                          />
                        </div>
                        <span className="eod-report-hour-value">
                          {h ? money(h.total_minor) : ''}
                        </span>
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>
          </div>

          {/* ── Summary table ─────────────────────────── */}
          {report.total_sales > 0 && (
            <Card shadow="sm" className="eod-report-section-card">
              <h2 className="eod-report-section-title">Today&apos;s Summary</h2>
              <div className="eod-report-summary-grid">
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Completed Sales</span>
                  <span className="eod-report-summary-value">{report.total_sales}</span>
                </div>
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Total Revenue</span>
                  <span className="eod-report-summary-value">{money(report.total_revenue)}</span>
                </div>
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Voided Sales</span>
                  <span className="eod-report-summary-value eod-report-summary-value--danger">{report.void_count}</span>
                </div>
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Voided Value</span>
                  <span className="eod-report-summary-value eod-report-summary-value--danger">{money(report.void_total)}</span>
                </div>
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Sales with Discounts</span>
                  <span className="eod-report-summary-value">{report.discount_count}</span>
                </div>
                <div className="eod-report-summary-item">
                  <span className="eod-report-summary-label">Payment Methods Used</span>
                  <span className="eod-report-summary-value">{report.payment_breakdown.length}</span>
                </div>
              </div>
            </Card>
          )}
        </>
      )}
    </div>
  );
}
