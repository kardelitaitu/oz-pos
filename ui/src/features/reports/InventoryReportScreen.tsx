import { useEffect, useState } from 'react';
import { Localized } from '@fluent/react';
import { getLowStockAlerts, type LowStockAlert } from '@/api/reports';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Spinner } from '@/components/Spinner';
import './InventoryReportScreen.css';

export default function InventoryReportScreen() {
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
    const headers = ['SKU', 'Product', 'Current Stock', 'Threshold'];
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

  if (loading) {
    return (
      <div className="inventory-report">
        <Spinner aria-label="Loading inventory report" />
      </div>
    );
  }

  return (
    <div className="inventory-report" role="region" aria-label="Inventory Report">
      <div className="inventory-report-header">
        <Localized id="inventory-report-title">
          <h1 className="inventory-report-title">Inventory Report</h1>
        </Localized>
        <div className="inventory-report-controls">
          <label htmlFor="threshold-input" className="inventory-report-label">
            <Localized id="inventory-report-threshold">Threshold</Localized>
          </label>
          <input
            id="threshold-input"
            type="number"
            min={0}
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            className="inventory-report-input"
            aria-label="Stock threshold"
          />
          <Button
            variant="secondary"
            onClick={exportCsv}
            aria-label="Export CSV"
          >
            <Localized id="inventory-report-export-csv">Export CSV</Localized>
          </Button>
        </div>
      </div>

      {error && <p className="inventory-report-error">{error}</p>}

      <Card shadow="sm" className="inventory-report-table-card">
        <div className="inventory-report-table">
          <div className="inventory-report-table-header">
            <span>
              <Localized id="inventory-report-sku">SKU</Localized>
            </span>
            <span>
              <Localized id="inventory-report-product">Product</Localized>
            </span>
            <span>
              <Localized id="inventory-report-current-stock">Stock</Localized>
            </span>
          </div>
          {items.length === 0 ? (
            <div className="inventory-report-empty">
              <Localized id="no-results">
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
