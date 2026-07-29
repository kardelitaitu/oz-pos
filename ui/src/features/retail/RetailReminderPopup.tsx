import { useState, useEffect } from 'react';
import { useLocalization } from '@fluent/react';

export interface RetailReminderPopupProps {
  /** Number of products below their low-stock threshold. */
  lowStockCount: number;
  /** Number of outstanding (unsettled) credit sales. */
  creditCount: number;
  /** Number of held (parked) carts available for resume. */
  heldCartCount: number;
  /** Called when the low-stock row is clicked. */
  onClickLowStock?: () => void;
  /** Called when the credit row is clicked. */
  onClickCredit?: () => void;
  /** Called when the held-carts row is clicked. */
  onClickHeldCarts?: () => void;
}

/** Floating corner popup showing low-stock and credit reminders.
 *  Dismissed once per session (resets on page reload). */
export default function RetailReminderPopup({ lowStockCount, creditCount, heldCartCount, onClickLowStock, onClickCredit, onClickHeldCarts }: RetailReminderPopupProps) {
  const { l10n } = useLocalization();
  const [dismissed, setDismissed] = useState(false);

  // Re-show if new reminders arrive after dismissal (count goes up)
  useEffect(() => {
    setDismissed(false);
  }, [lowStockCount, creditCount, heldCartCount]);

  const hasReminders = lowStockCount > 0 || creditCount > 0 || heldCartCount > 0;
  if (!hasReminders || dismissed) return null;

  return (
    <div className="retail-reminder-popup" role="status" aria-live="polite">
      {/* ── Low stock ──────────────────────────── */}
      {lowStockCount > 0 && (
        <button
          type="button"
          className="retail-reminder-row retail-reminder-row--low-stock"
          onClick={onClickLowStock}
          aria-label={`View ${lowStockCount} low-stock products`}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
            <path d="M10 2a1 1 0 011 1v8a1 1 0 11-2 0V3a1 1 0 011-1zM10 16a1 1 0 100-2 1 1 0 000 2z" />
          </svg>
          <span>{l10n.getString('retail-low-stock-banner', { count: lowStockCount }) || `${lowStockCount} product${lowStockCount > 1 ? 's' : ''} low on stock`}</span>
        </button>
      )}

      {/* ── Credit sales ───────────────────────── */}
      {creditCount > 0 && (
        <button
          type="button"
          className="retail-reminder-row retail-reminder-row--credit"
          onClick={onClickCredit}
          aria-label={`View ${creditCount} credit sales`}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
            <path d="M4 4a2 2 0 00-2 2v1h16V6a2 2 0 00-2-2H4z" />
            <path d="M18 9H2v5a2 2 0 002 2h12a2 2 0 002-2V9zM4 13a1 1 0 011-1h1a1 1 0 110 2H5a1 1 0 01-1-1zm5-1a1 1 0 100 2h1a1 1 0 100-2H9z" />
          </svg>
          <span>{l10n.getString('retail-credit-reminders', { count: creditCount }) || `Credit Reminders (${creditCount})`}</span>
        </button>
      )}

      {/* ── Held carts ─────────────────────────── */}
      {heldCartCount > 0 && (
        <button
          type="button"
          className="retail-reminder-row retail-reminder-row--held-cart"
          onClick={onClickHeldCarts}
          aria-label={`View ${heldCartCount} held carts`}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
            <path d="M5 3a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2V5a2 2 0 00-2-2H5zm0 1h2a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5a1 1 0 011-1zm8 0h2a1 1 0 011 1v2a1 1 0 01-1 1h-2a1 1 0 01-1-1V5a1 1 0 011-1zm-8 8a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2v-2a2 2 0 00-2-2H5zm0 1h2a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1v-2a1 1 0 011-1zm8 0h2a1 1 0 011 1v2a1 1 0 01-1 1h-2a1 1 0 01-1-1v-2a1 1 0 011-1z" />
          </svg>
          <span>{l10n.getString('retail-held-cart-reminders', { count: heldCartCount }) || `${heldCartCount} held cart${heldCartCount > 1 ? 's' : ''}`}</span>
        </button>
      )}

      {/* ── Dismiss ────────────────────────────── */}
      <button
        type="button"
        className="retail-reminder-dismiss"
        onClick={() => setDismissed(true)}
        aria-label={l10n.getString('retail-reminder-dismiss-aria') || 'Dismiss reminders'}
      >
        <svg viewBox="0 0 20 20" fill="currentColor" width="12" height="12" aria-hidden="true">
          <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
        </svg>
      </button>
    </div>
  );
}
