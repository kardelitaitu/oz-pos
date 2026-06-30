import { useState, useCallback, useEffect, useMemo } from 'react';
import {
  listSales,
  getSale,
  voidSale,
  type SaleListItem,
  type SaleDetail,
} from '@/api/sales';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import './VoidOrdersScreen.css';

// ── Reason options ──────────────────────────────────────────────────

const VOID_REASONS = [
  { value: 'cancelled-by-customer', label: 'Cancelled by customer' },
  { value: 'wrong-items', label: 'Wrong items scanned' },
  { value: 'duplicate-order', label: 'Duplicate order' },
  { value: 'price-dispute', label: 'Price dispute' },
  { value: 'payment-issue', label: 'Payment issue' },
  { value: 'customer-change-of-mind', label: 'Customer changed mind' },
  { value: 'manager-authorization', label: 'Manager override' },
  { value: 'other', label: 'Other reason…' },
] as const;

// ── Helpers ─────────────────────────────────────────────────────────

function statusBadgeVariant(status: string): 'success' | 'warning' | 'danger' | 'info' {
  switch (status) {
    case 'Completed': return 'success';
    case 'Active': return 'info';
    case 'Pending': return 'warning';
    case 'Voided': return 'danger';
    default: return 'info';
  }
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

// ── Component ───────────────────────────────────────────────────────

type ViewMode = 'list' | 'detail';
type FilterStatus = 'all' | 'Active' | 'Completed' | 'Voided' | 'Pending';

interface VoidOrdersScreenProps {
  /** ID of the sale to open directly (for navigating from POS after completion). */
  initialSaleId?: string;
}

export default function VoidOrdersScreen({ initialSaleId }: VoidOrdersScreenProps) {
  // Data
  const [sales, setSales] = useState<SaleListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<FilterStatus>('all');

  // Detail view
  const [viewMode, setViewMode] = useState<ViewMode>(initialSaleId ? 'detail' : 'list');
  const [activeSaleId, setActiveSaleId] = useState<string | null>(initialSaleId ?? null);
  const [detail, setDetail] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // Void flow
  const [voidReason, setVoidReason] = useState('');
  const [customReason, setCustomReason] = useState('');
  const [voiding, setVoiding] = useState(false);
  const [voidError, setVoidError] = useState<string | null>(null);
  const [voidSuccess, setVoidSuccess] = useState<string | null>(null);

  // ── Load sales ──────────────────────────────────────────────────

  const loadSales = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const items = await listSales();
      setSales(items);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load orders');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadSales(); }, [loadSales]);

  // Load detail when a sale is selected.
  useEffect(() => {
    if (!activeSaleId) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    setDetailLoading(true);
    (async () => {
      try {
        const sale = await getSale(activeSaleId);
        if (!cancelled) setDetail(sale);
      } catch {
        if (!cancelled) setDetail(null);
      } finally {
        if (!cancelled) setDetailLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [activeSaleId]);

  // ── Open initial sale if provided ────────────────────────────────

  useEffect(() => {
    if (initialSaleId && sales.length > 0) {
      const exists = sales.some((s) => s.id === initialSaleId);
      if (exists) {
        setActiveSaleId(initialSaleId);
        setViewMode('detail');
      }
    }
  }, [initialSaleId, sales]);

  // ── Filtered sales ──────────────────────────────────────────────

  const filteredSales = useMemo(() => {
    let items = sales;

    if (statusFilter !== 'all') {
      items = items.filter((s) => s.status === statusFilter);
    }

    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      items = items.filter(
        (s) =>
          s.id.toLowerCase().includes(q) ||
          (s.paymentMethod ?? '').toLowerCase().includes(q),
      );
    }

    return items;
  }, [sales, statusFilter, searchQuery]);

  // ── Void handler ────────────────────────────────────────────────

  const handleVoid = useCallback(async () => {
    if (!activeSaleId || !detail) return;

    const reason = voidReason === 'other' ? customReason.trim() : voidReason;
    if (!reason) {
      setVoidError('Please select or enter a void reason');
      return;
    }

    setVoiding(true);
    setVoidError(null);
    setVoidSuccess(null);

    try {
      await voidSale({
        saleId: activeSaleId,
        userId: 'admin', // Will be replaced once StaffLogin is implemented
        reason,
      });
      setVoidSuccess('Order voided successfully. Stock has been restored.');
      setVoidError(null);
      setVoidReason('');
      setCustomReason('');
      // Refresh just the detail without a full list reload to avoid flicker.
      const updated = await getSale(activeSaleId);
      if (updated) setDetail(updated);
    } catch (err) {
      setVoidError(err instanceof Error ? err.message : 'Failed to void order');
    } finally {
      setVoiding(false);
    }
  }, [activeSaleId, detail, voidReason, customReason, loadSales]);

  const openDetail = useCallback((id: string) => {
    setActiveSaleId(id);
    setViewMode('detail');
    setVoidError(null);
    setVoidSuccess(null);
    setVoidReason('');
    setCustomReason('');
  }, []);

  const backToList = useCallback(() => {
    setViewMode('list');
    setActiveSaleId(null);
    setDetail(null);
    setVoidError(null);
    setVoidSuccess(null);
    setVoidReason('');
    setCustomReason('');
  }, []);

  // ── Render: List view ──────────────────────────────────────────

  if (viewMode === 'list') {
    return (
      <div className="void-orders">
        <div className="void-orders-header">
          <h1 className="void-orders-title">Orders</h1>
        </div>

        {/* Filters */}
        <div className="void-orders-filters">
          <div className="void-orders-search-wrap">
            <svg className="void-orders-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              type="search"
              className="void-orders-search"
              placeholder="Search by order ID or payment method…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              aria-label="Search orders"
            />
          </div>

          <div className="void-orders-status-filters" role="radiogroup" aria-label="Filter by status">
            {(['all', 'Active', 'Completed', 'Voided', 'Pending'] as FilterStatus[]).map((status) => (
              <button
                key={status}
                type="button"
                className={`void-orders-status-chip ${statusFilter === status ? 'void-orders-status-chip--active' : ''}`}
                onClick={() => setStatusFilter(status)}
                role="radio"
                aria-checked={statusFilter === status}
              >
                {status === 'all' ? 'All' : status}
              </button>
            ))}
          </div>
        </div>

        {/* Content */}
        {loading ? (
          <div className="void-orders-loading">Loading orders…</div>
        ) : error ? (
          <Card shadow="sm">
            <div className="void-orders-error">
              <p>{error}</p>
              <Button variant="secondary" onClick={loadSales}>Retry</Button>
            </div>
          </Card>
        ) : filteredSales.length === 0 ? (
          <Card shadow="sm">
            <div className="void-orders-empty">
              {searchQuery || statusFilter !== 'all'
                ? 'No orders match the current filters.'
                : 'No orders recorded yet.'}
            </div>
          </Card>
        ) : (
          <div className="void-orders-table-wrap">
            <table className="void-orders-table" aria-label="Orders">
              <thead>
                <tr>
                  <th>Order ID</th>
                  <th>Date</th>
                  <th>Status</th>
                  <th>Total</th>
                  <th>Items</th>
                  <th>Payment</th>
                  <th aria-label="Actions"> </th>
                </tr>
              </thead>
              <tbody>
                {filteredSales.map((sale) => (
                  <tr key={sale.id} className={sale.status === 'Active' ? 'void-orders-row--active' : ''}>
                    <td className="void-orders-cell-id">{sale.id.slice(0, 8)}&hellip;</td>
                    <td>{formatDate(sale.createdAt)}</td>
                    <td>
                      <Badge variant={statusBadgeVariant(sale.status)}>
                        {sale.status}
                      </Badge>
                    </td>
                    <td className="void-orders-cell-total">{formatMoney(sale.total)}</td>
                    <td>{sale.lineCount}</td>
                    <td>{sale.paymentMethod ?? '\u2014'}</td>
                    <td className="void-orders-cell-actions">
                      <button
                        type="button"
                        className="void-orders-action-btn"
                        onClick={() => openDetail(sale.id)}
                        aria-label={`View order ${sale.id}`}
                      >
                        View
                      </button>
                      {sale.status === 'Active' && (
                        <button
                          type="button"
                          className="void-orders-action-btn void-orders-action-btn--void"
                          onClick={() => openDetail(sale.id)}
                          aria-label={`Void order ${sale.id}`}
                        >
                          Void
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    );
  }

  // ── Render: Detail view ────────────────────────────────────────

  const canVoid = detail?.status === 'Active';

  return (
    <div className="void-orders">
      <div className="void-orders-header">
        <button
          type="button"
          className="void-orders-back-btn"
          onClick={backToList}
          aria-label="Back to orders list"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" aria-hidden="true">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
          Back to Orders
        </button>
      </div>

      {detailLoading ? (
        <div className="void-orders-loading">Loading order details…</div>
      ) : !detail ? (
        <Card shadow="sm">
          <div className="void-orders-error">
            <p>Order not found.</p>
            <Button variant="secondary" onClick={backToList}>Go back</Button>
          </div>
        </Card>
      ) : (
        <div className="void-orders-detail">
          {/* Order summary card */}
          <Card shadow="sm">
            <div className="void-orders-detail-summary">
              <div className="void-orders-detail-header">
                <h2 className="void-orders-detail-id">Order {detail.id.slice(0, 8)}</h2>
                <Badge variant={statusBadgeVariant(detail.status)}>
                  {detail.status}
                </Badge>
              </div>

              <div className="void-orders-detail-meta">
                <div className="void-orders-meta-item">
                  <span className="void-orders-meta-label">Date</span>
                  <span className="void-orders-meta-value">{formatDate(detail.createdAt)}</span>
                </div>
                <div className="void-orders-meta-item">
                  <span className="void-orders-meta-label">Payment</span>
                  <span className="void-orders-meta-value">{detail.paymentMethod ?? '\u2014'}</span>
                </div>
                <div className="void-orders-meta-item">
                  <span className="void-orders-meta-label">Total</span>
                  <span className="void-orders-meta-value void-orders-meta-value--total">{formatMoney(detail.total)}</span>
                </div>
                <div className="void-orders-meta-item">
                  <span className="void-orders-meta-label">Items</span>
                  <span className="void-orders-meta-value">{detail.lineCount}</span>
                </div>
              </div>
            </div>
          </Card>

          {/* Line items */}
          <Card shadow="sm" className="void-orders-section">
            <h3 className="void-orders-section-title">Line Items</h3>
            <table className="void-orders-lines-table" aria-label="Order line items">
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
                    <td className="void-orders-cell-mono">{line.sku}</td>
                    <td>{line.name}</td>
                    <td>{line.qty}</td>
                    <td>{formatMoney(line.unit_price)}</td>
                    <td className="void-orders-cell-total">
                      {formatMoney({
                        minor_units: line.total_minor,
                        currency: line.unit_price.currency,
                      })}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>

          {/* Void action (only for Active sales) */}
          {canVoid && (
            <Card shadow="sm" className="void-orders-section void-orders-void-section">
              <h3 className="void-orders-section-title void-orders-section-title--danger">
                Void Order
              </h3>
              <p className="void-orders-void-desc">
                This will cancel the order, restore all items to inventory,
                and create an immutable audit log entry. This action cannot
                be undone.
              </p>

              {/* Reason picker */}
              <div className="void-orders-reason-group">
                <label className="void-orders-reason-label" htmlFor="void-reason-select">
                  Reason for void
                </label>
                <select
                  id="void-reason-select"
                  className="void-orders-reason-select"
                  value={voidReason}
                  onChange={(e) => {
                    setVoidReason(e.target.value);
                    setVoidError(null);
                  }}
                  aria-describedby="void-reason-desc"
                >
                  <option value="">Select a reason…</option>
                  {VOID_REASONS.map((r) => (
                    <option key={r.value} value={r.value}>
                      {r.label}
                    </option>
                  ))}
                </select>

                {voidReason === 'other' && (
                  <input
                    type="text"
                    className="void-orders-reason-input"
                    placeholder="Enter the reason for voiding this order…"
                    value={customReason}
                    onChange={(e) => {
                      setCustomReason(e.target.value);
                      setVoidError(null);
                    }}
                    aria-label="Custom void reason"
                  />
                )}
              </div>

              {voidError && (
                <div className="void-orders-void-error" role="alert">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
                    <circle cx="12" cy="12" r="10" />
                    <line x1="15" y1="9" x2="9" y2="15" />
                    <line x1="9" y1="9" x2="15" y2="15" />
                  </svg>
                  {voidError}
                </div>
              )}

              {voidSuccess && (
                <div className="void-orders-void-success" role="status">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                  {voidSuccess}
                </div>
              )}

              <div className="void-orders-void-actions">
                <Button variant="ghost" onClick={backToList}>
                  Cancel
                </Button>
                <Button
                  variant="danger"
                  onClick={handleVoid}
                  loading={voiding}
                  disabled={!voidReason || (voidReason === 'other' && !customReason.trim())}
                >
                  {voiding ? 'Voiding…' : 'Confirm Void'}
                </Button>
              </div>
            </Card>
          )}

          {/* Already voided notice */}
          {detail.status === 'Voided' && (
            <Card shadow="sm" className="void-orders-section">
              <div className="void-orders-voided-notice">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="20" height="20" aria-hidden="true">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="15" y1="9" x2="9" y2="15" />
                  <line x1="9" y1="9" x2="15" y2="15" />
                </svg>
                <span>This order has been voided.</span>
              </div>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}
