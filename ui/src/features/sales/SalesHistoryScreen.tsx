import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized } from '@fluent/react';
import {
  listSales,
  getSale,
  printSalesReceipt,
  listStaff,
  listRefunds,
  type SaleListItem,
  type SaleDetail,
  type StaffMemberDto,
  type RefundDto,
} from '@/api/pos';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import { useAuth } from '@/contexts/AuthContext';
import RefundModal from './RefundModal';
import './SalesHistoryScreen.css';

const STATUS_OPTIONS = ['All', 'Completed', 'Pending', 'Voided'] as const;

function statusBadgeVariant(status: string): 'success' | 'warning' | 'danger' | 'info' {
  switch (status) {
    case 'Completed': return 'success';
    case 'Pending': return 'warning';
    case 'Voided': return 'danger';
    default: return 'info';
  }
}

function statusFluentId(status: string): string {
  switch (status) {
    case 'Completed': return 'sales-history-status-completed';
    case 'Pending': return 'sales-history-status-pending';
    case 'Voided': return 'sales-history-status-voided';
    default: return 'sales-history-status-completed';
  }
}

export default function SalesHistoryScreen() {
  const [sales, setSales] = useState<SaleListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [staff, setStaff] = useState<StaffMemberDto[]>([]);
  const [detail, setDetail] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [printing, setPrinting] = useState(false);
  const [refundSaleId, setRefundSaleId] = useState<string | null>(null);
  const [refunds, setRefunds] = useState<RefundDto[]>([]);
  const [refundsLoading, setRefundsLoading] = useState(false);
  const { session } = useAuth();

  // ── Filters ────────────────────────────────────────────────────
  const [searchQuery, setSearchQuery] = useState('');

  // ── Sorting ─────────────────────────────────────────────────────
  type SortKey = 'id' | 'createdAt' | 'total' | 'lineCount' | 'status' | 'paymentMethod';
  const [sortKey, setSortKey] = useState<SortKey>('createdAt');
  const [sortAsc, setSortAsc] = useState(false);

  const toggleSort = useCallback((key: SortKey) => {
    setSortKey((prev) => {
      if (prev === key) {
        setSortAsc((a) => !a);
        return prev;
      }
      setSortAsc(key === 'createdAt' ? false : true);
      return key;
    });
  }, []);
  const [statusFilter, setStatusFilter] = useState<string>('All');
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');
  const [cashierFilter, setCashierFilter] = useState('');

  // ── Pagination ────────────────────────────────────────────────────
  const PAGE_SIZE_OPTIONS = [10, 25, 50, 100] as const;
  const [pageSize, setPageSize] = useState(25);
  const [page, setPage] = useState(1);

  // Reset to page 1 when filters or page size change.
  useEffect(() => { setPage(1); }, [searchQuery, statusFilter, dateFrom, dateTo, cashierFilter, pageSize]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [items, staffList] = await Promise.all([
        listSales(),
        listStaff().catch(() => [] as StaffMemberDto[]),
      ]);
      setSales(items);
      setStaff(staffList);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // ── Client-side filtering + sorting ────────────────────────────
  const filteredSales = useMemo(() => {
    const filtered = sales.filter((s) => {
      // Text search: match against sale ID, payment method, or user_id.
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const idMatch = s.id.toLowerCase().includes(q);
        const pmMatch = (s.paymentMethod ?? '').toLowerCase().includes(q);
        const uidMatch = (s.userId ?? '').toLowerCase().includes(q);
        if (!idMatch && !pmMatch && !uidMatch) return false;
      }

      // Status filter.
      if (statusFilter !== 'All' && s.status !== statusFilter) return false;

      // Cashier filter.
      if (cashierFilter && s.userId !== cashierFilter) return false;

      // Date range filter.
      if (dateFrom || dateTo) {
        const saleDate = new Date(s.createdAt);
        if (dateFrom) {
          const from = new Date(dateFrom);
          if (saleDate < from) return false;
        }
        if (dateTo) {
          const to = new Date(dateTo);
          to.setHours(23, 59, 59, 999);
          if (saleDate > to) return false;
        }
      }

      return true;
    });

    // Sort.
    filtered.sort((a, b) => {
      const dir = sortAsc ? 1 : -1;
      switch (sortKey) {
        case 'id':            return a.id.localeCompare(b.id) * dir;
        case 'createdAt':     return new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime() * dir;
        case 'total':         return (a.total.minor_units - b.total.minor_units) * dir;
        case 'lineCount':     return (a.lineCount - b.lineCount) * dir;
        case 'status':        return a.status.localeCompare(b.status) * dir;
        case 'paymentMethod': return (a.paymentMethod ?? '').localeCompare(b.paymentMethod ?? '') * dir;
        default:              return 0;
      }
    });

    return filtered;
  }, [sales, searchQuery, statusFilter, cashierFilter, dateFrom, dateTo, sortKey, sortAsc]);

  // ── Pagination slice ────────────────────────────────────────────
  const totalPages = Math.max(1, Math.ceil(filteredSales.length / pageSize));
  const safePage = Math.min(page, totalPages);
  const paginatedSales = useMemo(() => {
    const from = (safePage - 1) * pageSize;
    return filteredSales.slice(from, from + pageSize);
  }, [filteredSales, safePage, pageSize]);

  // ── Detail modal ───────────────────────────────────────────────
  const openDetail = useCallback(async (id: string) => {
    setDetailLoading(true);
    setRefunds([]);
    try {
      const [sale, refundData] = await Promise.all([
        getSale(id),
        listRefunds(id).catch(() => [] as RefundDto[]),
      ]);
      setDetail(sale);
      setRefunds(refundData);
    } catch {
      // IPC unavailable.
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const closeDetail = useCallback(() => {
    setDetail(null);
    setRefunds([]);
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

  // ── Refund handlers ──────────────────────────────────────────
  const openRefund = useCallback(() => {
    if (!detail) return;
    setRefundSaleId(detail.id);
  }, [detail]);

  const closeRefund = useCallback(() => {
    setRefundSaleId(null);
  }, []);

  const loadRefunds = useCallback(async (saleId: string) => {
    setRefundsLoading(true);
    try {
      const data = await listRefunds(saleId);
      setRefunds(data);
    } catch {
      setRefunds([]);
    } finally {
      setRefundsLoading(false);
    }
  }, []);

  const handleRefunded = useCallback(() => {
    closeRefund();
    if (detail) {
      loadRefunds(detail.id);
    }
    load();
  }, [closeRefund, detail, loadRefunds, load]);

  // ── Cashier display helper ─────────────────────────────────────
  const cashierName = useCallback((userId: string | null): string => {
    if (!userId) return '—';
    const s = staff.find((m) => m.id === userId);
    return s ? s.display_name : userId.slice(0, 8);
  }, [staff]);

  const handleExportCsv = useCallback(() => {
    const headers = ['Sale ID', 'Date', 'Total', 'Items', 'Status', 'Payment', 'Cashier'];
    // Export ALL filtered results, not just current page.
    const rows = filteredSales.map((s) => [
      s.id,
      new Date(s.createdAt).toLocaleString(),
      formatMoney(s.total),
      String(s.lineCount),
      s.status,
      s.paymentMethod ?? '',
      cashierName(s.userId),
    ]);
    const csv = [headers.join(','), ...rows.map((r) => r.map((c) => `"${c}"`).join(','))].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sales-export-${new Date().toISOString().split('T')[0]}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [filteredSales, cashierName]);

  return (
    <div className="sales-history">
      <div className="sales-history-header">
        <div className="sales-history-header-left">
          <Localized id="sales-history-title">
            <h1 className="sales-history-title">Sales History</h1>
          </Localized>
          {!loading && (
            <span className="sales-history-count">{filteredSales.length} sale{filteredSales.length !== 1 ? 's' : ''}</span>
          )}
          {!loading && filteredSales.length > pageSize && (
            <span className="sales-history-page-info">
              Page {safePage} of {totalPages}
            </span>
          )}
        </div>
        <div className="sales-history-header-actions">
          <button type="button" className="sales-history-export-btn" onClick={handleExportCsv}>
            Export CSV
          </button>
        </div>
      </div>

      {/* ── Filter bar ──────────────────────────────────────────── */}
      <div className="sales-history-filters" role="search" aria-label="Filter sales">
        {/* Search */}
        <div className="sales-history-filter-group">
          <label className="sales-history-filter-label" htmlFor="sh-search">Search</label>
          <input
            id="sh-search"
            type="text"
            className="sales-history-filter-input"
            placeholder="Search sale ID, payment, cashier…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search sales"
          />
        </div>

        {/* Status filter */}
        <div className="sales-history-filter-group">
          <label className="sales-history-filter-label">Status</label>
          <div className="sales-history-filter-chips" role="radiogroup" aria-label="Filter by status">
            {STATUS_OPTIONS.map((opt) => (
              <button
                key={opt}
                type="button"
                className={`sales-history-chip ${statusFilter === opt ? 'sales-history-chip--active' : ''}`}
                onClick={() => setStatusFilter(opt)}
                aria-pressed={statusFilter === opt}
              >
                {opt}
              </button>
            ))}
          </div>
        </div>

        {/* Date range */}
        <div className="sales-history-filter-group">
          <label className="sales-history-filter-label" htmlFor="sh-date-from">From</label>
          <input
            id="sh-date-from"
            type="date"
            className="sales-history-filter-date"
            value={dateFrom}
            onChange={(e) => setDateFrom(e.target.value)}
            aria-label="From date"
          />
        </div>
        <div className="sales-history-filter-group">
          <label className="sales-history-filter-label" htmlFor="sh-date-to">To</label>
          <input
            id="sh-date-to"
            type="date"
            className="sales-history-filter-date"
            value={dateTo}
            onChange={(e) => setDateTo(e.target.value)}
            aria-label="To date"
          />
        </div>

        {/* Cashier filter */}
        <div className="sales-history-filter-group">
          <label className="sales-history-filter-label" htmlFor="sh-cashier">Cashier</label>
          <select
            id="sh-cashier"
            className="sales-history-filter-select"
            value={cashierFilter}
            onChange={(e) => setCashierFilter(e.target.value)}
            aria-label="Filter by cashier"
          >
            <option value="">All Cashiers</option>
            {staff.map((m) => (
              <option key={m.id} value={m.id}>{m.display_name}</option>
            ))}
          </select>
        </div>
      </div>

      {/* ── Table ───────────────────────────────────────────────── */}
      {loading ? (
        <Localized id="sales-history-loading">
          <p className="sales-history-loading">Loading sales&hellip;</p>
        </Localized>
      ) : filteredSales.length === 0 ? (
        <Card shadow="sm">
          <div className="sales-history-empty">
            <Localized id="sales-history-empty">
              <p>{sales.length === 0 ? 'No sales recorded yet' : 'No sales match your filters'}</p>
            </Localized>
            {sales.length > 0 && (
              <button
                type="button"
                className="sales-history-clear-filters-btn"
                onClick={() => { setSearchQuery(''); setStatusFilter('All'); setDateFrom(''); setDateTo(''); setCashierFilter(''); }}
              >
                Clear filters
              </button>
            )}
          </div>
        </Card>
      ) : (
        <div className="sales-history-table-wrap">
          <table className="sales-history-table" aria-label="Sales history">
            <thead>
              <tr>
                <Localized id="sales-history-col-id">
                  <th className="sales-history-th" aria-sort={sortKey === 'id' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('id')}>
                      Sale ID
                      {sortKey === 'id' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <Localized id="sales-history-col-date">
                  <th className="sales-history-th" aria-sort={sortKey === 'createdAt' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('createdAt')}>
                      Date
                      {sortKey === 'createdAt' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <Localized id="sales-history-col-total">
                  <th className="sales-history-th" aria-sort={sortKey === 'total' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('total')}>
                      Total
                      {sortKey === 'total' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <Localized id="sales-history-col-items">
                  <th className="sales-history-th" aria-sort={sortKey === 'lineCount' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('lineCount')}>
                      Items
                      {sortKey === 'lineCount' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <Localized id="sales-history-col-status">
                  <th className="sales-history-th" aria-sort={sortKey === 'status' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('status')}>
                      Status
                      {sortKey === 'status' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <Localized id="sales-history-col-payment">
                  <th className="sales-history-th" aria-sort={sortKey === 'paymentMethod' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
                    <button type="button" className="sales-history-sort-btn" onClick={() => toggleSort('paymentMethod')}>
                      Payment
                      {sortKey === 'paymentMethod' && <span className="sales-history-sort-arrow" aria-hidden="true">{sortAsc ? ' \u25B2' : ' \u25BC'}</span>}
                    </button>
                  </th>
                </Localized>
                <th>Cashier</th>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {paginatedSales.map((s) => (
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
                  <td className="sales-history-cell-cashier">{cashierName(s.userId)}</td>
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

      {/* ── Pagination controls ────────────────────────────── */}
      {!loading && filteredSales.length > pageSize && (
        <nav className="sales-history-pagination" aria-label="Pagination">
          <button
            type="button"
            className="sales-history-page-btn"
            disabled={safePage <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            aria-label="Previous page"
          >
            &larr; Prev
          </button>
          <span className="sales-history-page-indicator">
            Page {safePage} of {totalPages}
          </span>
          <button
            type="button"
            className="sales-history-page-btn"
            disabled={safePage >= totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            aria-label="Next page"
          >
            Next &rarr;
          </button>
          <span className="sales-history-page-size-group">
            <label htmlFor="sh-page-size" className="sales-history-page-size-label">Per page</label>
            <select
              id="sh-page-size"
              className="sales-history-page-size-select"
              value={pageSize}
              onChange={(e) => setPageSize(Number(e.target.value))}
              aria-label="Results per page"
            >
              {PAGE_SIZE_OPTIONS.map((size) => (
                <option key={size} value={size}>{size}</option>
              ))}
            </select>
          </span>
        </nav>
      )}

      {/* ── Refund modal ────────────────────────────────────────── */}
      {detail && refundSaleId === detail.id && (
        <RefundModal
          open
          sale={detail}
          onClose={closeRefund}
          onRefunded={handleRefunded}
        />
      )}

      {/* ── Detail modal ────────────────────────────────────────── */}
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
                  <div><strong>Cashier:</strong> {cashierName(detail.userId)}</div>
                  <div>
                    <strong>Total:</strong> {formatMoney(detail.total)}
                    {refunds.length > 0 && (
                      <Badge variant="warning" style={{ marginLeft: 8 }}>
                        <Localized id="refund-title">
                          <span>Refunded</span>
                        </Localized>
                      </Badge>
                    )}
                  </div>
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
                    {detail.lines.map((line) => (
                      <tr key={line.id}>
                        <td>{line.sku}</td>
                        <td>{line.name}</td>
                        <td>{line.qty}</td>
                        <td>{formatMoney(line.unit_price)}</td>
                        <td>{formatMoney({ minor_units: line.total_minor, currency: line.unit_price.currency })}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                {/* ── Previous Refunds ──────────────────────── */}
                {refunds.length > 0 && (
                  <div className="sales-history-refunds">
                    <h3>
                      <Localized id="refund-previous-refunds">
                        <span>Previous Refunds</span>
                      </Localized>
                    </h3>
                    {refunds.map((rf) => (
                      <div key={rf.id} className="sales-history-refund-item">
                        <div className="sales-history-refund-meta">
                          <span>{new Date(rf.createdAt).toLocaleString()}</span>
                          <span className="sales-history-refund-total">{formatMoney(rf.total)}</span>
                        </div>
                        <div className="sales-history-refund-reason">{rf.reason}</div>
                        <table className="sales-history-lines-table" aria-label="Refund line items">
                          <thead>
                            <tr>
                              <th>SKU</th>
                              <th>Qty</th>
                              <th>Total</th>
                            </tr>
                          </thead>
                          <tbody>
                            {rf.lines.map((rfl) => (
                              <tr key={rfl.id}>
                                <td>{rfl.sku}</td>
                                <td>{rfl.qty}</td>
                                <td>{formatMoney(rfl.lineTotal)}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    ))}
                  </div>
                )}

                <div className="sales-history-modal-actions">
                  <Localized id="sales-history-detail-close">
                    <Button variant="ghost" onClick={closeDetail}>Close</Button>
                  </Localized>
                  {detail.status === 'Completed' && session && (
                    <Localized id="refund-action-refund">
                      <Button variant="secondary" onClick={openRefund}>Refund</Button>
                    </Localized>
                  )}
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
