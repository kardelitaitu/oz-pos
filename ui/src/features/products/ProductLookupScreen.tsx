import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { Localized } from '@/components/Localized';
import { formatMoney, type Product } from '@/types/domain';
import { lookupProductBySku } from '@/api/products';
import { lookupBundleBySku } from '@/api/bundles';
import { expandBundleItems } from '@/features/sales/bundleExpansion';
import { useProducts } from './useProducts';
import './ProductLookupScreen.css';

// ── Props ──────────────────────────────────────────────────────────

export interface ProductLookupScreenProps {
  /** Called when the user clicks "Add to cart" on a product. */
  onAddProduct?: (product: Product) => void;
}

// ── Helpers ────────────────────────────────────────────────────────

type Category = string;

/** Search icon SVG */
function SearchIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  );
}

/** Barcode scan icon SVG */
function BarcodeIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M2 4h2v16H2z" />
      <path d="M6 4h1v16H6z" />
      <path d="M9 4h2v16H9z" />
      <path d="M13 4h1v16h-1z" />
      <path d="M16 4h2v16h-2z" />
      <path d="M20 4h2v16h-2z" />
    </svg>
  );
}

/** Add (plus) icon SVG */
function AddIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}

/** Package/search icon for empty state */
function PackageIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="product-empty-icon" aria-hidden="true">
      <path d="M16.5 9.4 7.55 4.24" />
      <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
      <polyline points="3.29 7 12 12 20.71 7" />
      <line x1="12" y1="22" x2="12" y2="12" />
    </svg>
  );
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Product Lookup screen.
 *
 * Provides a search bar, barcode scanner input, category filter chips,
 * and a responsive product grid. Uses sample data for now — the IPC
 * bridge to the backend product catalog will be added in a follow-up.
 *
 * @example
 * ```tsx
 * <ProductLookupScreen onAddProduct={(p) => console.log('add', p.sku)} />
 * ```
 */
