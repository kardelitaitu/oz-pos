import { useState, useMemo, useCallback } from 'react';
import type { CartLine, LineId, Money, Product } from '@/types/domain';
let nextLineId = 0;
const genLineId = (): LineId =>
  `line-${Date.now()}-${nextLineId++}` as LineId;

/**
 * POS state hook — manages cart lines, add/remove/qty, discount, and total.
 *
 * Discount is applied locally (preview) and synced to the backend cart
 * via IPC so it's included in the completed sale.
 *
 * @example
 * ```tsx
 * const { lines, total, subtotal, discountPercent, discountLabel,
 *         addProduct, removeLine, updateQty, setDiscount, resetCart }
 *   = usePosState();
 * ```
 */
export function usePosState() {
  const [lines, setLines] = useState<CartLine[]>([]);
  const [discountPercent, setDiscountPercent] = useState(0);
  const [discountLabel, setDiscountLabel] = useState('');

  /**
   * Add a product to the cart, or increment qty if already present.
   * @param product The product to add.
   * @param qty Quantity to add (defaults to 1). Used for bundle expansion.
   */
  const addProduct = useCallback((product: Product, qty: number = 1) => {
    setLines((prev) => {
      const existing = prev.find((l) => l.sku === product.sku);
      if (existing) {
        return prev.map((l) =>
          l.id === existing.id ? { ...l, qty: l.qty + qty } : l,
        );
      }
      return [
        ...prev,
        {
          id: genLineId(),
          sku: product.sku,
          name: product.name,
          qty,
          unit_price: product.price,
        },
      ];
    });
  }, []);

  /** Remove a line from the cart by ID. */
  const removeLine = useCallback((lineId: LineId) => {
    setLines((prev) => prev.filter((l) => l.id !== lineId));
  }, []);

  /** Update the quantity of a line. */
  const updateQty = useCallback((lineId: LineId, qty: number) => {
    if (qty < 1) return;
    setLines((prev) =>
      prev.map((l) => (l.id === lineId ? { ...l, qty } : l)),
    );
  }, []);

  /** Computed subtotal (sum of all line totals, before discount). */
  const subtotal: Money | null = useMemo(() => {
    if (lines.length === 0) return null;
    const currency = lines[0]!.unit_price.currency;
    const sum = lines.reduce((acc, l) => {
      return acc + l.unit_price.minor_units * l.qty;
    }, 0);
    return { minor_units: sum, currency };
  }, [lines]);

  /** Computed total after applying any discount. */
  const total: Money | null = useMemo(() => {
    if (!subtotal) return null;
    if (discountPercent <= 0) return subtotal;
    const multiplier = 100 - discountPercent;
    const discounted = Math.floor(subtotal.minor_units * multiplier / 100);
    return { minor_units: discounted, currency: subtotal.currency };
  }, [subtotal, discountPercent]);

  /** Discount amount in minor units. */
  const discountAmount: Money | null = useMemo(() => {
    if (!subtotal || discountPercent <= 0) return null;
    const amount = Math.floor(subtotal.minor_units * discountPercent / 100);
    return { minor_units: amount, currency: subtotal.currency };
  }, [subtotal, discountPercent]);

  /**
   * Set a cart-level percentage discount.
   * Pass `percent = 0` to clear. `label` is optional.
   * This is a local preview; the backend applies it on complete.
   */
  const setDiscount = useCallback(async (percent: number, label: string) => {
    const clamped = Math.max(0, Math.min(100, Math.round(percent)));
    setDiscountPercent(clamped);
    setDiscountLabel(clamped > 0 ? label : '');
  }, []);

  /** Clear all lines and reset discount. */
  const resetCart = useCallback(() => {
    setLines([]);
    setDiscountPercent(0);
    setDiscountLabel('');
  }, []);

  return {
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
    /** Exposed for restoring held carts. Prefer addProduct/removeLine for normal use. */
    setLines,
  };
}
