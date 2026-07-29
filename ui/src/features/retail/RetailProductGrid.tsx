import { useEffect, useMemo, useRef, useCallback } from 'react';
import { useLocalization, Localized } from '@fluent/react';
import { formatMoney, type Money, type Sku } from '@/types/domain';
import type { ProductDto, CategoryDto } from '@/api/products';
import ScaleIndicator from './ScaleIndicator';

// ── Price volatility ───────────────────────────────────────────────

const PRICE_VOLATILITY_MS = 24 * 60 * 60 * 1000; // 24 h

function isPriceRecent(p: ProductDto): boolean {
  if (!p.price_updated_at) return false;
  const elapsed = Date.now() - new Date(p.price_updated_at).getTime();
  return elapsed >= 0 && elapsed < PRICE_VOLATILITY_MS;
}

// ── Sort types ─────────────────────────────────────────────────────

export type SortField = 'sku' | 'name' | 'stock' | 'price';
export type SortOrder = 'asc' | 'desc';

// ── Grouped prop interfaces ────────────────────────────────────────

export interface ProductGridData {
  productsLoading: boolean;
  categoriesLoading: boolean;
  categories: CategoryDto[];
  activeCategory: string | null;
  searchQuery: string;
  filteredProducts: ProductDto[];
  pagedProducts: ProductDto[];
  totalPages: number;
  productPage: number;
  sortField: SortField;
  sortOrder: SortOrder;
  allLabel: string;
  catLabels: Map<string, string>;
  skuInput: string;
  weighTarget: { sku: Sku; name: string } | null;
}

export interface ProductGridActions {
  onSetActiveCategory: (catId: string | null) => void;
  onSetSearchQuery: (q: string) => void;
  onSort: (field: SortField) => void;
  onSetProductPage: React.Dispatch<React.SetStateAction<number>>;
  onAddProduct: (p: ProductDto) => void;
  onEditProduct: (p: ProductDto) => void;
  onOpenQtyPicker: (p: ProductDto) => void;
  onSetWeighTarget: (p: ProductDto) => void;
  onClearWeighTarget: () => void;
  onAddCategory: () => void;
  onAddNewProduct: () => void;
  onSkuInputChange: (val: string) => void;
  onSkuSubmit: () => void;
  onWeighAdd: (sku: Sku, weightGrams: number) => void;
}

export interface RetailProductGridProps {
  data: ProductGridData;
  actions: ProductGridActions;
  isScaleEnabled: boolean;
  catHue: (catId: string | null) => number;
  skuInputRef: React.Ref<HTMLInputElement>;
}

// ── ProductCard sub-component ──────────────────────────────────────

