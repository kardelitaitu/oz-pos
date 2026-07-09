import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listSales,
  getSale,
  printSalesReceipt,
  listRefunds,
  voidSale,
  type SaleListItem,
  type SaleDetail,
  type RefundDto,
  type LineItemDto,
} from '@/api/sales';
import { listStaff, type StaffMemberDto } from '@/api/staff';
import { formatMoney } from '@/types/domain';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import { useAuth } from '@/contexts/AuthContext';
import { useSwipe } from '@/hooks/useSwipe';
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

// ── Swipeable order row ──────────────────────────────────────────────

interface SwipeableOrderRowProps {
  sale: SaleListItem;
  isManager: boolean;
  onView: (id: string) => void;
  onVoid: (sale: SaleListItem) => void;
  cashierName: string;
}

function SwipeableOrderRow({ sale, isManager, onView, onVoid, cashierName }: SwipeableOrderRowProps) {
  const { l10n } = useLocalization();
  const [revealed, setRevealed] = useState(false);
  const swipe = useSwipe({
    onSwipeLeft: () => { if (isManager) setRevealed(true); },
    onSwipeRight: () => setRevealed(false),
  });

  return (
    <tr
      className="sales-history-row-wrap"
      data-revealed={revealed ? 'true' : undefined}
      {...swipe}
    >
      <td className="sales-history-cell-id">{sale.id.slice(0, 8)}&hellip;</td>
      <td>{new Date(sale.createdAt).toLocaleString()}</td>
      <td className="sales-history-cell-total">{formatMoney(sale.total)}</td>
      <td>{sale.lineCount}</td>
      <td>
        <Badge variant={statusBadgeVariant(sale.status)}>
          <Localized id={statusFluentId(sale.status)}>
            <span>{sale.status}</span>
          </Localized>
        </Badge>
      </td>
      <td>{sale.paymentMethod ?? '\u2014'}</td>
      <td className="sales-history-cell-cashier">{cashierName}</td>
      <td className="sales-history-cell-actions">
        <div className="sales-history-cell-actions-inner">
          <Localized id="sales-history-action-view">
            <button
              type="button"
              className="sales-history-action-btn"
              onClick={() => onView(sale.id)}
              aria-label={`${l10n.getString('sales-history-view-aria', { id: sale.id })}`}
            >
              <span>View</span>
            </button>
          </Localized>
          {isManager && revealed && (
            <Localized id="sales-history-action-void">
              <button
                type="button"
                className="sales-history-void-btn"
                onClick={() => {
                  onVoid(sale);
                  setRevealed(false);
                }}
                aria-label={l10n.getString('sales-history-void-aria', { id: sale.id })}
              >
                <span>Void</span>
              </button>
            </Localized>
          )}
        </div>
      </td>
    </tr>
  );
}

