import { useState, useMemo, useCallback } from 'react';
import type { CartLine, LineId, Money, Product } from '@/types/domain';

let nextLineId = 0;
const genLineId = (): LineId =>
  `line-${Date.now()}-${nextLineId++}` as LineId;

/**
 * POS state hook — manages cart lines, add/remove/qty, and total.
 *
 * @example
 * ```tsx
 * const { lines, total, addProduct, removeLine, updateQty } = usePosState();
 * ```
 */
export function usePosState() {
  const [lines, setLines] = useState<CartLine[]>([]);

  /** Add a product to the cart, or increment qty if already present. */
  const addProduct = useCallback((product: Product) => {
    setLines((prev) => {
      const existing = prev.find((l) => l.sku === product.sku);
      if (existing) {
        return prev.map((l) =>
          l.id === existing.id ? { ...l, qty: l.qty + 1 } : l,
        );
      }        return [
        ...prev,
        {
          id: genLineId(),
          sku: product.sku,
          name: product.name,
          qty: 1,
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

  /** Computed total across all lines. */
  const total: Money | null = useMemo(() => {
    if (lines.length === 0) return null;
    const currency = lines[0]!.unit_price.currency;
    const sum = lines.reduce((acc, l) => {
      return acc + l.unit_price.minor_units * l.qty;
    }, 0);
    return { minor_units: sum, currency };
  }, [lines]);

  return { lines, total, addProduct, removeLine, updateQty };
}
