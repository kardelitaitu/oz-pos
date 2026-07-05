import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import type { CSSProperties } from 'react';
import { usePosState } from '@/features/sales/usePosState';
import { useBarcodeScanner } from '@/features/sales/useBarcodeScanner';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { useLocalization } from '@fluent/react';
import PaymentModal from '@/features/sales/PaymentModal';
import { listProducts, listCategories, lookupProductBySku, lookupByBarcode, type ProductDto, type CategoryDto } from '@/api/products';
import { getActiveShift, openShift, closeShift, type ShiftDto } from '@/api/shifts';
import { getStoreSettings, listCreditSales, settleCredit, type StoreSettingsDto, type CreditSaleDto } from '@/api/settings';
import { formatMoney, type Money, type Sku } from '@/types/domain';
import RetailOptionsScreen from './RetailOptionsScreen';
import './RetailPosScreen.css';

// ── Cart panel width, viewport-aware ──────────────────────────────────

const RETAIL_CART_WIDTH_MIN = 280;
const RETAIL_CART_WIDTH_DEFAULT = 340;
const RETAIL_CART_WIDTH_MAX_CAP = 800;

function clampRetailCartWidth(px: number, viewportWidth: number): number {
  const max = Math.max(
    RETAIL_CART_WIDTH_MIN,
    Math.min(viewportWidth * 0.5, RETAIL_CART_WIDTH_MAX_CAP),
  );
  return Math.max(RETAIL_CART_WIDTH_MIN, Math.min(Math.round(px), max));
}

function toProduct(p: ProductDto): {
  sku: Sku; name: string; category: string; price: Money;
  barcode: string | null; inStock: boolean; stockQty: number | null; createdAt?: string;
} {
  return {
    sku: p.sku as Sku,
    name: p.name,
    category: p.category ?? '',
    price: { minor_units: p.price.minor_units, currency: p.price.currency },
    barcode: p.barcode,
    inStock: p.in_stock,
    stockQty: p.stock_qty,
    createdAt: p.created_at,
  };
}

