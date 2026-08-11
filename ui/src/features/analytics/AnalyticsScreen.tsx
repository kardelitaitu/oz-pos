//! Staff Analytics Screen (analytics:view — owner/admin/manager).
//!
//! Per-staff shift + completed-sales summary for the session's store over
//! a selectable date range, plus a per-day series when a staff member is
//! chosen. Backed by `get_staff_analytics_scoped` /
//! `get_staff_analytics_daily_scoped`; the backend enforces the
//! `analytics:view` permission and the 0048 assignment scope (an
//! out-of-scope session is denied fail-closed).

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useCurrency } from '@/contexts/CurrencyContext';
import { requiredLocalized } from '@/frontend/shared';
import { l10nErrorMessage } from '@/utils/app-error';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import { formatMoney } from '@/types/domain';
import {
  getStaffAnalyticsScoped,
  getStaffAnalyticsDailyScoped,
  type StaffAnalyticsRow,
  type StaffAnalyticsDailyRow,
} from '@/api/analytics';
import './AnalyticsScreen.css';

// ── Date helpers ───────────────────────────────────────────────────

function isoDay(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function today(): string {
  return isoDay(new Date());
}

function daysAgo(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() - n);
  return isoDay(d);
}

// ── Component ───────────────────────────────────────────────────────

