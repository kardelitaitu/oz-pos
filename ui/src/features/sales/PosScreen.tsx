import { useCallback, useState, useEffect } from 'react';
import { Localized } from '@/components/Localized';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import { formatMoney, type LineId, type Product, type Sku } from '@/types/domain';
import {
  lookupByBarcode,
  holdCart,
  listHeldCarts,
  getHeldCart,
  deleteHeldCart,
  type BarcodeScannedPayload,
  type HeldCartRow,
} from '@/api/pos';
import { usePosState } from './usePosState';
import { useBarcodeScanner } from './useBarcodeScanner';
import PaymentModal from './PaymentModal';

import './PosScreen.css';

/** Trash icon SVG */
function TrashIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

/** Minus icon SVG */
function MinusIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}

/** Plus icon SVG */
function PlusIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}

/**
 * POS sales screen — product lookup on the left, cart panel on the right.
 *
 * The left panel shows the ProductLookupScreen (search, barcode, category
 * filters, product grid). Clicking a product adds it to the cart.
 *
 * The right panel shows the current cart with line items, quantity
 * controls, remove buttons, subtotal, discount controls, and a Pay button.
 */
export default function PosScreen() {
  const {
    lines,
    subtotal,
    total,
    discountPercent,
    discountLabel,
    discountAmount,
    addProduct,
    removeLine,
    updateQty,
    setDiscount,
    resetCart,
    setLines,
  } = usePosState();
  const [showPayment, setShowPayment] = useState(false);
  const [showDiscountInput, setShowDiscountInput] = useState(false);
  const [discountInput, setDiscountInput] = useState('');
  const [discountName, setDiscountName] = useState('');

  // ── Barcode scanner integration ─────────────────────────────
  useBarcodeScanner({
    onProductFound: useCallback(async (payload: BarcodeScannedPayload) => {
      try {
        const dto = await lookupByBarcode(payload.code);
        if (dto) {
          const product: Product = {
            sku: dto.sku as Sku,
            name: dto.name,
            category: dto.category ?? 'Uncategorised',
            price: { minor_units: dto.price.minor_units, currency: dto.price.currency },
            barcode: dto.barcode,
            inStock: dto.in_stock,
            stockQty: dto.stock_qty,
          };
          addProduct(product);
        }
      } catch {
        // Silently ignore — the scanner will beep, user retries.
      }
    }, [addProduct]),
  });

  const handlePay = useCallback(() => {
    if (!total) return;
    setShowPayment(true);
  }, [total]);

  const handlePaymentComplete = useCallback(() => {
    setShowPayment(false);
    resetCart();
  }, [resetCart]);

  const handleApplyDiscount = useCallback(() => {
    const pct = parseInt(discountInput, 10);
    if (Number.isNaN(pct) || pct < 1 || pct > 100) return;
    setDiscount(pct, discountName.trim() || `${pct}% Discount`);
    setShowDiscountInput(false);
    setDiscountInput('');
    setDiscountName('');
  }, [discountInput, discountName, setDiscount]);

  const handleClearDiscount = useCallback(() => {
    setDiscount(0, '');
  }, [setDiscount]);

  // ── Hold Order state ──────────────────────────────────────────
  const [showHoldInput, setShowHoldInput] = useState(false);
  const [holdLabel, setHoldLabel] = useState('');
  const [heldCarts, setHeldCarts] = useState<HeldCartRow[]>([]);
  const [showHeldCarts, setShowHeldCarts] = useState(false);
  const [holding, setHolding] = useState(false);

  // Load held carts count on mount and when the panel opens.
  const loadHeldCarts = useCallback(() => {
    listHeldCarts().then(setHeldCarts).catch(() => {});
  }, []);

  useEffect(() => {
    loadHeldCarts();
  }, [loadHeldCarts]);

  useEffect(() => {
    if (showHeldCarts) {
      loadHeldCarts();
    }
  }, [showHeldCarts, loadHeldCarts]);

  const handleHold = useCallback(async () => {
    if (!subtotal || lines.length === 0) return;
    setHolding(true);
    try {
      const cartData = JSON.stringify({
        lines: lines.map((l) => ({
          sku: l.sku,
          name: l.name,
          qty: l.qty,
          unit_price: l.unit_price,
        })),
        discountPercent,
        discountLabel,
      });
      await holdCart({
        label: holdLabel.trim() || `Order #${Date.now()}`,
        cart_data: cartData,
        item_count: lines.length,
        total_minor: subtotal.minor_units,
        currency: subtotal.currency,
      });
      resetCart();
      setShowHoldInput(false);
      setHoldLabel('');
      loadHeldCarts();
    } catch {
      // Handle silently.
    } finally {
      setHolding(false);
    }
  }, [lines, subtotal, holdLabel, discountPercent, discountLabel, resetCart]);

  const handleResumeCart = useCallback(async (id: string) => {
    try {
      const full = await getHeldCart(id);
      if (!full) return;
      const data = JSON.parse(full.cart_data);
      // Restore lines and discount.
      if (data.lines && Array.isArray(data.lines)) {
        setLines(data.lines.map((l: { sku: string; name?: string; qty: number; unit_price: { minor_units: number; currency: string } }) => ({
          id: `restored-${Date.now()}-${Math.random().toString(36).slice(2)}` as LineId,
          sku: l.sku as Sku,
          name: l.name,
          qty: l.qty,
          unit_price: l.unit_price,
        })));
      }
      if (typeof data.discountPercent === 'number') {
        setDiscount(data.discountPercent, data.discountLabel || '');
      }
      await deleteHeldCart(id);
      setHeldCarts((prev) => prev.filter((c) => c.id !== id));
      setShowHeldCarts(false);
    } catch {
      // Handle silently.
    }
  }, [setLines, setDiscount]);

  return (
    <div className="pos-screen">
      {/* ── Left: Product lookup ─────────────────── */}
      <div className="pos-products">
        <ProductLookupScreen onAddProduct={addProduct} />
      </div>

      {/* ── Right: Cart panel ────────────────────── */}
      <aside className="pos-cart-panel" aria-label="Cart">
        <div className="pos-cart-header">
          <h2 className="pos-cart-title">
            <Localized id="pos-cart-panel-title">
              <span>Current Sale</span>
            </Localized>
            {lines.length > 0 && (
              <span className="pos-cart-count">{lines.length}</span>
            )}
          </h2>
        </div>

        {/* ── Cart lines ────────────────────────────── */}
        <div className="pos-cart-lines">
          {lines.length === 0 ? (
            <div className="pos-cart-empty-msg">
              <Localized id="pos-cart-empty">
                <span>Cart is empty</span>
              </Localized>
            </div>
          ) : (
            lines.map((line) => (
              <div
                key={line.id}
                className="pos-cart-line"
                aria-label={`${line.sku}, ${line.qty} × ${formatMoney(line.unit_price)}`}
              >
                {/* Info: name, SKU, unit price */}
                <div className="pos-cart-line-info">
                  <div className="pos-cart-line-name">{line.name ?? line.sku}</div>
                  <div className="pos-cart-line-sku">{line.sku}</div>
                  <div className="pos-cart-line-price">
                    {formatMoney(line.unit_price)} each
                  </div>

                  {/* Quantity controls */}
                  <div className="pos-cart-line-controls">
                    <button
                      type="button"
                      className="pos-cart-qty-btn"
                      onClick={() => updateQty(line.id, line.qty - 1)}
                      disabled={line.qty <= 1}
                      aria-label={`Decrease quantity of ${line.sku}`}
                    >
                      <MinusIcon />
                    </button>
                    <span className="pos-cart-qty-value" aria-label={`Quantity: ${line.qty}`}>
                      {line.qty}
                    </span>
                    <button
                      type="button"
                      className="pos-cart-qty-btn"
                      onClick={() => updateQty(line.id, line.qty + 1)}
                      aria-label={`Increase quantity of ${line.sku}`}
                    >
                      <PlusIcon />
                    </button>
                  </div>
                </div>

                {/* Line total */}
                <div className="pos-cart-line-total">
                  {formatMoney({
                    minor_units: line.unit_price.minor_units * line.qty,
                    currency: line.unit_price.currency,
                  })}
                </div>

                {/* Remove button */}
                <button
                  type="button"
                  className="pos-cart-line-remove"
                  onClick={() => removeLine(line.id)}
                  aria-label={`Remove ${line.sku} from cart`}
                >
                  <TrashIcon />
                </button>
              </div>
            ))
          )}
        </div>

        {/* ── Footer: subtotal + discount + pay ────── */}
        {lines.length > 0 && subtotal && (
          <div className="pos-cart-footer">
            {/* Subtotal */}
            <div className="pos-cart-subtotal-row">
              <span className="pos-cart-subtotal-label">Subtotal</span>
              <span className="pos-cart-subtotal-amount">
                {formatMoney(subtotal)}
              </span>
            </div>

            {/* Discount */}
            <div className="pos-cart-discount-area">
              {discountPercent > 0 ? (
                <div className="pos-cart-discount-row">
                  <span className="pos-cart-discount-label">
                    Discount ({discountLabel || `${discountPercent}%`})
                  </span>
                  <span className="pos-cart-discount-amount">
                    -{discountAmount ? formatMoney(discountAmount) : ''}
                  </span>
                  <button
                    type="button"
                    className="pos-cart-discount-clear"
                    onClick={handleClearDiscount}
                    aria-label="Remove discount"
                  >
                    &times;
                  </button>
                </div>
              ) : !showDiscountInput ? (
                <button
                  type="button"
                  className="pos-cart-discount-btn"
                  onClick={() => setShowDiscountInput(true)}
                >
                  + Add Discount
                </button>
              ) : null}

              {/* Discount input form */}
              {showDiscountInput && (
                <div className="pos-cart-discount-form">
                  <div className="pos-cart-discount-input-row">
                    <input
                      type="number"
                      className="pos-cart-discount-pct"
                      min="1"
                      max="100"
                      placeholder="%"
                      value={discountInput}
                      onChange={(e) => setDiscountInput(e.target.value)}
                      aria-label="Discount percentage"
                    />
                    <input
                      type="text"
                      className="pos-cart-discount-name"
                      placeholder="Label (optional)"
                      value={discountName}
                      onChange={(e) => setDiscountName(e.target.value)}
                      aria-label="Discount label"
                    />
                    <button
                      type="button"
                      className="pos-cart-discount-apply"
                      onClick={handleApplyDiscount}
                      disabled={!discountInput || parseInt(discountInput, 10) < 1 || parseInt(discountInput, 10) > 100}
                    >
                      Apply
                    </button>
                    <button
                      type="button"
                      className="pos-cart-discount-cancel"
                      onClick={() => {
                        setShowDiscountInput(false);
                        setDiscountInput('');
                        setDiscountName('');
                      }}
                      aria-label="Cancel discount"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </div>

            {/* Total row */}
            {total && (
              <div className="pos-cart-total-row">
                <span className="pos-cart-total-label">
                  <Localized id="pos-cart-total">
                    <span>Total</span>
                  </Localized>
                </span>
                <span className="pos-cart-total-amount">
                  {formatMoney(total)}
                </span>
              </div>
            )}

            {/* Action buttons row */}
            <div className="pos-cart-actions-row">
              {/* Pay button */}
              <button
                type="button"
                className="pos-cart-pay-btn"
                onClick={handlePay}
                aria-label={`Charge the customer ${total ? formatMoney(total) : ''}`}
              >
                <Localized id="pos-cart-pay" vars={{ amount: total ? formatMoney(total) : '' }}>
                  <span>Charge {total ? formatMoney(total) : ''}</span>
                </Localized>
              </button>

              {/* Hold button */}
              <button
                type="button"
                className="pos-cart-hold-btn"
                onClick={() => setShowHoldInput(true)}
                aria-label="Hold this order"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="12" y1="8" x2="12" y2="16" />
                  <line x1="8" y1="12" x2="16" y2="12" />
                </svg>
                Hold
              </button>
            </div>
          </div>
        )}

        {/* ── Held Orders badge (always visible) ── */}
        <button
          type="button"
          className="pos-cart-held-badge"
          onClick={() => { setShowHeldCarts(true); }}
          aria-label="View held orders"
          title="View held orders"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
            <path d="M12 2L2 7l10 5 10-5-10-5z" />
            <path d="M2 17l10 5 10-5" />
            <path d="M2 12l10 5 10-5" />
          </svg>
          <span>Held Orders</span>
          {heldCarts.length > 0 && (
            <span className="pos-cart-held-count">{heldCarts.length}</span>
          )}
        </button>
      </aside>

      {/* ── Payment modal ──────────────────────────── */}
      {total && (
        <PaymentModal
          open={showPayment}
          lineItems={lines}
          total={total}
          discountPercent={discountPercent}
          discountLabel={discountLabel}
          onComplete={handlePaymentComplete}
          onClose={() => setShowPayment(false)}
        />
      )}

      {/* ── Hold Input modal ────────────────────────── */}
      {showHoldInput && (
        <div className="pos-hold-overlay" role="dialog" aria-modal="true" aria-label="Hold order">
          <div className="pos-hold-modal">
            <h3 className="pos-hold-title">Hold Current Order</h3>
            <p className="pos-hold-desc">
              Enter a name for this held order so you can find it later.
            </p>
            <input
              type="text"
              className="pos-hold-input"
              placeholder="e.g. Customer waiting for manager"
              value={holdLabel}
              onChange={(e) => setHoldLabel(e.target.value)}
              aria-label="Hold order label"
            />
            <div className="pos-hold-actions">
              <button
                type="button"
                className="pos-hold-cancel-btn"
                onClick={() => {
                  setShowHoldInput(false);
                  setHoldLabel('');
                }}
                disabled={holding}
              >
                Cancel
              </button>
              <button
                type="button"
                className="pos-hold-confirm-btn"
                onClick={handleHold}
                disabled={holding}
              >
                {holding ? 'Holding…' : 'Hold Order'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Held Carts panel ────────────────────────── */}
      {showHeldCarts && (
        <div className="pos-hold-overlay" role="dialog" aria-modal="true" aria-label="Held orders list">
          <div className="pos-held-list-modal">
            <div className="pos-held-list-header">
              <h3>Held Orders</h3>
              <button
                type="button"
                className="pos-held-list-close"
                onClick={() => setShowHeldCarts(false)}
                aria-label="Close held orders list"
              >
                &times;
              </button>
            </div>
            <div className="pos-held-list-body">
              {heldCarts.length === 0 ? (
                <p className="pos-held-list-empty">No held orders.</p>
              ) : (
                heldCarts.map((hc) => (
                  <div key={hc.id} className="pos-held-item">
                    <div className="pos-held-item-info">
                      <span className="pos-held-item-label">{hc.label}</span>
                      <span className="pos-held-item-meta">
                        {hc.item_count} item{hc.item_count !== 1 ? 's' : ''} &middot; {formatMoney({ minor_units: hc.total_minor, currency: hc.currency })} &middot; {new Date(hc.created_at).toLocaleString()}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="pos-held-item-resume"
                      onClick={() => handleResumeCart(hc.id)}
                      aria-label={`Resume order: ${hc.label}`}
                    >
                      Resume
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
