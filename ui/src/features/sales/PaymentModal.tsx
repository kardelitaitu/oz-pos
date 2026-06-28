import { useState, useMemo, useCallback, useEffect } from 'react';
import { startSale, addLine, completeSale, setCartDiscount, printSalesReceipt } from '@/api/pos';
import { Button } from '@/components/Button';
import { formatMoney, type Money, type CartLine } from '@/types/domain';
import './PaymentModal.css';

type PaymentMethod = 'cash' | 'card' | 'other';

export interface PaymentModalProps {
  open: boolean;
  lineItems: CartLine[];
  total: Money;
  discountPercent?: number;
  discountLabel?: string;
  /** User ID of the cashier processing this sale. */
  userId: string;
  onComplete: () => void;
  onClose: () => void;
}

/**
 * Payment modal — shown after the user clicks "Charge" in the POS.
 *
 * Lets the user choose a payment method, enter cash tendered (for cash
 * payments), see change due, and complete the sale. On completion it
 * prints a receipt via the Tauri IPC bridge and calls `onComplete`.
 */
export default function PaymentModal({
  open,
  lineItems,
  total,
  discountPercent = 0,
  discountLabel,
  userId,
  onComplete,
  onClose,
}: PaymentModalProps) {
  const [method, setMethod] = useState<PaymentMethod>('cash');
  const [otherLabel, setOtherLabel] = useState('');
  const [tendered, setTendered] = useState('');
  const [processing, setProcessing] = useState(false);
  const [done, setDone] = useState(false);
  const [changeDue, setChangeDue] = useState<Money | null>(null);

  // Reset state when modal opens/closes.
  useEffect(() => {
    if (open) {
      setMethod('cash');
      setOtherLabel('');
      setTendered('');
      setProcessing(false);
      setDone(false);
      setChangeDue(null);
    }
  }, [open]);

  const tenderedMinor = useMemo(() => {
    const num = parseFloat(tendered);
    if (Number.isNaN(num) || num < 0) return 0n;
    const known: Record<string, number> = {
      JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
      KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
    };
    const exp = known[total.currency] ?? 2;
    return BigInt(Math.round(num * 10 ** exp));
  }, [tendered, total.currency]);

  const { sufficient, change } = useMemo(() => {
    if (method !== 'cash') return { sufficient: true, change: null };
    const totalMin = BigInt(total.minor_units);
    if (tenderedMinor < totalMin) return { sufficient: false, change: null };
    const diff = Number(tenderedMinor - totalMin);
    return {
      sufficient: true,
      change: { minor_units: diff, currency: total.currency } as Money,
    };
  }, [method, total, tenderedMinor]);

  const canComplete = useMemo(() => {
    if (method === 'other' && !otherLabel.trim()) return false;
    if (method === 'cash') return sufficient;
    return true;
  }, [method, otherLabel, sufficient]);

  const complete = useCallback(async () => {
    setProcessing(true);

    const methodLabel = method === 'other' ? otherLabel.trim() || 'OTHER' : method.toUpperCase();

    try {
      // 1. Start a sale on the Rust backend — get a cart ID.
      const { cartId } = await startSale({ currency: total.currency });

      // 2. Apply discount to the backend cart (if any).
      if (discountPercent > 0) {
        const discountArgs: import('@/api/pos').SetCartDiscountArgs = { cartId, percent: discountPercent };
        if (discountLabel) discountArgs.label = discountLabel;
        await setCartDiscount(discountArgs);
      }

      // 3. Add each line item to the backend cart.
      for (const line of lineItems) {
        await addLine({
          cartId,
          sku: line.sku,
          qty: line.qty,
          unitPriceMinor: line.unit_price.minor_units,
        });
      }

      // 3. Complete the sale (consumes the cart, persists to DB, returns sale info).
      const saleResult = await completeSale({
        cartId,
        paymentMethod: methodLabel,
        tenderedMinor: method === 'cash' ? Number(tenderedMinor) : null,
        userId,
      });

      // 4. Print a receipt via IPC (non-blocking — printer may be absent).
      try {
        await printSalesReceipt({
          date: new Date().toLocaleDateString('en-US', {
            year: 'numeric', month: 'short', day: 'numeric',
          }),
          receiptNumber: `SALE-${saleResult.saleId}`,
          items: lineItems.map((line) => ({
            name: line.name ?? line.sku,
            quantity: line.qty,
            unitPrice: { minorUnits: line.unit_price.minor_units, currency: line.unit_price.currency },
            totalPrice: {
              minorUnits: line.unit_price.minor_units * line.qty,
              currency: line.unit_price.currency,
            },
          })),
          subtotal: { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          total: { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          payments: [
            {
              method: methodLabel,
              amount: { minorUnits: total.minor_units, currency: total.currency },
              change: change
                ? { minorUnits: change.minor_units, currency: change.currency }
                : null,
            },
          ],
        });
      } catch {
        // Printer may not be connected — still complete the sale.
      }

      if (change) setChangeDue(change);
      setDone(true);
    } catch (err) {
      // IPC sale failed — surface the error and let the user retry.
      console.error('Sale failed:', err);
      // Re-enable the Complete Sale button by resetting processing state.
    } finally {
      setProcessing(false);
    }
  }, [method, otherLabel, lineItems, total, discountPercent, discountLabel, change, userId]);

  // Auto-close after success.
  useEffect(() => {
    if (!done) return;
    const timer = setTimeout(() => {
      onComplete();
    }, changeDue ? 3000 : 1500);
    return () => clearTimeout(timer);
  }, [done, changeDue, onComplete]);

  if (!open) return null;

  return (
    <div className="payment-overlay" role="dialog" aria-modal="true" aria-label="Payment">
      <div className="payment-modal">
        {done ? (
          <div className="payment-done">
            <h2 className="payment-done-title">Sale Complete</h2>
            {changeDue && (
              <div className="payment-change">
                <span className="payment-change-label">Change due</span>
                <span className="payment-change-amount">
                  {formatMoney(changeDue)}
                </span>
              </div>
            )}
            <p className="payment-done-note">Receipt printed</p>
          </div>
        ) : (
          <>
            <div className="payment-header">
              <h2 className="payment-title">Complete Sale</h2>
              <button
                type="button"
                className="payment-close"
                onClick={onClose}
                aria-label="Cancel payment"
              >
                &times;
              </button>
            </div>

            {/* ── Total ───────────────────────────── */}
            <div className="payment-total-row">
              <span className="payment-total-label">Total Due</span>
              <span className="payment-total-amount">{formatMoney(total)}</span>
            </div>

            {/* ── Payment method ──────────────────── */}
            <fieldset className="payment-methods">
              <legend className="payment-section-title">Payment Method</legend>
              <div className="payment-method-options">
                {(['cash', 'card'] as const).map((m) => (
                  <label key={m} className="payment-method-label">
                    <input
                      type="radio"
                      name="payment-method"
                      value={m}
                      checked={method === m}
                      onChange={() => setMethod(m)}
                    />
                    <span className="payment-method-name">
                      {m === 'cash' ? 'Cash' : 'Card'}
                    </span>
                  </label>
                ))}
                <label className="payment-method-label">
                  <input
                    type="radio"
                    name="payment-method"
                    value="other"
                    checked={method === 'other'}
                    onChange={() => setMethod('other')}
                  />
                  <input
                    type="text"
                    className="payment-other-input"
                    placeholder="Other…"
                    value={otherLabel}
                    onChange={(e) => {
                      setMethod('other');
                      setOtherLabel(e.target.value);
                    }}
                    disabled={method !== 'other'}
                    aria-label="Other payment method name"
                  />
                </label>
              </div>
            </fieldset>

            {/* ── Cash tendered ───────────────────── */}
            {method === 'cash' && (
              <div className="payment-cash-section">
                <label className="payment-tendered-label">
                  <span>Amount Tendered</span>
                  <input
                    type="text"
                    className="payment-tendered-input"
                    inputMode="decimal"
                    placeholder="0.00"
                    value={tendered}
                    onChange={(e) => setTendered(e.target.value)}
                    aria-label="Amount tendered"
                  />
                </label>

                {/* Quick-cash amount buttons */}
                <div className="payment-quick-cash">
                  {[5, 10, 20, 50, 100].map((amount) => {
                    const totalNum = Number(total.minor_units) / 100;
                    const quickVal = Math.ceil(totalNum / amount) * amount;
                    return (
                      <button
                        key={amount}
                        type="button"
                        className="payment-quick-btn"
                        onClick={() => setTendered(quickVal.toFixed(2))}
                        aria-label={`Tender $${quickVal.toFixed(2)}`}
                      >
                        ${quickVal}
                      </button>
                    );
                  })}
                  <button
                    type="button"
                    className="payment-quick-btn"
                    onClick={() => setTendered((Number(total.minor_units) / 100).toFixed(2))}
                    aria-label="Tend exact amount"
                  >
                    Exact
                  </button>
                </div>

                {tendered.length > 0 && (
                  <div className="payment-change-preview">
                    <span className="payment-change-label">Change</span>
                    <span
                      className={`payment-change-amount ${!sufficient ? 'payment-change-insufficient' : ''}`}
                    >
                      {sufficient
                        ? formatMoney(change!)
                        : 'Insufficient amount'}
                    </span>
                  </div>
                )}
              </div>
            )}

            {/* ── Actions ─────────────────────────── */}
            <div className="payment-actions">
              <Button variant="ghost" onClick={onClose} disabled={processing}>
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={processing}
                disabled={!canComplete}
                onClick={complete}
              >
                Complete Sale
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
