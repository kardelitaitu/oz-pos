import { useCallback, useState, useEffect, useRef } from 'react';
import { useToast } from '@/components/Toast';
import { useAuth } from '@/contexts/AuthContext';
import { Localized } from '@/components/Localized';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import { formatMoney, type CartLine, type LineId, type Product, type Sku } from '@/types/domain';
import { useSwipe } from '@/hooks/useSwipe';
import {
  holdCart,
  listHeldCarts,
  getHeldCart,
  deleteHeldCart,
  type HeldCartRow,
} from '@/api/sales';
import { lookupByBarcode, lookupProductBySku } from '@/api/products';
import { lookupBundleBySku } from '@/api/bundles';
import { expandBundleItems } from './bundleExpansion';
import type { BarcodeScannedPayload } from '@/api/hardware';
import { usePosState } from './usePosState';
import { useBarcodeScanner } from './useBarcodeScanner';
import { useCustomerDisplay } from './useCustomerDisplay';
import PaymentModal from './PaymentModal';
import {
  getActiveShift,
  openShift,
  closeShift,
  type ShiftDto,
} from '@/api/shifts';

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

// ── Swipeable cart line item ──────────────────────────────────────────

interface CartLineItemProps {
  line: CartLine;
  onRemove: (line: CartLine) => void;
  onDecreaseQty: (line: CartLine) => void;
  onIncreaseQty: (line: CartLine) => void;
}

