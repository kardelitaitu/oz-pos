import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import {
  listSales,
  getSale,
  printSalesReceipt,
  type SaleListItem,
  type SaleDetail,
} from '@/api/pos';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import './SalesHistoryScreen.css';

function statusBadgeVariant(status: string): 'success' | 'warning' | 'danger' | 'info' {
  switch (status) {
    case 'Completed': return 'success';
    case 'Pending': return 'warning';
    case 'Cancelled': return 'danger';
    default: return 'info';
  }
}

function statusFluentId(status: string): string {
  switch (status) {
    case 'Completed': return 'sales-history-status-completed';
    case 'Pending': return 'sales-history-status-pending';
    case 'Cancelled': return 'sales-history-status-cancelled';
    default: return 'sales-history-status-completed';
  }
}

export default function SalesHistoryScreen() {
  const [sales, setSales] = useState<SaleListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [detail, setDetail] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [printing, setPrinting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const items = await listSales();
      setSales(items);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const openDetail = useCallback(async (id: string) => {
    setDetailLoading(true);
    try {
      const sale = await getSale(id);
      setDetail(sale);
    } catch {
      // IPC unavailable.
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const closeDetail = useCallback(() => {
    setDetail(null);
  }, []);

  const handleReprint = useCallback(async () => {
    if (!detail) return;
    setPrinting(true);
    try {
      const subtotalMinor = detail.lines.reduce((s, l) => s + l.total_minor, 0);
      await printSalesReceipt({
        date: detail.createdAt,
        receiptNumber: detail.id,
        items: detail.lines.map((l) => ({
          name: l.name,
          quantity: l.qty,
          unitPrice: { minorUnits: l.unit_price.minor_units, currency: l.unit_price.currency },
          totalPrice: { minorUnits: l.total_minor, currency: l.unit_price.currency },
        })),
        subtotal: { minorUnits: subtotalMinor, currency: detail.total.currency },
        total: { minorUnits: detail.total.minor_units, currency: detail.total.currency },
        payments: [
          {
            method: detail.paymentMethod ?? 'Unknown',
            amount: { minorUnits: detail.total.minor_units, currency: detail.total.currency },
            change: detail.tenderedMinor !== null
              ? { minorUnits: Math.max(0, detail.tenderedMinor - detail.total.minor_units), currency: detail.total.currency }
              : null,
          },
        ],
      });
    } catch {
      // Ignore print errors.
    } finally {
      setPrinting(false);
    }
  }, [detail]);

  const handleExportCsv = useCallback(() => {
    const headers = ['Sale ID', 'Date', 'Total', 'Items', 'Status', 'Payment'];
    const rows = sales.map((s) => [
      s.id,
      new Date(s.createdAt).toLocaleString(),
      formatMoney(s.total),
      String(s.lineCount),
      s.status,
      s.paymentMethod ?? '',
    ]);
    const csv = [headers.join(','), ...rows.map((r) => r.map((c) => `"${c}"`).join(','))].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sales-export-${new Date().toISOString().split('T')[0]}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [sales]);

  return (
    <div className="sales-history">
      <div className="sales-history-header">
        <Localized id="sales-history-title">
          <h1 className="sales-history-title">Sales History</h1>
        </Localized>
        <Localized id="sales-history-export-csv">
          <button type="button" className="sales-history-export-btn" onClick={handleExportCsv}>
            Export CSV
          </button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="sales-history-loading">
          <p className="sales-history-loading">Loading sales&hellip;</p>
        </Localized>
      ) : sales.length === 0 ? (
        <Card shadow="sm">
          <div className="sales-history-empty">
            <Localized id="sales-history-empty">
              <p>No sales recorded yet</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="sales-history-table-wrap">
          <table className="sales-history-table" aria-label="Sales history">
            <thead>
              <tr>
                <Localized id="sales-history-col-id"><th>Sale ID</th></Localized>
                <Localized id="sales-history-col-date"><th>Date</th></Localized>
                <Localized id="sales-history-col-total"><th>Total</th></Localized>
                <Localized id="sales-history-col-items"><th>Items</th></Localized>
                <Localized id="sales-history-col-status"><th>Status</th></Localized>
                <Localized id="sales-history-col-payment"><th>Payment</th></Localized>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {sales.map((s) => (
                <tr key={s.id}>
                  <td className="sales-history-cell-id">{s.id.slice(0, 8)}&hellip;</td>
                  <td>{new Date(s.createdAt).toLocaleString()}</td>
                  <td className="sales-history-cell-total">{formatMoney(s.total)}</td>
                  <td>{s.lineCount}</td>
                  <td>
                    <Badge variant={statusBadgeVariant(s.status)}>
                      <Localized id={statusFluentId(s.status)}>
                        <span>{s.status}</span>
                      </Localized>
                    </Badge>
                  </td>
                  <td>{s.paymentMethod ?? '\u2014'}</td>
                  <td className="sales-history-cell-actions">
                    <button
                      type="button"
                      className="sales-history-action-btn"
                      onClick={() => openDetail(s.id)}
                      aria-label={`View ${s.id}`}
                    >
                      View
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {detail && (
        <div className="sales-history-overlay" role="dialog" aria-modal="true" aria-label="Sale detail">
          <div className="sales-history-modal">
            <div className="sales-history-modal-header">
              <Localized id="sales-history-detail-title">
                <h2>Sale Detail</h2>
              </Localized>
              <Localized id="sales-history-detail-close">
                <button
                  type="button"
                  className="sales-history-modal-close"
                  onClick={closeDetail}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>

            {detailLoading ? (
              <p>Loading&hellip;</p>
            ) : (
              <div className="sales-history-modal-body">
                <div className="sales-history-detail-meta">
                  <div><strong>ID:</strong> {detail.id}</div>
                  <div><strong>Date:</strong> {new Date(detail.createdAt).toLocaleString()}</div>
                  <div>
                    <strong>Status:</strong>{' '}
                    <Badge variant={statusBadgeVariant(detail.status)}>
                      <Localized id={statusFluentId(detail.status)}>
                        <span>{detail.status}</span>
                      </Localized>
                    </Badge>
                  </div>
                  <div><strong>Payment:</strong> {detail.paymentMethod ?? '\u2014'}</div>
                  <div><strong>Total:</strong> {formatMoney(detail.total)}</div>
                </div>

                <h3>Line Items</h3>
                <table className="sales-history-lines-table" aria-label="Sale line items">
                  <thead>
                    <tr>
                      <th>SKU</th>
                      <th>Name</th>
                      <th>Qty</th>
                      <th>Unit Price</th>
                      <th>Total</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.lines.map((line, i) => (
                      <tr key={i}>
                        <td>{line.sku}</td>
                        <td>{line.name}</td>
                        <td>{line.qty}</td>
                        <td>{formatMoney(line.unit_price)}</td>
                        <td>{formatMoney({ minor_units: line.total_minor, currency: line.unit_price.currency })}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                <div className="sales-history-modal-actions">
                  <Localized id="sales-history-detail-close">
                    <Button variant="ghost" onClick={closeDetail}>Close</Button>
                  </Localized>
                  <Localized id="sales-history-detail-print">
                    <Button variant="secondary" onClick={handleReprint} loading={printing}>
                      Reprint Receipt
                    </Button>
                  </Localized>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
