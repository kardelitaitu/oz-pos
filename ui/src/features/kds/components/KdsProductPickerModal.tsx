import { useState, useEffect, useRef, useCallback, memo } from 'react';
import { useLocalization } from '@fluent/react';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import { listProductsScoped, type ProductDto } from '@/api/products';
import { type CreateKdsLineItemInput, type KdsModifier } from '@/api/kds';
import './KdsProductPickerModal.css';

/** Result emitted when the user confirms a selection. */
export interface ProductPickerResult {
  /** The KDS order ID to add items to. */
  orderId: string;
  /** Picked items with sku, display_name, qty, course, modifiers. */
  items: CreateKdsLineItemInput[];
}

/** Props for the KdsProductPickerModal. */
export interface KdsProductPickerModalProps {
  /** KDS order ID to add items to. */
  orderId: string;
  /** Session token for scoped API calls. */
  sessionToken: string;
  /** Whether the modal is open. */
  isOpen: boolean;
  /** Called when the user confirms the selection. */
  onConfirm: (result: ProductPickerResult) => void;
  /** Called when the modal is dismissed without saving. */
  onClose: () => void;
}

/** Selected product entry in the picker. */
interface PickedEntry {
  sku: string;
  display_name: string;
  qty: number;
  course: string | null;
}

/** Course display labels. */
const COURSE_OPTIONS: { value: string | null; label: string }[] = [
  { value: null, label: 'None' },
  { value: 'appetizer', label: 'Appetizer' },
  { value: 'main', label: 'Main' },
  { value: 'side', label: 'Side' },
  { value: 'dessert', label: 'Dessert' },
  { value: 'beverage', label: 'Beverage' },
];

/**
 * KdsProductPickerModal — searchable product selector for adding items
 * to a KDS order mid-preparation (TODO 3f).
 *
 * Fetches all products from the store, filters by restaurant/both type,
 * and lets the user pick items with quantities and course assignments.
 * Returns CreateKdsLineItemInput[] on confirm.
 */
