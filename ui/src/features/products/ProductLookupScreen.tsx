import { useState, useMemo, useCallback } from 'react';
import { Localized } from '@/components/Localized';
import { formatMoney, type Product, type Sku } from '@/types/domain';
import './ProductLookupScreen.css';

// ── Sample product data (backed by IPC in a follow-up) ───────────

const SAMPLE_PRODUCTS: Product[] = [
  { sku: 'LATTE' as Sku, name: 'Caffè Latte', category: 'Beverages', price: { minor_units: 450, currency: 'USD' }, barcode: '4901234567890', inStock: true },
  { sku: 'CAPPU' as Sku, name: 'Cappuccino', category: 'Beverages', price: { minor_units: 420, currency: 'USD' }, barcode: '4901234567891', inStock: true },
  { sku: 'ESPR' as Sku, name: 'Espresso Shot', category: 'Beverages', price: { minor_units: 280, currency: 'USD' }, barcode: '4901234567892', inStock: true },
  { sku: 'MATCHA' as Sku, name: 'Matcha Latte', category: 'Beverages', price: { minor_units: 520, currency: 'USD' }, barcode: null, inStock: true },
  { sku: 'BAGEL' as Sku, name: 'Plain Bagel', category: 'Food', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567894', inStock: true },
  { sku: 'BAGEL-S' as Sku, name: 'Sesame Bagel', category: 'Food', price: { minor_units: 275, currency: 'USD' }, barcode: '4901234567895', inStock: true },
  { sku: 'CROISS' as Sku, name: 'Butter Croissant', category: 'Food', price: { minor_units: 350, currency: 'USD' }, barcode: '4901234567896', inStock: true },
  { sku: 'MUFFIN-B' as Sku, name: 'Blueberry Muffin', category: 'Food', price: { minor_units: 320, currency: 'USD' }, barcode: '4901234567897', inStock: true },
  { sku: 'MUFFIN-C' as Sku, name: 'Chocolate Muffin', category: 'Food', price: { minor_units: 340, currency: 'USD' }, barcode: null, inStock: false },
  { sku: 'SANDW-C' as Sku, name: 'Chicken Sandwich', category: 'Food', price: { minor_units: 750, currency: 'USD' }, barcode: '4901234567899', inStock: true },
  { sku: 'SANDW-V' as Sku, name: 'Veggie Sandwich', category: 'Food', price: { minor_units: 680, currency: 'USD' }, barcode: '4901234567900', inStock: true },
  { sku: 'COOKIE' as Sku, name: 'Chocolate Chip Cookie', category: 'Food', price: { minor_units: 195, currency: 'USD' }, barcode: '4901234567901', inStock: true },
  { sku: 'TEA-G' as Sku, name: 'Green Tea', category: 'Beverages', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567902', inStock: true },
  { sku: 'TEA-C' as Sku, name: 'Chai Tea', category: 'Beverages', price: { minor_units: 320, currency: 'USD' }, barcode: null, inStock: true },
  { sku: 'JUICE-O' as Sku, name: 'Orange Juice', category: 'Beverages', price: { minor_units: 380, currency: 'USD' }, barcode: '4901234567904', inStock: true },
  { sku: 'WATER-S' as Sku, name: 'Sparkling Water', category: 'Beverages', price: { minor_units: 180, currency: 'USD' }, barcode: '4901234567905', inStock: true },
  { sku: 'BROWNIE' as Sku, name: 'Fudge Brownie', category: 'Food', price: { minor_units: 295, currency: 'USD' }, barcode: '4901234567906', inStock: false },
  { sku: 'MUFFIN-BA' as Sku, name: 'Banana Muffin', category: 'Food', price: { minor_units: 310, currency: 'USD' }, barcode: null, inStock: true },
];

// ── Props ──────────────────────────────────────────────────────────

export interface ProductLookupScreenProps {
  /** Called when the user clicks "Add to cart" on a product. */
  onAddProduct?: (product: Product) => void;
}

// ── Helpers ────────────────────────────────────────────────────────

const CATEGORIES = ['All', ...Array.from(new Set(SAMPLE_PRODUCTS.map((p) => p.category)))] as const;

type Category = (typeof CATEGORIES)[number];

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
  const [searchQuery, setSearchQuery] = useState('');
  const [barcodeInput, setBarcodeInput] = useState('');
  const [activeCategory, setActiveCategory] = useState<Category>('All');

  // Filter products based on search, barcode, and category
  const filtered = useMemo(() => {
    let results = SAMPLE_PRODUCTS;

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
  }, [searchQuery, activeCategory]);

  // Handle barcode scan submission
  const handleBarcodeScan = useCallback(() => {
    if (!barcodeInput.trim()) return;
    const barcode = barcodeInput.trim();
    const found = SAMPLE_PRODUCTS.find((p) => p.barcode === barcode);
    if (found && found.inStock) {
      onAddProduct?.(found);
    }
    setBarcodeInput('');
  }, [barcodeInput, onAddProduct]);

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
        </div>

        <div className="product-barcode-wrapper">
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
          <button
            type="button"
            className="product-scan-btn"
            onClick={handleBarcodeScan}
            aria-label="Submit barcode"
          >
            <BarcodeIcon />
            <Localized id="product-lookup-barcode-scan">
              <span>Scan</span>
            </Localized>
          </button>
        </div>
      </div>

      {/* ── Category filters ───────────────────────── */}
      <div className="product-categories" role="radiogroup" aria-label="Filter by category">
        {CATEGORIES.map((cat) => (
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
                <span>All Categories</span>
              </Localized>
            ) : (
              cat
            )}
          </button>
        ))}
      </div>

      {/* ── Product grid ───────────────────────────── */}
      {filtered.length === 0 ? (
        <div className="product-empty">
          <PackageIcon />
          <span className="product-empty-text">
            <Localized id="product-lookup-no-results">
              <span>No products found</span>
            </Localized>
          </span>
        </div>
      ) : (
        <div className="product-grid" role="list" aria-label="Products">
          {filtered.map((product) => (
            <ProductCard
              key={product.sku}
              product={product}
              {...(onAddProduct ? { onAdd: onAddProduct } : {})}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ── ProductCard sub-component ──────────────────────────────────────

interface ProductCardProps {
  product: Product;
  onAdd?: (product: Product) => void;
}

function ProductCard({ product, onAdd }: ProductCardProps) {
  const handleAdd = useCallback(() => {
    onAdd?.(product);
  }, [product, onAdd]);

  return (
    <div
      className={`product-card${!product.inStock ? ' product-card--disabled' : ''}`}
      role="listitem"
    >
      {/* Clickable area wraps the entire card content */}
      <button
        type="button"
        className="product-card-btn"
        onClick={handleAdd}
        disabled={!product.inStock}
        aria-label={`${product.name} — ${formatMoney(product.price)}`}
      >
        {/* Row: name + category badge */}
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
                <span>In stock</span>
              </Localized>
            ) : (
              <Localized id="product-lookup-out-of-stock">
                <span>Out of stock</span>
              </Localized>
            )}
          </span>

          <span className="product-card-add-icon">
            <AddIcon />
          </span>
        </div>
      </button>
    </div>
  );
}
