//! Shift Management Screen
//!
//! Displays the active shift status, a list of all shifts with
//! reconciliation details, and allows opening/closing shifts.

import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
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
  const { l10n } = useLocalization();
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
        <Localized id="shift-title">
          <h1 className="shift-mgmt-title">Shift Management</h1>
        </Localized>
      </div>

      {loading && (
        <Localized id="shift-loading">
          <p className="shift-mgmt-loading">Loading shifts…</p>
        </Localized>
      )}

      {!loading && (
        <>
          {/* ── Active shift card ──────────────────────── */}
          {activeShift && !closedShiftSummary && (
            <Card shadow="md" className="shift-mgmt-active-card">
              <div className="shift-mgmt-active-header">
                <div className="shift-mgmt-active-info">
                  <span className="shift-mgmt-active-dot" />
                  <Localized id="shift-active-label">
                    <span className="shift-mgmt-active-label">Active Shift</span>
                  </Localized>
                </div>
                <div className="shift-mgmt-active-actions">
                  <Localized id="shift-btn-payout">
                    <button
                      type="button"
                      className="shift-mgmt-payout-btn"
                      onClick={() => {
                        setPayoutAmount('');
                        setPayoutReason('');
                        setError(null);
                        setShowPayoutModal(true);
                      }}
                      aria-label={l10n.getString('shift-btn-payout-label')}
                    >
                      Record Payout
                    </button>
                  </Localized>
                  <Localized id="shift-btn-close">
                    <button
                      type="button"
                      className="shift-mgmt-close-btn"
                      onClick={() => {
                        setClosingBalance('');
                        setShiftNotes('');
                        setError(null);
                        setShowCloseModal(true);
                      }}
                      aria-label={l10n.getString('shift-btn-close-label')}
                    >
                      Close Shift
                    </button>
                  </Localized>
                </div>
              </div>
              <div className="shift-mgmt-active-details">
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-stat-since">
                    <span className="shift-mgmt-stat-label">Since</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    {time(activeShift.openedAt)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-opening-balance">
                    <span className="shift-mgmt-stat-label">Opening Balance</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.openingBalanceMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-stat-sales">
                    <span className="shift-mgmt-stat-label">Sales</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalSalesMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-stat-cash-sales">
                    <span className="shift-mgmt-stat-label">Cash Sales</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalCashMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-stat-card-sales">
                    <span className="shift-mgmt-stat-label">Card Sales</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    {fmt(activeShift.totalCardMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-active-stat">
                  <Localized id="shift-stat-transactions">
                    <span className="shift-mgmt-stat-label">Transactions</span>
                  </Localized>
                  <span className="shift-mgmt-stat-value">
                    <Localized id={activeShift.totalSalesMinor > 0 ? 'shift-stat-active' : 'shift-stat-none'}>{activeShift.totalSalesMinor > 0 ? 'Active' : 'None'}</Localized>
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
                  <Localized id="shift-no-active-title">
                    <p className="shift-mgmt-no-active-title">No active shift</p>
                  </Localized>
                  <Localized id="shift-no-active-sub">
                    <p className="shift-mgmt-no-active-sub">
                      Open a shift to start tracking cashier sessions.
                    </p>
                  </Localized>
                </div>
                <Localized id="shift-btn-open">
                  <Button onClick={() => { setOpeningBalance(''); setError(null); setShowOpenModal(true); }}>
                    Open Shift
                  </Button>
                </Localized>
              </div>
            </Card>
          )}

          {/* ── Shifts table ────────────────────────────── */}
          <Card shadow="sm" className="shift-mgmt-table-card">
            <Localized id="shift-table-title">
              <h2 className="shift-mgmt-table-title">Shift History</h2>
            </Localized>

            {shifts.length === 0 ? (
              <div className="shift-mgmt-empty">
                <Localized id="shift-empty">
                  <p>No shifts recorded yet.</p>
                </Localized>
              </div>
            ) : (
              <div className="shift-mgmt-table-wrap">
                <table className="shift-mgmt-table" aria-label={l10n.getString('shift-table-label')}>
                  <thead>
                    <tr>
                      <Localized id="shift-table-status"><th>Status</th></Localized>
                      <Localized id="shift-table-opened"><th>Opened</th></Localized>
                      <Localized id="shift-table-closed"><th>Closed</th></Localized>
                      <Localized id="shift-table-opening"><th>Opening</th></Localized>
                      <Localized id="shift-table-counted"><th>Counted</th></Localized>
                      <Localized id="shift-table-expected"><th>Expected</th></Localized>
                      <Localized id="shift-table-diff"><th>Diff</th></Localized>
                      <Localized id="shift-table-sales"><th>Sales</th></Localized>
                      <Localized id="shift-table-actions"><th>Actions</th></Localized>
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
                              <Localized id={s.status === 'open' ? 'shift-status-open' : 'shift-status-closed'}>
                                <span>{s.status === 'open' ? 'Open' : 'Closed'}</span>
                              </Localized>
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
                              <span className="shift-mgmt-tag">
                                <Localized id={diff > 0 ? 'shift-tag-over' : 'shift-tag-short'}>
                                  <span>{diff > 0 ? 'Over' : 'Short'}</span>
                                </Localized>
                              </span>
                            )}
                          </td>
                          <td className="shift-mgmt-cell-mono">{fmt(s.totalSalesMinor, currency)}</td>
                          <td>
                            <Localized id="shift-btn-view">
                              <button
                                type="button"
                                className="shift-mgmt-view-btn"
                                onClick={() => setShowDetailModal(s)}
                                aria-label={l10n.getString('shift-btn-view')}
                              >
                                View
                              </button>
                            </Localized>
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
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('shift-modal-open-label')}>
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <Localized id="shift-modal-open-title">
                <h2>Open Shift</h2>
              </Localized>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowOpenModal(false)}
                aria-label={l10n.getString('close')}
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
                <Localized id="shift-open-balance-label">
                  <span className="shift-mgmt-label">Opening balance (minor units)</span>
                </Localized>
                <Localized id="shift-open-balance-placeholder" attrs={{ placeholder: true }}>
                  <input
                    id="open-balance"
                    type="number"
                    className="shift-mgmt-input"
                    min="0"
                    placeholder="e.g. 500 for $5.00"
                    value={openingBalance}
                    onChange={(e) => setOpeningBalance(e.target.value)}
                    aria-label={l10n.getString('shift-field-opening-balance')}
                    disabled={saving}
                  />
                </Localized>
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Localized id="shift-btn-cancel">
                <Button variant="ghost" onClick={() => setShowOpenModal(false)} disabled={saving}>
                  Cancel
                </Button>
              </Localized>
              <Localized id="shift-btn-open">
                <Button variant="primary" onClick={handleOpenShift} loading={saving}>
                  Open Shift
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}

      {/* ── Payout Modal ─────────────────────────────── */}
      {showPayoutModal && activeShift && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('shift-modal-payout-label')}>
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <Localized id="shift-modal-payout-title">
                <h2>Record Cash Payout</h2>
              </Localized>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowPayoutModal(false)}
                aria-label={l10n.getString('close')}
                disabled={saving}
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              {error && (
                <div className="shift-mgmt-modal-error" role="alert">{error}</div>
              )}
              <Localized id="shift-payout-hint">
                <p className="shift-mgmt-payout-hint">
                  Record cash removed from the drawer (safe drop, manager pickup, etc.).
                </p>
              </Localized>
              <label className="shift-mgmt-field" htmlFor="payout-amount">
                <Localized id="shift-payout-amount-label">
                  <span className="shift-mgmt-label">Amount (minor units)</span>
                </Localized>
                <Localized id="shift-payout-amount-placeholder" attrs={{ placeholder: true }}>
                  <input
                    id="payout-amount"
                    type="number"
                    className="shift-mgmt-input"
                    min="1"
                    placeholder="e.g. 20000 for $200.00"
                    value={payoutAmount}
                    onChange={(e) => setPayoutAmount(e.target.value)}
                    aria-label={l10n.getString('shift-field-payout-amount')}
                    disabled={saving}
                  />
                </Localized>
              </label>
              <label className="shift-mgmt-field" htmlFor="payout-reason">
                <Localized id="shift-payout-reason-label">
                  <span className="shift-mgmt-label">Reason</span>
                </Localized>
                <Localized id="shift-payout-reason-placeholder" attrs={{ placeholder: true }}>
                  <input
                    id="payout-reason"
                    type="text"
                    className="shift-mgmt-input"
                    placeholder="e.g. bank drop, manager pickup"
                    value={payoutReason}
                    onChange={(e) => setPayoutReason(e.target.value)}
                    aria-label={l10n.getString('shift-field-payout-reason')}
                    disabled={saving}
                  />
                </Localized>
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Localized id="shift-btn-cancel">
                <Button variant="ghost" onClick={() => setShowPayoutModal(false)} disabled={saving}>
                  Cancel
                </Button>
              </Localized>
              <Localized id="shift-btn-payout">
                <Button
                  variant="primary"
                  onClick={handleCreatePayout}
                  loading={saving}
                  disabled={!payoutAmount || parseInt(payoutAmount, 10) <= 0 || Number.isNaN(parseInt(payoutAmount, 10))}
                >
                  Record Payout
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift Modal ─────────────────────────── */}
      {showCloseModal && activeShift && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('shift-modal-close-label')}>
          <div className="shift-mgmt-modal shift-mgmt-modal--wide">
            <div className="shift-mgmt-modal-header">
              <Localized id="shift-modal-close-title">
                <h2>Close Shift</h2>
              </Localized>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowCloseModal(false)}
                aria-label={l10n.getString('close')}
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
                <Localized id="shift-recon-title">
                  <h3 className="shift-mgmt-recon-title">Cash Reconciliation</h3>
                </Localized>
                <div className="shift-mgmt-recon-row">
                  <Localized id="shift-recon-opening">
                    <span>Opening balance</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.openingBalanceMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <Localized id="shift-recon-cash-sales">
                    <span>+ Cash sales</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <Localized id="shift-recon-payouts">
                    <span>− Payouts</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">−{fmt(activeShift.totalPayoutsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-row shift-mgmt-recon-row--total">
                  <Localized id="shift-recon-expected">
                    <span>Expected cash</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">
                    {fmt(activeShift.openingBalanceMinor + activeShift.totalCashMinor - activeShift.totalPayoutsMinor, currency)}
                  </span>
                </div>
                <div className="shift-mgmt-recon-row">
                  <Localized id="shift-recon-payouts-returned">
                    <span>+ Payouts returned</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(activeShift.totalPayoutsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-recon-divider" />
                <div className="shift-mgmt-close-info-row">
                  <Localized id="shift-recon-opened">
                    <span>Shift opened</span>
                  </Localized>
                  <span>{dateTime(activeShift.openedAt)}</span>
                </div>
                <div className="shift-mgmt-close-info-row">
                  <Localized id="shift-recon-sales">
                    <span>Sales this shift</span>
                  </Localized>
                  <span>{fmt(activeShift.totalSalesMinor, currency)}</span>
                </div>
              </div>

              <label className="shift-mgmt-field" htmlFor="close-balance">
                <Localized id="shift-close-counted-label">
                  <span className="shift-mgmt-label">Counted cash in drawer (minor units)</span>
                </Localized>
                <Localized id="shift-close-counted-placeholder" attrs={{ placeholder: true }}>
                  <input
                    id="close-balance"
                    type="number"
                    className="shift-mgmt-input"
                    min="0"
                    placeholder="e.g. 15000 for $150.00"
                    value={closingBalance}
                    onChange={(e) => setClosingBalance(e.target.value)}
                    aria-label={l10n.getString('shift-field-closing-balance')}
                    disabled={saving}
                  />
                </Localized>
              </label>

              <label className="shift-mgmt-field" htmlFor="close-notes">
                <Localized id="shift-close-notes-label">
                  <span className="shift-mgmt-label">Notes (optional)</span>
                </Localized>
                <Localized id="shift-close-notes-placeholder" attrs={{ placeholder: true }}>
                  <textarea
                    id="close-notes"
                    className="shift-mgmt-textarea"
                    rows={2}
                    placeholder="Any notes about this shift…"
                    value={shiftNotes}
                    onChange={(e) => setShiftNotes(e.target.value)}
                    aria-label={l10n.getString('shift-field-notes')}
                    disabled={saving}
                  />
                </Localized>
              </label>
            </div>
            <div className="shift-mgmt-modal-actions">
              <Localized id="shift-btn-cancel">
                <Button variant="ghost" onClick={() => setShowCloseModal(false)} disabled={saving}>
                  Cancel
                </Button>
              </Localized>
              <Localized id="shift-btn-close">
                <Button
                  variant="primary"
                  onClick={handleCloseShift}
                  loading={saving}
                  disabled={!closingBalance || parseInt(closingBalance, 10) < 0 || Number.isNaN(parseInt(closingBalance, 10))}
                >
                  Close Shift
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}

      {/* ── Closed Shift Summary ──────────────────────── */}
      {closedShiftSummary && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('shift-modal-closed-label')}>
          <div className="shift-mgmt-modal">
            <div className="shift-mgmt-modal-header">
              <Localized id="shift-modal-closed-title">
                <h2>Shift Closed</h2>
              </Localized>
            </div>
            <div className="shift-mgmt-modal-body">
              <div className="shift-mgmt-summary-grid">
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-total-sales">
                    <span className="shift-mgmt-summary-label">Total Sales</span>
                  </Localized>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalSalesMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-cash-sales">
                    <span className="shift-mgmt-summary-label">Cash Sales</span>
                  </Localized>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-card-sales">
                    <span className="shift-mgmt-summary-label">Card Sales</span>
                  </Localized>
                  <span className="shift-mgmt-summary-value">{fmt(closedShiftSummary.totalCardMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-expected-cash">
                    <span className="shift-mgmt-summary-label">Expected Cash</span>
                  </Localized>
                  <span className="shift-mgmt-summary-value">
                    {closedShiftSummary.expectedCashMinor !== null ? fmt(closedShiftSummary.expectedCashMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-counted">
                    <span className="shift-mgmt-summary-label">Counted</span>
                  </Localized>
                  <span className="shift-mgmt-summary-value">
                    {closedShiftSummary.closingBalanceMinor !== null ? fmt(closedShiftSummary.closingBalanceMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-summary-item">
                  <Localized id="shift-summary-difference">
                    <span className="shift-mgmt-summary-label">Difference</span>
                  </Localized>
                  <span className={`shift-mgmt-summary-value ${
                    closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor < 0
                      ? 'shift-mgmt-diff--negative'
                      : closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor > 0
                        ? 'shift-mgmt-diff--positive'
                        : ''
                  }`}>
                    {closedShiftSummary.cashDifferenceMinor !== null ? fmt(closedShiftSummary.cashDifferenceMinor, currency) : '—'}
                    {closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor !== 0 && (
                      <span className="shift-mgmt-tag">
                        <Localized id={closedShiftSummary.cashDifferenceMinor > 0 ? 'shift-tag-over' : 'shift-tag-short'}>
                          <span>{closedShiftSummary.cashDifferenceMinor > 0 ? 'Over' : 'Short'}</span>
                        </Localized>
                      </span>
                    )}
                  </span>
                </div>
              </div>

              {closedShiftSummary.notes && (
                <div className="shift-mgmt-summary-notes">
                  <Localized id="shift-summary-notes">
                    <span className="shift-mgmt-summary-label">Notes</span>
                  </Localized>
                  <p>{closedShiftSummary.notes}</p>
                </div>
              )}
            </div>
            <div className="shift-mgmt-modal-actions">
              <Localized id="shift-btn-done">
                <Button variant="primary" onClick={dismissCloseSummary}>
                  Done
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}

      {/* ── Shift Detail Modal ────────────────────────── */}
      {showDetailModal && (
        <div className="shift-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('shift-modal-detail-label')}>
          <div className="shift-mgmt-modal shift-mgmt-modal--wide">
            <div className="shift-mgmt-modal-header">
              <Localized id="shift-modal-detail-title">
                <h2>Shift Details</h2>
              </Localized>
              <button
                type="button"
                className="shift-mgmt-modal-close"
                onClick={() => setShowDetailModal(null)}
                aria-label={l10n.getString('close')}
              >
                &times;
              </button>
            </div>
            <div className="shift-mgmt-modal-body">
              <div className="shift-mgmt-detail-grid">
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-status">
                    <span>Status</span>
                  </Localized>
                  <span className={`shift-mgmt-status-badge shift-mgmt-status-badge--${showDetailModal.status}`}>
                    <Localized id={showDetailModal.status === 'open' ? 'shift-status-open' : 'shift-status-closed'}>
                      <span>{showDetailModal.status === 'open' ? 'Open' : 'Closed'}</span>
                    </Localized>
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-opened">
                    <span>Opened</span>
                  </Localized>
                  <span>{dateTime(showDetailModal.openedAt)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-closed">
                    <span>Closed</span>
                  </Localized>
                  <span>{showDetailModal.closedAt ? dateTime(showDetailModal.closedAt) : '—'}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-opening-balance">
                    <span>Opening Balance</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.openingBalanceMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-closing-balance">
                    <span>Closing Balance</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">
                    {showDetailModal.closingBalanceMinor !== null ? fmt(showDetailModal.closingBalanceMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-expected-cash">
                    <span>Expected Cash</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">
                    {showDetailModal.expectedCashMinor !== null ? fmt(showDetailModal.expectedCashMinor, currency) : '—'}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-difference">
                    <span>Difference</span>
                  </Localized>
                  <span className={`shift-mgmt-cell-mono ${
                    showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor < 0
                      ? 'shift-mgmt-diff--negative'
                      : showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor > 0
                        ? 'shift-mgmt-diff--positive'
                        : ''
                  }`}>
                    {showDetailModal.cashDifferenceMinor !== null ? fmt(showDetailModal.cashDifferenceMinor, currency) : '—'}
                    {showDetailModal.cashDifferenceMinor !== null && showDetailModal.cashDifferenceMinor !== 0 && (
                      <span className="shift-mgmt-tag">
                        <Localized id={showDetailModal.cashDifferenceMinor > 0 ? 'shift-tag-over' : 'shift-tag-short'}>
                          <span>{showDetailModal.cashDifferenceMinor > 0 ? 'Over' : 'Short'}</span>
                        </Localized>
                      </span>
                    )}
                  </span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-total-sales">
                    <span>Total Sales</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalSalesMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-cash-sales">
                    <span>Cash Sales</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalCashMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-card-sales">
                    <span>Card Sales</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalCardMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-other-sales">
                    <span>Other Sales</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalOtherMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-voids">
                    <span>Voids</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalVoidsMinor, currency)}</span>
                </div>
                <div className="shift-mgmt-detail-row">
                  <Localized id="shift-detail-refunds">
                    <span>Refunds</span>
                  </Localized>
                  <span className="shift-mgmt-cell-mono">{fmt(showDetailModal.totalRefundsMinor, currency)}</span>
                </div>
              </div>

              {/* ── Shift Report sections ──────────────── */}
              {reportLoading && (
                <Localized id="shift-report-loading">
                  <div className="shift-mgmt-report-loading">Loading report…</div>
                </Localized>
              )}

              {shiftReport && !reportLoading && (
                <>
                  {/* Payment breakdown */}
                  {shiftReport.paymentBreakdown.length > 0 && (
                    <div className="shift-mgmt-report-section">
                      <Localized id="shift-report-payment-breakdown">
                        <h3 className="shift-mgmt-report-title">Payment Breakdown</h3>
                      </Localized>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <Localized id="shift-report-col-method">
                            <span>Method</span>
                          </Localized>
                          <Localized id="shift-report-col-transactions">
                            <span>Transactions</span>
                          </Localized>
                          <Localized id="shift-report-col-total">
                            <span>Total</span>
                          </Localized>
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
                      <Localized id="shift-report-hourly-sales">
                        <h3 className="shift-mgmt-report-title">Hourly Sales</h3>
                      </Localized>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <Localized id="shift-report-col-hour">
                            <span>Hour</span>
                          </Localized>
                          <Localized id="shift-report-col-sales">
                            <span>Sales</span>
                          </Localized>
                          <Localized id="shift-report-col-total">
                            <span>Total</span>
                          </Localized>
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
                      <Localized id="shift-report-transaction-summary">
                        <h3 className="shift-mgmt-report-title">Transaction Summary</h3>
                      </Localized>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <Localized id="shift-report-col-type">
                            <span>Type</span>
                          </Localized>
                          <Localized id="shift-report-col-count">
                            <span>Count</span>
                          </Localized>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <Localized id="shift-report-completed-sales">
                            <span>Completed sales</span>
                          </Localized>
                          <span className="shift-mgmt-cell-mono">{shiftReport.saleCount}</span>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <Localized id="shift-report-voids">
                            <span>Voids</span>
                          </Localized>
                          <span className="shift-mgmt-cell-mono">{shiftReport.voidCount}</span>
                        </div>
                        <div className="shift-mgmt-report-row">
                          <Localized id="shift-report-refunds">
                            <span>Refunds</span>
                          </Localized>
                          <span className="shift-mgmt-cell-mono">{shiftReport.refundCount}</span>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Cash payouts */}
                  {shiftReport.cashPayouts.length > 0 && (
                    <div className="shift-mgmt-report-section">
                      <Localized id="shift-report-cash-payouts">
                        <h3 className="shift-mgmt-report-title">Cash Payouts (Safe Drops)</h3>
                      </Localized>
                      <div className="shift-mgmt-report-table">
                        <div className="shift-mgmt-report-table-header">
                          <Localized id="shift-report-col-amount">
                            <span>Amount</span>
                          </Localized>
                          <Localized id="shift-report-col-reason">
                            <span>Reason</span>
                          </Localized>
                          <Localized id="shift-report-col-time">
                            <span>Time</span>
                          </Localized>
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
                            <Localized id="shift-report-total">
                              <strong>Total: </strong>
                            </Localized>
                            {fmt(
                              shiftReport.cashPayouts.reduce((s, p) => s + p.amountMinor, 0),
                              currency,
                            )}
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
                  <Localized id="shift-detail-notes">
                    <span className="shift-mgmt-summary-label">Notes</span>
                  </Localized>
                  <p>{showDetailModal.notes}</p>
                </div>
              )}
            </div>
            <div className="shift-mgmt-modal-actions">
              <Localized id="close">
                <Button variant="ghost" onClick={() => setShowDetailModal(null)}>
                  Close
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