export default function ProductLookupScreen({ onAddProduct }: ProductLookupScreenProps) {
  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { products, categories, loading, usingFallback } = useProducts();
  const [searchQuery, setSearchQuery] = useState('');
  const [barcodeInput, setBarcodeInput] = useState('');
  const [activeCategory, setActiveCategory] = useState<Category>('All');
  const [addedSku, setAddedSku] = useState<string | null>(null);
  const addedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clean up add-to-cart animation timer on unmount
  useEffect(() => {
    return () => {
      if (addedTimerRef.current) clearTimeout(addedTimerRef.current);
    };
  }, []);

  // Wrap onAddProduct to trigger the green flash animation
  const handleAddProduct = useCallback((product: Product) => {
    onAddProduct?.(product);
    setAddedSku(product.sku);
    if (addedTimerRef.current) clearTimeout(addedTimerRef.current);
    addedTimerRef.current = setTimeout(() => setAddedSku(null), 450);
  }, [onAddProduct]);

  // All category options: "All" + each unique category
  const categoryOptions = useMemo<Category[]>(
    () => ['All', ...categories],
    [categories],
  );

  // Filter products based on search, barcode, and category
  const filtered = useMemo(() => {
    let results = products;

    // Filter by search query
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      results = results.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.sku.toLowerCase().includes(q) ||
          (p.barcode !== null && p.barcode.includes(q)),
      );
    }

    // Filter by active category
    if (activeCategory !== 'All') {
      results = results.filter((p) => p.category === activeCategory);
    }

    return results;
  }, [searchQuery, activeCategory, products]);

  // Handle barcode scan submission with bundle expansion fallback
  const handleBarcodeScan = useCallback(async () => {
    if (!barcodeInput.trim()) return;
    const code = barcodeInput.trim();
    const found = products.find((p) => p.barcode === code);
    if (found && found.inStock) {
      handleAddProduct(found);
      setBarcodeInput('');
      return;
    }

    // Fall back to bundle SKU expansion with proportional pricing.
    try {
      const bundle = await lookupBundleBySku(code);
      if (bundle && bundle.bundle.active) {
        const expanded = await expandBundleItems(
          bundle.items,
          bundle.bundle.currency,
          bundle.bundle.bundle_price_minor,
          lookupProductBySku,
        );
        for (const item of expanded) {
          // Add once per quantity — onAddProduct uses default qty=1.
          for (let i = 0; i < item.qty; i++) {
            onAddProduct?.(item.product);
          }
        }
        setAddedSku(code);
        addedTimerRef.current = setTimeout(() => setAddedSku(null), 450);
        addToast({
          type: 'success',
          message: l10n.getString('product-lookup-bundle-added', {
            name: bundle.bundle.name,
            count: expanded.length,
          }),
        });
      } else {
        addToast({
          type: 'warning',
          message: l10n.getString('product-lookup-no-match'),
        });
      }
    } catch {
      // If bundle lookup fails, silently ignore.
    }
    setBarcodeInput('');
  }, [barcodeInput, handleAddProduct, products, addToast]);

  // Handle Enter key in barcode input
  const handleBarcodeKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        handleBarcodeScan();
      }
    },
    [handleBarcodeScan],
  );

  // Handle Enter key in search input (could focus barcode, etc.)
  const handleSearchKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter' && searchQuery.trim()) {
        // Focus the first product card or scroll to grid
        const firstCardBtn = document.querySelector('.product-card-btn');
        if (firstCardBtn instanceof HTMLElement) {
          firstCardBtn.focus();
        }
      }
    },
    [searchQuery],
  );

  return (
    <div className="product-lookup">
      {/* ── Toolbar: Search + Barcode ────────────── */}
      <div className="product-toolbar">
        <div className="product-search-wrapper">
          <span className="product-search-icon">
            <SearchIcon />
          </span>
          <Localized id="product-lookup-search-placeholder" attrs={{ placeholder: true }}>
            <Localized id="product-lookup-search-aria" attrs={{ 'aria-label': true }}>
              <input
                type="search"
                className="product-search-input"
                placeholder="Search products…"
                aria-label="Search products"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={handleSearchKeyDown}
                autoComplete="off"
              />
            </Localized>
          </Localized>
        </div>

        <div className="product-barcode-wrapper">
          <Localized id="product-lookup-barcode-placeholder" attrs={{ placeholder: true }}>
            <Localized id="product-lookup-barcode-aria" attrs={{ 'aria-label': true }}>
              <input
                id="barcode-input"
                type="text"
                className="product-barcode-input"
                placeholder="Scan barcode…"
                aria-label="Barcode input"
                value={barcodeInput}
                onChange={(e) => setBarcodeInput(e.target.value)}
                onKeyDown={handleBarcodeKeyDown}
                autoComplete="off"
              />
            </Localized>
          </Localized>
          <Localized id="product-lookup-scan-btn-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="product-scan-btn"
              onClick={handleBarcodeScan}
              aria-label="Submit barcode"
            >
              <BarcodeIcon />
              <Localized id="product-lookup-barcode-scan">
                <span />
              </Localized>
            </button>
          </Localized>
        </div>
      </div>

      {/* ── Category filters ───────────────────────── */}
      <Localized id="product-lookup-categories-aria" attrs={{ 'aria-label': true }}>
        <div className="product-categories" role="radiogroup" aria-label="Filter by category">
          {categoryOptions.map((cat) => (
            <button
              key={cat}
              type="button"
              role="radio"
              aria-checked={activeCategory === cat}
              className={
                activeCategory === cat
                  ? 'product-category-chip product-category-chip--active'
                  : 'product-category-chip'
              }
              onClick={() => setActiveCategory(cat)}
            >
              {cat === 'All' ? (
                <Localized id="product-lookup-all-categories">
                  <span />
                </Localized>
              ) : (
                cat
              )}
            </button>
          ))}
        </div>
      </Localized>

      {/* ── Loading state ────────────────────────────── */}
      {loading ? (
        <div className="product-empty">
          <span className="product-empty-text">
            <Localized id="product-lookup-loading">
              <span />
            </Localized>
          </span>
        </div>
      ) : filtered.length === 0 ? (
        <div className="product-empty">
          <PackageIcon />
          <span className="product-empty-text">
            <Localized id="product-lookup-no-results">
              <span />
            </Localized>
          </span>
        </div>
      ) : (
        <Localized id="product-lookup-grid-aria" attrs={{ 'aria-label': true }}>
          <div className="product-grid" role="list" aria-label="Products">
            {filtered.map((product) => (
              <ProductCard
                key={product.sku}
                product={product}
                onAdd={handleAddProduct}
                added={product.sku === addedSku}
              />
            ))}
          </div>
        </Localized>
      )
      }

      {/* Dev notice when using fallback data */}
      {usingFallback && (
        <div
          style={{
            fontSize: 'var(--text-xs)',
            color: 'var(--color-fg-tertiary)',
            textAlign: 'center',
            padding: 'var(--space-2)',
          }}
        >
          <Localized id="product-lookup-dev-fallback">
            <span />
          </Localized>
        </div>
      )}
    </div>
  );
}