/** Staff analytics screen — per-staff shifts and sales over a date range. */
export default function AnalyticsScreen() {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const { currency } = useCurrency();

  const [fromDraft, setFromDraft] = useState(daysAgo(29));
  const [toDraft, setToDraft] = useState(today());
  const [from, setFrom] = useState(daysAgo(29));
  const [to, setTo] = useState(today());
  const [rows, setRows] = useState<StaffAnalyticsRow[]>([]);
  const [selectedUserId, setSelectedUserId] = useState('');
  const [daily, setDaily] = useState<StaffAnalyticsDailyRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [dailyLoading, setDailyLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── Summary load ─────────────────────────────────────────────────

  const loadSummary = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await getStaffAnalyticsScoped(sessionToken, from, to);
      setRows(result);
      // Reset the selection if the picked member is no longer in range.
      setSelectedUserId((current) =>
        current && result.some((r) => r.user_id === current) ? current : '',
      );
    } catch (e) {
      setError(l10nErrorMessage(e, l10n, 'analytics-error'));
    } finally {
      setLoading(false);
    }
  }, [sessionToken, from, to]);

  useEffect(() => { loadSummary(); }, [loadSummary]);

  // ── Daily series load ────────────────────────────────────────────

  useEffect(() => {
    if (!selectedUserId) {
      setDaily([]);
      return;
    }
    let cancelled = false;
    setDailyLoading(true);
    getStaffAnalyticsDailyScoped(sessionToken, selectedUserId, from, to)
      .then((result) => {
        if (!cancelled) setDaily(result);
      })
      .catch(() => {
        if (!cancelled) setDaily([]);
      })
      .finally(() => {
        if (!cancelled) setDailyLoading(false);
      });
    return () => { cancelled = true; };
  }, [sessionToken, selectedUserId, from, to]);

  const selectedRow = useMemo(
    () => rows.find((r) => r.user_id === selectedUserId) ?? null,
    [rows, selectedUserId],
  );

  const applyFilters = () => {
    setFrom(fromDraft);
    setTo(toDraft);
  };

  // ── Render ───────────────────────────────────────────────────────

  return (
    <div className="analytics" role="region" aria-label={requiredLocalized(l10n, 'analytics-region-aria')}>
      <div className="analytics-header">
        <Localized id="analytics-title">
          <h1 className="analytics-title">Staff Analytics</h1>
        </Localized>
        <Localized id="analytics-subtitle">
          <p className="analytics-subtitle">Per-staff shifts and sales over time</p>
        </Localized>
      </div>

      {/* ── Filters ─────────────────────────────────────── */}
      <Card shadow="sm" className="analytics-filters">
        <div className="analytics-filter-field">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="analytics-from" className="analytics-filter-label">
            <Localized id="analytics-filter-from"><span>From</span></Localized>
          </label>
          <input
            id="analytics-from"
            type="date"
            className="analytics-filter-input"
            value={fromDraft}
            max={toDraft}
            onChange={(e) => setFromDraft(e.target.value)}
            aria-label={l10n.getString('analytics-filter-from')}
          />
        </div>
        <div className="analytics-filter-field">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="analytics-to" className="analytics-filter-label">
            <Localized id="analytics-filter-to"><span>To</span></Localized>
          </label>
          <input
            id="analytics-to"
            type="date"
            className="analytics-filter-input"
            value={toDraft}
            min={fromDraft}
            onChange={(e) => setToDraft(e.target.value)}
            aria-label={l10n.getString('analytics-filter-to')}
          />
        </div>
        <div className="analytics-filter-field">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="analytics-staff" className="analytics-filter-label">
            <Localized id="analytics-filter-staff"><span>Staff Member</span></Localized>
          </label>
          <select
            id="analytics-staff"
            className="analytics-filter-select"
            value={selectedUserId}
            onChange={(e) => setSelectedUserId(e.target.value)}
            aria-label={l10n.getString('analytics-filter-staff')}
          >
            <option value="">
              {l10n.getString('analytics-filter-all-staff')}
            </option>
            {rows.map((r) => (
              <option key={r.user_id} value={r.user_id}>{r.display_name}</option>
            ))}
          </select>
        </div>
        <Localized id="analytics-btn-apply">
          <button
            type="button"
            className="analytics-apply-btn"
            onClick={applyFilters}
            aria-label={l10n.getString('analytics-btn-apply')}
          >
            Apply
          </button>
        </Localized>
      </Card>

      {error && (
        <div className="analytics-error" role="alert">{error}</div>
      )}

      {/* ── Staff summary ───────────────────────────────── */}
      <Card shadow="sm" className="analytics-card">
        <Localized id="analytics-summary-title">
          <h2 className="analytics-card-title">Staff Summary</h2>
        </Localized>
        {loading ? (
          <div className="analytics-loading">
            <Spinner aria-label={l10n.getString('analytics-loading')} />
          </div>
        ) : rows.length === 0 ? (
          <Localized id="analytics-empty">
            <p className="analytics-empty">No staff activity in this period.</p>
          </Localized>
        ) : (
          <div className="analytics-table-wrap">
            <table
              className="analytics-table"
              aria-label={l10n.getString('analytics-summary-title')}
            >
              <thead>
                <tr>
                  <Localized id="analytics-table-staff"><th>Staff Member</th></Localized>
                  <Localized id="analytics-table-shifts"><th>Shifts</th></Localized>
                  <Localized id="analytics-table-closed"><th>Closed</th></Localized>
                  <Localized id="analytics-table-shift-sales"><th>Shift Sales</th></Localized>
                  <Localized id="analytics-table-sales"><th>Sales</th></Localized>
                  <Localized id="analytics-table-sales-total"><th>Sales Total</th></Localized>
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => (
                  <tr
                    key={r.user_id}
                    className={r.user_id === selectedUserId ? 'analytics-row--selected' : 'analytics-row'}
                    onClick={() => setSelectedUserId(r.user_id)}
                  >
                    <td className="analytics-cell-name">{r.display_name}</td>
                    <td className="analytics-cell-num">{r.shift_count}</td>
                    <td className="analytics-cell-num">{r.closed_shift_count}</td>
                    <td className="analytics-cell-mono">{formatMoney({ minor_units: r.shift_sales_minor, currency })}</td>
                    <td className="analytics-cell-num">{r.sale_count}</td>
                    <td className="analytics-cell-mono">{formatMoney({ minor_units: r.sale_total_minor, currency })}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      {/* ── Daily activity ──────────────────────────────── */}
      <Card shadow="sm" className="analytics-card">
        <Localized id="analytics-daily-title">
          <h2 className="analytics-card-title">Daily Activity</h2>
        </Localized>
        {!selectedRow ? (
          <Localized id="analytics-daily-empty">
            <p className="analytics-empty">Select a staff member to see daily activity.</p>
          </Localized>
        ) : dailyLoading ? (
          <div className="analytics-loading">
            <Spinner aria-label={l10n.getString('analytics-loading')} />
          </div>
        ) : daily.length === 0 ? (
          <Localized id="analytics-empty">
            <p className="analytics-empty">No staff activity in this period.</p>
          </Localized>
        ) : (
          <div className="analytics-table-wrap">
            <table
              className="analytics-table"
              aria-label={l10n.getString('analytics-daily-title')}
            >
              <thead>
                <tr>
                  <Localized id="analytics-daily-day"><th>Day</th></Localized>
                  <Localized id="analytics-daily-shifts"><th>Shifts</th></Localized>
                  <Localized id="analytics-daily-shift-sales"><th>Shift Sales</th></Localized>
                  <Localized id="analytics-daily-sales"><th>Sales</th></Localized>
                  <Localized id="analytics-daily-sales-total"><th>Sales Total</th></Localized>
                </tr>
              </thead>
              <tbody>
                {daily.map((d) => (
                  <tr key={d.day}>
                    <td className="analytics-cell-date">{d.day}</td>
                    <td className="analytics-cell-num">{d.shift_count}</td>
                    <td className="analytics-cell-mono">{formatMoney({ minor_units: d.shift_sales_minor, currency })}</td>
                    <td className="analytics-cell-num">{d.sale_count}</td>
                    <td className="analytics-cell-mono">{formatMoney({ minor_units: d.sale_total_minor, currency })}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}
