import { useState, useEffect, useMemo } from 'react';
import { useLocalization } from '@fluent/react';
import { listProducts, type ProductDto } from '@/api/products';
import { type Product, type Sku } from '@/types/domain';

// ── Sample product fallback ─────────────────────────────────────────

const SAMPLE_PRODUCTS: Product[] = [
  { sku: 'LATTE' as Sku, name: 'Caffè Latte', category: 'Beverages', price: { minor_units: 450, currency: 'USD' }, barcode: '4901234567890', inStock: true, stockQty: 50 },
  { sku: 'CAPPU' as Sku, name: 'Cappuccino', category: 'Beverages', price: { minor_units: 420, currency: 'USD' }, barcode: '4901234567891', inStock: true, stockQty: 40 },
  { sku: 'ESPR' as Sku, name: 'Espresso Shot', category: 'Beverages', price: { minor_units: 280, currency: 'USD' }, barcode: '4901234567892', inStock: true, stockQty: 60 },
  { sku: 'MATCHA' as Sku, name: 'Matcha Latte', category: 'Beverages', price: { minor_units: 520, currency: 'USD' }, barcode: null, inStock: true, stockQty: 30 },
  { sku: 'BAGEL' as Sku, name: 'Plain Bagel', category: 'Food', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567894', inStock: true, stockQty: 100 },
  { sku: 'BAGEL-S' as Sku, name: 'Sesame Bagel', category: 'Food', price: { minor_units: 275, currency: 'USD' }, barcode: '4901234567895', inStock: true, stockQty: 75 },
  { sku: 'CROISS' as Sku, name: 'Butter Croissant', category: 'Food', price: { minor_units: 350, currency: 'USD' }, barcode: '4901234567896', inStock: true, stockQty: 45 },
  { sku: 'MUFFIN-B' as Sku, name: 'Blueberry Muffin', category: 'Food', price: { minor_units: 320, currency: 'USD' }, barcode: '4901234567897', inStock: true, stockQty: 20 },
  { sku: 'MUFFIN-C' as Sku, name: 'Chocolate Muffin', category: 'Food', price: { minor_units: 340, currency: 'USD' }, barcode: null, inStock: false, stockQty: 0 },
  { sku: 'SANDW-C' as Sku, name: 'Chicken Sandwich', category: 'Food', price: { minor_units: 750, currency: 'USD' }, barcode: '4901234567899', inStock: true, stockQty: 15 },
  { sku: 'SANDW-V' as Sku, name: 'Veggie Sandwich', category: 'Food', price: { minor_units: 680, currency: 'USD' }, barcode: '4901234567900', inStock: true, stockQty: 10 },
  { sku: 'COOKIE' as Sku, name: 'Chocolate Chip Cookie', category: 'Food', price: { minor_units: 195, currency: 'USD' }, barcode: '4901234567901', inStock: true, stockQty: 200 },
  { sku: 'TEA-G' as Sku, name: 'Green Tea', category: 'Beverages', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567902', inStock: true, stockQty: 80 },
  { sku: 'TEA-C' as Sku, name: 'Chai Tea', category: 'Beverages', price: { minor_units: 320, currency: 'USD' }, barcode: null, inStock: true, stockQty: 35 },
  { sku: 'JUICE-O' as Sku, name: 'Orange Juice', category: 'Beverages', price: { minor_units: 380, currency: 'USD' }, barcode: '4901234567904', inStock: true, stockQty: 25 },
  { sku: 'WATER-S' as Sku, name: 'Sparkling Water', category: 'Beverages', price: { minor_units: 180, currency: 'USD' }, barcode: '4901234567905', inStock: true, stockQty: 150 },
  { sku: 'BROWNIE' as Sku, name: 'Fudge Brownie', category: 'Food', price: { minor_units: 295, currency: 'USD' }, barcode: '4901234567906', inStock: false, stockQty: 0 },
  { sku: 'MUFFIN-BA' as Sku, name: 'Banana Muffin', category: 'Food', price: { minor_units: 310, currency: 'USD' }, barcode: null, inStock: true, stockQty: 12 },
];

// ── DTO → Product mapper ────────────────────────────────────────────

/** Map a `ProductDto` from IPC to the front-end `Product` type. */
function dtoToProduct(dto: ProductDto, uncategorisedLabel: string): Product {
  return {
    sku: dto.sku as Sku,
    name: dto.name,
    category: dto.category ?? uncategorisedLabel,
    price: {
      minor_units: dto.price.minor_units,
      currency: dto.price.currency,
    },
    barcode: dto.barcode,
    inStock: dto.in_stock,
    stockQty: dto.stock_qty,
  };
}

// ── Hook ─────────────────────────────────────────────────────────────

export interface UseProductsResult {
  /** The list of products (from IPC or sample fallback). */
  products: Product[];
  /** Unique category names derived from the product list. */
  categories: string[];
  /** Whether products are still loading (IPC call in flight). */
  loading: boolean;
  /** Error message if the IPC call failed (excludes IPC-unavailable). */
  error: string | null;
  /** Whether we're using the sample data fallback. */
  usingFallback: boolean;
}

/**
 * Fetch products from the Rust backend via IPC on mount.
 *
 * Falls back to hardcoded sample data when IPC is unavailable
 * (e.g. running outside Tauri during development).
 *
 * @example
 * ```tsx
 * const { products, categories, loading, usingFallback } = useProducts();
 * ```
 */
export function useProducts(): UseProductsResult {
  const { l10n } = useLocalization();
  const [products, setProducts] = useState<Product[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [usingFallback, setUsingFallback] = useState(false);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const dtos = await listProducts();
        if (cancelled) return;
        if (dtos.length > 0) {
          const uncategorisedLabel = l10n.getString('product-lookup-uncategorised');
          setProducts(dtos.map(dto => dtoToProduct(dto, uncategorisedLabel)));
          setUsingFallback(false);
        } else {
          // Empty DB from backend — use samples as a development fallback.
          setProducts(SAMPLE_PRODUCTS);
          setUsingFallback(true);
        }
      } catch (err) {
        // IPC unavailable — fall back to sample data
        if (cancelled) return;
        setError(err instanceof Error ? err.message : l10n.getString('product-lookup-error-load'));
        setProducts(SAMPLE_PRODUCTS);
        setUsingFallback(true);
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [l10n]);

  // Derive categories from products (memoized).
  const categories = useMemo(() => {
    if (!products) return [];
    const cats = new Set(products.map((p) => p.category));
    return Array.from(cats).sort();
  }, [products]);

  return {
    products: products ?? [],
    categories,
    loading,
    error,
    usingFallback,
  };
}
