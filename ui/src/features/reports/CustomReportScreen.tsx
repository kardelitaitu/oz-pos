import { useState, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { buildCustomReport, type CustomReportRequest, type CustomReportResponse } from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './CustomReportScreen.css';

/** Dataset definitions with human-readable column labels. Matches backend `get_dataset_def()`. */
const DATASETS: Record<string, { label: string; columns: Record<string, string>; hasDateFilter: boolean }> = {
  sales: {
    label: 'Sales History',
    columns: { id: 'Sale ID', total_minor: 'Total', created_at: 'Created', status: 'Status', customer_id: 'Customer' },
    hasDateFilter: true,
  },
  inventory: {
    label: 'Current Inventory',
    columns: { sku: 'SKU', name: 'Name', price_minor: 'Price', category_id: 'Category', barcode: 'Barcode', product_type: 'Type' },
    hasDateFilter: false,
  },
  customers: {
    label: 'Customers',
    columns: { id: 'Customer ID', name: 'Name', email: 'Email', phone: 'Phone', loyalty_points: 'Loyalty Points', total_spent_minor: 'Total Spent', created_at: 'Created' },
    hasDateFilter: true,
  },
  staff: {
    label: 'Staff',
    columns: { id: 'User ID', username: 'Username', display_name: 'Display Name', is_active: 'Active', created_at: 'Created' },
    hasDateFilter: false,
  },
  tax_rates: {
    label: 'Tax Rates',
    columns: { id: 'Rate ID', name: 'Name', rate_bps: 'Rate (bps)', is_default: 'Default', created_at: 'Created' },
    hasDateFilter: false,
  },
  shifts: {
    label: 'Shifts',
    columns: { id: 'Shift ID', user_id: 'User ID', opened_at: 'Opened', closed_at: 'Closed', status: 'Status', total_sales_minor: 'Total Sales', opening_balance_minor: 'Opening Balance', closing_balance_minor: 'Closing Balance' },
    hasDateFilter: true,
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

/** A column entry with selection state and display label. */
interface ColumnItem {
  key: string;
  label: string;
  selected: boolean;
}

/** Custom report builder — dataset picker, drag-and-drop column selector, preview table, CSV export. */
export default function CustomReportScreen() {
  const [dataset, setDataset] = useState('sales');
  const [startDate, setStartDate] = useState(monthAgo());
  const [endDate, setEndDate] = useState(today());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CustomReportResponse | null>(null);
  const [searchTerm, setSearchTerm] = useState('');

  // Drag state
  const dragItemRef = useRef<number | null>(null);
  const dragOverRef = useRef<number | null>(null);

  const dsDef = DATASETS[dataset]!;

  // Build ordered column list — start with all columns selected in definition order
  const [columnOrder, setColumnOrder] = useState<string[]>(() => Object.keys(dsDef.columns));
  const [selectedCols, setSelectedCols] = useState<Set<string>>(() => new Set(Object.keys(dsDef.columns)));

  // Reset when dataset changes
  const changeDataset = useCallback((newDs: string) => {
    setDataset(newDs);
    setResult(null);
    const cols = Object.keys(DATASETS[newDs]!.columns);
    setColumnOrder(cols);
    setSelectedCols(new Set(cols));
    setSearchTerm('');
  }, []);

  /** Build the ordered column items respecting selected state and current order. */
  const columnItems = useMemo((): ColumnItem[] => {
    // Start with all keys from the definition, in the user's chosen order
    const allKeys = new Set(Object.keys(dsDef.columns));
    const ordered: ColumnItem[] = [];

    // Add keys from current order that exist in this dataset
    for (const key of columnOrder) {
      if (allKeys.has(key)) {
        ordered.push({ key, label: dsDef.columns[key]!, selected: selectedCols.has(key) });
        allKeys.delete(key);
      }
    }
    // Add any remaining keys not yet in the order (shouldn't happen normally)
    for (const key of allKeys) {
      ordered.push({ key, label: dsDef.columns[key]!, selected: selectedCols.has(key) });
    }

    return ordered;
  }, [dsDef.columns, columnOrder, selectedCols]);

  /** Filter column items by search term. */
  const filteredItems = useMemo(() => {
    if (!searchTerm.trim()) return columnItems;
    const lower = searchTerm.toLowerCase();
    return columnItems.filter(
      (item) => item.key.toLowerCase().includes(lower) || item.label.toLowerCase().includes(lower),
    );
  }, [columnItems, searchTerm]);

  const toggleCol = useCallback((key: string) => {
    setSelectedCols((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  const selectedCount = useMemo(() => selectedCols.size, [selectedCols]);

  // Drag-over visual feedback state
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

  // ── Drag-and-drop handlers ──

  const handleDragStart = useCallback((index: number) => {
    dragItemRef.current = index;
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, index: number) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    dragOverRef.current = index;
    setDragOverIndex(index);
  }, []);

  const handleDrop = useCallback(() => {
    setDragOverIndex(null);
    const fromIdx = dragItemRef.current;
    const toIdx = dragOverRef.current;
    if (fromIdx === null || toIdx === null || fromIdx === toIdx) {
      dragItemRef.current = null;
      dragOverRef.current = null;
      return;
    }

    setColumnOrder((prev) => {
      const next = [...prev];
      const moved = next.splice(fromIdx, 1)[0]!;
      next.splice(toIdx, 0, moved);
      return next;
    });
    dragItemRef.current = null;
    dragOverRef.current = null;
  }, []);

  const handleDragEnd = useCallback(() => {
    dragItemRef.current = null;
    dragOverRef.current = null;
    setDragOverIndex(null);
  }, []);

  const runReport = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Build columns array in drag-order, only including selected ones
      const orderedSelected = columnOrder.filter((c) => selectedCols.has(c));
      const req: CustomReportRequest = {
        dataset,
        columns: orderedSelected,
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
  }, [dataset, columnOrder, selectedCols, startDate, endDate, dsDef]);

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
              onChange={(e) => { changeDataset(e.target.value); }}
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
          <div className="custom-report-columns-header">
            <div className="custom-report-columns-title-row">
              <Localized id="custom-report-columns">
                <span className="custom-report-label">Columns</span>
              </Localized>
              <span className="custom-report-columns-count" aria-live="polite">
                <span>{selectedCount} / {filteredItems.length} selected</span>
              </span>
            </div>
            <div className="custom-report-columns-search">
              <input
                type="text"
                className="custom-report-search-input"
                placeholder="Search columns…"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                aria-label="Search columns"
              />
              {searchTerm && (
                <button
                  className="custom-report-search-clear"
                  onClick={() => setSearchTerm('')}
                  aria-label="Clear search"
                >
                  ×
                </button>
              )}
            </div>
          </div>

          <div className="custom-report-columns-list" role="listbox" aria-label="Column selection" aria-multiselectable="true">
            {filteredItems.map((item, index) => (
              <div
                key={item.key}
                className={`custom-report-col-item ${item.selected ? 'custom-report-col-item--selected' : ''} ${dragOverIndex === index ? 'drag-over' : ''}`}
                draggable
                onDragStart={() => handleDragStart(index)}
                onDragOver={(e) => handleDragOver(e, index)}
                onDrop={handleDrop}
                onDragEnd={handleDragEnd}
                role="option"
                tabIndex={0}
                aria-selected={item.selected}
              >
                <span className="custom-report-col-drag-handle" aria-hidden="true">
                  ⠿
                </span>
                <input
                  type="checkbox"
                  checked={item.selected}
                  onChange={() => toggleCol(item.key)}
                  aria-label={item.label}
                  className="custom-report-col-checkbox-input"
                />
                <span className="custom-report-col-label">{item.label}</span>
                <span className="custom-report-col-key">{item.key}</span>
              </div>
            ))}
            {filteredItems.length === 0 && (
              <p className="custom-report-col-no-results">
                <Localized id="custom-report-no-columns-match">No columns match your search</Localized>
              </p>
            )}
          </div>
        </div>

        <div className="custom-report-actions">
          <Button onClick={runReport} disabled={loading || selectedCount === 0} aria-label="Run report">
            {loading ? (
              <Localized id="shared-loading">Loading…</Localized>
            ) : (
              <Localized id="custom-report-run">Run Report</Localized>
            )}
          </Button>
        </div>
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