const PRICE_VOLATILITY_MS = 24 * 60 * 60 * 1000;

function isPriceRecent(p: Product): boolean {
  if (!p.priceUpdatedAt) return false;
  const elapsed = Date.now() - new Date(p.priceUpdatedAt).getTime();
  return elapsed >= 0 && elapsed < PRICE_VOLATILITY_MS;
}

// ── ProductCard sub-component ──────────────────────────────────────

interface ProductCardProps {
  product: Product;
  onAdd?: (product: Product) => void;
  added?: boolean;
}

function ProductCard({ product, onAdd, added }: ProductCardProps) {
  const { l10n } = useLocalization();
  const handleAdd = useCallback(() => {
    onAdd?.(product);
  }, [product, onAdd]);

  const stockLabel = product.inStock
    ? l10n.getString('product-lookup-in-stock')
    : l10n.getString('product-lookup-out-of-stock');

  let cardClass = 'product-card';
  if (!product.inStock) cardClass += ' product-card--disabled';
  if (added) cardClass += ' product-card--added';

  return (
    <div
      className={cardClass}
      role="listitem"
    >
      {/* Clickable area wraps the entire card content */}
      <Localized
        id="product-lookup-card-aria"
        attrs={{ 'aria-label': true }}
        vars={{
          name: product.name,
          price: formatMoney(product.price),
          sku: product.sku,
          stock: stockLabel,
        }}
      >
        <button
          type="button"
          className="product-card-btn"
          onClick={handleAdd}
          disabled={!product.inStock}
          aria-label={`${product.name} — ${formatMoney(product.price)}`}
        >
          {/* Row: name + category badge */}
          {isPriceRecent(product) && <span className="product-card-price-volatility" title="Price changed recently" />}
          <div className="product-card-header">
            <h3 className="product-card-name" title={product.name}>
              {product.name}
            </h3>
            <span className="product-card-category">{product.category}</span>
          </div>

          {/* Price */}
          <span className="product-card-price">{formatMoney(product.price)}</span>

          {/* SKU */}
          <span className="product-card-sku">{product.sku}</span>

          {/* Footer: stock indicator + add icon */}
          <div className="product-card-footer">
            <span
              className={
                product.inStock
                  ? 'product-card-stock product-card-stock--in'
                  : 'product-card-stock product-card-stock--out'
              }
            >
              <span
                className={
                  product.inStock
                    ? 'product-card-stock-dot product-card-stock-dot--in'
                    : 'product-card-stock-dot product-card-stock-dot--out'
                }
              />
              {product.inStock ? (
                <Localized id="product-lookup-in-stock">
                  <span />
                </Localized>
              ) : (
                <Localized id="product-lookup-out-of-stock">
                  <span />
                </Localized>
              )}
            </span>

            <span className="product-card-add-icon">
              <AddIcon />
            </span>
          </div>
        </button>
      </Localized>
    </div>
  );
}
