import { useEffect, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { printSalesReceipt } from '@/api/sales';
import { getLowStockAlerts, type LowStockAlert } from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './InventoryReportScreen.css';

/** Inventory report screen — view and export low-stock alerts with configurable threshold, CSV download, and print support. */
export default function InventoryReportScreen() {
  const { l10n } = useLocalization();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState<LowStockAlert[]>([]);
  const [threshold, setThreshold] = useState(10);

  useEffect(() => {
    setLoading(true);
    getLowStockAlerts(threshold)
      .then(setItems)
      .catch((e) => setError(e.message ?? String(e)))
      .finally(() => setLoading(false));
  }, [threshold]);

  const exportCsv = () => {
    const headers = [
      l10n.getString('inv-report-csv-header-sku'),
      l10n.getString('inv-report-csv-header-product'),
      l10n.getString('inv-report-csv-header-stock'),
      l10n.getString('inv-report-csv-header-threshold'),
    ];
    const rows = items.map((i) =>
      [i.sku, `"${i.name}"`, i.current_qty, i.threshold].join(','),
    );
    const csv = [headers.join(','), ...rows].join('\n');
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `inventory-report-${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const printReport = async () => {
    await printSalesReceipt({
      date: new Date().toISOString().slice(0, 10),
      receiptNumber: `INV-${Date.now()}`,
      items: items.map((i) => ({
        name: i.name,
        quantity: i.current_qty,
        unitPrice: { minorUnits: 0, currency: 'USD' },
        totalPrice: { minorUnits: 0, currency: 'USD' },
      })),
      subtotal: { minorUnits: 0, currency: 'USD' },
      total: { minorUnits: 0, currency: 'USD' },
      payments: [{ method: 'Report', amount: { minorUnits: 0, currency: 'USD' }, change: null }],
    });
  };

  if (loading) {
    return (
      <div className="inventory-report-loading-skeleton" aria-hidden="true">
        {/* Header: title + controls */}
        <div className="inventory-report-header">
          <Skeleton width="10rem" height="1.75rem" />
          <div className="inventory-report-controls">
            <Skeleton width="4rem" height="2rem" />
            <Skeleton width="4rem" height="2rem" />
            <Skeleton width="6rem" height="2rem" />
          </div>
        </div>
        {/* Table card */}
        <Card shadow="sm" className="inventory-report-table-card">
          <div className="inventory-report-table-header">
            <Skeleton width="5rem" height="0.75rem" />
            <Skeleton width="4rem" height="0.75rem" />
            <Skeleton width="3rem" height="0.75rem" />
          </div>
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="inventory-report-row">
              <Skeleton width="4rem" height="0.875rem" />
              <Skeleton width="7rem" height="0.875rem" />
              <Skeleton width="2rem" height="0.875rem" />
            </div>
          ))}
        </Card>
      </div>
    );
  }

  return (
    <div className="inventory-report" role="region" aria-label={l10n.getString('inv-report-region-aria')}>
      <div className="inventory-report-header">
        <Localized id="inv-report-title">
          <h1 className="inventory-report-title">Inventory Report</h1>
        </Localized>
        <div className="inventory-report-controls">
          <Localized id="inv-report-threshold">
            <label htmlFor="threshold-input" className="inventory-report-label">
              Threshold
            </label>
          </Localized>
          <input
            id="threshold-input"
            type="number"
            min={0}
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            className="inventory-report-input"
            aria-label={l10n.getString('inv-report-threshold-aria')}
          />
          <Button
            variant="secondary"
            onClick={printReport}
            aria-label={l10n.getString('inv-report-print-aria')}
          >
            <Localized id="print">Print</Localized>
          </Button>
          <Button
            variant="secondary"
            onClick={exportCsv}
            aria-label={l10n.getString('inv-report-export-aria')}
          >
            <Localized id="inv-report-export-csv">Export CSV</Localized>
          </Button>
        </div>
      </div>

      {error && <p className="inventory-report-error">{error}</p>}

      <Card shadow="sm" className="inventory-report-table-card">
        <div className="inventory-report-table">
          <div className="inventory-report-table-header">
            <span>
              <Localized id="inv-report-sku">SKU</Localized>
            </span>
            <span>
              <Localized id="inv-report-product">Product</Localized>
            </span>
            <span>
              <Localized id="inv-report-current-stock">Stock</Localized>
            </span>
          </div>
          {items.length === 0 ? (
            <div className="inventory-report-empty">
              <Localized id="inv-report-no-results">
                <p>No results found</p>
              </Localized>
            </div>
          ) : (
            items.map((item) => {
              const qty = item.current_qty;
              const isCritical = qty === 0;
              const isLow = qty > 0 && qty <= threshold;
              return (
                <div
                  key={item.product_id}
                  className={`inventory-report-row ${isCritical ? 'critical' : isLow ? 'low' : ''}`}
                >
                  <span className="inventory-report-sku">{item.sku}</span>
                  <span className="inventory-report-name">{item.name}</span>
                  <span
                    className={`inventory-report-qty ${isCritical ? 'critical' : isLow ? 'low' : ''}`}
                  >
                    {qty}
                  </span>
                </div>
              );
            })
          )}
        </div>
      </Card>
    </div>
  );
}