export const KdsProductPickerModal = memo(function KdsProductPickerModal({
  orderId,
  sessionToken,
  isOpen,
  onConfirm,
  onClose,
}: KdsProductPickerModalProps) {
  const { l10n } = useLocalization();
  const panelRef = useRef<HTMLDivElement>(null);
  useFocusTrap(panelRef, isOpen, onClose);

  const [products, setProducts] = useState<ProductDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [picked, setPicked] = useState<PickedEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  // Fetch products when modal opens.
  useEffect(() => {
    if (!isOpen) return;
    setLoading(true);
    setSearch('');
    setPicked([]);
    setError(null);

    listProductsScoped(sessionToken)
      .then((all) => {
        // Only show restaurant/both type products.
        const restaurant = all.filter(
          (p) => p.product_type === 'restaurant' || p.product_type === 'both',
        );
        setProducts(restaurant);
        setLoading(false);
      })
      .catch((e) => {
        setError(String(e));
        setLoading(false);
      });

    // Focus search input on open.
    requestAnimationFrame(() => searchRef.current?.focus());
  }, [isOpen, sessionToken]);

  const filtered = products.filter(
    (p) =>
      !search ||
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.sku.toLowerCase().includes(search.toLowerCase()),
  );

  const addProduct = useCallback((product: ProductDto) => {
    setPicked((prev) => {
      const existing = prev.find((e) => e.sku === product.sku);
      if (existing) {
        return prev.map((e) =>
          e.sku === product.sku ? { ...e, qty: e.qty + 1 } : e,
        );
      }
      // Resolve course from product category.
      const category = product.category?.toLowerCase() ?? '';
      let course: string | null = null;
      if (category.includes('appetizer') || category.includes('starter')) course = 'appetizer';
      else if (category.includes('main') || category.includes('entree')) course = 'main';
      else if (category.includes('side')) course = 'side';
      else if (category.includes('dessert')) course = 'dessert';
      else if (category.includes('drink') || category.includes('beverage')) course = 'beverage';

      return [
        ...prev,
        { sku: product.sku, display_name: product.name, qty: 1, course },
      ];
    });
  }, []);

  const removeProduct = useCallback((sku: string) => {
    setPicked((prev) => prev.filter((e) => e.sku !== sku));
  }, []);

  const updateQty = useCallback((sku: string, qty: number) => {
    if (qty < 1) return;
    setPicked((prev) =>
      prev.map((e) => (e.sku === sku ? { ...e, qty } : e)),
    );
  }, []);

  const updateCourse = useCallback((sku: string, course: string | null) => {
    setPicked((prev) =>
      prev.map((e) => (e.sku === sku ? { ...e, course } : e)),
    );
  }, []);

  const handleConfirm = useCallback(() => {
    if (picked.length === 0) return;
    const items: CreateKdsLineItemInput[] = picked.map((p) => ({
      sku: p.sku,
      display_name: p.display_name,
      qty: p.qty,
      course: p.course,
      modifiers: [] as KdsModifier[],
    }));
    onConfirm({ orderId, items });
  }, [picked, orderId, onConfirm]);

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  if (!isOpen) return null;

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
    <div
      className="kds-picker-overlay"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-label={l10n.getString('kds-picker-title') || 'Add Items to Order'}
    >
      <div className="kds-picker-modal" ref={panelRef}>
        {/* ── Header ────────────────────────────────────────────── */}
        <div className="kds-picker-header">
          <h2 className="kds-picker-title">
            {l10n.getString('kds-picker-title') || 'Add Items to Order'}
          </h2>
          <button
            className="kds-picker-close"
            onClick={onClose}
            aria-label={l10n.getString('kds-picker-close-aria') || 'Close picker'}
          >
            &times;
          </button>
        </div>

        {/* ── Search ────────────────────────────────────────────── */}
        <div className="kds-picker-search-wrap">
          <input
            ref={searchRef}
            className="kds-picker-search"
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={l10n.getString('kds-picker-search-placeholder') || 'Search products...'}
            aria-label={l10n.getString('kds-picker-search-aria') || 'Search products'}
          />
        </div>

        {/* ── Error ─────────────────────────────────────────────── */}
        {error && <p className="kds-picker-error">{error}</p>}

        <div className="kds-picker-body">
          {/* ── Product list (left) ─────────────────────────────── */}
          <div className="kds-picker-products">
            {loading ? (
              <p className="kds-picker-loading">
                {l10n.getString('kds-picker-loading') || 'Loading products...'}
              </p>
            ) : filtered.length === 0 ? (
              <p className="kds-picker-empty">
                {l10n.getString('kds-picker-no-products') || 'No products found'}
              </p>
            ) : (
              filtered.map((product) => {
                const isPicked = picked.some((e) => e.sku === product.sku);
                return (
                  <button
                    key={product.sku}
                    className={`kds-picker-product${isPicked ? ' kds-picker-product--picked' : ''}`}
                    onClick={() => addProduct(product)}
                    aria-label={`${product.name}${isPicked ? ` (${l10n.getString('kds-picker-added-label') || 'added'})` : ''}`}
                  >
                    <span className="kds-picker-product-name">{product.name}</span>
                    <span className="kds-picker-product-course">
                      {product.category ?? ''}
                    </span>
                  </button>
                );
              })
            )}
          </div>

          {/* ── Picked items (right) ────────────────────────────── */}
          <div className="kds-picker-picked">
            <h3 className="kds-picker-picked-title">
              {l10n.getString('kds-picker-selected') || 'Selected'}{' '}
              <span className="kds-picker-picked-count">{picked.length}</span>
            </h3>
            {picked.length === 0 ? (
              <p className="kds-picker-picked-empty">
                {l10n.getString('kds-picker-picked-empty') || 'Click products to add them'}
              </p>
            ) : (
              <ul className="kds-picker-picked-list">
                {picked.map((entry) => (
                  <li key={entry.sku} className="kds-picker-picked-item">
                    <span className="kds-picker-picked-name">{entry.display_name}</span>
                    <div className="kds-picker-picked-controls">
                      {/* Course dropdown */}
                      <select
                        className="kds-picker-picked-course"
                        value={entry.course ?? ''}
                        onChange={(e) =>
                          updateCourse(entry.sku, e.target.value || null)
                        }
                        aria-label={l10n.getString('kds-picker-course-aria') || 'Course'}
                      >
                        {COURSE_OPTIONS.map((opt) => (
                          <option key={String(opt.value)} value={opt.value ?? ''}>
                            {opt.label}
                          </option>
                        ))}
                      </select>
                      {/* Quantity stepper */}
                      <div className="kds-picker-picked-qty">
                        <button
                          className="kds-picker-qty-btn"
                          onClick={() => updateQty(entry.sku, entry.qty - 1)}
                          disabled={entry.qty <= 1}
                          aria-label={l10n.getString('kds-picker-qty-decrease') || 'Decrease quantity'}
                        >
                          &minus;
                        </button>
                        <span className="kds-picker-qty-value" aria-live="polite">
                          {entry.qty}
                        </span>
                        <button
                          className="kds-picker-qty-btn"
                          onClick={() => updateQty(entry.sku, entry.qty + 1)}
                          aria-label={l10n.getString('kds-picker-qty-increase') || 'Increase quantity'}
                        >
                          +
                        </button>
                      </div>
                      {/* Remove */}
                      <button
                        className="kds-picker-picked-remove"
                        onClick={() => removeProduct(entry.sku)}
                        aria-label={l10n.getString('kds-picker-remove-aria', { name: entry.display_name }) || `Remove ${entry.display_name}`}
                      >
                        <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
                          <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                        </svg>
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {/* ── Footer ────────────────────────────────────────────── */}
        <div className="kds-picker-footer">
          <button
            className="kds-picker-cancel"
            onClick={onClose}
          >
            {l10n.getString('kds-picker-cancel') || 'Cancel'}
          </button>
          <button
            className="kds-picker-confirm"
            onClick={handleConfirm}
            disabled={picked.length === 0}
          >
            {l10n.getString('kds-picker-add-btn', { count: picked.length }) ||
              `Add ${picked.length} item(s)`}
          </button>
        </div>
      </div>
    </div>
  );
});
