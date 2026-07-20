import { useState, useCallback, useMemo } from 'react';
import { Localized } from '@fluent/react';
import { buildCustomReport, type CustomReportRequest, type CustomReportResponse } from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './CustomReportScreen.css';

/** Dataset definitions with human-readable column labels. */
const DATASETS: Record<string, { label: string; columns: Record<string, string>; hasDateFilter: boolean }> = {
  sales: {
    label: 'Sales History',
    columns: { id: 'Sale ID', total_minor: 'Total', created_at: 'Created', status: 'Status', customer_id: 'Customer' },
    hasDateFilter: true,
  },
  inventory: {
    label: 'Current Inventory',
    columns: { sku: 'SKU', name: 'Name', price_minor: 'Price', category_id: 'Category', barcode: 'Barcode' },
    hasDateFilter: false,
  },
};

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function monthAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().slice(0, 10);
}

/** Custom report builder — dataset picker, column selector, preview table, CSV export. */
export default function CustomReportScreen() {
  const [dataset, setDataset] = useState('sales');
  const [selectedCols, setSelectedCols] = useState<Set<string>>(new Set(['id', 'total_minor', 'status']));
  const [startDate, setStartDate] = useState(monthAgo());
  const [endDate, setEndDate] = useState(today());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CustomReportResponse | null>(null);

  const dsDef = DATASETS[dataset]!;
  const colEntries = useMemo(() => Object.entries(dsDef.columns), [dsDef]);

  const toggleCol = useCallback((col: string) => {
    setSelectedCols((prev) => {
      const next = new Set(prev);
      if (next.has(col)) next.delete(col);
      else next.add(col);
      return next;
    });
  }, []);

  const runReport = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const req: CustomReportRequest = {
        dataset,
        columns: Array.from(selectedCols),
        start_date: dsDef.hasDateFilter ? startDate : null,
        end_date: dsDef.hasDateFilter ? endDate : null,
      };
      const resp = await buildCustomReport(req);
      setResult(resp);
    } catch (e: unknown) {
      setError((e as Error).message ?? String(e));
    } finally {
      setLoading(false);
    }
  }, [dataset, selectedCols, startDate, endDate, dsDef]);

  const exportCsv = useCallback(() => {
    if (!result || result.rows.length === 0) return;
    const bom = '\uFEFF';
    const csv = [
      result.columns.join(','),
      ...result.rows.map((row) => row.map((cell) => `"${String(cell).replace(/"/g, '""')}"`).join(',')),
    ].join('\n');
    const blob = new Blob([bom + csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `custom-report-${dataset}-${startDate}-${endDate}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [result, dataset, startDate, endDate]);

  const hasResults = result && result.rows.length > 0;
  const hasEmptyResults = result && result.rows.length === 0;

  return (
    <div className="custom-report" role="region" aria-label="Custom Report Builder">
      <div className="custom-report-header">
        <Localized id="custom-report-title">
          <h1 className="custom-report-title">Custom Report</h1>
        </Localized>
      </div>

      <Card shadow="sm" className="custom-report-config-card">
        <div className="custom-report-controls">
          <div className="custom-report-field">
            <label htmlFor="cr-dataset" className="custom-report-label">
              <Localized id="custom-report-dataset">Dataset</Localized>
            </label>
            <select
              id="cr-dataset"
              value={dataset}
              onChange={(e) => { setDataset(e.target.value); setResult(null); }}
              className="custom-report-select"
              aria-label="Dataset"
            >
              {Object.entries(DATASETS).map(([key, def]) => (
                <option key={key} value={key}>{def.label}</option>
              ))}
            </select>
          </div>

          {dsDef.hasDateFilter && (
            <>
              <div className="custom-report-field">
                <label htmlFor="cr-start" className="custom-report-label">
                  <Localized id="custom-report-start">Start</Localized>
                </label>
                <input id="cr-start" type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)} className="custom-report-input" aria-label="Start date" />
              </div>
              <div className="custom-report-field">
                <label htmlFor="cr-end" className="custom-report-label">
                  <Localized id="custom-report-end">End</Localized>
                </label>
                <input id="cr-end" type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)} className="custom-report-input" aria-label="End date" />
              </div>
            </>
          )}
        </div>

        <div className="custom-report-columns-section">
          <Localized id="custom-report-columns">
            <span className="custom-report-label">Columns</span>
          </Localized>
          <div className="custom-report-columns-grid" role="group" aria-label="Column selection">
            {colEntries.map(([key, label]) => (
              <label key={key} className="custom-report-col-checkbox">
                <input
                  type="checkbox"
                  checked={selectedCols.has(key)}
                  onChange={() => toggleCol(key)}
                  aria-label={label}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </div>

        <Button onClick={runReport} disabled={loading || selectedCols.size === 0} aria-label="Run report">
          {loading ? (
            <Localized id="shared-loading">Loading…</Localized>
          ) : (
            <Localized id="custom-report-run">Run Report</Localized>
          )}
        </Button>
      </Card>

      {error && <p className="custom-report-error"><Localized id="error-occurred"><span>An error occurred</span></Localized>: {error}</p>}

      {hasEmptyResults && (
        <p className="custom-report-no-data">
          <Localized id="no-results"><span>No results found</span></Localized>
        </p>
      )}

      {hasResults && (
        <Card shadow="sm" className="custom-report-result-card">
          <div className="custom-report-result-header">
            <Localized id="custom-report-results">
              <h2 className="custom-report-section-title">Results</h2>
            </Localized>
            <Button variant="secondary" onClick={exportCsv} aria-label="Export CSV">
              <Localized id="custom-report-export-csv">Export CSV</Localized>
            </Button>
          </div>
          <div className="custom-report-table-wrap">
            <table className="custom-report-table">
              <thead>
                <tr>
                  {result!.columns.map((col) => (
                    <th key={col}>{dsDef.columns[col] ?? col}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {result!.rows.map((row, i) => (
                  <tr key={i}>
                    {row.map((cell, j) => (
                      <td key={j}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}
    </div>
  );
}
