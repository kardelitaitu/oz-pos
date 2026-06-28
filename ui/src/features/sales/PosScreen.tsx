import { useCallback } from 'react';
import { Localized } from '@/components/Localized';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import { formatMoney } from '@/types/domain';
import { usePosState } from './usePosState';
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
 * controls, remove buttons, subtotal, and a Pay button.
 */
export default function PosScreen() {
  const { lines, total, addProduct, removeLine, updateQty } = usePosState();

  const handlePay = useCallback(() => {
    if (!total) return;
    // Placeholder — IPC call to completeSale will go here.
    alert(`Pay ${formatMoney(total)}`);
  }, [total]);

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

        {/* ── Footer: subtotal + pay ───────────────── */}
        {lines.length > 0 && total && (
          <div className="pos-cart-footer">
            <div className="pos-cart-subtotal-row">
              <span className="pos-cart-subtotal-label">
                <Localized id="pos-cart-total">
                  <span>Total</span>
                </Localized>
              </span>
              <span className="pos-cart-subtotal-amount">
                {formatMoney(total)}
              </span>
            </div>
            <button
              type="button"
              className="pos-cart-pay-btn"
              onClick={handlePay}
              aria-label={`Charge the customer ${formatMoney(total)}`}
            >
              <Localized id="pos-cart-pay" vars={{ amount: formatMoney(total) }}>
                <span>Charge {formatMoney(total)}</span>
              </Localized>
            </button>
          </div>
        )}
      </aside>
    </div>
  );
}