function CartLineItem({ line, onRemove, onDecreaseQty, onIncreaseQty }: CartLineItemProps) {
  const [revealed, setRevealed] = useState(false);
  const swipe = useSwipe({
    onSwipeLeft: () => setRevealed(true),
    onSwipeRight: () => setRevealed(false),
  });

  return (
    <div
      className={`pos-cart-line-wrap ${revealed ? 'pos-cart-line-wrap--revealed' : ''}`}
      {...swipe}
    >
      <div
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
              onClick={() => onDecreaseQty(line)}
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
              onClick={() => onIncreaseQty(line)}
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
          onClick={() => {
            onRemove(line);
            setRevealed(false);
          }}
          aria-label={`Remove ${line.sku} from cart`}
        >
          <TrashIcon />
        </button>
      </div>

      {/* Revealed swipe action */}
      <div className="pos-cart-line-swipe-action" aria-hidden={!revealed}>
        <button
          type="button"
          className="pos-cart-line-swipe-remove"
          onClick={() => {
            onRemove(line);
            setRevealed(false);
          }}
          aria-label={`Remove ${line.sku}`}
        >
          <Localized id="pos-cart-remove">
            <span>Remove</span>
          </Localized>
        </button>
      </div>
    </div>
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
  const { addToast } = useToast();
  const { session, logout } = useAuth();
  const userId = session.user_id;
  const [showPayment, setShowPayment] = useState(false);
  const [showDiscountInput, setShowDiscountInput] = useState(false);
  const [discountInput, setDiscountInput] = useState('');
  const [discountName, setDiscountName] = useState('');

  // ── Barcode scanner integration ─────────────────────────────
  useBarcodeScanner({
    onProductFound: useCallback(async (payload: BarcodeScannedPayload) => {
      try {
        const code = payload.code;
        // 1. Try product barcode lookup first.
        const dto = await lookupByBarcode(code);
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
          return;
        }

        // 2. Fall back to bundle SKU expansion with proportional pricing.
        const bundle = await lookupBundleBySku(code);
        if (bundle && bundle.bundle.active) {
          const expanded = await expandBundleItems(
            bundle.items,
            bundle.bundle.currency,
            bundle.bundle.bundle_price_minor,
            lookupProductBySku,
          );
          for (const item of expanded) {
            addProduct(item.product, item.qty);
          }
          addToast({
            type: 'success',
            message: `Bundle "${bundle.bundle.name}" added — ${expanded.length} items`,
          });
        }
        // If neither product nor bundle matched, silently ignore.
      } catch {
        // Silently ignore — the scanner will beep, user retries.
      }
    }, [addProduct, addToast]),
  });

  const handlePay = useCallback(() => {
    if (!total) return;
    setShowPayment(true);
  }, [total]);

  const { handlePaymentComplete: customerDisplayPaymentComplete } = useCustomerDisplay({
    lines,
    total,
  });

  const handlePaymentComplete = useCallback(() => {
    setShowPayment(false);
    resetCart();
    // Also clear the customer-facing pole display.
    customerDisplayPaymentComplete();
  }, [resetCart, customerDisplayPaymentComplete]);

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

  // ── Swipe-to-remove undo ────────────────────────────────────────
  const [undoCartLine, setUndoCartLine] = useState<CartLine | null>(null);
  const undoTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleRemoveLine = useCallback((line: CartLine) => {
    removeLine(line.id);
    setUndoCartLine(line);
    if (undoTimeoutRef.current) clearTimeout(undoTimeoutRef.current);
    undoTimeoutRef.current = setTimeout(() => {
      setUndoCartLine(null);
      undoTimeoutRef.current = null;
    }, 3000);
  }, [removeLine]);

  const handleUndoRemove = useCallback(() => {
    if (!undoCartLine) return;
    setLines((prev) => [undoCartLine, ...prev]);
    setUndoCartLine(null);
    if (undoTimeoutRef.current) {
      clearTimeout(undoTimeoutRef.current);
      undoTimeoutRef.current = null;
    }
  }, [undoCartLine, setLines]);

  const handleDecreaseQty = useCallback((line: CartLine) => {
    updateQty(line.id, line.qty - 1);
  }, [updateQty]);

  const handleIncreaseQty = useCallback((line: CartLine) => {
    updateQty(line.id, line.qty + 1);
  }, [updateQty]);

  // ── Shift management ──────────────────────────────────────────
  const [activeShift, setActiveShift] = useState<ShiftDto | null>(null);
  const [shiftLoading, setShiftLoading] = useState(true);
  const [showCloseShift, setShowCloseShift] = useState(false);
  const [showOpenShift, setShowOpenShift] = useState(false);
  const [closingBalance, setClosingBalance] = useState('');
  const [openingBalance, setOpeningBalance] = useState('');
  const [shiftNotes, setShiftNotes] = useState('');
  const [closingShift, setClosingShift] = useState(false);
  const [openingShift, setOpeningShift] = useState(false);
  const [closeShiftError, setCloseShiftError] = useState<string | null>(null);
  const [closedShiftSummary, setClosedShiftSummary] = useState<ShiftDto | null>(null);

  // Load active shift on mount and when session changes.
  useEffect(() => {
    if (!userId) {
      setActiveShift(null);
      setShiftLoading(false);
      return;
    }
    setShiftLoading(true);
    getActiveShift(userId)
      .then((shift) => {
        setActiveShift(shift);
      })
      .catch(() => {
        setActiveShift(null);
      })
      .finally(() => setShiftLoading(false));
  }, [userId]);

  const handleCloseShiftClick = useCallback(() => {
    setCloseShiftError(null);
    setClosedShiftSummary(null);
    // Enforce: cart must be empty before closing shift.
    if (lines.length > 0) {
      setCloseShiftError('Complete or clear the current sale before closing the shift.');
      return;
    }
    setClosingBalance('');
    setShiftNotes('');
    setShowCloseShift(true);
  }, [lines]);

  const handleConfirmCloseShift = useCallback(async () => {
    if (!activeShift) return;
    const balance = parseInt(closingBalance, 10);
    if (Number.isNaN(balance) || balance < 0) return;

    setClosingShift(true);
    setCloseShiftError(null);
    try {
      const closed = await closeShift(
        activeShift.id,
        balance,
        shiftNotes.trim() || null,
      );
      setClosedShiftSummary(closed);
      setActiveShift(null); // no longer active
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to close shift';
      setCloseShiftError(msg);
    } finally {
      setClosingShift(false);
    }
  }, [activeShift, closingBalance, shiftNotes]);

  const handleOpenShiftClick = useCallback(() => {
    setOpeningBalance('');
    setShowOpenShift(true);
  }, []);

  const handleConfirmOpenShift = useCallback(async () => {
    const balance = parseInt(openingBalance, 10);
    const safeBalance = !Number.isNaN(balance) && balance >= 0 ? balance : 0;

    setOpeningShift(true);
    try {
      const shift = await openShift(userId, safeBalance);
      setActiveShift(shift);
      setShowOpenShift(false);
    } catch {
      // Handled silently — shift open failure is rare.
    } finally {
      setOpeningShift(false);
    }
  }, [openingBalance, userId]);

  const handleDismissShiftSummary = useCallback(() => {
    setClosedShiftSummary(null);
    setShowCloseShift(false);
    setCloseShiftError(null);
  }, []);

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

  if (!session) {
    return (
      <div className="pos-screen">
        <div className="pos-login-required">
          <Localized id="pos-login-required">
            <h2>Login Required</h2>
          </Localized>
          <Localized id="pos-login-desc">
            <p>Please log in to use the POS.</p>
          </Localized>
        </div>
      </div>
    );
  }

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
          <Localized id="pos-cart-lock">
            <button
              type="button"
              className="pos-cart-lock-btn"
              onClick={logout}
              aria-label="Lock terminal and log out"
              title="Lock terminal"
            >
              Lock
            </button>
          </Localized>
        </div>

        {/* ── Shift status bar ───────────────────────── */}
        <div className="pos-shift-bar" aria-label="Shift status">
          {shiftLoading ? (
            <Localized id="pos-shift-loading">
              <span className="pos-shift-bar-label">Loading shift…</span>
            </Localized>
          ) : activeShift ? (
            <>
              <span className="pos-shift-bar-indicator pos-shift-bar-indicator--open" />
              <Localized id="pos-shift-open-since" vars={{ time: new Date(activeShift.openedAt).toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              }) }}>
                <span className="pos-shift-bar-label">
                  Shift open since{' '}
                  {new Date(activeShift.openedAt).toLocaleTimeString([], {
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </span>
              </Localized>
              <Localized id="pos-shift-header-close">
                <button
                  type="button"
                  className="pos-shift-close-btn"
                  onClick={handleCloseShiftClick}
                  aria-label="Close current shift"
                >
                  Close Shift
                </button>
              </Localized>
            </>
          ) : (
            <>
              <span className="pos-shift-bar-indicator pos-shift-bar-indicator--closed" />
              <Localized id="pos-shift-no-active">
                <span className="pos-shift-bar-label">No active shift</span>
              </Localized>
              <Localized id="pos-shift-header-open">
                <button
                  type="button"
                  className="pos-shift-open-btn"
                  onClick={handleOpenShiftClick}
                  aria-label="Open a new shift"
                >
                  Open Shift
                </button>
              </Localized>
            </>
          )}
          {/* ── Inline shift error (cart not empty) ──── */}
          {closeShiftError && !showCloseShift && (
            <div className="pos-shift-error" role="alert">
              {closeShiftError}
              <button
                type="button"
                className="pos-shift-error-dismiss"
                onClick={() => setCloseShiftError(null)}
                aria-label="Dismiss error"
              >
                &times;
              </button>
            </div>
          )}
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
              <CartLineItem
                key={line.id}
                line={line}
                onRemove={handleRemoveLine}
                onDecreaseQty={handleDecreaseQty}
                onIncreaseQty={handleIncreaseQty}
              />
            ))
          )}
        </div>

        {/* ── Footer: subtotal + discount + pay ────── */}
        {lines.length > 0 && subtotal && (
          <div className="pos-cart-footer">
            {/* Subtotal */}
            <div className="pos-cart-subtotal-row">
              <Localized id="pos-cart-subtotal">
                <span className="pos-cart-subtotal-label">Subtotal</span>
              </Localized>
              <span className="pos-cart-subtotal-amount">
                {formatMoney(subtotal)}
              </span>
            </div>

            {/* Discount */}
            <div className="pos-cart-discount-area">
              {discountPercent > 0 ? (
                <div className="pos-cart-discount-row">
                  <span className="pos-cart-discount-label">
                    <Localized id="pos-cart-discount-label" vars={{ label: discountLabel || `${discountPercent}%` }}>
                      <span>Discount ({discountLabel || `${discountPercent}%`})</span>
                    </Localized>
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
                <Localized id="pos-cart-add-discount">
                  <button
                    type="button"
                    className="pos-cart-discount-btn"
                    onClick={() => setShowDiscountInput(true)}
                  >
                    + Add Discount
                  </button>
                </Localized>
              ) : null}

              {/* Discount input form */}
              {showDiscountInput && (
                <div className="pos-cart-discount-form">
                  <div className="pos-cart-discount-input-row">
                    <Localized id="pos-cart-pct-placeholder" attrs={{ placeholder: true }}>
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
                    </Localized>
                    <Localized id="pos-cart-label-placeholder" attrs={{ placeholder: true }}>
                      <input
                        type="text"
                        className="pos-cart-discount-name"
                        placeholder="Label (optional)"
                        value={discountName}
                        onChange={(e) => setDiscountName(e.target.value)}
                        aria-label="Discount label"
                      />
                    </Localized>
                    <Localized id="pos-cart-apply">
                      <button
                        type="button"
                        className="pos-cart-discount-apply"
                        onClick={handleApplyDiscount}
                        disabled={!discountInput || parseInt(discountInput, 10) < 1 || parseInt(discountInput, 10) > 100}
                      >
                        Apply
                      </button>
                    </Localized>
                    <Localized id="pos-cart-cancel">
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
                    </Localized>
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
            )}              {/* Action buttons row */}
            <div className="pos-cart-actions-row">
              {/* Clear cart button */}
              <Localized id="pos-cart-clear">
                <button
                  type="button"
                  className="pos-cart-clear-btn"
                  onClick={resetCart}
                  aria-label="Clear all items from cart"
                  title="Clear cart"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                    <polyline points="3 6 5 6 21 6" />
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  </svg>
                  Clear
                </button>
              </Localized>

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
              <Localized id="pos-cart-hold">
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
              </Localized>
            </div>

            {/* ── Undo bar ────────────────────────────── */}
            {undoCartLine && (
              <div className="pos-cart-undo-bar" role="status" aria-live="polite">
                <Localized id="pos-cart-removed" vars={{ name: undoCartLine.name ?? undoCartLine.sku }}>
                  <span className="pos-cart-undo-message">
                    Removed {undoCartLine.name ?? undoCartLine.sku}
                  </span>
                </Localized>
                <Localized id="pos-cart-undo">
                  <button
                    type="button"
                    className="pos-cart-undo-btn"
                    onClick={handleUndoRemove}
                    aria-label="Undo remove from cart"
                  >
                    Undo
                  </button>
                </Localized>
              </div>
            )}
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
          <Localized id="pos-held-orders">
            <span>Held Orders</span>
          </Localized>
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
          userId={userId}
          onComplete={handlePaymentComplete}
          onClose={() => setShowPayment(false)}
        />
      )}

      {/* ── Hold Input modal ────────────────────────── */}
      {showHoldInput && (
        <div className="pos-hold-overlay" role="dialog" aria-modal="true" aria-label="Hold order">
          <div className="pos-hold-modal">
            <Localized id="pos-hold-title">
              <h3 className="pos-hold-title">Hold Current Order</h3>
            </Localized>
            <Localized id="pos-hold-desc">
              <p className="pos-hold-desc">
                Enter a name for this held order so you can find it later.
              </p>
            </Localized>
            <Localized id="pos-hold-label-placeholder" attrs={{ placeholder: true }}>
              <input
                type="text"
                className="pos-hold-input"
                placeholder="e.g. Customer waiting for manager"
                value={holdLabel}
                onChange={(e) => setHoldLabel(e.target.value)}
                aria-label="Hold order label"
              />
            </Localized>
            <div className="pos-hold-actions">
              <Localized id="pos-hold-cancel">
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
              </Localized>
              <button
                type="button"
                className="pos-hold-confirm-btn"
                onClick={handleHold}
                disabled={holding}
              >
                <Localized id={holding ? 'pos-hold-confirming' : 'pos-hold-confirm'}>
                  <span>{holding ? 'Holding…' : 'Hold Order'}</span>
                </Localized>
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
              <Localized id="pos-held-orders">
                <h3>Held Orders</h3>
              </Localized>
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
                <Localized id="pos-held-empty">
                  <p className="pos-held-list-empty">No held orders.</p>
                </Localized>
              ) : (
                heldCarts.map((hc) => (
                  <div key={hc.id} className="pos-held-item">
                    <div className="pos-held-item-info">
                      <span className="pos-held-item-label">{hc.label}</span>
                      <span className="pos-held-item-meta">
                        {hc.item_count} item{hc.item_count !== 1 ? 's' : ''} &middot; {formatMoney({ minor_units: hc.total_minor, currency: hc.currency })} &middot; {new Date(hc.created_at).toLocaleString()}
                      </span>
                    </div>
                    <Localized id="pos-held-resume">
                      <button
                        type="button"
                        className="pos-held-item-resume"
                        onClick={() => handleResumeCart(hc.id)}
                        aria-label={`Resume order: ${hc.label}`}
                      >
                        Resume
                      </button>
                    </Localized>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift Confirmation Modal ───────── */}
      {showCloseShift && activeShift && !closedShiftSummary && (
        <div
          className="pos-close-shift-overlay"
          role="dialog"
          aria-modal="true"
          aria-label="Close shift"
        >
          <div className="pos-close-shift-modal">
            <Localized id="pos-close-shift-title">
              <h3 className="pos-close-shift-title">Close Shift</h3>
            </Localized>

            {closeShiftError && (
              <div className="pos-close-shift-error">
                {closeShiftError}
              </div>
            )}

            <div className="pos-close-shift-info">
              <div className="pos-close-shift-info-row">
                <Localized id="pos-close-shift-opened">
                  <span>Opened</span>
                </Localized>
                <span>{new Date(activeShift.openedAt).toLocaleString()}</span>
              </div>
              <div className="pos-close-shift-info-row">
                <Localized id="pos-close-shift-opening-balance">
                  <span>Opening balance</span>
                </Localized>
                <span>{formatMoney({ minor_units: activeShift.openingBalanceMinor, currency: 'USD' })}</span>
              </div>
            </div>

            <div className="pos-close-shift-field">
              <Localized id="pos-close-shift-counted-label">
                <label htmlFor="closing-balance" className="pos-close-shift-label">
                  Counted cash in drawer (minor units)
                </label>
              </Localized>
              <Localized id="pos-close-shift-counted-placeholder" attrs={{ placeholder: true }}>
                <input
                  id="closing-balance"
                  type="number"
                  className="pos-close-shift-input"
                  min="0"
                  placeholder="e.g. 15000 for $150.00"
                  value={closingBalance}
                  onChange={(e) => setClosingBalance(e.target.value)}
                  aria-label="Closing balance in minor units"
                />
              </Localized>
            </div>

            <div className="pos-close-shift-field">
              <Localized id="pos-close-shift-notes-label">
                <label htmlFor="shift-notes" className="pos-close-shift-label">
                  Notes (optional)
                </label>
              </Localized>
              <Localized id="pos-close-shift-notes-placeholder" attrs={{ placeholder: true }}>
                <textarea
                  id="shift-notes"
                  className="pos-close-shift-textarea"
                  rows={2}
                  placeholder="Any notes about this shift…"
                  value={shiftNotes}
                  onChange={(e) => setShiftNotes(e.target.value)}
                  aria-label="Shift notes"
                />
              </Localized>
            </div>

            <div className="pos-close-shift-actions">
              <Localized id="cancel">
                <button
                  type="button"
                  className="pos-close-shift-cancel-btn"
                  onClick={() => {
                    setShowCloseShift(false);
                    setCloseShiftError(null);
                  }}
                  disabled={closingShift}
                >
                  Cancel
                </button>
              </Localized>
              <button
                type="button"
                className="pos-close-shift-confirm-btn"
                onClick={handleConfirmCloseShift}
                disabled={
                  closingShift ||
                  !closingBalance ||
                  parseInt(closingBalance, 10) < 0 ||
                  Number.isNaN(parseInt(closingBalance, 10))
                }
              >
                <Localized id={closingShift ? 'pos-close-shift-closing' : 'pos-close-shift-confirm'}>
                  <span>{closingShift ? 'Closing…' : 'Close Shift'}</span>
                </Localized>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift Success Summary ────────────── */}
      {closedShiftSummary && (
        <div
          className="pos-close-shift-overlay"
          role="dialog"
          aria-modal="true"
          aria-label="Shift closed summary"
        >
          <div className="pos-close-shift-modal pos-close-shift-summary">
            <Localized id="pos-shift-closed-title">
              <h3 className="pos-close-shift-title">
                Shift Closed
              </h3>
            </Localized>

            <div className="pos-close-shift-summary-grid">
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-total-sales">
                  <span className="pos-close-shift-summary-label">Total Sales</span>
                </Localized>
                <span className="pos-close-shift-summary-value">
                  {formatMoney({ minor_units: closedShiftSummary.totalSalesMinor, currency: 'USD' })}
                </span>
              </div>
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-cash-sales">
                  <span className="pos-close-shift-summary-label">Cash Sales</span>
                </Localized>
                <span className="pos-close-shift-summary-value">
                  {formatMoney({ minor_units: closedShiftSummary.totalCashMinor, currency: 'USD' })}
                </span>
              </div>
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-card-sales">
                  <span className="pos-close-shift-summary-label">Card Sales</span>
                </Localized>
                <span className="pos-close-shift-summary-value">
                  {formatMoney({ minor_units: closedShiftSummary.totalCardMinor, currency: 'USD' })}
                </span>
              </div>
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-expected-cash">
                  <span className="pos-close-shift-summary-label">Expected Cash</span>
                </Localized>
                <span className="pos-close-shift-summary-value">
                  {closedShiftSummary.expectedCashMinor !== null
                    ? formatMoney({ minor_units: closedShiftSummary.expectedCashMinor, currency: 'USD' })
                    : '—'}
                </span>
              </div>
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-counted">
                  <span className="pos-close-shift-summary-label">Counted</span>
                </Localized>
                <span className="pos-close-shift-summary-value">
                  {closedShiftSummary.closingBalanceMinor !== null
                    ? formatMoney({ minor_units: closedShiftSummary.closingBalanceMinor, currency: 'USD' })
                    : '—'}
                </span>
              </div>
              <div className="pos-close-shift-summary-item">
                <Localized id="pos-shift-difference">
                  <span className="pos-close-shift-summary-label">Difference</span>
                </Localized>
                <span
                  className={`pos-close-shift-summary-value ${
                    closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor < 0
                      ? 'pos-close-shift-diff--negative'
                      : closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor > 0
                        ? 'pos-close-shift-diff--positive'
                        : ''
                  }`}
                >
                  {closedShiftSummary.cashDifferenceMinor !== null
                    ? formatMoney({ minor_units: closedShiftSummary.cashDifferenceMinor, currency: 'USD' })
                    : '—'}
                  {closedShiftSummary.cashDifferenceMinor !== null && closedShiftSummary.cashDifferenceMinor !== 0 && (
                    <span className="pos-close-shift-diff-tag">
                      <Localized id={closedShiftSummary.cashDifferenceMinor > 0 ? 'pos-shift-over' : 'pos-shift-short'}>
                        <span>{closedShiftSummary.cashDifferenceMinor > 0 ? 'Over' : 'Short'}</span>
                      </Localized>
                    </span>
                  )}
                </span>
              </div>
            </div>

            {closedShiftSummary.notes && (
              <div className="pos-close-shift-notes-display">
                <Localized id="pos-shift-notes">
                  <span className="pos-close-shift-summary-label">Notes</span>
                </Localized>
                <p>{closedShiftSummary.notes}</p>
              </div>
            )}

            <Localized id="pos-shift-summary-done">
              <button
                type="button"
                className="pos-close-shift-dismiss-btn"
                onClick={handleDismissShiftSummary}
              >
                Done
              </button>
            </Localized>
          </div>
        </div>
      )}

      {/* ── Open Shift Modal ───────────────────────── */}
      {showOpenShift && (
        <div
          className="pos-close-shift-overlay"
          role="dialog"
          aria-modal="true"
          aria-label="Open shift"
        >
          <div className="pos-close-shift-modal">
            <Localized id="pos-open-shift-title">
              <h3 className="pos-close-shift-title">Open Shift</h3>
            </Localized>

            <div className="pos-close-shift-field">
              <Localized id="pos-open-shift-balance-label">
                <label htmlFor="opening-balance" className="pos-close-shift-label">
                  Opening balance (minor units)
                </label>
              </Localized>
              <Localized id="pos-open-shift-balance-placeholder" attrs={{ placeholder: true }}>
                <input
                  id="opening-balance"
                  type="number"
                  className="pos-close-shift-input"
                  min="0"
                  placeholder="e.g. 500 for $5.00"
                  value={openingBalance}
                  onChange={(e) => setOpeningBalance(e.target.value)}
                  aria-label="Opening balance in minor units"
                />
              </Localized>
            </div>

            <div className="pos-close-shift-actions">
              <Localized id="cancel">
                <button
                  type="button"
                  className="pos-close-shift-cancel-btn"
                  onClick={() => setShowOpenShift(false)}
                  disabled={openingShift}
                >
                  Cancel
                </button>
              </Localized>
              <button
                type="button"
                className="pos-close-shift-confirm-btn"
                onClick={handleConfirmOpenShift}
                disabled={openingShift}
              >
                <Localized id={openingShift ? 'pos-open-shift-opening' : 'pos-open-shift-title'}>
                  <span>{openingShift ? 'Opening…' : 'Open Shift'}</span>
                </Localized>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
