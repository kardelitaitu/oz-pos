//! Shift Management Screen
//!
//! Displays the active shift status, a list of all shifts with
//! reconciliation details, and allows opening/closing shifts.

import { useState, useCallback, useEffect } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { formatMoney } from '@/types/domain';
import {
  listShifts,
  openShift,
  closeShift,
  getActiveShift,
  getShiftReport,
  createCashPayout,
  type ShiftDto,
  type ShiftReportDto,

} from '@/api/shifts';
import './ShiftManagementScreen.css';

// ── Format helper ───────────────────────────────────────────────────

const fmt = (minor: number, currency = 'USD') =>
  formatMoney({ minor_units: minor, currency });

// ── Component ───────────────────────────────────────────────────────

export default function ShiftManagementScreen() {
  const { session } = useAuth();
  const userId = session?.user_id ?? '';
  const [shifts, setShifts] = useState<ShiftDto[]>([]);
  const [activeShift, setActiveShift] = useState<ShiftDto | null>(null);
  const [loading, setLoading] = useState(true);
  const currency = 'USD';

  // ── Modals ────────────────────────────────────────────────────────
  const [showOpenModal, setShowOpenModal] = useState(false);
  const [showCloseModal, setShowCloseModal] = useState(false);
  const [showDetailModal, setShowDetailModal] = useState<ShiftDto | null>(null);
  const [shiftReport, setShiftReport] = useState<ShiftReportDto | null>(null);
  const [reportLoading, setReportLoading] = useState(false);
  const [openingBalance, setOpeningBalance] = useState('');
  const [closingBalance, setClosingBalance] = useState('');
  const [shiftNotes, setShiftNotes] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [closedShiftSummary, setClosedShiftSummary] = useState<ShiftDto | null>(null);

  // ── Payout modal state ─────────────────────────────
  const [showPayoutModal, setShowPayoutModal] = useState(false);
  const [payoutAmount, setPayoutAmount] = useState('');
  const [payoutReason, setPayoutReason] = useState('');

  // ── Load data ─────────────────────────────────────────────────────

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [allShifts, active] = await Promise.all([
        listShifts(),
        userId ? getActiveShift(userId) : Promise.resolve(null),
      ]);
      setShifts(allShifts);
      setActiveShift(active);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, [userId]);

  useEffect(() => { load(); }, [load]);

  // Load shift report when detail modal opens.
  useEffect(() => {
    if (!showDetailModal) {
      setShiftReport(null);
      return;
    }
    setReportLoading(true);
    getShiftReport(showDetailModal.id)
      .then(setShiftReport)
      .catch(() => setShiftReport(null))
      .finally(() => setReportLoading(false));
  }, [showDetailModal]);

  // ── Open shift ────────────────────────────────────────────────────

  const handleOpenShift = useCallback(async () => {
    const balance = parseInt(openingBalance, 10);
    const safeBalance = !Number.isNaN(balance) && balance >= 0 ? balance : 0;

    setSaving(true);
    setError(null);
    try {
      await openShift(userId, safeBalance);
      setShowOpenModal(false);
      setOpeningBalance('');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to open shift');
    } finally {
      setSaving(false);
    }
  }, [openingBalance, userId, load]);

  // ── Close shift ───────────────────────────────────────────────────

  const handleCloseShift = useCallback(async () => {
    if (!activeShift) return;
    const balance = parseInt(closingBalance, 10);
    if (Number.isNaN(balance) || balance < 0) return;

    setSaving(true);
    setError(null);
    try {
      const closed = await closeShift(
        activeShift.id,
        balance,
        shiftNotes.trim() || null,
      );
      setClosedShiftSummary(closed);
      setActiveShift(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to close shift');
    } finally {
      setSaving(false);
    }
  }, [activeShift, closingBalance, shiftNotes]);

  const dismissCloseSummary = useCallback(async () => {
    setClosedShiftSummary(null);
    setShowCloseModal(false);
    setClosingBalance('');
    setShiftNotes('');
    setError(null);
    await load();
  }, [load]);

  // ── Create payout ─────────────────────────────────────────────────

  const handleCreatePayout = useCallback(async () => {
    if (!activeShift) return;
    const amount = parseInt(payoutAmount, 10);
    if (Number.isNaN(amount) || amount <= 0) return;
    const reason = payoutReason.trim() || 'safe drop';

    setSaving(true);
    setError(null);
    try {
      await createCashPayout(activeShift.id, amount, reason);
      setShowPayoutModal(false);
      setPayoutAmount('');
      setPayoutReason('');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to record payout');
    } finally {
      setSaving(false);
    }
  }, [activeShift, payoutAmount, payoutReason, load]);

  // ── Reconciliation helper ─────────────────────────────────────────

  const _reconciliationBreakdown = (shift: ShiftDto) => {
    const opening = shift.openingBalanceMinor;
    const cashSales = shift.totalCashMinor;
    const payouts = shift.totalPayoutsMinor;
    const expected = opening + cashSales - payouts;
    const counted = shift.closingBalanceMinor ?? 0;
    const diff = counted - expected;
    return { opening, cashSales, payouts, expected, counted, diff };
  };

  // ── Format time/date helpers ───────────────────────────────────────

  const time = (iso: string) =>
    new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  const dateTime = (iso: string) =>
    new Date(iso).toLocaleString([], {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
    });

  // ── Render ────────────────────────────────────────────────────────

  return (
    <div className="shift-mgmt">
      <div className="shift-mgmt-header">
        <h1 className="shift-mgmt-title">Shift Management</h1>
      </div>

      {loading && (
        <p className="shift-mgmt-loading">Loading shifts…</p>
      )}

      {!loading && (
        <>
          {/* ── Active shift card ──────────────────────── */}
          {activeShift && !closedShiftSummary && (
            <Card shadow="md" className="shift-mgmt-active-card">
              <div className="shift-mgmt-active-header">
                <div className="shift-mgmt-active-info">
                  <span className="shift-mgmt-active-dot" />
                  <span className="shift-mgmt-active-label">Active Shift</span>
                </div>
                <div className="shift-mgmt-active-actions">
                  <button
                    type="button"
                    className="shift-mgmt-payout-btn"
                    onClick={() => {
                      setPayoutAmount('');
                      setPayoutReason('');
                      setError(null);
                      setShowPayoutModal(true);
                    }}
                    aria-label="Record cash payout"
                  >
                    Record Payout
                  </button>
                  <button
                    type="button"
                    className="shift-mgmt-close-btn"
                    onClick={() => {
                      setClosingBalance('');
                      setShiftNotes('');
                      setError(null);
                      setShowCloseModal(true);
                    }}
                    aria-label="Close active shift"
                  >
                    Close Shift
                  </button>
                </div>
              </div>
              <div className="shift-mgmt-active-details">
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Since</span>
                  <span className="shift-mgmt-stat-value">
                    {time(activeShift.openedAt)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Opening Balance</span>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.openingBalanceMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Sales</span>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalSalesMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Cash Sales</span>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalCashMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Card Sales</span>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalCardMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <span className="shift-mgmt-stat-label">Transactions</span>
                  <span className="shift-mgmt-stat-value">
                    {activeShift.totalSalesMinor > 0 ? 'Active' : 'None'}
                  </span>
                </div>
              </div>
            </Card>
          )}

          {/* ── No active shift banner ──────────────────── */}
          {!activeShift && !closedShiftSummary && (
            <Card shadow="sm" className="shift-mgmt-no-active">
              <div className="shift-mgmt-no-active-content">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="32" height="32" aria-hidden="true">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="12" y1="8" x2="12" y2="12" />
                  <line x1="12" y1="16" x2="12.01" y2="16" />
                </svg>
                <div>
                  <p className="shift-mgmt-no-active-title">No active shift</p>
                  <p className="shift-mgmt-no-active-sub">
                    Open a shift to start tracking cashier sessions.
                  </p>
                </div>
                <Button onClick={() => { setOpeningBalance(''); setError(null); setShowOpenModal(true); }}>
                  Open Shift
                </Button>
              </div>
            </Card>
          )}

          {/* ── Shifts table ────────────────────────────── */}
          <Card shadow="sm" className="shift-mgmt-table-card">
            <h2 className="shift-mgmt-table-title">Shift History</h2>

            {shifts.length === 0 ? (
              <div className="shift-mgmt-empty">
                <p>No shifts recorded yet.</p>
              </div>
            ) : (
              <div className="shift-mgmt-table-wrap">
                <table className="shift-mgmt-table" aria-label="Shift history">
                  <thead>
                    <tr>
                      <th>Status</th>
                      <th>Opened</th>
                      <th>Closed</th>
                      <th>Opening</th>
                      <th>Counted</th>
                      <th>Expected</th>
                      <th>Diff</th>
                      <th>Sales</th>
                      <th>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {shifts.map((s) => {
                      const diff = s.cashDifferenceMinor;
                      const diffClass =
                        diff !== null && diff < 0
                          ? 'shift-mgmt-diff--negative'
                          : diff !== null && diff > 0
                            ? 'shift-mgmt-diff--positive'
                            : '';
                      return (
                        <tr key={s.id} className={s.status === 'open' ? 'shift-mgmt-row--open' : ''}>
                          <td>
                            <span className={`shift-mgmt-status-badge shift-mgmt-status-badge--${s.status}`}>
                              {s.status === 'open' ? 'Open' : 'Closed'}
                            </span>
                          </td>
                          <td className="shift-mgmt-cell-date">{dateTime(s.openedAt)}</td>
                          <td className="shift-mgmt-cell-date">
                            {s.closedAt ? dateTime(s.closedAt) : '—'}
                          </td>
                          <td className="shift-mgmt-cell-mono">{fmt(s.openingBalanceMinor, currency)}</td>
                          <td className="shift-mgmt-cell-mono">
                            {s.closingBalanceMinor !== null ? fmt(s.closingBalanceMinor, currency) : '—'}
                          </td>
                          <td className="shift-mgmt-cell-mono">
                            {s.expectedCashMinor !== null ? fmt(s.expectedCashMinor, currency) : '—'}
                          </td>
                          <td className={`shift-mgmt-cell-mono ${diffClass}`}>
                            {diff !== null ? fmt(diff, currency) : '—'}
                            {diff !== null && diff !== 0 && (
                              <span className="shift-mgmt-tag">{diff > 0 ? 'Over' : 'Short'}</span>
                            )}
                          </td>
                          <td className="shift-mgmt-cell-mono">{fmt(s.totalSalesMinor, currency)}</td>
                          <td>
                            <button
                              type="button"
                              className="shift-mgmt-view-btn"
                              onClick={() => setShowDetailModal(s)}
                              aria-label={`View shift details`}
                            >
                              View
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </Card>
        </>
      )}

      {/* ── Open Shift Modal ──────────────────────────── */}
      {showOpenModal && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Open shift">
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <h2>Open Shift</h2>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowOpenModal(false)}
                aria-label="Close"
                disabled={saving}
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              {error && (
                <div className="shift-mgmt-modal-error" role="alert">{error}</div>
              )}
              <label className="shift-mgmt-field" htmlFor="open-balance">
                <span className="shift-mgmt-label">Opening balance (minor units)</span>
                <input
                  id="open-balance"
                  type="number"
                  className="shift-mgmt-input"
                  min="0"
                  placeholder="e.g. 500 for $5.00"
                  value={openingBalance}
                  onChange={(e) => setOpeningBalance(e.target.value)}
                  aria-label="Opening balance in minor units"
                  disabled={saving}
                />
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setShowOpenModal(false)} disabled={saving}>
                Cancel
              </Button>
              <Button variant="primary" onClick={handleOpenShift} loading={saving}>
                Open Shift
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Payout Modal ─────────────────────────────── */}
      {showPayoutModal && activeShift && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Record cash payout">
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <h2>Record Cash Payout</h2>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowPayoutModal(false)}
                aria-label="Close"
                disabled={saving}
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              {error && (
                <div className="shift-mgmt-modal-error" role="alert">{error}</div>
              )}
              <p className="shift-mgmt-payout-hint">
                Record cash removed from the drawer (safe drop, manager pickup, etc.).
              </p>
              <label className="shift-mgmt-field" htmlFor="payout-amount">
                <span className="shift-mgmt-label">Amount (minor units)</span>
                <input
                  id="payout-amount"
                  type="number"
                  className="shift-mgmt-input"
                  min="1"
                  placeholder="e.g. 20000 for $200.00"
                  value={payoutAmount}
                  onChange={(e) => setPayoutAmount(e.target.value)}
                  aria-label="Payout amount in minor units"
                  disabled={saving}
                />
              </label>
              <label className="shift-mgmt-field" htmlFor="payout-reason">
                <span className="shift-mgmt-label">Reason</span>
                <input
                  id="payout-reason"
                  type="text"
                  className="shift-mgmt-input"
                  placeholder="e.g. bank drop, manager pickup"
                  value={payoutReason}
                  onChange={(e) => setPayoutReason(e.target.value)}
                  aria-label="Payout reason"
                  disabled={saving}
                />
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setShowPayoutModal(false)} disabled={saving}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={handleCreatePayout}
                loading={saving}
                disabled={!payoutAmount || parseInt(payoutAmount, 10) <= 0 || Number.isNaN(parseInt(payoutAmount, 10))}
              >
                Record Payout
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift Modal ─────────────────────────── */}
      {showCloseModal && activeShift && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Close shift">
          <div className="shift-mgmt-modal shift-mgmt-modal--wide">
            <div className="shift-mgmt-modal-header">
              <h2>Close Shift</h2>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowCloseModal(false)}
                aria-label="Close"
                disabled={saving}
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              {error && (
                <div className="shift-mgmt-modal-error" role="alert">{error}</div>
              )}

              {/* Reconciliation breakdown */}
              <div className="shift-mgmt-recon">
                <h3 className="shift-mgmt-recon-title">Cash Reconciliation</h3>
                <div className="shift-mgmt-recon-row">
                  <span>Opening balance</span>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.openingBalanceMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <span>+ Cash sales</span>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <span>− Payouts</span>
                  <span className="shift-mgmt-cell-mono">−{fmt(activeShift.totalPayoutsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row shift-mgmt-recon-row--total">
                  <span>Expected cash</span>
                  <span className="shift-mgmt-cell-mono">
                    {fmt(activeShift.openingBalanceMinor + activeShift.totalCashMinor - activeShift.totalPayoutsMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <span>+ Payouts returned</span>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.totalPayoutsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-divider" />
                <div className="shift-mgmt-close-info-row">
                  <span>Shift opened</span>
                  <span>{dateTime(activeShift.openedAt)}</span>
                </div>
                <div className="shift-mgmt-close-info-row">
                  <span>Sales this shift</span>
                  <span>{fmt(activeShift.totalSalesMinor, currency)}</span>
                </div>
              </div>

              <label className="shift-mgmt-field" htmlFor="close-balance">
                <span className="shift-mgmt-label">Counted cash in drawer (minor units)</span>
                <input
                  id="close-balance"
                  type="number"
                  className="shift-mgmt-input"
                  min="0"
                  placeholder="e.g. 15000 for $150.00"
                  value={closingBalance}
                  onChange={(e) => setClosingBalance(e.target.value)}
                  aria-label="Closing balance in minor units"
                  disabled={saving}
                />
              </label>

              <label className="shift-mgmt-field" htmlFor="close-notes">
                <span className="shift-mgmt-label">Notes (optional)</span>
                <textarea
                  id="close-notes"
                  className="shift-mgmt-textarea"
                  rows={2}
                  placeholder="Any notes about this shift…"
                  value={shiftNotes}
                  onChange={(e) => setShiftNotes(e.target.value)}
                  aria-label="Shift notes"
                  disabled={saving}
                />
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setShowCloseModal(false)} disabled={saving}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={handleCloseShift}
                loading={saving}
                disabled={!closingBalance || parseInt(closingBalance, 10) < 0 || Number.isNaN(parseInt(closingBalance, 10))}
              >
                Close Shift
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Closed Shift Summary ──────────────────────── */}
      {closedShiftSummary && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Shift closed summary">
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <h2>Shift Closed</h2>
            </div>
            <div className="shift-mgmt-modal-body">
              <div className="shift-mgmt-summary-grid">
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Total Sales</span>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalSalesMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Cash Sales</span>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Card Sales</span>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalCardMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Expected Cash</span>
                  <span className="shift-mgmt-summary-value">
                    {closedShiftSummary.expectedCashMinor !== null ? fmt(closedShiftSummary.expectedCashMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Counted</span>
                  <span className="shift-mgmt-summary-value">
                    {closedShiftSummary.closingBalanceMinor !== null ? fmt(closedShiftSummary.closingBalanceMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <span className="shift-mgmt-summary-label">Difference</span>
                  <span className={`shift-mgmt-summary-value ${
                    closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor < 0
                      ? 'shift-mgmt-diff--negative'
                      : closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor > 0
                        ? 'shift-mgmt-diff--positive'
                        : ''
                  }`}>
                    {closedShiftSummary.cashDifferenceMinor !== null ? fmt(closedShiftSummary.cashDifferenceMinor, currency) : '—'}
                    {closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor !== 0 && (
                      <span className="shift-mgmt-tag">{closedShiftSummary.cashDifferenceMinor > 0 ? 'Over' : 'Short'}</span>
                    )}
                  </span>
                </div>
              </div>

              {closedShiftSummary.notes && (
                <div className="shift-mgmt-summary-notes">
                  <span className="shift-mgmt-summary-label">Notes</span>
                  <p>{closedShiftSummary.notes}</p>
                </div>
              )}
            </div>
            <div className="shift-mgmt-modal-actions">
              <Button variant="primary" onClick={dismissCloseSummary}>
                Done
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Shift Detail Modal ────────────────────────── */}
      {showDetailModal && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Shift details">
          <div className="shift-mgmt-modal shift-mgmt-modal--wide">
            <div className="shift-mgmt-modal-header">
              <h2>Shift Details</h2>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowDetailModal(null)}
                aria-label="Close"
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              <div className="shift-mgmt-detail-grid">
                <div className="shift-mgmt-detail-row">
                  <span>Status</span>
                  <span className={`shift-mgmt-status-badge shift-mgmt-status-badge--${showDetailModal.status}`}>
                    {showDetailModal.status === 'open' ? 'Open' : 'Closed'}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Opened</span>
                  <span>{dateTime(showDetailModal.openedAt)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Closed</span>
                  <span>{showDetailModal.closedAt ? dateTime(showDetailModal.closedAt) : '—'}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Opening Balance</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.openingBalanceMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Closing Balance</span>
                  <span className="shift-mgmt-cell-mono">
                    {showDetailModal.closingBalanceMinor !== null ? fmt(showDetailModal.closingBalanceMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Expected Cash</span>
                  <span className="shift-mgmt-cell-mono">
                    {showDetailModal.expectedCashMinor !== null ? fmt(showDetailModal.expectedCashMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Difference</span>
                  <span className={`shift-mgmt-cell-mono ${
                    showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor < 0
                      ? 'shift-mgmt-diff--negative'
                      : showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor > 0
                        ? 'shift-mgmt-diff--positive'
                        : ''
                  }`}>
                    {showDetailModal.cashDifferenceMinor !== null ? fmt(showDetailModal.cashDifferenceMinor, currency) : '—'}
                    {showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor !== 0 && (
                      <span className="shift-mgmt-tag">{showDetailModal.cashDifferenceMinor > 0 ? 'Over' : 'Short'}</span>
                    )}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Total Sales</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalSalesMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Cash Sales</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Card Sales</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalCardMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Other Sales</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalOtherMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Voids</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalVoidsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <span>Refunds</span>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalRefundsMinor, currency)}</span>
                </div>
              </div>

              {/* ── Shift Report sections ──────────────── */}
              {reportLoading && (
                <div className="shift-mgmt-report-loading">Loading report…</div>
              )}

              {shiftReport && !reportLoading && (
                <>
                  {/* Payment breakdown */}
                  {shiftReport.paymentBreakdown.length > 0 && (
                    <div className="shift-mgmt-report-section">
                      <h3 className="shift-mgmt-report-title">Payment Breakdown</h3>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <span>Method</span>
                          <span>Transactions</span>
                          <span>Total</span>
                        </div>
                        {shiftReport.paymentBreakdown.map((pmt) => (
                          <div key={pmt.method} className="shift-mgmt-report-row">
                            <span>{pmt.method.charAt(0).toUpperCase() + pmt.method.slice(1)}</span>
                            <span className="shift-mgmt-cell-mono">{pmt.count}</span>
                            <span className="shift-mgmt-cell-mono">{fmt(pmt.totalMinor, currency)}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Hourly breakdown */}
                  {shiftReport.hourlyBreakdown.length > 0 && (
                    <div className="shift-mgmt-report-section">
                      <h3 className="shift-mgmt-report-title">Hourly Sales</h3>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <span>Hour</span>
                          <span>Sales</span>
                          <span>Total</span>
                        </div>
                        {shiftReport.hourlyBreakdown.map((h) => (
                          <div key={h.hour} className="shift-mgmt-report-row">
                            <span>{String(h.hour).padStart(2, '0')}:00</span>
                            <span className="shift-mgmt-cell-mono">{h.saleCount}</span>
                            <span className="shift-mgmt-cell-mono">{fmt(h.totalMinor, currency)}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Counts summary */}
                  {(shiftReport.saleCount > 0 || shiftReport.voidCount > 0 || shiftReport.refundCount > 0) && (
                    <div className="shift-mgmt-report-section">
                      <h3 className="shift-mgmt-report-title">Transaction Summary</h3>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <span>Type</span>
                          <span>Count</span>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <span>Completed sales</span>
                          <span className="shift-mgmt-cell-mono">{shiftReport.saleCount}</span>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <span>Voids</span>
                          <span className="shift-mgmt-cell-mono">{shiftReport.voidCount}</span>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <span>Refunds</span>
                          <span className="shift-mgmt-cell-mono">{shiftReport.refundCount}</span>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Cash payouts */}
                  {shiftReport.cashPayouts.length > 0 && (
                    <div className="shift-mgmt-report-section">
                      <h3 className="shift-mgmt-report-title">Cash Payouts (Safe Drops)</h3>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <span>Amount</span>
                          <span>Reason</span>
                          <span>Time</span>
                        </div>
                        {shiftReport.cashPayouts.map((p) => (
                          <div key={p.id} className="shift-mgmt-report-row">
                            <span className="shift-mgmt-cell-mono">{fmt(p.amountMinor, currency)}</span>
                            <span>{p.reason}</span>
                            <span className="shift-mgmt-cell-date">{time(p.createdAt)}</span>
                          </div>
                        ))}
                        <div className="shift-mgmt-report-row shift-mgmt-report-row--total">
                          <span className="shift-mgmt-cell-mono">
                            <strong>Total: {fmt(
                              shiftReport.cashPayouts.reduce((s, p) => s + p.amountMinor, 0),
                              currency,
                            )}</strong>
                          </span>
                          <span />
                          <span />
                        </div>
                      </div>
                    </div>
                  )}
                </>
              )}

              {showDetailModal.notes && (
                <div className="shift-mgmt-summary-notes">
                  <span className="shift-mgmt-summary-label">Notes</span>
                  <p>{showDetailModal.notes}</p>
                </div>
              )}
            </div>
            <div className="shift-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setShowDetailModal(null)}>
                Close
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