export default function SalesHistoryScreen() {
  const { l10n } = useLocalization();
  const [sales, setSales] = useState<SaleListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [staff, setStaff] = useState<StaffMemberDto[]>([]);
  const [detail, setDetail] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [printing, setPrinting] = useState(false);
  const [refundSaleId, setRefundSaleId] = useState<string | null>(null);
  const [refunds, setRefunds] = useState<RefundDto[]>([]);
  const [_refundsLoading, setRefundsLoading] = useState(false);
  const { session, isManager } = useAuth();

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

  // ── Void state ──────────────────────────────────────────────────────
  const [voidTarget, setVoidTarget] = useState<SaleListItem | null>(null);
  const [voidReason, setVoidReason] = useState('');
  const [voiding, setVoiding] = useState(false);
  const [voidError, setVoidError] = useState<string | null>(null);

  const handleOpenVoid = useCallback((sale: SaleListItem) => {
    setVoidTarget(sale);
    setVoidReason('');
    setVoidError(null);
  }, []);

  const handleCloseVoid = useCallback(() => {
    setVoidTarget(null);
    setVoidReason('');
    setVoidError(null);
  }, []);

  const handleConfirmVoid = useCallback(async () => {
    if (!voidTarget) return;
    setVoiding(true);
    setVoidError(null);
    try {
      await voidSale({
        saleId: voidTarget.id,
        userId: session?.user_id ?? 'unknown',
        reason: voidReason || l10n.getString('sales-history-void-default-reason'),
      });
      setVoidTarget(null);
      setVoidReason('');
      load();
    } catch (err) {
      setVoidError(err instanceof Error ? err.message : l10n.getString('sales-history-void-error'));
    } finally {
      setVoiding(false);
    }
  }, [voidTarget, voidReason, session, load, l10n]);

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
        case 'createdAt':     return (new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()) * dir;
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
      await printSalesReceipt({
        date: detail.createdAt,
        receiptNumber: detail.id,
        items: detail.lines.map((l): LineItemDto => {
          const item: LineItemDto = {
            name: l.name,
            quantity: l.qty,
            unitPrice: { minorUnits: l.unit_price.minor_units, currency: l.unit_price.currency },
            totalPrice: { minorUnits: l.total_minor, currency: l.unit_price.currency },
          };
          if (l.tax_amount) {
            item.taxAmount = { minorUnits: l.tax_amount.minor_units, currency: l.tax_amount.currency };
          }
          return item;
        }),
        subtotal: { minorUnits: detail.subtotal.minor_units, currency: detail.total.currency },
        ...(detail.taxTotal.minor_units > 0
          ? { tax: { minorUnits: detail.taxTotal.minor_units, currency: detail.total.currency } }
          : {}),
        total: { minorUnits: detail.total.minor_units, currency: detail.total.currency },
        payments: [
          {
            method: detail.paymentMethod ?? l10n.getString('sales-history-export-payment'),
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
  }, [detail, l10n]);

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
    const headers = [
      l10n.getString('sales-history-export-id'),
      l10n.getString('sales-history-export-date'),
      l10n.getString('sales-history-export-total'),
      l10n.getString('sales-history-export-items'),
      l10n.getString('sales-history-export-status'),
      l10n.getString('sales-history-export-payment'),
      l10n.getString('sales-history-export-cashier'),
    ];
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
  }, [filteredSales, cashierName, l10n]);

  return (
    <div className="sales-history">
      <div className="sales-history-header">
        <div className="sales-history-header-left">
          <Localized id="sales-history-title">
            <h1 className="sales-history-title">Sales History</h1>
          </Localized>
          {!loading && (
            <span className="sales-history-count">
              <Localized id="sales-history-count" vars={{ count: filteredSales.length }}>
                <>{filteredSales.length} sale{filteredSales.length !== 1 ? 's' : ''}</>
              </Localized>
            </span>
          )}
          {!loading && filteredSales.length > pageSize && (
            <span className="sales-history-page-info">
              <Localized id="sales-history-page-info" vars={{ current: safePage, total: totalPages }}>
                <span>Page {safePage} of {totalPages}</span>
              </Localized>
            </span>
          )}
        </div>
        <div className="sales-history-header-actions">
          <Localized id="sales-history-export-csv">
            <button type="button" className="sales-history-export-btn" onClick={handleExportCsv}>
              <span>Export CSV</span>
            </button>
          </Localized>
        </div>
      </div>

      {/* ── Filter bar ──────────────────────────────────────────── */}
      <Localized id="sales-history-filter-aria" attrs={{ 'aria-label': true }}>
        <div className="sales-history-filters" role="search" aria-label="Filter sales">
        {/* Search */}
        <div className="sales-history-filter-group">
          <Localized id="sales-history-search-label">
            <label className="sales-history-filter-label" htmlFor="sh-search"><span>Search</span></label>
          </Localized>
          <Localized id="sales-history-search-placeholder" attrs={{ placeholder: true }}>
            <Localized id="sales-history-search-aria" attrs={{ 'aria-label': true }}>
              <input
                id="sh-search"
                type="text"
                className="sales-history-filter-input"
                placeholder="Search sale ID, payment, cashier…"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                aria-label="Search sales"
              />
            </Localized>
          </Localized>
        </div>

        {/* Status filter */}
        <div className="sales-history-filter-group">
          <Localized id="sales-history-status-label">
            <span className="sales-history-filter-label"><span>Status</span></span>
          </Localized>
          <Localized id="sales-history-status-filter-aria" attrs={{ 'aria-label': true }}>
            <div className="sales-history-filter-chips" role="radiogroup" aria-label="Filter by status">
              {STATUS_OPTIONS.map((opt) => {
                const statusIds: Record<string, string> = {
                  'All': 'sales-history-status-all',
                  'Completed': 'sales-history-status-completed',
                  'Pending': 'sales-history-status-pending',
                  'Voided': 'sales-history-status-voided',
                };
                return (
                  <Localized id={statusIds[opt] ?? opt} key={opt}>
                    <button
                      type="button"
                      role="radio"
                      className={`sales-history-chip ${statusFilter === opt ? 'sales-history-chip--active' : ''}`}
                      onClick={() => setStatusFilter(opt)}
                      aria-checked={statusFilter === opt}
                    >
                      <span>{opt}</span>
                    </button>
                  </Localized>
                );
              })}
            </div>
          </Localized>
        </div>

        {/* Date range */}
        <div className="sales-history-filter-group">
          <Localized id="sales-history-from-label">
            <label className="sales-history-filter-label" htmlFor="sh-date-from"><span>From</span></label>
          </Localized>
          <Localized id="sales-history-date-from-aria" attrs={{ 'aria-label': true }}>
            <input
              id="sh-date-from"
              type="date"
              className="sales-history-filter-date"
              value={dateFrom}
              onChange={(e) => setDateFrom(e.target.value)}
              aria-label="From date"
            />
          </Localized>
        </div>
        <div className="sales-history-filter-group">
          <Localized id="sales-history-to-label">
            <label className="sales-history-filter-label" htmlFor="sh-date-to"><span>To</span></label>
          </Localized>
          <Localized id="sales-history-date-to-aria" attrs={{ 'aria-label': true }}>
            <input
              id="sh-date-to"
              type="date"
              className="sales-history-filter-date"
              value={dateTo}
              onChange={(e) => setDateTo(e.target.value)}
              aria-label="To date"
            />
          </Localized>
        </div>

        {/* Cashier filter */}
        <div className="sales-history-filter-group">
          <Localized id="sales-history-cashier-label">
            <label className="sales-history-filter-label" htmlFor="sh-cashier"><span>Cashier</span></label>
          </Localized>
          <Localized id="sales-history-cashier-aria" attrs={{ 'aria-label': true }}>
            <select
              id="sh-cashier"
              className="sales-history-filter-select"
              value={cashierFilter}
              onChange={(e) => setCashierFilter(e.target.value)}
              aria-label="Filter by cashier"
            >
            <Localized id="sales-history-cashier-all">
              <option value=""><span>All Cashiers</span></option>
            </Localized>
            {staff.map((m) => (
              <option key={m.id} value={m.id}>{m.display_name}</option>
            ))}
          </select>
          </Localized>
        </div>
      </div>
      </Localized>

      {/* ── Table ───────────────────────────────────────────────── */}
      {loading ? (
        <Localized id="sales-history-loading">
          <p className="sales-history-loading">Loading sales&hellip;</p>
        </Localized>
      ) : filteredSales.length === 0 ? (
        <Card shadow="sm">
          <div className="sales-history-empty">
            {sales.length === 0 ? (
              <Localized id="sales-history-empty">
                <p>No sales recorded yet</p>
              </Localized>
            ) : (
              <Localized id="sales-history-empty-filtered">
                <p>No sales match your filters</p>
              </Localized>
            )}
            {sales.length > 0 && (
              <Localized id="sales-history-clear-filters">
                <button
                  type="button"
                  className="sales-history-clear-filters-btn"
                  onClick={() => { setSearchQuery(''); setStatusFilter('All'); setDateFrom(''); setDateTo(''); setCashierFilter(''); }}
                >
                  <span>Clear filters</span>
                </button>
              </Localized>
            )}
          </div>
        </Card>
      ) : (
        <div className="sales-history-table-wrap">
          <Localized id="sales-history-table-aria" attrs={{ 'aria-label': true }}>
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
                <Localized id="sales-history-col-cashier">
                  <th><span>Cashier</span></th>
                </Localized>
                <Localized id="sales-history-actions-aria" attrs={{ 'aria-label': true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>
              {paginatedSales.map((s) => (
                <SwipeableOrderRow
                  key={s.id}
                  sale={s}
                  isManager={isManager}
                  onView={openDetail}
                  onVoid={handleOpenVoid}
                  cashierName={cashierName(s.userId)}
                />
              ))}
            </tbody>
            </table>
          </Localized>
        </div>
      )}

      {/* ── Pagination controls ────────────────────────────── */}
      {!loading && filteredSales.length > pageSize && (
        <Localized id="sales-history-pagination-aria" attrs={{ 'aria-label': true }}>
        <nav className="sales-history-pagination" aria-label="Pagination">
          <Localized id="sales-history-prev-aria" attrs={{ 'aria-label': true }}>
          <Localized id="sales-history-prev-page">
            <button
              type="button"
              className="sales-history-page-btn"
              disabled={safePage <= 1}
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              aria-label="Previous page"
            >
              <span>&larr; Prev</span>
            </button>
          </Localized>
          </Localized>
          <span className="sales-history-page-indicator">
            <Localized id="sales-history-page-info" vars={{ current: safePage, total: totalPages }}>
              <span>Page {safePage} of {totalPages}</span>
            </Localized>
          </span>
          <Localized id="sales-history-next-aria" attrs={{ 'aria-label': true }}>
          <Localized id="sales-history-next-page">
            <button
              type="button"
              className="sales-history-page-btn"
              disabled={safePage >= totalPages}
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              aria-label="Next page"
            >
              <span>Next &rarr;</span>
            </button>
          </Localized>
          </Localized>
          <span className="sales-history-page-size-group">
            <Localized id="sales-history-per-page-label">
              <label htmlFor="sh-page-size" className="sales-history-page-size-label"><span>Per page</span></label>
            </Localized>
            <Localized id="sales-history-per-page-aria" attrs={{ 'aria-label': true }}>
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
            </Localized>
          </span>
        </nav>
        </Localized>
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

      {/* ── Void Confirmation Modal ──────────────────────────── */}
      {voidTarget && (
        <Localized id="sales-history-void-overlay-aria" attrs={{ 'aria-label': true }}>
        <div className="sales-history-overlay" role="dialog" aria-modal="true" aria-label="Void order">
          <div className="sales-history-modal sales-history-void-modal">
            <div className="sales-history-modal-header">
              <Localized id="sales-history-void-title">
                <h2><span>Void Order</span></h2>
              </Localized>
              <Localized id="sales-history-void-close-aria" attrs={{ 'aria-label': true }}>
                <button
                  type="button"
                  className="sales-history-modal-close"
                  onClick={handleCloseVoid}
                  aria-label="Close void dialog"
                >
                  &times;
                </button>
              </Localized>
            </div>
            <div className="sales-history-modal-body">
              <Localized id="sales-history-void-desc" vars={{ id: voidTarget.id.slice(0, 8), amount: formatMoney(voidTarget.total) }}>
                <p className="sales-history-void-desc">
                  <span>This will cancel order <strong>{voidTarget.id.slice(0, 8)}</strong>
                  {' '}for {formatMoney(voidTarget.total)} and restore inventory.
                  This action cannot be undone.</span>
                </p>
              </Localized>

              <div className="sales-history-void-field">
                <Localized id="sales-history-void-reason-label">
                  <label htmlFor="sh-void-reason" className="sales-history-void-label">
                    <span>Reason for void</span>
                  </label>
                </Localized>
                <Localized id="sales-history-void-reason-aria" attrs={{ 'aria-label': true }}>
                <Localized id="sales-history-void-reason-placeholder" attrs={{ placeholder: true }}>
                  <input
                    id="sh-void-reason"
                    type="text"
                    className="sales-history-void-input"
                    placeholder="e.g. Customer cancellation"
                    value={voidReason}
                    onChange={(e) => setVoidReason(e.target.value)}
                    aria-label="Void reason"
                  />
                </Localized>
                </Localized>
              </div>

              {voidError && (
                <div className="sales-history-void-error" role="alert">
                  {voidError}
                </div>
              )}

              <div className="sales-history-modal-actions">
                <Localized id="sales-history-void-cancel">
                  <Button variant="ghost" onClick={handleCloseVoid} disabled={voiding}>
                    <span>Cancel</span>
                  </Button>
                </Localized>
                <Localized id={voiding ? 'sales-history-void-progress' : 'sales-history-void-confirm'}>
                  <Button
                    variant="danger"
                    onClick={handleConfirmVoid}
                    loading={voiding}
                  >
                    <span>{voiding ? 'Voiding…' : 'Confirm Void'}</span>
                  </Button>
                </Localized>
              </div>
            </div>
          </div>
        </div>
        </Localized>
      )}

      {/* ── Detail modal ────────────────────────────────────────── */}
      {detail && (
        <Localized id="sales-history-detail-overlay-aria" attrs={{ 'aria-label': true }}>
        <div className="sales-history-overlay" role="dialog" aria-modal="true" aria-label="Sale detail">
          <div className="sales-history-modal">
            <div className="sales-history-modal-header">
              <Localized id="sales-history-detail-title">
                <h2>Sale Detail</h2>
              </Localized>
              <Localized id="sales-history-detail-close-aria" attrs={{ 'aria-label': true }}>
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
              </Localized>
            </div>            {detailLoading ? (
              <Localized id="sales-history-detail-loading">
                <p><span>Loading&hellip;</span></p>
              </Localized>
            ) : (
              <div className="sales-history-modal-body">
                <div className="sales-history-detail-meta">
                  <div>
                    <Localized id="sales-history-detail-id">
                      <strong><span>ID:</span></strong>
                    </Localized>
                    {' '}{detail.id}
                  </div>
                  <div>
                    <Localized id="sales-history-detail-date">
                      <strong><span>Date:</span></strong>
                    </Localized>
                    {' '}{new Date(detail.createdAt).toLocaleString()}
                  </div>
                  <div>
                    <Localized id="sales-history-detail-status">
                      <strong><span>Status:</span></strong>
                    </Localized>
                    {' '}
                    <Badge variant={statusBadgeVariant(detail.status)}>
                      <Localized id={statusFluentId(detail.status)}>
                        <span>{detail.status}</span>
                      </Localized>
                    </Badge>
                  </div>
                  <div>
                    <Localized id="sales-history-detail-payment">
                      <strong><span>Payment:</span></strong>
                    </Localized>
                    {' '}{detail.paymentMethod ?? '\u2014'}
                  </div>
                  <div>
                    <Localized id="sales-history-detail-cashier">
                      <strong><span>Cashier:</span></strong>
                    </Localized>
                    {' '}{cashierName(detail.userId)}
                  </div>
                  <div>
                    <Localized id="sales-history-detail-subtotal">
                      <strong><span>Subtotal:</span></strong>
                    </Localized>
                    {' '}{formatMoney(detail.subtotal)}
                  </div>
                  {detail.taxTotal.minor_units > 0 && (
                    <div>
                      <Localized id="sales-history-detail-tax">
                        <strong><span>Tax:</span></strong>
                      </Localized>
                      {' '}{formatMoney(detail.taxTotal)}
                    </div>
                  )}
                  <div>
                    <Localized id="sales-history-detail-total">
                      <strong><span>Total:</span></strong>
                    </Localized>
                    {' '}{formatMoney(detail.total)}
                    {refunds.length > 0 && (
                      <Badge variant="warning" style={{ marginLeft: 8 }}>
                        <Localized id="refund-title">
                          <span>Refunded</span>
                        </Localized>
                      </Badge>
                    )}
                  </div>
                </div>

                <Localized id="sales-history-lines-title">
                  <h3><span>Line Items</span></h3>
                </Localized>
                <Localized id="sales-history-lines-aria" attrs={{ 'aria-label': true }}>
                <table className="sales-history-lines-table" aria-label="Sale line items">
                  <thead>
                    <tr>
                      <Localized id="sales-history-line-sku"><th><span>SKU</span></th></Localized>
                      <Localized id="sales-history-line-name"><th><span>Name</span></th></Localized>
                      <Localized id="sales-history-line-qty"><th><span>Qty</span></th></Localized>
                      <Localized id="sales-history-line-unit-price"><th><span>Unit Price</span></th></Localized>
                      <Localized id="sales-history-line-total"><th><span>Total</span></th></Localized>
                      {detail.lines.some((l) => l.tax_amount) && (
                        <Localized id="sales-history-line-tax"><th><span>Tax</span></th></Localized>
                      )}
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
                        <td>{line.tax_amount ? formatMoney(line.tax_amount) : '\u2014'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                </Localized>

                {/* ── Previous Refunds ──────────────────────── */}
                {refunds.length > 0 && (
                  <div className="sales-history-refunds">
                    <Localized id="refund-previous-refunds">
                      <h3><span>Previous Refunds</span></h3>
                    </Localized>
                    {refunds.map((rf) => (
                      <div key={rf.id} className="sales-history-refund-item">
                        <div className="sales-history-refund-meta">
                          <span>{new Date(rf.createdAt).toLocaleString()}</span>
                          <span className="sales-history-refund-total">{formatMoney(rf.total)}</span>
                        </div>
                        <div className="sales-history-refund-reason">{rf.reason}</div>
                        <Localized id="sales-history-refund-lines-aria" attrs={{ 'aria-label': true }}>
                        <table className="sales-history-lines-table" aria-label="Refund line items">
                          <thead>
                            <tr>
                              <Localized id="refund-line-sku"><th><span>SKU</span></th></Localized>
                              <Localized id="refund-line-qty"><th><span>Qty</span></th></Localized>
                              <Localized id="refund-line-total"><th><span>Total</span></th></Localized>
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
                        </Localized>
                      </div>
                    ))}
                  </div>
                )
}

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
        </Localized>
      )}
    </div>
  );
}
