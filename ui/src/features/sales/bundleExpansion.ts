// ── Bundle Expansion ──────────────────────────────────────────────
//
// Shared utility used by both the barcode scanner (PosScreen) and the
// manual barcode input (ProductLookupScreen) to expand a bundle SKU
// into individual cart items.
//
// When `bundle.bundle_price_minor` is set, the bundle-level price is
// distributed across items proportionally to their base prices.
// Rounding uses a carry-forward strategy so the actual cart total
// (sum of unit_price × qty) is within a few minor units of the
// bundle price (exact when every item has qty=1).

import type { Product, Sku } from '@/types/domain';
import type { BundleItem } from '@/api/bundles';
import type { ProductDto } from '@/api/products';

/** One expanded item ready to be added to the cart. */
export interface ExpandedItem {
  product: Product;
  qty: number;
}

/** Input item for price distribution with already-resolved base price. */
interface PriceItem {
  qty: number;
  basePriceMinor: number;
}

/**
 * Distribute a bundle-level price across items proportionally to their
 * base prices, using carry-forward rounding.
 *
 * Post-condition: the sum of `unitPrices[i] × items[i].qty` is within
 * a few minor units of `bundlePriceMinor` (exact when every item has qty=1).
 *
 * @visibleForTesting Exported for unit test access.
 */
export function distributeBundlePrice(
  bundlePriceMinor: number,
  items: PriceItem[],
): number[] {
  const baseTotals = items.map((it) => it.qty * it.basePriceMinor);
  const baseTotal = baseTotals.reduce((a, b) => a + b, 0);

  if (baseTotal <= 0) {
    return items.map((it) => it.basePriceMinor);
  }

  // 1. Proportional shares — last item gets rounding remainder.
  const n = items.length;
  const shares: number[] = [];
  let allocated = 0;
  for (let i = 0; i < n; i++) {
    const share =
      i === n - 1
        ? bundlePriceMinor - allocated
        : Math.floor((bundlePriceMinor * baseTotals[i]!) / baseTotal);
    shares.push(share);
    allocated += share;
  }

  // 2. Convert shares to unit prices with carry-forward rounding.
  const unitPrices: number[] = [];
  let carry = 0;
  for (let i = 0; i < n; i++) {
    const item = items[i]!;
    if (item.qty <= 0) {
      unitPrices.push(0);
      continue;
    }
    const totalWithCarry = shares[i]! + carry;
    const unitPrice = Math.floor(totalWithCarry / item.qty);
    unitPrices.push(unitPrice);
    carry = totalWithCarry - unitPrice * item.qty;
  }

  return unitPrices;
}

/**
 * Expand a bundle's items into cart-ready products, applying
 * proportional price distribution when a bundle-level price is set.
 *
 * @param items             Bundle items from `BundleWithItems.items`
 * @param currency          The bundle's currency code
 * @param bundlePriceMinor  Optional bundle-level price override in minor units
 * @param lookupItem        Async callback that fetches product details by SKU
 * @returns Array of expanded items ready to add to the cart
 */
export async function expandBundleItems(
  items: BundleItem[],
  currency: string,
  bundlePriceMinor: number | null,
  lookupItem: (sku: string) => Promise<ProductDto | null>,
): Promise<ExpandedItem[]> {
  if (items.length === 0) return [];

  // Fetch all product details in parallel.
  const dtos = await Promise.all(
    items.map((item) => lookupItem(item.sku)),
  );

  // Pair each item with its DTO, skipping any that didn't resolve.
  interface ResolvedEntry {
    item: BundleItem;
    dto: ProductDto;
    basePrice: number;
  }
  const resolved: ResolvedEntry[] = [];
  for (let idx = 0; idx < items.length; idx++) {
    const dto = dtos[idx];
    const item = items[idx]!;
    if (dto == null) continue;
    const basePrice =
      item.unit_price_minor != null
        ? item.unit_price_minor
        : dto.price.minor_units;
    resolved.push({ item, dto, basePrice });
  }

  if (resolved.length === 0) return [];

  // Calculate unit prices — either proportional distribution or direct.
  const unitPrices: number[] =
    bundlePriceMinor != null
      ? distributeBundlePrice(
          bundlePriceMinor,
          resolved.map((r) => ({ qty: r.item.qty, basePriceMinor: r.basePrice })),
        )
      : resolved.map((r) => r.basePrice);

  // Build expanded items from resolved pairs.
  const result: ExpandedItem[] = [];
  for (let idx = 0; idx < resolved.length; idx++) {
    const entry = resolved[idx]!;
    const { item, dto } = entry;
    result.push({
      product: {
        sku: dto.sku as Sku,
        name: dto.name,
        category: dto.category ?? 'Uncategorised',
        price: {
          minor_units: unitPrices[idx]!,
          currency,
        },
        barcode: dto.barcode,
        inStock: dto.in_stock,
        stockQty: dto.stock_qty,
      },
      qty: item.qty,
    });
  }

  return result;
}
