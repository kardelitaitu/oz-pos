import { useEffect, useMemo, useRef, useCallback, memo } from 'react';
import { requiredLocalized } from '@/frontend/shared';
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
  /** Whether the low-stock filter is active (shows a filter-specific empty state). */
  filterLowStock: boolean;
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
// Memoized: props are referentially stable (handlers are useCallbacks,
// catHue is a useCallback, formatMoney is module-level, and product
// objects come from the memoized pagedProducts slice) so cards skip
// re-renders when cart/totals change (P4).

const ProductCard = memo(function ProductCard({ product, catHue, formatMoney, handleAdd, handleEdit, handleOpenQtyPicker, scaleEnabled, onSetWeighTarget, outOfStockLabel, addToCartTitle, addToCartAria, editProductTitle, editProductAria, weighProductAria, priceChangedHint }: {
  product: ProductDto;
  catHue: (catId: string | null) => number;
  formatMoney: (m: Money) => string;
  handleAdd: (p: ProductDto) => void;
  handleEdit: (p: ProductDto) => void;
  handleOpenQtyPicker: (p: ProductDto) => void;
  scaleEnabled: boolean;
  onSetWeighTarget: (p: ProductDto) => void;
  outOfStockLabel: string;
  addToCartTitle: string;
  addToCartAria: string;
  editProductTitle: string;
  editProductAria: string;
  weighProductAria: string;
  priceChangedHint: string;
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
          <span className="retail-product-out-label">{outOfStockLabel}</span>
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
          aria-label={`${product.name} ${formatMoney(product.price)}${isOutOfStock ? ` (${outOfStockLabel.toLowerCase()})` : ''}`}
          aria-disabled={isOutOfStock}
          disabled={isOutOfStock}
        >
          <span>{product.name}</span>
          {priceRecent && <span className="retail-price-volatility-hint" title={priceChangedHint} />}
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
            title={addToCartTitle}
            aria-label={addToCartAria}
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
            title={editProductTitle}
            aria-label={editProductAria}
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
              aria-label={weighProductAria}
            >
              ⚖
            </button>
          )}
        </div>
      </td>
    </tr>
  );
});

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
    filterLowStock,
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
            title={requiredLocalized(l10n, 'retail-add-category-btn-title')}
            aria-label={requiredLocalized(l10n, 'retail-add-category-btn-aria')}
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
            title={requiredLocalized(l10n, 'retail-add-product-btn-title')}
            aria-label={requiredLocalized(l10n, 'retail-add-product-btn-aria')}
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
        <div className="retail-skeleton-grid">
          <table className="retail-skeleton-table">
            <thead>
              <tr>
                <th className="retail-col-sku"><span className="retail-skeleton-shimmer">&nbsp;</span></th>
                <th className="retail-col-stock"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--stock">&nbsp;</span></th>
                <th className="retail-col-name"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--wide">&nbsp;</span></th>
                <th className="retail-col-price"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--price">&nbsp;</span></th>
                <th className="retail-col-action"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--action">&nbsp;</span></th>
              </tr>
            </thead>
            <tbody role="status" aria-label={requiredLocalized(l10n, 'retail-products-loading')}>
              {Array.from({ length: 8 }).map((_, i) => (
                <tr key={i} className="retail-skeleton-row">
                  <td className="retail-col-sku"><span className="retail-skeleton-shimmer">&nbsp;</span></td>
                  <td className="retail-col-stock"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--stock">&nbsp;</span></td>
                  <td className="retail-col-name"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--wide">&nbsp;</span></td>
                  <td className="retail-col-price"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--price">&nbsp;</span></td>
                  <td className="retail-col-action"><span className="retail-skeleton-shimmer retail-skeleton-shimmer--action">&nbsp;</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : filteredProducts.length === 0 ? (
        <div className="retail-grid-empty" role="status">
          {filterLowStock
            ? (requiredLocalized(l10n, 'retail-no-low-stock-products'))
            : searchQuery.trim()
              ? (requiredLocalized(l10n, 'retail-no-products-match'))
              : activeCategory
                ? (requiredLocalized(l10n, 'retail-no-products-in-category'))
                : (requiredLocalized(l10n, 'retail-no-products'))}
        </div>
      ) : (
        <div className="retail-grid" data-testid="product-grid-scroll">
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
                    <span>{requiredLocalized(l10n, 'retail-col-sku')}</span>
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
                  <div className="retail-th-content retail-th-content--center">
                    <span>{requiredLocalized(l10n, 'retail-col-stock')}</span>
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
                    <span>{requiredLocalized(l10n, 'retail-col-name')}</span>
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
                  <div className="retail-th-content retail-th-content--end">
                    <span>{requiredLocalized(l10n, 'retail-col-price')}</span>
                    <span className="retail-sort-icon">
                      {sortField === 'price' ? (sortOrder === 'asc' ? ' ▲' : ' ▼') : ' ↕'}
                    </span>
                  </div>
                </th>
                <th className="retail-col-action">{requiredLocalized(l10n, 'retail-col-action')}</th>
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
                  outOfStockLabel={requiredLocalized(l10n, 'retail-product-out-of-stock')}
                  addToCartTitle={requiredLocalized(l10n, 'retail-product-add-title')}
                  addToCartAria={requiredLocalized(l10n, 'retail-product-add-aria', { name: p.name })}
                  editProductTitle={requiredLocalized(l10n, 'retail-product-edit-title')}
                  editProductAria={requiredLocalized(l10n, 'retail-product-edit-aria', { name: p.name })}
                  weighProductAria={requiredLocalized(l10n, 'retail-product-weigh-aria', { name: p.name })}
                  priceChangedHint={requiredLocalized(l10n, 'retail-price-volatility-hint')}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
      {totalPages > 1 && (
        <div className="retail-page-nav" role="navigation" aria-label={requiredLocalized(l10n, 'retail-page-nav-aria')}>
          <button type="button" className="retail-page-btn" disabled={productPage === 0} onClick={() => actions.onSetProductPage((p) => p - 1)} aria-label={requiredLocalized(l10n, 'retail-page-prev-aria')}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true"><path d="M15 18l-6-6 6-6" /></svg>
          </button>
          <span className="retail-page-info" aria-current="true">{productPage + 1} / {totalPages}</span>
          <button type="button" className="retail-page-btn" disabled={productPage >= totalPages - 1} onClick={() => actions.onSetProductPage((p) => p + 1)} aria-label={requiredLocalized(l10n, 'retail-page-next-aria')}>
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
          aria-label={requiredLocalized(l10n, 'retail-sku-lookup-aria')}
        >
          {l10n.getString('retail-sku-go')}
        </button>
      </div>
    </div>
  );
}
