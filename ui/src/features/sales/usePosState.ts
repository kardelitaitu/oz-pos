import { useState, useMemo, useCallback } from 'react';
import type { CartLine, LineId, Money, Product } from '@/types/domain';
import { triggerInteraction } from '@/utils/interaction';

let nextLineId = 0;
const genLineId = (): LineId =>
  `line-${Date.now()}-${nextLineId++}` as LineId;

/**
 * Default service charge percentage applied when the toggle is on.
 * Configurable per-store via Settings; this is the local fallback.
 */
const SERVICE_CHARGE_DEFAULT_PERCENT = 10;

/**
 * POS state hook — manages cart lines, add/remove/qty, discount, tip,
 * service charge, and total.
 *
 * Discount is applied locally (preview) and synced to the backend cart
 * via IPC so it's included in the completed sale. Tip and service charge
 * are local previews for now — the cashier sees the running total adjust
 * in real time. Finalising them on the backend will mirror the discount
 * pattern in a follow-up.
 *
 * @example
 * ```tsx
 * const { lines, total, subtotal, discountPercent, tipPercent,
 *         serviceChargeEnabled, addProduct, setDiscount, setTipPercent,
 *         setServiceCharge, resetCart } = usePosState();
 * ```
 */
export function usePosState() {
  const [lines, setLines] = useState<CartLine[]>([]);
  const [discountPercent, setDiscountPercent] = useState(0);
  const [discountLabel, setDiscountLabel] = useState('');
  const [tipPercent, setTipPercentState] = useState(0);
  const [serviceChargeEnabled, setServiceChargeEnabled] = useState(false);
  const [serviceChargePercent, setServiceChargePercentState] = useState<number>(
    SERVICE_CHARGE_DEFAULT_PERCENT,
  );

  /**
   * Add a product to the cart, or increment qty if already present.
   * Category is captured on the cart line so that course chips render
   * without re-querying the product catalogue for every line.
   * @param product The product to add.
   * @param qty Quantity to add (defaults to 1). Used for bundle expansion.
   */
  const addProduct = useCallback((product: Product, qty: number = 1) => {
    triggerInteraction('add-to-cart');
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
          category: product.category,
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

  /** Override the unit price of a line (manager price override). */
  const updateLinePrice = useCallback((lineId: LineId, newPrice: Money) => {
    setLines((prev) =>
      prev.map((line) =>
        line.id === lineId ? { ...line, unit_price: newPrice } : line,
      ),
    );
  }, [setLines]);

  /** Computed subtotal (sum of all line qty × unit_price). */
  const subtotal: Money | null = useMemo(() => {
    if (lines.length === 0) return null;
    const currency = lines[0]!.unit_price.currency;
    const sum = lines.reduce((acc, l) => {
      return acc + l.unit_price.minor_units * l.qty;
    }, 0);
    return { minor_units: sum, currency };
  }, [lines]);

  /** Subtotal after discount — base for service charge and tip math. */
  const discounted: Money | null = useMemo(() => {
    if (!subtotal) return null;
    if (discountPercent <= 0) return subtotal;
    const multiplier = 100 - discountPercent;
    return {
      minor_units: Math.floor((subtotal.minor_units * multiplier) / 100),
      currency: subtotal.currency,
    };
  }, [subtotal, discountPercent]);

  /** Computed grand total after discount + service charge + tip. */
  const total: Money | null = useMemo(() => {
    if (!discounted) return null;
    let sum = discounted.minor_units;
    if (serviceChargeEnabled && serviceChargePercent > 0) {
      sum += Math.floor((discounted.minor_units * serviceChargePercent) / 100);
    }
    if (tipPercent > 0) {
      sum += Math.floor((discounted.minor_units * tipPercent) / 100);
    }
    return { minor_units: sum, currency: discounted.currency };
  }, [discounted, serviceChargeEnabled, serviceChargePercent, tipPercent]);

  /** Discount amount in minor units. */
  const discountAmount: Money | null = useMemo(() => {
    if (!subtotal || discountPercent <= 0) return null;
    return {
      minor_units: Math.floor((subtotal.minor_units * discountPercent) / 100),
      currency: subtotal.currency,
    };
  }, [subtotal, discountPercent]);

  /** Service charge amount in minor units (null when disabled). */
  const serviceChargeAmount: Money | null = useMemo(() => {
    if (!discounted || !serviceChargeEnabled || serviceChargePercent <= 0) {
      return null;
    }
    return {
      minor_units: Math.floor((discounted.minor_units * serviceChargePercent) / 100),
      currency: discounted.currency,
    };
  }, [discounted, serviceChargeEnabled, serviceChargePercent]);

  /** Tip amount in minor units (zero tip → null to suppress preview row). */
  const tipAmount: Money | null = useMemo(() => {
    if (!discounted || tipPercent <= 0) return null;
    return {
      minor_units: Math.floor((discounted.minor_units * tipPercent) / 100),
      currency: discounted.currency,
    };
  }, [discounted, tipPercent]);

  /**
   * Set a cart-level percentage discount.
   * Pass `percent = 0` to clear. `label` is optional.
   * This is a local preview; the backend applies it on complete.
   */
  const setDiscount = useCallback(
    async (percent: number, label: string) => {
      const clamped = Math.max(0, Math.min(100, Math.round(percent)));
      setDiscountPercent(clamped);
      setDiscountLabel(clamped > 0 ? label : '');
    },
    [],
  );

  /**
   * Set the tip percentage (0..100). The tip preview row re-renders as
   * the cashier taps different segments.
   */
  const setTipPercent = useCallback((percent: number) => {
    const clamped = Math.max(0, Math.min(100, Math.round(percent)));
    setTipPercentState(clamped);
  }, []);

  /**
   * Toggle the service charge on/off. Optional percent override (the
   * toggle UI keeps the existing percent by default).
   */
  const setServiceCharge = useCallback(
    (enabled: boolean, percent?: number) => {
      setServiceChargeEnabled(enabled);
      if (typeof percent === 'number') {
        setServiceChargePercentState(
          Math.max(0, Math.min(100, Math.round(percent))),
        );
      }
    },
    [],
  );

  /** Clear all lines and reset discount, tip, service charge. */
  const resetCart = useCallback(() => {
    setLines([]);
    setDiscountPercent(0);
    setDiscountLabel('');
    setTipPercentState(0);
    setServiceChargeEnabled(false);
  }, []);

  return {
    lines,
    subtotal,
    total,
    discountPercent,
    discountLabel,
    discountAmount,
    tipPercent,
    tipAmount,
    serviceChargeEnabled,
    serviceChargePercent,
    serviceChargeAmount,
    addProduct,
    removeLine,
    updateQty,
    updateLinePrice,
    setDiscount,
    setTipPercent,
    setServiceCharge,
    resetCart,
    /** Exposed for restoring held carts. Prefer addProduct/removeLine for normal use. */
    setLines,
  };
}