function ProductCard({ product, catHue, formatMoney, handleAdd, handleEdit, handleOpenQtyPicker, scaleEnabled, onSetWeighTarget, outOfStockLabel }: {
  product: ProductDto;
  catHue: (catId: string | null) => number;
  formatMoney: (m: Money) => string;
  handleAdd: (p: ProductDto) => void;
  handleEdit: (p: ProductDto) => void;
  handleOpenQtyPicker: (p: ProductDto) => void;
  scaleEnabled: boolean;
  onSetWeighTarget: (p: ProductDto) => void;
  outOfStockLabel: string;
}) {
  const isOutOfStock = !product.in_stock || (product.stock_qty != null && product.stock_qty <= 0);
  const priceRecent = useMemo(() => isPriceRecent(product), [product]);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isLongPress = useRef(false);
  // Refs to avoid stale closures in the long-press timeout (P2-7).
  const productRef = useRef(product);
  productRef.current = product;
  const handleOpenQtyPickerRef = useRef(handleOpenQtyPicker);
  handleOpenQtyPickerRef.current = handleOpenQtyPicker;
  const handleAddRef = useRef(handleAdd);
  handleAddRef.current = handleAdd;

  const lowThreshold = product.low_stock_threshold ?? 5;
  const highThreshold = product.high_stock_threshold ?? 10;
  const stockLevel =
    product.stock_qty != null && product.stock_qty <= lowThreshold
      ? 'low'
      : product.stock_qty != null && product.stock_qty <= highThreshold
      ? 'medium'
      : 'high';

  const handlePointerDown = useCallback(() => {
    if (isOutOfStock) return;
    isLongPress.current = false;
    longPressTimer.current = setTimeout(() => {
      isLongPress.current = true;
      handleOpenQtyPickerRef.current(productRef.current);
    }, 400);
  }, [isOutOfStock]);

  const handlePointerUp = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
    if (!isLongPress.current && !isOutOfStock) handleAddRef.current(productRef.current);
  }, [isOutOfStock]);

  const handlePointerLeave = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
  }, []);

  // Clear the long-press timer on unmount (stale timeout after product list change)
  useEffect(() => {
    return () => {
      if (longPressTimer.current) clearTimeout(longPressTimer.current);
    };
  }, []);

  return (
    <tr className={`retail-product-row${isOutOfStock ? ' retail-product-row--out-of-stock' : ''}`}>
      <td className="retail-col-sku">{product.sku}</td>
      <td className="retail-col-stock">
        {product.stock_qty != null && product.stock_qty > 0 ? (
          <span className={`retail-product-stock-badge retail-stock-${stockLevel}`}>
            {product.stock_qty}
          </span>
        ) : (
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-fg-tertiary)' }}>{outOfStockLabel}</span>
        )}
      </td>
      <td className="retail-col-name">
        <button
          type="button"
          className={`retail-product-btn${isOutOfStock ? ' retail-product-btn--out-of-stock' : ''}`}
          style={{ '--cat-hue': catHue(product.category) } as React.CSSProperties}
          onPointerDown={handlePointerDown}
          onPointerUp={handlePointerUp}
          onPointerLeave={handlePointerLeave}
          aria-label={`${product.name} ${formatMoney(product.price)}${isOutOfStock ? ' (out of stock)' : ''}`}
          aria-disabled={isOutOfStock}
          disabled={isOutOfStock}
        >
          <span>{product.name}</span>
          {priceRecent && <span className="retail-price-volatility-hint" title="Price changed recently" />}
        </button>
      </td>
      <td className="retail-col-price">{formatMoney(product.price)}</td>
      <td className="retail-col-action">
        <div className="retail-col-action-group">
          <button
            type="button"
            className="retail-table-add-btn"
            disabled={isOutOfStock}
            onClick={() => {
              if (!isOutOfStock) handleAdd(product);
            }}
            title="Add to Cart"
            aria-label={`Add ${product.name} to cart`}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
              <circle cx="9" cy="21" r="1" />
              <circle cx="20" cy="21" r="1" />
              <path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6" />
            </svg>
          </button>
          <button
            type="button"
            className="retail-table-edit-btn"
            onClick={(e) => {
              e.stopPropagation();
              e.preventDefault();
              handleEdit(product);
            }}
            title="Edit Product"
            aria-label={`Edit ${product.name}`}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
              <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
          </button>
          {scaleEnabled && (
            <button
              type="button"
              className="retail-product-weigh-btn"
              onClick={(e) => { e.stopPropagation(); e.preventDefault(); onSetWeighTarget(product); }}
              aria-label={`Weigh ${product.name}`}
            >
              ⚖
            </button>
          )}
        </div>
      </td>
    </tr>
  );
}

// ── Main component ─────────────────────────────────────────────────

