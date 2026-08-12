//! Staff Analytics Screen — KPIs, multi-staff stacked bar, drill-down.
//!
//! Loads daily data for all staff in parallel to populate the stacked bar
//! chart. Click a table row to see individual deep-dive combo chart.

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useCurrency } from '@/contexts/CurrencyContext';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { requiredLocalized } from '@/frontend/shared';
import { l10nErrorMessage } from '@/utils/app-error';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import { formatMoney } from '@/types/domain';
import { downloadCsv } from '@/utils/export-csv';
import ReactEChartsCore from 'echarts-for-react/lib/core';
import * as echarts from 'echarts/core';
import { BarChart, LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import {
  getStaffAnalyticsScoped, getStaffAnalyticsDailyScoped,
  type StaffAnalyticsRow, type StaffAnalyticsDailyRow,
} from '@/api/analytics';
import './AnalyticsScreen.css';

echarts.use([BarChart, LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

function isoDay(d: Date): string { return d.toISOString().slice(0, 10); }
function today(): string { return isoDay(new Date()); }
function daysAgo(n: number): string { const d = new Date(); d.setDate(d.getDate() - n); return isoDay(d); }

interface KpiData {
  totalShifts: number; closedShifts: number;
  totalSales: number; totalSalesMinor: number; staffCount: number;
}

// ── Component ───────────────────────────────────────────────────────

export default function AnalyticsScreen() {
  const { l10n } = useLocalization();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const { currency } = useCurrency();

  const [fromDraft, setFromDraft] = useState(daysAgo(29));
  const [toDraft, setToDraft] = useState(today());
  const [from, setFrom] = useState(daysAgo(29));
  const [to, setTo] = useState(today());
  const [rows, setRows] = useState<StaffAnalyticsRow[]>([]);
  const [selectedUserId, setSelectedUserId] = useState('');
  // All staff daily data: userId → daily rows
  const [allDailyMap, setAllDailyMap] = useState<Map<string, StaffAnalyticsDailyRow[]>>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await getStaffAnalyticsScoped(sessionToken, from, to);
      setRows(result);
      setSelectedUserId((current) =>
        current && result.some((r) => r.user_id === current) ? current : '',
      );

      // Load daily data for all staff in parallel
      const dailyResults = await Promise.all(
        result.map((r) =>
          getStaffAnalyticsDailyScoped(sessionToken, r.user_id, from, to)
            .then((d) => ({ userId: r.user_id, data: d }))
            .catch(() => ({ userId: r.user_id, data: [] as StaffAnalyticsDailyRow[] })),
        ),
      );
      const map = new Map<string, StaffAnalyticsDailyRow[]>();
      for (const { userId, data } of dailyResults) map.set(userId, data);
      setAllDailyMap(map);
    } catch (e) {
      setError(l10nErrorMessage(e, l10n, 'analytics-error'));
    } finally {
      setLoading(false);
    }
  }, [sessionToken, from, to]);

  useEffect(() => { loadData(); }, [loadData]);

  const selectedRow = useMemo(
    () => rows.find((r) => r.user_id === selectedUserId) ?? null,
    [rows, selectedUserId],
  );

  const daily = useMemo(
    () => allDailyMap.get(selectedUserId) ?? [],
    [allDailyMap, selectedUserId],
  );

  // ── KPIs ──────────────────────────────────────────────────────────

  const kpis = useMemo<KpiData>(() => {
    const totalShifts = rows.reduce((s, r) => s + r.shift_count, 0);
    const closedShifts = rows.reduce((s, r) => s + r.closed_shift_count, 0);
    const totalSales = rows.reduce((s, r) => s + r.sale_count, 0);
    const totalSalesMinor = rows.reduce((s, r) => s + r.sale_total_minor, 0);
    return { totalShifts, closedShifts, totalSales, totalSalesMinor, staffCount: rows.length };
  }, [rows]);

  // ── ECharts: stacked bar — daily sales by staff ──────────────────

  const stackedBarOption = useMemo(() => {
    if (rows.length === 0) return null;
    const staffNames = rows.map((r) => r.display_name);
    const colors = ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4'];

    // Collect all unique dates across all staff
    const allDates = new Set<string>();
    for (const [, data] of allDailyMap) {
      for (const d of data) allDates.add(d.day);
    }
    const dates = [...allDates].sort();

    if (dates.length > 0) {
      const series = staffNames.map((name, i) => {
        const userId = rows[i]?.user_id ?? '';
        const staffDaily = allDailyMap.get(userId) ?? [];
        const dateMap = new Map(staffDaily.map((d) => [d.day, d.sale_total_minor]));
        return {
          name, type: 'bar' as const, stack: 'total',
          emphasis: { focus: 'series' as const },
          itemStyle: { color: colors[i % colors.length] },
          data: dates.map((date) => dateMap.get(date) ?? 0),
        };
      });

      return {
        tooltip: { trigger: 'axis' as const, axisPointer: { type: 'shadow' as const },
          valueFormatter: (val: unknown) => formatMoney({ minor_units: Number(val), currency }) },
        legend: { top: 0, type: 'scroll' as const },
        grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
        xAxis: { type: 'category' as const, data: dates, axisLabel: { rotate: 45 } },
        yAxis: { type: 'value' as const, axisLabel: { formatter: (v: number) => `${(v / 1_000_000).toFixed(1)}M` } },
        series,
      };
    }

    // Fallback: grouped bar of per-staff totals
    return {
      tooltip: { trigger: 'axis' as const, valueFormatter: (val: unknown) => formatMoney({ minor_units: Number(val), currency }) },
      legend: { top: 0, type: 'scroll' as const },
      grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
      xAxis: { type: 'category' as const, data: staffNames },
      yAxis: { type: 'value' as const, axisLabel: { formatter: (v: number) => `${(v / 1_000_000).toFixed(1)}M` } },
      series: [{
        name: l10n.getString('analytics-kpi-total-sales'), type: 'bar' as const,
        data: rows.map((r) => r.sale_total_minor),
        itemStyle: { color: colors[0] }, emphasis: { focus: 'series' as const },
      }],
    };
  }, [rows, allDailyMap, currency, l10n]);

  // ── Deep-dive combo chart ────────────────────────────────────────

  const deepDiveOption = useMemo(() => {
    if (daily.length === 0 || !selectedRow) return null;
    const dates = daily.map((d) => d.day);
    return {
      tooltip: { trigger: 'axis' as const },
      legend: { data: [l10n.getString('analytics-chart-sales'), l10n.getString('analytics-chart-shifts')] },
      grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
      xAxis: { type: 'category' as const, data: dates, axisLabel: { rotate: 45 } },
      yAxis: [
        { type: 'value' as const, name: l10n.getString('analytics-chart-sales'), axisLabel: { formatter: (v: number) => `${(v / 1_000_000).toFixed(1)}M` } },
        { type: 'value' as const, name: l10n.getString('analytics-chart-shifts') },
      ],
      series: [
        { name: l10n.getString('analytics-chart-sales'), type: 'bar' as const, data: daily.map((d) => d.sale_total_minor), itemStyle: { color: '#5470c6' } },
        { name: l10n.getString('analytics-chart-shifts'), type: 'line' as const, yAxisIndex: 1, data: daily.map((d) => d.shift_count), itemStyle: { color: '#ee6666' }, symbol: 'circle', symbolSize: 6 },
      ],
    };
  }, [daily, selectedRow, l10n]);

  const applyFilters = () => { setFrom(fromDraft); setTo(toDraft); };

  // ── Render ───────────────────────────────────────────────────────

  return (
    <div className="analytics analytics--fullscreen" role="region" aria-label={requiredLocalized(l10n, 'analytics-region-aria')}>
      {/* Back button */}
      <button type="button" className="analytics-back-btn" onClick={goToWorkspacePicker}
        aria-label={l10n.getString('analytics-back-aria')}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="18" height="18" aria-hidden="true">
          <line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
        </svg>
        <Localized id="analytics-back"><span>Back</span></Localized>
      </button>
      <div className="analytics-header">
        <div className="analytics-header-row">
          <div>
            <Localized id="analytics-title"><h1 className="analytics-title">Staff Analytics</h1></Localized>
            <Localized id="analytics-subtitle"><p className="analytics-subtitle">Per-staff shifts and sales over time</p></Localized>
          </div>
          {rows.length > 0 && (
            <button type="button" className="analytics-export-btn"
              onClick={() => downloadCsv(`staff-analytics-${from}-to-${to}.csv`,
                [{ key: 'display_name', label: 'Staff' }, { key: 'shift_count', label: 'Shifts' },
                 { key: 'closed_shift_count', label: 'Closed' }, { key: 'sale_count', label: 'Sales' },
                 { key: 'sale_total_minor', label: 'Sales Total' }],
                rows.map((r) => ({ ...r, sale_total_minor: String(r.sale_total_minor) })),
              )}
              aria-label={l10n.getString('analytics-export-csv-aria')}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <Localized id="analytics-export-csv"><span>CSV</span></Localized>
            </button>
          )}
        </div>
      </div>

      <Card shadow="sm" className="analytics-filters">
        <div className="analytics-filter-field">
          <label htmlFor="analytics-from" className="analytics-filter-label"><Localized id="analytics-filter-from"><span>From</span></Localized></label>
          <input id="analytics-from" type="date" className="analytics-filter-input" value={fromDraft} max={toDraft}
            onChange={(e) => setFromDraft(e.target.value)} aria-label={l10n.getString('analytics-filter-from')} />
        </div>
        <div className="analytics-filter-field">
          <label htmlFor="analytics-to" className="analytics-filter-label"><Localized id="analytics-filter-to"><span>To</span></Localized></label>
          <input id="analytics-to" type="date" className="analytics-filter-input" value={toDraft} min={fromDraft}
            onChange={(e) => setToDraft(e.target.value)} aria-label={l10n.getString('analytics-filter-to')} />
        </div>
        <Localized id="analytics-btn-apply">
          <button type="button" className="analytics-apply-btn" onClick={applyFilters} aria-label={l10n.getString('analytics-btn-apply')}>Apply</button>
        </Localized>
      </Card>

      {error && <div className="analytics-error" role="alert">{error}</div>}

      {loading ? (
        <div className="analytics-loading"><Spinner aria-label={l10n.getString('analytics-loading')} /></div>
      ) : rows.length === 0 ? (
        <Localized id="analytics-empty"><p className="analytics-empty">No staff activity in this period.</p></Localized>
      ) : (
        <>
          <div className="analytics-kpi-row">
            <Card shadow="sm" className="analytics-kpi">
              <span className="analytics-kpi-label"><Localized id="analytics-kpi-shifts"><span>Total Shifts</span></Localized></span>
              <span className="analytics-kpi-value">{kpis.totalShifts}</span>
              <span className="analytics-kpi-sub">{kpis.closedShifts} {l10n.getString('analytics-kpi-closed')}</span>
            </Card>
            <Card shadow="sm" className="analytics-kpi">
              <span className="analytics-kpi-label"><Localized id="analytics-kpi-avg-sale"><span>Avg Sale / Shift</span></Localized></span>
              <span className="analytics-kpi-value">
                {kpis.totalShifts > 0 ? formatMoney({ minor_units: Math.round(kpis.totalSalesMinor / kpis.totalShifts), currency }) : '-'}
              </span>
            </Card>
            <Card shadow="sm" className="analytics-kpi">
              <span className="analytics-kpi-label"><Localized id="analytics-kpi-top-performer"><span>Top Performer</span></Localized></span>
              <span className="analytics-kpi-value analytics-kpi-value--name">
                {rows.length > 0 ? rows.reduce((a, b) => a.sale_total_minor > b.sale_total_minor ? a : b).display_name : '-'}
              </span>
            </Card>
            <Card shadow="sm" className="analytics-kpi">
              <span className="analytics-kpi-label"><Localized id="analytics-kpi-coverage"><span>Coverage</span></Localized></span>
              <span className="analytics-kpi-value">
                {kpis.totalShifts > 0 ? `${Math.round((kpis.closedShifts / kpis.totalShifts) * 100)}%` : '-'}
              </span>
              <span className="analytics-kpi-sub">{kpis.staffCount} {l10n.getString('analytics-kpi-staff')}</span>
            </Card>
          </div>

          <div className="analytics-chart-row">
            <Card shadow="sm" className="analytics-chart-card">
              <Localized id="analytics-chart-daily-sales"><h2 className="analytics-card-title">Daily Sales by Staff</h2></Localized>
              {stackedBarOption ? (
                <ReactEChartsCore echarts={echarts} option={stackedBarOption} style={{ height: 320 }} notMerge
                  aria-label={l10n.getString('analytics-chart-daily-sales-aria')} />
              ) : (
                <div className="analytics-chart-placeholder">
                  <Localized id="analytics-chart-select-hint"><p className="analytics-empty">No daily breakdown available.</p></Localized>
                </div>
              )}
            </Card>
            <Card shadow="sm" className="analytics-table-card">
              <Localized id="analytics-summary-title"><h2 className="analytics-card-title">Staff Summary</h2></Localized>
              <div className="analytics-table-wrap">
                <table className="analytics-table" aria-label={l10n.getString('analytics-summary-title')}>
                  <thead><tr>
                    <Localized id="analytics-table-staff"><th>Staff</th></Localized>
                    <Localized id="analytics-table-shifts"><th>Shifts</th></Localized>
                    <Localized id="analytics-table-sales"><th>Sales</th></Localized>
                    <Localized id="analytics-table-sales-total"><th>Total</th></Localized>
                  </tr></thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr key={r.user_id} onClick={() => setSelectedUserId(r.user_id)}
                        className={r.user_id === selectedUserId ? 'analytics-row--selected' : 'analytics-row'}>
                        <td className="analytics-cell-name">{r.display_name}</td>
                        <td className="analytics-cell-num">{r.shift_count}</td>
                        <td className="analytics-cell-num">{r.sale_count}</td>
                        <td className="analytics-cell-mono">{formatMoney({ minor_units: r.sale_total_minor, currency })}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          </div>

          {selectedRow && (
            <Card shadow="sm" className="analytics-card">
              <Localized id="analytics-deepdive-title" vars={{ name: selectedRow.display_name }}>
                <h2 className="analytics-card-title">{selectedRow.display_name} — Daily Detail</h2>
              </Localized>
              {deepDiveOption ? (
                <ReactEChartsCore echarts={echarts} option={deepDiveOption} style={{ height: 280 }} notMerge
                  aria-label={l10n.getString('analytics-deepdive-aria', { name: selectedRow.display_name })} />
              ) : (
                <Localized id="analytics-deepdive-empty"><p className="analytics-empty">No daily data available.</p></Localized>
              )}
            </Card>
          )}
        </>
      )}
    </div>
  );
}