export default function RetailPosScreen() {
  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { session } = useAuth();
  const userId = session!.user_id;

  const {
    lines, total, subtotal, discountPercent, discountLabel, discountAmount,
    addProduct, removeLine, updateQty, setDiscount, resetCart,
  } = usePosState();

  const lineCount = lines.reduce((a, l) => a + l.qty, 0);

  // ── Cart panel resize state ───────────────────────────────────────
  const [retailCartWidth, setRetailCartWidth] = useState(() => {
    const saved = localStorage.getItem('retail-cart-width');
    const parsed = saved ? parseInt(saved, 10) : NaN;
    const initial = Number.isFinite(parsed) && parsed > 0 ? parsed : RETAIL_CART_WIDTH_DEFAULT;
    const vw = typeof window !== 'undefined' ? window.innerWidth : RETAIL_CART_WIDTH_DEFAULT * 2;
    return clampRetailCartWidth(initial, vw);
  });
  const isResizing = useRef(false);
  const retailPosRef = useRef<HTMLDivElement>(null);

  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizing.current = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, []);

  useEffect(() => {
    const stopResize = () => {
      if (!isResizing.current) return;
      isResizing.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    const onMouseMove = (e: MouseEvent) => {
      if (!isResizing.current || !retailPosRef.current) return;
      const rect = retailPosRef.current.getBoundingClientRect();
      const clamped = clampRetailCartWidth(rect.right - e.clientX, window.innerWidth);
      setRetailCartWidth(clamped);
      localStorage.setItem('retail-cart-width', String(clamped));
    };
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', stopResize);
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', stopResize);
      stopResize();
    };
  }, []);

  useEffect(() => {
    const onResize = () => {
      setRetailCartWidth((w) => {
        const clamped = clampRetailCartWidth(w, window.innerWidth);
        localStorage.setItem('retail-cart-width', String(clamped));
        return clamped;
      });
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // ── Undo stack ───────────────────────────────────────────────────
  const MAX_UNDO = 5;
  const [undoStack, setUndoStack] = useState<{ sku: Sku; name: string; category: string; unit_price: Money }[]>([]);

  const handleRemoveLine = useCallback((id: string, line: { sku: Sku; name: string; category: string; unit_price: Money }) => {
    removeLine(id as any);
    setUndoStack((prev) => [line, ...prev].slice(0, MAX_UNDO));
  }, [removeLine]);

  const handleUndoRemove = useCallback(() => {
    if (undoStack.length === 0) return;
    const item = undoStack[0]!;
    addProduct({ ...item, price: item.unit_price, barcode: null, inStock: true, stockQty: null });
    setUndoStack((prev) => prev.slice(1));
  }, [undoStack, addProduct]);

  const handleDismissUndo = useCallback(() => {
    setUndoStack([]);
  }, []);

  // ── Quantity picker ──────────────────────────────────────────────
  const [showQtyPicker, setShowQtyPicker] = useState(false);
  const [pendingProduct, setPendingProduct] = useState<ProductDto | null>(null);
  const [qtyInput, setQtyInput] = useState('1');

  const handleOpenQtyPicker = useCallback((p: ProductDto) => {
    setPendingProduct(p);
    setQtyInput('1');
    setShowQtyPicker(true);
  }, []);

  const handleConfirmQty = useCallback(() => {
    if (!pendingProduct) return;
    const qty = Math.max(1, parseInt(qtyInput, 10) || 1);
    for (let i = 0; i < qty; i++) addProduct(toProduct(pendingProduct));
    setShowQtyPicker(false);
    setPendingProduct(null);
  }, [pendingProduct, qtyInput, addProduct]);

  // ── Keyboard shortcut overlay ────────────────────────────────────
  const [showShortcuts, setShowShortcuts] = useState(false);

  // ── Barcode scan flash ───────────────────────────────────────────
  const [scanFlash, setScanFlash] = useState(false);

  // ── Confirm clear cart ────────────────────────────────────────────
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const handleRequestClear = useCallback(() => {
    if (lines.length === 0) return;
    setShowClearConfirm(true);
  }, [lines.length]);

  const handleConfirmClear = useCallback(() => {
    resetCart();
    setUndoStack([]);
    setShowClearConfirm(false);
  }, [resetCart]);

  // ── Recent products (last 8) ──────────────────────────────────────
  const MAX_RECENT = 8;
  const [recentProducts, setRecentProducts] = useState<ProductDto[]>([]);

  const addToRecent = useCallback((p: ProductDto) => {
    setRecentProducts((prev) => {
      const filtered = prev.filter((x) => x.sku !== p.sku);
      const next = [p, ...filtered].slice(0, MAX_RECENT);
      return next;
    });
  }, []);

  // ── Products & Categories ────────────────────────────────────

  const [products, setProducts] = useState<ProductDto[]>([]);
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);

  useEffect(() => {
    listProducts().then(setProducts).catch(() => {});
    listCategories().then((cats) => {
      setCategories(cats);
      const first = cats[0];
      if (first) setActiveCategory(first.id);
    }).catch(() => {});
  }, []);

  const [searchQuery, setSearchQuery] = useState('');

  const allLabel = l10n.getString('product-lookup-all-categories') || 'All';
  const catLabels = useMemo(() => {
    const m = new Map<string, string>();
    categories.forEach((c) => {
      const label = l10n.getString(`category-${c.id}`);
      m.set(c.id, label || c.name);
    });
    return m;
  }, [categories, l10n]);

  const filteredProducts = useMemo(() => {
    let list = products;
    if (activeCategory) list = list.filter((p) => p.category === activeCategory);
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      list = list.filter((p) => p.name.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q));
    }
    return list;
  }, [products, activeCategory, searchQuery]);

  const catHue = useCallback((catId: string | null) => {
    if (!catId) return 210;
    let h = 0;
    for (let i = 0; i < catId.length; i++) h = (h * 31 + catId.charCodeAt(i)) | 0;
    return Math.abs(h) % 360;
  }, []);

  const handleAdd = useCallback((p: ProductDto) => {
    addProduct(toProduct(p));
    addToRecent(p);
  }, [addProduct, addToRecent]);

  // ── SKU / Barcode input ──────────────────────────────────────

  const productsRef = useRef(products);
  productsRef.current = products;

  const [skuInput, setSkuInput] = useState('');
  const skuInputRef = useRef<HTMLInputElement>(null);
  const handleSkuSubmit = useCallback(async () => {
    const val = skuInput.trim();
    if (!val) return;
    setSkuInput('');
    const list = productsRef.current;
    const p = list.find((x) => x.sku === val || x.barcode === val);
    if (p) { handleAdd(p); return; }
    try {
      const found = await lookupProductBySku(val);
      if (found) { handleAdd(found); return; }
    } catch {}
    addToast({ message: l10n.getString('pos-no-barcode-match') || 'Product not found', type: 'warning' });
  }, [skuInput, handleAdd, addToast, l10n]);

  const handleBarcode = useCallback(async (payload: { code: string }) => {
    const list = productsRef.current;
    const found = list.find((x) => x.barcode === payload.code);
    if (found) { handleAdd(found); setScanFlash(true); setTimeout(() => setScanFlash(false), 300); return; }
    try {
      const p = await lookupByBarcode(payload.code);
      if (p) { handleAdd(p); setScanFlash(true); setTimeout(() => setScanFlash(false), 300); return; }
    } catch {}
    addToast({ message: l10n.getString('pos-no-barcode-match') || 'Product not found', type: 'warning' });
  }, [handleAdd, addToast, l10n]);

  useBarcodeScanner({ onProductFound: handleBarcode });

  // ── Store settings ──────────────────────────────────────────

  const [storeSettings, setStoreSettings] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  useEffect(() => {
    getStoreSettings().then(setStoreSettings).catch(() => {});
  }, []);

  // ── Shift management ─────────────────────────────────────────

  const [activeShift, setActiveShift] = useState<ShiftDto | null>(null);
  const [shiftLoading, setShiftLoading] = useState(true);
  const [showOpenShift, setShowOpenShift] = useState(false);
  const [showCloseShift, setShowCloseShift] = useState(false);
  const [openingBalance, setOpeningBalance] = useState('');
  const [closingBalance, setClosingBalance] = useState('');
  const [shiftNotes, setShiftNotes] = useState('');
  const [openingShift, setOpeningShift] = useState(false);
  const [closingShift, setClosingShift] = useState(false);
  const [closeShiftError, setCloseShiftError] = useState<string | null>(null);
  const [closedShiftSummary, setClosedShiftSummary] = useState<ShiftDto | null>(null);

  useEffect(() => {
    setActiveShift(null);
    setShiftLoading(true);
    getActiveShift(userId)
      .then((s) => setActiveShift(s))
      .catch(() => setActiveShift(null))
      .finally(() => setShiftLoading(false));
  }, [userId]);

  const handleOpenShift = useCallback(async () => {
    const val = Math.round(parseFloat(openingBalance) * 100);
    if (Number.isNaN(val) || val < 0) return;
    setOpeningShift(true);
    try {
      const s = await openShift(userId, val);
      setActiveShift(s);
      setShowOpenShift(false);
      setOpeningBalance('');
    } catch {
      addToast({ message: 'Failed to open shift', type: 'error' });
    } finally {
      setOpeningShift(false);
    }
  }, [openingBalance, userId, addToast]);

  const handleCloseShift = useCallback(async () => {
    if (!activeShift) return;
    const val = Math.round(parseFloat(closingBalance) * 100);
    if (Number.isNaN(val) || val < 0) return;
    setClosingShift(true);
    setCloseShiftError(null);
    try {
      const s = await closeShift(activeShift.id, val, shiftNotes || null);
      setClosedShiftSummary(s);
      setActiveShift(null);
    } catch (e: any) {
      setCloseShiftError(e?.message ?? 'Failed to close shift');
    } finally {
      setClosingShift(false);
    }
  }, [activeShift, closingBalance, shiftNotes]);

  // ── Discount modal ───────────────────────────────────────────

  const [showDiscount, setShowDiscount] = useState(false);
  const [discountInput, setDiscountInput] = useState('');

  const handleApplyDiscount = useCallback(() => {
    const pct = parseFloat(discountInput);
    if (Number.isNaN(pct) || pct < 0 || pct > 100) return;
    setDiscount(pct, '');
    setShowDiscount(false);
    setDiscountInput('');
  }, [discountInput, setDiscount]);

  // ── Payment modal ────────────────────────────────────────────

  const [showPayment, setShowPayment] = useState(false);

  const handlePay = useCallback(() => {
    if (!activeShift) { addToast({ message: 'Open a shift first', type: 'warning' }); return; }
    setShowPayment(true);
  }, [activeShift, addToast]);

  // ── Hold cart ────────────────────────────────────────────────

  const [heldCart, setHeldCart] = useState<{ lines: { sku: Sku; name: string; category: string; unit_price: Money }[]; discount?: number; discountLabel?: string } | null>(null);

  const handleHold = useCallback(() => {
    if (lines.length === 0) return;
    setHeldCart({ lines: lines.map((l) => ({ sku: l.sku as Sku, name: l.name ?? '', category: l.category ?? '', unit_price: l.unit_price })), discount: discountPercent, discountLabel });
    resetCart();
    addToast({ message: 'Order held', type: 'success' });
  }, [lines, discountPercent, discountLabel, resetCart, addToast]);

  const handleResume = useCallback(() => {
    if (!heldCart) return;
    heldCart.lines.forEach((l) => addProduct({ ...l, price: l.unit_price, barcode: null, inStock: true, stockQty: null }));
    if (heldCart.discount) setDiscount(heldCart.discount, heldCart.discountLabel ?? '');
    setHeldCart(null);
  }, [heldCart, addProduct, setDiscount]);

  // ── Clock ────────────────────────────────────────────────────

  // ── Options full-screen page ─────────────────────────────────

  const [showOptions, setShowOptions] = useState(false);

  // ── Credit reminders ──────────────────────────────────────────

  const [creditSales, setCreditSales] = useState<CreditSaleDto[]>([]);
  const [showCreditList, setShowCreditList] = useState(false);
  const [settlingId, setSettlingId] = useState<string | null>(null);
  const roleId = session!.role_id;

  const loadCreditSales = useCallback(async () => {
    try {
      const list = await listCreditSales();
      setCreditSales(list.filter((c) => !c.settledAt));
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadCreditSales();
  }, [loadCreditSales]);

  const handleSettleCredit = useCallback(async (saleId: string) => {
    setSettlingId(saleId);
    try {
      await settleCredit(saleId, roleId);
      setCreditSales((prev) => prev.filter((c) => c.saleId !== saleId));
      addToast({ message: 'Credit settled', type: 'success' });
    } catch {
      addToast({ message: 'Failed to settle credit', type: 'error' });
    } finally {
      setSettlingId(null);
    }
  }, [roleId, addToast]);

  // ── Clock ────────────────────────────────────────────────────

  const [clock, setClock] = useState(new Date());
  useEffect(() => {
    const id = setInterval(() => setClock(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const timeStr = clock.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' });

  // ── Keyboard shortcuts ────────────────────────────────────────

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (showOptions || showPayment || showOpenShift || showCloseShift || showDiscount || showQtyPicker || showShortcuts || showCreditList || showClearConfirm) return;
      switch (e.key) {
        case 'F1': handlePay(); break;
        case 'F2': if (lines.length > 0) handleRequestClear(); break;
        case 'F3': if (lines.length > 0) setShowDiscount(true); break;
        case 'F4': heldCart ? handleResume() : handleHold(); break;
        case 'F5': skuInputRef.current?.focus(); break;
        case 'F6': addToast({ message: 'Sales history coming soon', type: 'info' }); break;
        case 'F7': addToast({ message: 'Customer lookup coming soon', type: 'info' }); break;
        case 'F8': addToast({ message: 'Stock inquiry coming soon', type: 'info' }); break;
        case 'F9': activeShift ? setShowCloseShift(true) : setShowOpenShift(true); break;
        case 'F10': if (session?.role_name !== 'cashier') setShowOptions(true); break;
        case 'F11': case '?': setShowShortcuts((v) => !v); break;
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [showOptions, showPayment, showOpenShift, showCloseShift, showDiscount, showQtyPicker, showShortcuts, showClearConfirm, handlePay, lines.length, handleRequestClear, handleHold, handleResume, heldCart, activeShift, session, addToast]);

  // ── Render ───────────────────────────────────────────────────

  if (showOptions) {
    return <RetailOptionsScreen onClose={() => setShowOptions(false)} />;
  }

  if (showPayment && total) {
    return (
      <PaymentModal
        open
        lineItems={lines.map((l) => ({
          ...l, sku: l.sku, name: l.name ?? '', qty: l.qty, unit_price: l.unit_price,
        }))}
        total={total}
        discountPercent={discountPercent}
        discountLabel={discountLabel}
        userId={userId}
        onClose={() => setShowPayment(false)}
        onComplete={() => { setShowPayment(false); resetCart(); addToast({ message: 'Sale complete', type: 'success' }); }}
      />
    );
  }

  return (
    <div className="retail-pos">
      {/* ── Header ──────────────────────────── */}
      <header className="retail-header">
        <div className="retail-header-store">
          {storeSettings.logo && (
            <img src={`data:image/png;base64,${storeSettings.logo}`} alt="" className="retail-header-logo" style={{ height: 32, marginRight: 8 }} />
          )}
          <div>
            <span className="retail-header-name">{storeSettings.name || 'TOKO'}</span>
            {storeSettings.branch && <span className="retail-header-branch"> &middot; {storeSettings.branch}</span>}
            <span className="retail-header-address">{storeSettings.address || ''}</span>
          </div>
        </div>
        <div className="retail-header-right">
          {shiftLoading ? (
            <span className="retail-shift-badge">Loading…</span>
          ) : activeShift ? (
            <span className="retail-shift-badge">
              Shift &middot; {formatMoney({ minor_units: activeShift.totalSalesMinor, currency: 'IDR' })}
            </span>
          ) : (
            <span className="retail-shift-badge" style={{ opacity: 0.6 }}>No shift</span>
          )}
          <div className="retail-header-cashier">
            <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
              <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
            </svg>
            <span>{session?.display_name ?? ''}</span>
          </div>
          <span className="retail-header-clock">{timeStr}</span>
        </div>
      </header>

      {/* ── Main area ───────────────────────── */}
      <div className="retail-main" ref={retailPosRef}>
        {/* Left: product grid */}
        <div className="retail-products">
          <div className="retail-categories">
            <button
              className={`retail-cat-btn${!activeCategory ? ' retail-cat-btn--active' : ''}`}
              onClick={() => setActiveCategory(null)}
            >
              {allLabel}
            </button>
            {categories.map((cat) => (
              <button
                key={cat.id}
                className={`retail-cat-btn${activeCategory === cat.id ? ' retail-cat-btn--active' : ''}`}
                onClick={() => setActiveCategory(cat.id)}
              >
                {catLabels.get(cat.id) ?? cat.name}
              </button>
            ))}
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
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Cari produk…"
            />
            {searchQuery && (
              <button className="retail-search-clear" onClick={() => setSearchQuery('')} aria-label="Clear search">
                &times;
              </button>
            )}
          </div>

          {/* ── Recent products ──────────────── */}
          {recentProducts.length > 0 && !searchQuery.trim() && !activeCategory && (
            <div className="retail-recent-strip">
              <span className="retail-recent-label">Recent</span>
              <div className="retail-recent-items">
                {recentProducts.map((p) => (
                  <button
                    key={p.sku}
                    className="retail-recent-btn"
                    onClick={() => handleAdd(p)}
                  >
                    <span className="retail-recent-name">{p.name}</span>
                    <span className="retail-recent-price">{formatMoney(p.price)}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {filteredProducts.length === 0 ? (
            <div className="retail-grid-empty">
              {searchQuery.trim() ? 'No products match your search' : 'No products'}
            </div>
          ) : (
            <div className="retail-grid">
              {filteredProducts.map((p) => (
                <button
                  key={p.sku}
                  className="retail-product-btn"
                  style={{ '--cat-hue': catHue(p.category) } as React.CSSProperties}
                  onClick={() => handleOpenQtyPicker(p)}
                >
                  {p.stock_qty != null && p.stock_qty <= 5 && (
                    <span className="retail-product-stock-badge">{p.stock_qty}</span>
                  )}
                  <span className="retail-product-name">{p.name}</span>
                  <span className="retail-product-price">{formatMoney(p.price)}</span>
                </button>
              ))}
            </div>
          )}
          <div className="retail-sku-bar">
            <span className="retail-sku-label">SKU</span>
            <input
              ref={skuInputRef}
              className="retail-sku-input"
              type="text"
              value={skuInput}
              onChange={(e) => setSkuInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleSkuSubmit(); }}
              placeholder="Scan or type barcode / SKU"
            />
            <button
              style={{
                padding: '4px 12px', background: '#1a3a5c', color: '#fff',
                border: 'none', cursor: 'pointer', fontWeight: 700, fontSize: 12,
              }}
              onClick={handleSkuSubmit}
            >
              GO
            </button>
          </div>
        </div>

        {/* ── Resize handle ────────────────── */}
        <div
          className="retail-resize-handle"
          onMouseDown={startResize}
          aria-hidden="true"
        />

        {/* Right: cart */}
        <div className="retail-cart" style={{ width: retailCartWidth } as CSSProperties}>
          <div className="retail-cart-header">
            <span>Cart</span>
            <span>{lineCount} item{lineCount !== 1 ? 's' : ''}</span>
          </div>
          {lines.length === 0 ? (
            <div className="retail-cart-empty">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M6 2 4 6v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6l-2-4H6z" />
                <path d="M4 6h16" />
                <path d="M9 10V8a3 3 0 0 1 6 0v2" />
              </svg>
              <span>Cart is empty</span>
            </div>
          ) : (
            <>
              <div className="retail-cart-table">
                <table className="retail-cart-table-inner">
                  <thead>
                    <tr>
                      <th style={{ width: 40 }}>#</th>
                      <th>Item</th>
                      <th style={{ width: 56 }}>Qty</th>
                      <th style={{ width: 72 }}>@Price</th>
                      <th style={{ width: 80 }}>Subtotal</th>
                      <th style={{ width: 24 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {lines.map((line, idx) => (
                      <tr key={line.id}>
                        <td className="retail-cart-line-sku">{idx + 1}</td>
                        <td>
                          <div style={{ fontWeight: 600, fontSize: 11 }}>{line.name ?? line.sku}</div>
                        </td>
                        <td>
                          <span className="retail-cart-line-qty">
                            <button
                              className="retail-cart-qty-btn"
                              onClick={() => updateQty(line.id, Math.max(1, line.qty - 1))}
                            >
                              &minus;
                            </button>
                            <span className="retail-cart-qty-value">{line.qty}</span>
                            <button
                              className="retail-cart-qty-btn"
                              onClick={() => updateQty(line.id, line.qty + 1)}
                            >
                              +
                            </button>
                          </span>
                        </td>
                        <td className="retail-cart-line-unit">{formatMoney(line.unit_price)}</td>
                        <td className="retail-cart-line-subtotal">{formatMoney({ minor_units: line.unit_price.minor_units * line.qty, currency: line.unit_price.currency })}</td>
                        <td>
                          <button className="retail-cart-remove-btn" onClick={() => handleRemoveLine(line.id, { sku: line.sku, name: line.name ?? '', category: line.category ?? '', unit_price: line.unit_price })}>
                            &times;
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* ── Undo bar ───────────── */}
              {undoStack.length > 0 && (
                <div className="retail-undo-bar" role="status" aria-live="polite">
                  <span className="retail-undo-bar-label">{undoStack.length} item{undoStack.length > 1 ? 's' : ''} removed</span>
                  <button className="retail-undo-bar-btn" onClick={handleUndoRemove}>Undo</button>
                  <button className="retail-undo-bar-dismiss" onClick={handleDismissUndo} aria-label="Dismiss">&times;</button>
                </div>
              )}

              <div className="retail-cart-totals">
                <div className="retail-total-row">
                  <span>Subtotal</span>
                  <span>{subtotal ? formatMoney(subtotal) : '—'}</span>
                </div>
                {discountPercent > 0 && discountAmount && (
                  <div className="retail-total-row">
                    <span>Discount {discountPercent}%</span>
                    <span style={{ color: '#c00' }}>&minus;{formatMoney(discountAmount)}</span>
                  </div>
                )}
                <div className="retail-total-row retail-total-row--grand">
                  <span>Total</span>
                  <span>{total ? formatMoney(total) : '—'}</span>
                </div>
              </div>
              <div className="retail-cart-actions">
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--pay"
                  onClick={handlePay}
                  disabled={lines.length === 0 || !activeShift}
                >
                  Pay
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--discount"
                  onClick={() => setShowDiscount(true)}
                  disabled={lines.length === 0}
                >
                  Diskon
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--hold"
                  onClick={heldCart ? handleResume : handleHold}
                  disabled={!heldCart && lines.length === 0}
                >
                  {heldCart ? 'Resume' : 'Hold'}
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--void"
                  onClick={handleRequestClear}
                  disabled={lines.length === 0}
                >
                  Clear
                </button>
              </div>
              <div style={{ padding: '4px 8px' }}>
                <button
                  onClick={() => { setShowCreditList(true); loadCreditSales(); }}
                  style={{
                    width: '100%', padding: '6px', fontSize: 11, background: creditSales.length > 0 ? '#b8860b' : '#555',
                    color: '#fff', border: 'none', cursor: 'pointer', fontWeight: 700,
                  }}
                >
                  Credit Reminders ({creditSales.length})
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* ── Function bar (bottom) ──────────── */}
      <div className="retail-fn-bar">
        <button className="retail-fn-btn" onClick={handlePay} disabled={lines.length === 0}>
          <span className="retail-fn-key">F1</span> Pay
        </button>
        <button className="retail-fn-btn" onClick={handleRequestClear} disabled={lines.length === 0}>
          <span className="retail-fn-key">F2</span> Void
        </button>
        <button className="retail-fn-btn" onClick={() => setShowDiscount(true)} disabled={lines.length === 0}>
          <span className="retail-fn-key">F3</span> Diskon
        </button>
        <button className="retail-fn-btn" onClick={heldCart ? handleResume : handleHold} disabled={!heldCart && lines.length === 0}>
          <span className="retail-fn-key">F4</span> {heldCart ? 'Resume' : 'Hold'}
        </button>
        <button className="retail-fn-btn" onClick={() => skuInputRef.current?.focus()}>
          <span className="retail-fn-key">F5</span> Cari
        </button>
        <button className="retail-fn-btn" onClick={() => addToast({ message: 'Sales history coming soon', type: 'info' })}>
          <span className="retail-fn-key">F6</span> History
        </button>
        <button className="retail-fn-btn" onClick={() => addToast({ message: 'Customer lookup coming soon', type: 'info' })}>
          <span className="retail-fn-key">F7</span> Pelanggan
        </button>
        <button className="retail-fn-btn" onClick={() => addToast({ message: 'Stock inquiry coming soon', type: 'info' })}>
          <span className="retail-fn-key">F8</span> Stok
        </button>
        <button
          className="retail-fn-btn"
          onClick={() => activeShift ? setShowCloseShift(true) : setShowOpenShift(true)}
        >
          <span className="retail-fn-key">F9</span> {activeShift ? 'Close' : 'Open'} Shift
        </button>
        <button className="retail-fn-btn" onClick={() => setShowOptions(true)} disabled={session?.role_name === 'cashier'} style={session?.role_name === 'cashier' ? { opacity: 0.4, cursor: 'not-allowed' } : undefined}>
          <span className="retail-fn-key">F10</span> Options
        </button>
      </div>

      {/* ── Open Shift modal ────────────────── */}
      {showOpenShift && (
        <div className="retail-shift-overlay" onClick={() => setShowOpenShift(false)}>
          <div className="retail-shift-modal" onClick={(e) => e.stopPropagation()}>
            <h3>Open Shift</h3>
            <label htmlFor="retail-opening">Opening balance (Rp)</label>
            <input
              id="retail-opening"
              type="number"
              min="0"
              value={openingBalance}
              onChange={(e) => setOpeningBalance(e.target.value)}
              autoFocus
            />
            <div className="retail-shift-modal-actions">
              <button onClick={() => setShowOpenShift(false)} disabled={openingShift}>Cancel</button>
              <button className="retail-shift-confirm-btn" onClick={handleOpenShift} disabled={openingShift}>
                {openingShift ? 'Opening…' : 'Open'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift modal ───────────────── */}
      {showCloseShift && activeShift && !closedShiftSummary && (
        <div className="retail-shift-overlay" onClick={() => setShowCloseShift(false)}>
          <div className="retail-shift-modal" onClick={(e) => e.stopPropagation()}>
            <h3>Close Shift</h3>
            {closeShiftError && <div className="retail-shift-error">{closeShiftError}</div>}
            <div style={{ fontSize: 12, color: '#555', marginBottom: 10 }}>
              Opened: {new Date(activeShift.openedAt).toLocaleString()}
            </div>
            <label htmlFor="retail-closing">Counted cash (Rp)</label>
            <input
              id="retail-closing"
              type="number"
              min="0"
              value={closingBalance}
              onChange={(e) => setClosingBalance(e.target.value)}
              autoFocus
            />
            <label htmlFor="retail-notes" style={{ marginTop: 8 }}>Notes</label>
            <textarea
              id="retail-notes"
              rows={2}
              value={shiftNotes}
              onChange={(e) => setShiftNotes(e.target.value)}
            />
            <div className="retail-shift-modal-actions">
              <button onClick={() => setShowCloseShift(false)} disabled={closingShift}>Cancel</button>
              <button className="retail-shift-confirm-btn" onClick={handleCloseShift} disabled={closingShift}>
                {closingShift ? 'Closing…' : 'Close'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Closed Shift Summary ────────────── */}
      {closedShiftSummary && (
        <div className="retail-shift-overlay">
          <div className="retail-shift-modal">
            <h3>Shift Closed</h3>
            <div style={{ fontSize: 13, lineHeight: 1.8 }}>
              <div>Total Sales: {formatMoney({ minor_units: closedShiftSummary.totalSalesMinor, currency: 'IDR' })}</div>
              <div>Cash Sales: {formatMoney({ minor_units: closedShiftSummary.totalCashMinor, currency: 'IDR' })}</div>
              <div>Expected: {closedShiftSummary.expectedCashMinor != null ? formatMoney({ minor_units: closedShiftSummary.expectedCashMinor, currency: 'IDR' }) : '—'}</div>
              <div>Difference: {closedShiftSummary.cashDifferenceMinor != null ? formatMoney({ minor_units: closedShiftSummary.cashDifferenceMinor, currency: 'IDR' }) : '—'}</div>
            </div>
            <div className="retail-shift-modal-actions">
              <button className="retail-shift-confirm-btn" onClick={() => setClosedShiftSummary(null)}>Done</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Credit list overlay ─────────────── */}
      {showCreditList && (
        <div className="retail-shift-overlay" onClick={() => setShowCreditList(false)}>
          <div className="retail-shift-modal" onClick={(e) => e.stopPropagation()} style={{ maxHeight: '70vh', overflowY: 'auto', width: 480 }}>
            <h3>Credit Reminders</h3>
            {creditSales.length === 0 ? (
              <div style={{ padding: 16, textAlign: 'center', color: '#888' }}>No outstanding credits</div>
            ) : (
              <table style={{ width: '100%', fontSize: 12, borderCollapse: 'collapse' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid #ccc' }}>
                    <th style={{ textAlign: 'left', padding: 4 }}>Customer</th>
                    <th style={{ textAlign: 'right', padding: 4 }}>Amount</th>
                    <th style={{ textAlign: 'center', padding: 4 }}>Date</th>
                    <th style={{ padding: 4 }}></th>
                  </tr>
                </thead>
                <tbody>
                  {creditSales.map((c) => (
                    <tr key={c.saleId} style={{ borderBottom: '1px solid #eee' }}>
                      <td style={{ padding: 4 }}>{c.customerName || '—'}</td>
                      <td style={{ textAlign: 'right', padding: 4 }}>
                        {formatMoney({ minor_units: c.totalMinor, currency: c.currency })}
                      </td>
                      <td style={{ textAlign: 'center', padding: 4, fontSize: 11 }}>
                        {new Date(c.createdAt).toLocaleDateString()}
                      </td>
                      <td style={{ padding: 4 }}>
                        <button
                          onClick={() => handleSettleCredit(c.saleId)}
                          disabled={settlingId === c.saleId}
                          style={{
                            padding: '4px 8px', fontSize: 11, background: '#1a7a2a',
                            color: '#fff', border: 'none', cursor: 'pointer',
                          }}
                        >
                          {settlingId === c.saleId ? '…' : 'Settle'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <div className="retail-shift-modal-actions">
              <button className="retail-shift-confirm-btn" onClick={() => setShowCreditList(false)}>Close</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Clear confirm modal ────────────── */}
      {showClearConfirm && (
        <div className="retail-shift-overlay" onClick={() => setShowClearConfirm(false)}>
          <div className="retail-shift-modal" onClick={(e) => e.stopPropagation()}>
            <h3>Clear Cart</h3>
            <p style={{ fontSize: 13, margin: '0 0 16px', color: '#555' }}>
              Remove all {lineCount} item{lineCount !== 1 ? 's' : ''} from the cart?
            </p>
            <div className="retail-shift-modal-actions">
              <button onClick={() => setShowClearConfirm(false)}>Cancel</button>
              <button className="retail-shift-confirm-btn" onClick={handleConfirmClear}>Clear</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Discount modal ──────────────────── */}
      {showDiscount && (
        <div className="retail-discount-overlay" onClick={() => setShowDiscount(false)}>
          <div className="retail-discount-modal" onClick={(e) => e.stopPropagation()}>
            <h3>Discount</h3>
            <label>Discount (%)</label>
            <input
              type="number"
              min="0"
              max="100"
              value={discountInput}
              onChange={(e) => setDiscountInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleApplyDiscount(); }}
              autoFocus
            />
            <div className="retail-discount-actions">
              <button onClick={() => { setShowDiscount(false); setDiscountInput(''); }}>Cancel</button>
              <button onClick={handleApplyDiscount}>Apply</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Quantity picker modal ──────────── */}
      {showQtyPicker && pendingProduct && (
        <div className="retail-qty-overlay" onClick={() => setShowQtyPicker(false)}>
          <div className="retail-qty-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="retail-qty-heading">{pendingProduct.name}</h3>
            <div className="retail-qty-price">{formatMoney(pendingProduct.price)}</div>
            <div className="retail-qty-controls">
              <button
                className="retail-qty-btn"
                onClick={() => setQtyInput((v) => String(Math.max(1, (parseInt(v, 10) || 1) - 1)))}
              >
                &minus;
              </button>
              <input
                className="retail-qty-input"
                type="number"
                min={1}
                value={qtyInput}
                onChange={(e) => setQtyInput(e.target.value)}
                onFocus={(e) => e.target.select()}
                autoFocus
              />
              <button
                className="retail-qty-btn"
                onClick={() => setQtyInput((v) => String((parseInt(v, 10) || 1) + 1))}
              >
                +
              </button>
            </div>
            <div className="retail-qty-total">
              Total: {formatMoney({
                minor_units: pendingProduct.price.minor_units * Math.max(1, parseInt(qtyInput, 10) || 1),
                currency: pendingProduct.price.currency,
              })}
            </div>
            <div className="retail-qty-actions">
              <button className="retail-qty-cancel" onClick={() => setShowQtyPicker(false)}>Cancel</button>
              <button className="retail-qty-confirm" onClick={handleConfirmQty}>Add</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Shortcuts overlay ──────────────── */}
      {showShortcuts && (
        <div className="retail-shortcuts-overlay" onClick={() => setShowShortcuts(false)}>
          <div className="retail-shortcuts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="retail-shortcuts-heading">Keyboard Shortcuts</h3>
            <div className="retail-shortcuts-grid">
              <span className="retail-shortcuts-key">F1</span><span>Pay / Charge</span>
              <span className="retail-shortcuts-key">F2</span><span>Clear cart (Void)</span>
              <span className="retail-shortcuts-key">F3</span><span>Discount</span>
              <span className="retail-shortcuts-key">F4</span><span>Hold / Resume order</span>
              <span className="retail-shortcuts-key">F5</span><span>Focus SKU input</span>
              <span className="retail-shortcuts-key">F9</span><span>Open / Close shift</span>
              <span className="retail-shortcuts-key">F10</span><span>Options</span>
              <span className="retail-shortcuts-key">F11 / ?</span><span>This shortcut list</span>
              <span className="retail-shortcuts-key">Esc</span><span>Close modal / Options</span>
            </div>
            <button className="retail-shortcuts-close" onClick={() => setShowShortcuts(false)}>Close</button>
          </div>
        </div>
      )}

      {/* ── Scan flash overlay ─────────────── */}
      {scanFlash && <div className="retail-scan-flash" />}
    </div>
  );
}