/** Product grid — categories, search, product table with sorting, pagination, SKU input, and scale indicator. */
export default function RetailProductGrid({
  data,
  actions,
  isScaleEnabled,
  catHue,
  skuInputRef,
}: RetailProductGridProps) {
  const { l10n } = useLocalization();

  const {
    productsLoading,
    categoriesLoading,
    categories,
    activeCategory,
    searchQuery,
    filteredProducts,
    pagedProducts,
    totalPages,
    productPage,
    sortField,
    sortOrder,
    allLabel,
    catLabels,
    skuInput,
    weighTarget,
  } = data;

  return (
    <div className="retail-products">
      <div
        className="retail-categories"
        onWheel={(e) => {
          if (e.deltaY) {
            e.currentTarget.scrollLeft += e.deltaY;
          }
        }}
      >
        <button
          className={`retail-cat-btn${!activeCategory ? ' retail-cat-btn--active' : ''}`}
          onClick={() => actions.onSetActiveCategory(null)}
        >
          {allLabel}
        </button>
        {categories.map((cat) => (
          <button
            key={cat.id}
            className={`retail-cat-btn${activeCategory === cat.id ? ' retail-cat-btn--active' : ''}`}
            onClick={() => actions.onSetActiveCategory(cat.id)}
            aria-label={catLabels.get(cat.id) ?? cat.name}
            aria-pressed={activeCategory === cat.id}
          >
            {catLabels.get(cat.id) ?? cat.name}
          </button>
        ))}
        <Localized id="retail-add-category-btn">
          <button
            type="button"
            className="retail-cat-btn retail-cat-btn--add"
            onClick={actions.onAddCategory}
            title="Add new category"
            aria-label="Add new category"
          >
            + Category
          </button>
        </Localized>
      </div>

      {/* ── Search bar ────────────────────── */}
      <div className="retail-search-bar">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
          <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          className="retail-search-input"
          type="text"
          value={searchQuery}
          onChange={(e) => actions.onSetSearchQuery(e.target.value)}
          placeholder={l10n.getString('retail-search-placeholder')}
        />
        {searchQuery && (
          <button type="button" className="retail-search-clear" onClick={() => actions.onSetSearchQuery('')} aria-label={l10n.getString('retail-search-clear-aria')}>
            &times;
          </button>
        )}
        <Localized id="retail-add-product-btn">
          <button
            type="button"
            className="retail-add-product-btn"
            onClick={actions.onAddNewProduct}
            title="Add new product"
            aria-label="Add new product"
          >
            + Product
          </button>
        </Localized>
      </div>

      {isScaleEnabled && (
        <ScaleIndicator
          weighTarget={weighTarget}
          onWeighAdd={actions.onWeighAdd}
          onClearWeighTarget={actions.onClearWeighTarget}
        />
      )}



      {productsLoading || categoriesLoading ? (
        <div className="retail-grid">
          <div className="retail-grid-loading" role="status" aria-label={l10n.getString('retail-products-loading') || 'Loading products'}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="24" height="24" aria-hidden="true">
              <path d="M21 12a9 9 0 11-6.219-8.56" />
            </svg>
            <span>{l10n.getString('retail-products-loading') || 'Loading products…'}</span>
          </div>
        </div>
      ) : filteredProducts.length === 0 ? (
        <div className="retail-grid-empty">
          {searchQuery.trim()
            ? (l10n.getString('retail-no-products-match') || 'No products match your search')
            : activeCategory
              ? (l10n.getString('retail-no-products-in-category') || 'No products in this category')
              : (l10n.getString('retail-no-products') || 'No products')}
        </div>
      ) : (
        <div className="retail-grid">
          <table className="retail-product-table">
            <thead>
              <tr>
                <th
                  className="retail-col-sku retail-col-sortable"
                  onClick={() => actions.onSort('sku')}
                  role="columnheader"
                  aria-sort={sortField === 'sku' ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'}
                >
                  <div className="retail-th-content">
                    <span>{l10n.getString('retail-col-sku') || 'SKU / Code'}</span>
                    <span className="retail-sort-icon">
                      {sortField === 'sku' ? (sortOrder === 'asc' ? ' ▲' : ' ▼') : ' ↕'}
                    </span>
                  </div>
                </th>
                <th
                  className="retail-col-stock retail-col-sortable"
                  onClick={() => actions.onSort('stock')}
                  role="columnheader"
                  aria-sort={sortField === 'stock' ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'}
                >
                  <div className="retail-th-content" style={{ justifyContent: 'center' }}>
                    <span>{l10n.getString('retail-col-stock') || 'Stock'}</span>
                    <span className="retail-sort-icon">
                      {sortField === 'stock' ? (sortOrder === 'asc' ? ' ▲' : ' ▼') : ' ↕'}
                    </span>
                  </div>
                </th>
                <th
                  className="retail-col-name retail-col-sortable"
                  onClick={() => actions.onSort('name')}
                  role="columnheader"
                  aria-sort={sortField === 'name' ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'}
                >
                  <div className="retail-th-content">
                    <span>{l10n.getString('retail-col-name') || 'Product Name'}</span>
                    <span className="retail-sort-icon">
                      {sortField === 'name' ? (sortOrder === 'asc' ? ' ▲' : ' ▼') : ' ↕'}
                    </span>
                  </div>
                </th>
                <th
                  className="retail-col-price retail-col-sortable"
                  onClick={() => actions.onSort('price')}
                  role="columnheader"
                  aria-sort={sortField === 'price' ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'}
                >
                  <div className="retail-th-content" style={{ justifyContent: 'flex-end' }}>
                    <span>{l10n.getString('retail-col-price') || 'Price'}</span>
                    <span className="retail-sort-icon">
                      {sortField === 'price' ? (sortOrder === 'asc' ? ' ▲' : ' ▼') : ' ↕'}
                    </span>
                  </div>
                </th>
                <th className="retail-col-action">{l10n.getString('retail-col-action') || 'Action'}</th>
              </tr>
            </thead>
            <tbody>
              {pagedProducts.map((p) => (
                <ProductCard
                  key={p.sku}
                  product={p}
                  catHue={catHue}
                  formatMoney={formatMoney}
                  handleAdd={actions.onAddProduct}
                  handleEdit={actions.onEditProduct}
                  handleOpenQtyPicker={actions.onOpenQtyPicker}
                  scaleEnabled={isScaleEnabled}
                  onSetWeighTarget={actions.onSetWeighTarget}
                  outOfStockLabel={l10n.getString('retail-product-out-of-stock') || 'Out of stock'}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
      {totalPages > 1 && (
        <div className="retail-page-nav" role="navigation" aria-label={l10n.getString('retail-page-nav-aria') || 'Product pages'}>
          <button type="button" className="retail-page-btn" disabled={productPage === 0} onClick={() => actions.onSetProductPage((p) => p - 1)} aria-label={l10n.getString('retail-page-prev-aria') || 'Previous page'}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true"><path d="M15 18l-6-6 6-6" /></svg>
          </button>
          <span className="retail-page-info" aria-current="true">{productPage + 1} / {totalPages}</span>
          <button type="button" className="retail-page-btn" disabled={productPage >= totalPages - 1} onClick={() => actions.onSetProductPage((p) => p + 1)} aria-label={l10n.getString('retail-page-next-aria') || 'Next page'}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true"><path d="M9 18l6-6-6-6" /></svg>
          </button>
        </div>
      )}
      <div className="retail-sku-bar">
        <span className="retail-sku-label">{l10n.getString('retail-sku-label')}</span>
        <input
          ref={skuInputRef}
          className="retail-sku-input"
          type="text"
          value={skuInput}
          onChange={(e) => actions.onSkuInputChange(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') actions.onSkuSubmit(); }}
          placeholder={l10n.getString('retail-sku-placeholder')}
        />
        <button
          className="retail-sku-go-btn"
          onClick={actions.onSkuSubmit}
          aria-label="Look up SKU"
        >
          {l10n.getString('retail-sku-go')}
        </button>
      </div>
    </div>
  );
}
