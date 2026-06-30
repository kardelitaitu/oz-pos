import { useState, useCallback, useEffect, useRef } from 'react';
import {
  exportEodReport,
  type EodReport,
} from '@/api/sales';
import { listShifts, type ShiftDto } from '@/api/shifts';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import { printReceipt } from '@/api/hardware';
import './EodReportScreen.css';

/**
 * EOD (End-of-Day) Report screen.
 *
 * Displays a comprehensive summary of today's sales activity including
 * revenue KPIs, payment method breakdown, void/discount statistics,
 * and an hourly sales chart.
 */
// ── Shift Summary Sub-component ──────────────────────────────────

interface ShiftSummaryProps {
  shifts: ShiftDto[];
  currency: string;
}

function ShiftSummarySection({ shifts, currency }: ShiftSummaryProps) {
  const today = new Date().toISOString().slice(0, 10);
  const todayClosed = shifts.filter(
    (s) => s.status === 'closed' && s.closedAt && s.closedAt.startsWith(today),
  );
  const activeShift = shifts.find((s) => s.status === 'open');

  if (todayClosed.length === 0 && !activeShift) {
    return null;
  }

  const fmt = (minor: number) => formatMoney({ minor_units: minor, currency });
  const totalOpening = todayClosed.reduce((acc, s) => acc + s.openingBalanceMinor, 0);
  const totalClosing = todayClosed.reduce((acc, s) => acc + (s.closingBalanceMinor ?? 0), 0);
  const totalExpected = todayClosed.reduce((acc, s) => acc + (s.expectedCashMinor ?? 0), 0);
  const totalCashDiff = todayClosed.reduce((acc, s) => acc + (s.cashDifferenceMinor ?? 0), 0);

  return (
    <Card shadow="sm" className="eod-report-section-card">
      {/* ── Section header ──────────────────────── */}
      <div className="eod-report-shift-header">
        <h2 className="eod-report-section-title" style={{ margin: 0 }}>Cashier Shifts</h2>
        {activeShift && (
          <span className="eod-report-shift-active-badge">
            <span className="eod-report-shift-active-dot" />
            Shift in progress
          </span>
        )}
      </div>

      {/* ── Active shift card ────────────────────── */}
      {activeShift && (
        <div className="eod-report-active-shift">
          <div className="eod-report-active-shift-row">
            <span className="eod-report-active-shift-label">Active shift since</span>
            <span className="eod-report-active-shift-value">
              {new Date(activeShift.openedAt).toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              })}
            </span>
          </div>
          <div className="eod-report-active-shift-row">
            <span className="eod-report-active-shift-label">Opening balance</span>
            <span className="eod-report-active-shift-value">
              {fmt(activeShift.openingBalanceMinor)}
            </span>
          </div>
          <div className="eod-report-active-shift-row">
            <span className="eod-report-active-shift-label">Sales this shift</span>
            <span className="eod-report-active-shift-value">
              {fmt(activeShift.totalSalesMinor)}
            </span>
          </div>
        </div>
      )}

      {/* ── Closed shifts list ──────────────────── */}
      {todayClosed.length > 0 && (
        <>
          <div className="eod-report-shift-list-header">
            <span>Closed Shifts Today</span>
            <span className="eod-report-shift-count">{todayClosed.length}</span>
          </div>

          <div className="eod-report-shift-table">
            <div className="eod-report-shift-table-header">
              <span>Opened</span>
              <span>Closed</span>
              <span>Opening</span>
              <span>Counted</span>
              <span>Expected</span>
              <span>Diff</span>
            </div>
            {todayClosed.map((s) => {
              const diff = s.cashDifferenceMinor;
              const diffClass =
                diff !== null && diff < 0
                  ? 'eod-report-shift-diff--negative'
                  : diff !== null && diff > 0
                    ? 'eod-report-shift-diff--positive'
                    : '';
              return (
                <div key={s.id} className="eod-report-shift-row">
                  <span className="eod-report-shift-cell">
                    {new Date(s.openedAt).toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </span>
                  <span className="eod-report-shift-cell">
                    {s.closedAt
                      ? new Date(s.closedAt).toLocaleTimeString([], {
                          hour: '2-digit',
                          minute: '2-digit',
                        })
                      : '—'}
                  </span>
                  <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                    {fmt(s.openingBalanceMinor)}
                  </span>
                  <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                    {s.closingBalanceMinor !== null
                      ? fmt(s.closingBalanceMinor)
                      : '—'}
                  </span>
                  <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                    {s.expectedCashMinor !== null
                      ? fmt(s.expectedCashMinor)
                      : '—'}
                  </span>
                  <span className={`eod-report-shift-cell eod-report-shift-cell--mono ${diffClass}`}>
                    {diff !== null ? fmt(diff) : '—'}
                    {diff !== null && diff !== 0 && (
                      <span className="eod-report-shift-tag">
                        {diff > 0 ? 'Over' : 'Short'}
                      </span>
                    )}
                  </span>
                </div>
              );
            })}

            {/* ── Totals row ──────────────────────── */}
            <div className="eod-report-shift-row eod-report-shift-row--total">
              <span className="eod-report-shift-cell" />
              <span className="eod-report-shift-cell">Total</span>
              <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                {fmt(totalOpening)}
              </span>
              <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                {fmt(totalClosing)}
              </span>
              <span className="eod-report-shift-cell eod-report-shift-cell--mono">
                {fmt(totalExpected)}
              </span>
              <span className={`eod-report-shift-cell eod-report-shift-cell--mono ${
                totalCashDiff < 0
                  ? 'eod-report-shift-diff--negative'
                  : totalCashDiff > 0
                    ? 'eod-report-shift-diff--positive'
                    : ''
              }`}>
                {fmt(totalCashDiff)}
                {totalCashDiff !== 0 && (
                  <span className="eod-report-shift-tag">
                    {totalCashDiff > 0 ? 'Over' : 'Short'}
                  </span>
                )}
              </span>
            </div>
          </div>
        </>
      )}

      {/* ── Combined cash summary ───────────────── */}
      {todayClosed.length > 1 && (
        <div className="eod-report-shift-cash-summary">
          <span className="eod-report-shift-cash-summary-label">Cash Reconciliation</span>
          <div className="eod-report-shift-cash-grid">
            <div className="eod-report-shift-cash-item">
              <span className="eod-report-shift-cash-label">Total opening</span>
              <span className="eod-report-shift-cash-value">{fmt(totalOpening)}</span>
            </div>
            <div className="eod-report-shift-cash-item">
              <span className="eod-report-shift-cash-label">Total counted</span>
              <span className="eod-report-shift-cash-value">{fmt(totalClosing)}</span>
            </div>
            <div className="eod-report-shift-cash-item">
              <span className="eod-report-shift-cash-label">Total expected</span>
              <span className="eod-report-shift-cash-value">{fmt(totalExpected)}</span>
            </div>
            <div className="eod-report-shift-cash-item">
              <span className="eod-report-shift-cash-label">Net difference</span>
              <span
                className={`eod-report-shift-cash-value ${
                  totalCashDiff < 0
                    ? 'eod-report-shift-diff--negative'
                    : totalCashDiff > 0
                      ? 'eod-report-shift-diff--positive'
                      : ''
                }`}
              >
                {fmt(totalCashDiff)}
              </span>
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}

export default function EodReportScreen() {
  const [report, setReport] = useState<EodReport | null>(null);
  const [shifts, setShifts] = useState<ShiftDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [printing, setPrinting] = useState(false);
  const reportRef = useRef(report);
  const shiftsRef = useRef(shifts);

  // Keep refs in sync for the print callback to access latest values.
  reportRef.current = report;
  shiftsRef.current = shifts;

  const handlePrint = useCallback(async () => {
    const r = reportRef.current;
    if (!r) return;
    setPrinting(true);
    try {
      const cur = r.currency ?? 'USD';
      const line = (text = '') => `${text}\n`;
      const sep = line('─'.repeat(38));
      const money = (minor: number) => formatMoney({ minor_units: minor, currency: cur });

      // Build a hard-wrapped receipt body (~38 chars wide for 58mm paper).
      let body = '';

      // Header.
      body += line('    END-OF-DAY REPORT');
      body += line(`    ${lastRefresh.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}`);
      body += sep;

      // Revenue KPIs.
      body += line(`Total Revenue        ${money(r.total_revenue).padStart(14)}`);
      body += line(`Completed Sales      ${String(r.total_sales).padStart(14)}`);
      if (r.total_sales > 0) {
        body += line(`Average Sale         ${money(Math.round(r.total_revenue / r.total_sales)).padStart(14)}`);
      }
      body += line(`Voids                ${String(r.void_count).padStart(8)}  ${money(r.void_total).padStart(10)}`);
      body += line(`Discounts Applied    ${String(r.discount_count).padStart(14)}`);
      body += sep;

      // Payment breakdown.
      body += line('  PAYMENT BREAKDOWN');
      body += line('');
      for (const pmt of r.payment_breakdown) {
        const pct = r.total_revenue > 0 ? Math.round((pmt.total / r.total_revenue) * 100) : 0;
        const label = pmt.method.charAt(0).toUpperCase() + pmt.method.slice(1);
        body += line(`${label.padEnd(12)} ${String(pmt.count).padStart(3)} tx  ${money(pmt.total).padStart(12)}  ${String(pct).padStart(2)}%`);
      }
      body += sep;

      // Shift summary.
      const activeShift = shiftsRef.current.find((s) => s.status === 'open');
      if (activeShift) {
        body += line('  ACTIVE SHIFT IN PROGRESS');
        body += line(`  Since: ${new Date(activeShift.openedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`);
        body += line(`  Opening: ${money(activeShift.openingBalanceMinor)}`);
        body += line(`  Sales: ${money(activeShift.totalSalesMinor)}`);
        body += sep;
      }

      const todayClosed = shiftsRef.current.filter(
        (s) => s.status === 'closed' && s.closedAt && s.closedAt.startsWith(new Date().toISOString().slice(0, 10)),
      );
      if (todayClosed.length > 0) {
        body += line('  CLOSED SHIFTS');
        for (const s of todayClosed) {
          const diff = s.cashDifferenceMinor;
          const diffStr = diff !== null ? `${diff > 0 ? '+' : ''}${money(diff)}` : '—';
          body += line(`  ${new Date(s.openedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}  ${money(s.openingBalanceMinor).padStart(10)}  ${diffStr.padStart(12)}`);
        }
        body += sep;
      }

      // Hourly breakdown (condensed).
      if (r.hourly_breakdown.length > 0) {
        body += line('  HOURLY BREAKDOWN');
        body += line('');
        const peak = Math.max(...r.hourly_breakdown.map((h) => h.total_minor), 1);
        for (let hour = 0; hour < 24; hour++) {
          const h = r.hourly_breakdown.find((r) => r.hour === hour);
          if (h) {
            const barLen = Math.round((h.total_minor / peak) * 12);
            const bar = '█'.repeat(Math.max(barLen, 1));
            body += line(`${String(hour).padStart(2)}:00 ${bar.padEnd(14)} ${money(h.total_minor).padStart(10)}`);
          }
        }
        body += sep;
      }

      // Footer.
      body += line('           *** END ***');
      body += line('');

      await printReceipt({ body });
    } catch {
      // Printing error — silently handled.
    } finally {
      setPrinting(false);
    }
  }, [lastRefresh]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [data, shiftData] = await Promise.all([
        exportEodReport(),
        listShifts(),
      ]);
      setReport(data);
      setShifts(shiftData);
      setLastRefresh(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load report');
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
          <button
            type="button"
            className="eod-report-refresh-btn eod-report-print-btn"
            onClick={handlePrint}
            disabled={printing || !report}
            aria-label="Print EOD report"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
              <polyline points="6 9 6 2 18 2 18 9" />
              <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
              <rect x="6" y="14" width="12" height="8" />
            </svg>
            {printing ? 'Printing…' : 'Print'}
          </button>
        </div>
      </div>

      {/* ── Shift Summary Section ────────────────── */}
      {!loading && !error && report && (
        <ShiftSummarySection shifts={shifts} currency={currency} />
      )}

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
