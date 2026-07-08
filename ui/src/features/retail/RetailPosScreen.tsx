import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import type { CSSProperties } from 'react';
import { usePosState } from '@/features/sales/usePosState';
import { useBarcodeScanner } from '@/features/sales/useBarcodeScanner';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { useLocalization } from '@fluent/react';
import { useExitAnimation } from '@/hooks/useExitAnimation';
import PaymentModal from '@/features/sales/PaymentModal';
import PriceOverrideModal from '@/features/sales/PriceOverrideModal';
import { overrideLinePrice, startSale, getProductTrackSerial, lookupSaleByReceiptBarcode } from '@/api/sales';
import { useFeatures, FEATURES } from '@/hooks/useFeatures';
import RefundModal from '@/features/sales/RefundModal';
import { listProducts, listCategories, lookupProductBySku, lookupByBarcode, type ProductDto, type CategoryDto } from '@/api/products';
import { listCustomers, type CustomerDto } from '@/api/customers';
import { getActiveShift, openShift, closeShift, type ShiftDto } from '@/api/shifts';
import { holdCart, listHeldCarts, getHeldCart, deleteHeldCart, type HeldCartRow, type SaleDetail } from '@/api/sales';
import { getStoreSettings, listCreditSales, settleCredit, type StoreSettingsDto, type CreditSaleDto } from '@/api/settings';
import { computeCartTax, type CartLineTaxInput } from '@/api/tax';
import { formatMoney, type CartId, type LineId, type Money, type Sku } from '@/types/domain';
import { useSound } from '@/frontend/shared/useSound';
import ScaleIndicator from './ScaleIndicator';
import RetailOptionsScreen from './RetailOptionsScreen';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import KdsScreen from '@/features/kds/KdsScreen';
import TableManagementScreen from '@/features/tables/TableManagementScreen';
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
  barcode: string | null; inStock: boolean; stockQty: number | null;
  createdAt?: string; priceUpdatedAt?: string;
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
    priceUpdatedAt: p.price_updated_at,
  };
}

export default function RetailPosScreen() {
  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { session, isManager } = useAuth();
  const userId = session!.user_id;

  const {
    lines, total, subtotal, discountPercent, discountLabel, discountAmount,
    addProduct, removeLine, updateQty, updateLinePrice, setDiscount, resetCart,
  } = usePosState();

  const lineCount = lines.reduce((a, l) => a + l.qty, 0);

  const { playBeep, playError, playSuccess, setSoundEnabled } = useSound();

  // ── Sound toggle from options ─────────────────────────────────
  useEffect(() => {
    const check = () => {
      const enabled = localStorage.getItem('retail-sound-enabled') !== 'false';
      setSoundEnabled(enabled);
    };
    check();
    window.addEventListener('storage', check);
    return () => window.removeEventListener('storage', check);
  }, [setSoundEnabled]);

  // ── Tender presets from options ───────────────────────────────
  const tenderPresets = useMemo(() => {
    try {
      const saved = localStorage.getItem('retail-tender-presets');
      if (saved) {
        const parsed = JSON.parse(saved) as number[];
        // Filter out zero/NaN values to avoid division-by-zero in PaymentModal
        const filtered = parsed.filter((n) => Number.isFinite(n) && n > 0);
        if (filtered.length > 0) return filtered;
      }
    } catch { /* ignore */ }
    return [5000, 10000, 20000, 50000, 100000];
  }, []);

  const { isEnabled } = useFeatures();

  const [weighTarget, setWeighTarget] = useState<{ sku: Sku; name: string } | null>(null);

  // ── Serial Number Capture ──────────────────────────────────────────
  const [serialNumbers, setSerialNumbers] = useState<Record<string, string>>({});
  const [trackSerialMap, setTrackSerialMap] = useState<Record<string, boolean>>({});
  const pendingTrackFetchRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const uniqueSkus = [...new Set(lines.map((l) => l.sku))];
    for (const sku of uniqueSkus) {
      if (trackSerialMap[sku] === undefined && !pendingTrackFetchRef.current.has(sku)) {
        pendingTrackFetchRef.current.add(sku);
        getProductTrackSerial(sku).then((track) => {
          setTrackSerialMap((prev) => ({ ...prev, [sku]: track }));
        }).catch(() => {});
      }
    }
  }, [lines]);

  const handleSerialChange = useCallback((lineId: string, serial: string) => {
    setSerialNumbers((prev) => ({ ...prev, [lineId]: serial }));
  }, []);

  // ── Quick Return ───────────────────────────────────────────────────
  const [showQuickReturn, setShowQuickReturn] = useState(false);
  const [quickReturnBarcode, setQuickReturnBarcode] = useState('');
  const [quickReturnLoading, setQuickReturnLoading] = useState(false);
  const [quickReturnSale, setQuickReturnSale] = useState<SaleDetail | null>(null);
  const [showQuickReturnRefund, setShowQuickReturnRefund] = useState(false);

  const handleQuickReturnSubmit = useCallback(async () => {
    const barcode = quickReturnBarcode.trim();
    if (!barcode) return;
    setQuickReturnLoading(true);
    try {
      const sale = await lookupSaleByReceiptBarcode(barcode);
      if (sale) {
        setQuickReturnSale(sale);
        setShowQuickReturn(false);
        setShowQuickReturnRefund(true);
        setQuickReturnBarcode('');
      } else {
        addToast({ message: l10n.getString('retail-quick-return-not-found') || 'Sale not found for this receipt barcode', type: 'error' });
        playError();
      }
    } catch {
      addToast({ message: l10n.getString('retail-quick-return-error') || 'Failed to look up receipt', type: 'error' });
      playError();
    } finally {
      setQuickReturnLoading(false);
    }
  }, [quickReturnBarcode, addToast, l10n, playError]);

  const handleQuickReturnRefundDone = useCallback(() => {
    setShowQuickReturnRefund(false);
    setQuickReturnSale(null);
  }, []);

  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const saved = localStorage.getItem('retail-theme');
    if (saved === 'dark' || saved === 'light') return saved;
    try { return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'; }
    catch { return 'light'; }
  });
  const handleThemeChange = useCallback((t: 'light' | 'dark') => {
    setTheme(t);
    localStorage.setItem('retail-theme', t);
  }, []);

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
    removeLine(id as LineId);
    setUndoStack((prev) => [line, ...prev].slice(0, MAX_UNDO));
  }, [removeLine]);

  const handleUndoRemove = useCallback(() => {
    if (undoStack.length === 0) return;
    const item = undoStack[0]!;
    addProduct({ ...item, price: item.unit_price, barcode: null, inStock: true, stockQty: null });
    setUndoStack((prev) => prev.slice(1));
  }, [undoStack, addProduct]);

  const undoBarExit = useExitAnimation(
    undoStack.length > 0,
    () => setUndoStack([]),
  );

  const handleDismissUndo = useCallback(() => {
    undoBarExit.requestClose();
  }, [undoBarExit]);

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
    if (pendingProduct.stock_qty != null) {
      const inCart = lines.filter((l) => l.sku === pendingProduct.sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + qty > pendingProduct.stock_qty) {
        addToast({ message: l10n.getString('retail-toast-insufficient-stock') || `Insufficient stock for ${pendingProduct.name}`, type: 'warning' });
        return;
      }
    }
    for (let i = 0; i < qty; i++) addProduct(toProduct(pendingProduct));
    setShowQtyPicker(false);
    setPendingProduct(null);
  }, [pendingProduct, qtyInput, addProduct, addToast, l10n, lines]);

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
    setCartId(null);
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
    listProducts().then(setProducts).catch(() => { addToast({ message: l10n.getString('retail-toast-failed-products') || 'Failed to load products', type: 'error' }); playError(); });
    listCategories().then((cats) => {
      setCategories(cats);
      const first = cats[0];
      if (first) setActiveCategory(first.id);
    }).catch(() => { addToast({ message: l10n.getString('retail-toast-failed-categories') || 'Failed to load categories', type: 'error' }); playError(); });
  }, []);

  const [searchQuery, setSearchQuery] = useState('');

  const allLabel = l10n.getString('product-lookup-all-categories') || 'All';
  const catLabels = useMemo(() => {
    const m = new Map<string, string>();
    categories.forEach((c) => {
      const catId = `category-${c.id}`;
      const label = l10n.getString(catId);
      m.set(c.id, label !== catId ? label : c.name);
    });
    return m;
  }, [categories, l10n]);

  const lowStockCount = useMemo(
    () => products.filter((p) => p.stock_qty != null && p.stock_qty > 0 && p.stock_qty <= 5).length,
    [products],
  );

  const [productPage, setProductPage] = useState(0);
  const PAGE_SIZE = 50;

  const filteredProducts = useMemo(() => {
    let list = products;
    if (activeCategory) list = list.filter((p) => p.category === activeCategory);
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      list = list.filter((p) => p.name.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q));
    }
    return list;
  }, [products, activeCategory, searchQuery]);

  const totalPages = Math.max(1, Math.ceil(filteredProducts.length / PAGE_SIZE));
  const pagedProducts = useMemo(
    () => filteredProducts.slice(productPage * PAGE_SIZE, (productPage + 1) * PAGE_SIZE),
    [filteredProducts, productPage],
  );

  // Reset page when filter changes
  useEffect(() => { setProductPage(0); }, [activeCategory, searchQuery]);

  const catHue = useCallback((catId: string | null) => {
    if (!catId) return 210;
    let h = 0;
    for (let i = 0; i < catId.length; i++) h = (h * 31 + catId.charCodeAt(i)) | 0;
    return Math.abs(h) % 360;
  }, []);

  const handleAdd = useCallback((p: ProductDto) => {
    if (p.stock_qty != null) {
      const inCart = lines.filter((l) => l.sku === p.sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + 1 > p.stock_qty) {
        addToast({ message: l10n.getString('retail-toast-insufficient-stock') || `Insufficient stock for ${p.name}`, type: 'warning' });
        return;
      }
    }
    addProduct(toProduct(p));
    addToRecent(p);
  }, [addProduct, addToRecent, addToast, l10n, lines]);

  const handleWeighAdd = useCallback((sku: Sku, weightGrams: number) => {
    const product = products.find((p) => p.sku === sku);
    if (!product) return;
    const qty = Math.max(1, Math.round(weightGrams));
    if (product.stock_qty != null) {
      const inCart = lines.filter((l) => l.sku === sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + qty > product.stock_qty) {
        addToast({ message: l10n.getString('retail-toast-insufficient-stock') || `Insufficient stock for ${product.name}`, type: 'warning' });
        return;
      }
    }
    addProduct(toProduct(product), qty);
    addToRecent(product);
    setWeighTarget(null);
    addToast({ message: l10n.getString('scale-weigh-added', { name: product.name, weight: qty }) || `Added ${qty}g of ${product.name}`, type: 'success' });
  }, [products, lines, addProduct, addToRecent, addToast, l10n]);

  const handleSetWeighTarget = useCallback((p: ProductDto) => {
    if (weighTarget?.sku === p.sku) return;
    setWeighTarget({ sku: p.sku as Sku, name: p.name });
    addToast({ message: l10n.getString('scale-target-set', { name: p.name }) || `${p.name} selected for weighing`, type: 'info' });
  }, [weighTarget, addToast, l10n]);

  /** Stock-aware cart qty increase — checks stock_qty before incrementing. */
  const handleIncreaseQty = useCallback((line: { sku: string; id: LineId; qty: number }) => {
    const product = products.find((p) => p.sku === line.sku);
    if (product?.stock_qty != null) {
      const otherLinesQty = lines
        .filter((l) => l.sku === line.sku && l.id !== line.id)
        .reduce((s, l) => s + l.qty, 0);
      if (otherLinesQty + line.qty + 1 > product.stock_qty) {
        addToast({ message: l10n.getString('retail-toast-insufficient-stock') || `Insufficient stock for ${product.name}`, type: 'warning' });
        return;
      }
    }
    updateQty(line.id, line.qty + 1);
  }, [products, lines, updateQty, addToast, l10n]);

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
    } catch { /* unreachable */ }
    addToast({ message: l10n.getString('pos-no-barcode-match') || 'Product not found', type: 'warning' });
  }, [skuInput, handleAdd, addToast, l10n]);

  const handleBarcode = useCallback(async (payload: { code: string }) => {
    const list = productsRef.current;
    const found = list.find((x) => x.barcode === payload.code);
    if (found) { handleAdd(found); setScanFlash(true); playBeep(); setTimeout(() => setScanFlash(false), 300); return; }
    try {
      const p = await lookupByBarcode(payload.code);
      if (p) { handleAdd(p); setScanFlash(true); playBeep(); setTimeout(() => setScanFlash(false), 300); return; }
    } catch { /* unreachable */ }
    playError();
    addToast({ message: l10n.getString('pos-no-barcode-match') || 'Product not found', type: 'warning' });
  }, [handleAdd, addToast, l10n, playBeep, playError]);

  useBarcodeScanner({ onProductFound: handleBarcode });

  // ── Store settings ──────────────────────────────────────────

  const [storeSettings, setStoreSettings] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  useEffect(() => {
    getStoreSettings().then(setStoreSettings).catch(() => addToast({ message: l10n.getString('retail-toast-failed-settings') || 'Failed to load store settings', type: 'error' }));
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

  // Fade the retail modals out with mirror keyframes before the
  // parent setter flips the boolean gate. Used by Cancel buttons
  // and × icons. Confirm-success paths that either reload the
  // app or swap to a sibling summary (close-shift↔summary) snap
  // intentionally per the navigate-to-next-state rule.
  const retailOpenShiftExit = useExitAnimation(showOpenShift, () => setShowOpenShift(false));
  const retailCloseShiftExit = useExitAnimation(
    showCloseShift && !closedShiftSummary,
    () => { setShowCloseShift(false); setCloseShiftError(null); },
  );
  const retailShiftSummaryExit = useExitAnimation(
    !!closedShiftSummary,
    () => setClosedShiftSummary(null),
  );

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
      addToast({ message: l10n.getString('retail-toast-failed-open-shift') || 'Failed to open shift', type: 'error' });
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
      const s = await closeShift({ userId, id: activeShift.id, closingBalanceMinor: val, notes: shiftNotes || null });
      setClosedShiftSummary(s);
      setActiveShift(null);
    } catch (e) {
      setCloseShiftError((e instanceof Error ? e.message : String(e)) ?? (l10n.getString('pos-close-shift-failed') || 'Failed to close shift'));
    } finally {
      setClosingShift(false);
    }
  }, [activeShift, closingBalance, shiftNotes]);

  // ── Live tax preview ────────────────────────────────────────

  const [cartTax, setCartTax] = useState<number>(0);

  useEffect(() => {
    if (lines.length === 0 || !subtotal) {
      setCartTax(0);
      return;
    }
    const currency = subtotal.currency;
    const taxLines: CartLineTaxInput[] = lines.map((l) => ({
      sku: String(l.sku),
      qty: l.qty,
      unit_price_minor: l.unit_price.minor_units,
    }));
    computeCartTax(taxLines, currency)
      .then(setCartTax)
      .catch(() => setCartTax(0));
  }, [lines, subtotal]);

  // ── Discount modal ───────────────────────────────────────────

  const [showDiscount, setShowDiscount] = useState(false);
  const [discountTab, setDiscountTab] = useState<'pct' | 'rp'>('pct');
  const retailDiscountExit = useExitAnimation(showDiscount, () => setShowDiscount(false));
  const [discountInput, setDiscountInput] = useState('');
  const [discountRpInput, setDiscountRpInput] = useState('');

  const handleApplyDiscount = useCallback(() => {
    const pct = Math.min(100, parseFloat(discountInput));
    if (Number.isNaN(pct) || pct <= 0) return;
    setDiscount(pct, '');
    setShowDiscount(false);
    setDiscountInput('');
    setDiscountRpInput('');
  }, [discountInput, setDiscount]);

  const handleApplyDiscountRp = useCallback(() => {
    const rp = parseFloat(discountRpInput);
    if (Number.isNaN(rp) || rp <= 0 || !subtotal) return;
    const rpMinor = Math.min(subtotal.minor_units, Math.round(rp * 100));
    const pct = Math.round((rpMinor / subtotal.minor_units) * 100 * 100) / 100;
    setDiscount(pct, '');
    setShowDiscount(false);
    setDiscountRpInput('');
  }, [discountRpInput, subtotal, setDiscount]);

  // ── Customer selection ─────────────────────────────────────

  const [selectedCustomer, setSelectedCustomer] = useState<CustomerDto | null>(null);
  const [showCustomerSearch, setShowCustomerSearch] = useState(false);
  const [customerSearchQuery, setCustomerSearchQuery] = useState('');
  const [customerSearchResults, setCustomerSearchResults] = useState<CustomerDto[]>([]);
  const [loadingCustomers, setLoadingCustomers] = useState(false);
  const [overrideTarget, setOverrideTarget] = useState<{ id: LineId; name: string; unit_price: Money } | null>(null);
  const [cartId, setCartId] = useState<CartId | null>(null);
  const ensureCart = useCallback(async (currency: string): Promise<CartId | null> => {
    if (cartId) return cartId;
    try {
      const { cartId: newCartId } = await startSale({ currency });
      setCartId(newCartId);
      return newCartId;
    } catch {
      addToast({ message: 'Failed to create sale cart', type: 'error' });
      return null;
    }
  }, [cartId, addToast]);

  const handleOverrideConfirm = useCallback(async (newPriceMinor: number, authorizingUserId: string) => {
    if (!overrideTarget) return;
    const cId = cartId;
    if (!cId) {
      addToast({ message: 'No active sale cart', type: 'error' });
      setOverrideTarget(null);
      return;
    }
    try {
      await overrideLinePrice({
        cartId: cId,
        lineId: overrideTarget.id,
        newPriceMinor,
        userId: authorizingUserId,
      });
      updateLinePrice(overrideTarget.id, {
        minor_units: newPriceMinor,
        currency: overrideTarget.unit_price.currency,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Override failed';
      addToast({ message: msg, type: 'error' });
    } finally {
      setOverrideTarget(null);
    }
  }, [overrideTarget, cartId, addToast, updateLinePrice]);

  const allCustomersRef = useRef<CustomerDto[]>([]);

  useEffect(() => {
    if (!showCustomerSearch) { setCustomerSearchResults([]); return; }
    setLoadingCustomers(true);
    listCustomers()
      .then((customers) => {
        allCustomersRef.current = customers;
        const q = customerSearchQuery.trim().toLowerCase();
        setCustomerSearchResults(
          !q ? customers : customers.filter(
            (c) =>
              c.name.toLowerCase().includes(q) ||
              (c.phone && c.phone.includes(q)) ||
              (c.email && c.email.toLowerCase().includes(q)),
          ),
        );
      })
      .catch(() => setCustomerSearchResults([]))
      .finally(() => setLoadingCustomers(false));
  }, [showCustomerSearch]);

  useEffect(() => {
    if (!showCustomerSearch) return;
    const customers = allCustomersRef.current;
    if (customers.length === 0) return;
    const q = customerSearchQuery.trim().toLowerCase();
    setCustomerSearchResults(
      !q ? customers : customers.filter(
        (c) =>
          c.name.toLowerCase().includes(q) ||
          (c.phone && c.phone.includes(q)) ||
          (c.email && c.email.toLowerCase().includes(q)),
      ),
    );
  }, [showCustomerSearch, customerSearchQuery]);

  // ── Payment modal ────────────────────────────────────────────

  const [showPayment, setShowPayment] = useState(false);

  const handlePay = useCallback(() => {
    if (!activeShift) { addToast({ message: l10n.getString('retail-toast-open-shift-first') || 'Open a shift first', type: 'warning' }); return; }
    setShowPayment(true);
  }, [activeShift, addToast]);

  // ── Hold cart ────────────────────────────────────────────────

  const [heldCartId, setHeldCartId] = useState<string | null>(null);
  const [showHeldCartsList, setShowHeldCartsList] = useState(false);
  const [heldCartsList, setHeldCartsList] = useState<HeldCartRow[]>([]);

  const handleHold = useCallback(async () => {
    if (lines.length === 0) return;
    try {
      const cartData = JSON.stringify({
        lines: lines.map((l) => ({ sku: l.sku, name: l.name, category: l.category, qty: l.qty, unit_price: l.unit_price })),
        discountPercent,
        discountLabel,
      });
      if (!subtotal) return;
      const { id } = await holdCart({
        label: `Hold #${Date.now()}`,
        cart_data: cartData,
        item_count: lines.length,
        total_minor: subtotal.minor_units,
        currency: subtotal.currency,
        bill_type: 'hold',
      });
      setHeldCartId(id);
      resetCart();
      addToast({ message: l10n.getString('retail-toast-order-held') || 'Order held', type: 'success' });
    } catch {
      addToast({ message: l10n.getString('retail-toast-failed-hold') || 'Failed to hold order', type: 'error' });
    }
  }, [lines, discountPercent, discountLabel, subtotal, resetCart, addToast]);

  const handleResumeCart = useCallback(async (cartId: string) => {
    try {
      const full = await getHeldCart(cartId);
      if (!full) return;
      const data = JSON.parse(full.cart_data);
      for (const l of data.lines) {
        for (let i = 0; i < (l.qty || 1); i++) {
          addProduct({ sku: l.sku as Sku, name: l.name, category: l.category ?? '', price: l.unit_price, barcode: null, inStock: true, stockQty: null });
        }
      }
      if (data.discountPercent) setDiscount(data.discountPercent, data.discountLabel ?? '');
      await deleteHeldCart(cartId);
      setHeldCartId(null);
      setShowHeldCartsList(false);
    } catch {
      addToast({ message: l10n.getString('retail-toast-failed-resume') || 'Failed to resume order', type: 'error' });
    }
  }, [addProduct, setDiscount, addToast]);

  const handleResume = useCallback(async () => {
    const carts = await listHeldCarts();
    const held = carts.filter((c) => c.bill_type === 'hold');
    if (held.length === 0) return;
    if (held.length === 1) {
      await handleResumeCart(held[0]!.id);
      return;
    }
    setHeldCartsList(held);
    setShowHeldCartsList(true);
  }, [handleResumeCart]);

  const handleDeleteHeldCart = useCallback(async (cartId: string) => {
    try {
      await deleteHeldCart(cartId);
      setHeldCartsList((prev) => prev.filter((c) => c.id !== cartId));
      if (heldCartId === cartId) setHeldCartId(null);
      addToast({ type: 'success', message: l10n.getString('retail-toast-held-cart-deleted') || 'Held cart deleted' });
    } catch {
      addToast({ type: 'error', message: l10n.getString('retail-toast-failed-delete-held') || 'Failed to delete held cart' });
    }
  }, [heldCartId, addToast]);

  // ── Load persisted held carts on mount ───────────────────────

  useEffect(() => {
    listHeldCarts()
      .then((carts) => {
        const held = carts.find((c) => c.bill_type === 'hold');
        if (held) setHeldCartId(held.id);
      })
      .catch(() => addToast({ message: 'Failed to load held carts', type: 'error' }));
  }, []);

  // ── Options full-screen page ─────────────────────────────────

  const [showOptions, setShowOptions] = useState(false);
  const [showSalesHistory, setShowSalesHistory] = useState(false);
  const [showStockInquiry, setShowStockInquiry] = useState(false);
  const [showKds, setShowKds] = useState(false);
  const [showTables, setShowTables] = useState(false);

  // ── Credit reminders ──────────────────────────────────────────

  const [creditSales, setCreditSales] = useState<CreditSaleDto[]>([]);
  const [showCreditList, setShowCreditList] = useState(false);
  const [settlingId, setSettlingId] = useState<string | null>(null);

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
      await settleCredit(saleId, userId);
      setCreditSales((prev) => prev.filter((c) => c.saleId !== saleId));
      addToast({ message: l10n.getString('retail-toast-credit-settled') || 'Credit settled', type: 'success' });
    } catch {
      addToast({ message: l10n.getString('retail-toast-failed-settle') || 'Failed to settle credit', type: 'error' });
    } finally {
      setSettlingId(null);
    }
  }, [userId, addToast]);

  // ── Clock ────────────────────────────────────────────────────

  const [clock, setClock] = useState(new Date());
  useEffect(() => {
    const id = setInterval(() => setClock(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const timeStr = clock.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' });
  const dateStr = clock.toLocaleDateString('id-ID', { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' });

  const shiftDuration = useMemo(() => {
    if (!activeShift) return null;
    const opened = new Date(activeShift.openedAt);
    const diffMs = clock.getTime() - opened.getTime();
    const h = Math.floor(diffMs / 3600000);
    const m = Math.floor((diffMs % 3600000) / 60000);
    return `${h}h ${m}m`;
  }, [activeShift, clock]);

  // ── Keyboard shortcuts ────────────────────────────────────────

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (showOptions || showPayment || showOpenShift || showCloseShift || showDiscount || showQtyPicker || showShortcuts || showCreditList || showClearConfirm || showSalesHistory || showStockInquiry || showKds || showTables) return;
      switch (e.key) {
        case 'F1': handlePay(); break;
        case 'F2': if (lines.length > 0) handleRequestClear(); break;
        case 'F3': if (lines.length > 0) setShowDiscount(true); break;
        case 'F4': heldCartId ? handleResume() : handleHold(); break;
        case 'F5': skuInputRef.current?.focus(); break;
        case 'F6': setShowSalesHistory(true); break;
        case 'F7': setShowCustomerSearch(true); break;
        case 'F8': setShowStockInquiry(true); break;
        case 'F9': activeShift ? setShowCloseShift(true) : setShowOpenShift(true); break;
        case 'F10': if (session?.role_name !== 'cashier') setShowOptions(true); break;
        case 'F11': case '?': setShowShortcuts((v) => !v); break;
        case 'F12': setShowKds(true); break;
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [showOptions, showPayment, showOpenShift, showCloseShift, showDiscount, showQtyPicker, showShortcuts, showCustomerSearch, showClearConfirm, showSalesHistory, showStockInquiry, showKds, showTables, handlePay, lines.length, handleRequestClear, handleHold, handleResume, heldCartId, activeShift, session, addToast]);

  // ── Render ───────────────────────────────────────────────────

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
        selectedCustomer={selectedCustomer}
        {...(isEnabled(FEATURES.SERIAL_TRACKING) ? { serialNumbers } : {})}
        onCustomerChange={(c) => setSelectedCustomer(c)}
        tenderPresets={tenderPresets}
        onComplete={() => { setShowPayment(false); resetCart(); setSelectedCustomer(null); playSuccess(); addToast({ message: l10n.getString('retail-toast-sale-complete') || 'Sale complete', type: 'success' }); }}
        onClose={() => setShowPayment(false)}
      />
    );
  }

  // ── Options screen ──────────────────────────────────────────
  if (showOptions) {
    return <RetailOptionsScreen onClose={() => setShowOptions(false)} theme={theme} onThemeChange={handleThemeChange} />;
  }

  // ── Sales History screen ────────────────────────────────────
  if (showSalesHistory) {
    return (
      <div className="retail-pos" data-theme={theme}>
        <header className="retail-header" style={{ justifyContent: 'space-between' }}>
          <div className="retail-header-store">
            <span className="retail-header-name">{l10n.getString('retail-fn-history') || 'Sales History'}</span>
          </div>
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={() => setShowSalesHistory(false)}
          >
            &larr; {l10n.getString('back')}
          </button>
        </header>
        <div style={{ flex: 1, overflow: 'auto' }}>
          <SalesHistoryScreen />
        </div>
      </div>
    );
  }

  // ── KDS screen ─────────────────────────────────────────────
  if (showKds) {
    return (
      <div className="retail-pos" data-theme={theme}>
        <header className="retail-header" style={{ justifyContent: 'space-between' }}>
          <div className="retail-header-store">
            <span className="retail-header-name">{l10n.getString('kds-title') || 'Kitchen Display'}</span>
          </div>
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={() => setShowKds(false)}
          >
            &larr; {l10n.getString('back')}
          </button>
        </header>
        <div style={{ flex: 1, overflow: 'auto' }}>
          <KdsScreen />
        </div>
      </div>
    );
  }

  // ── Table Management screen ────────────────────────────────
  if (showTables) {
    return (
      <div className="retail-pos" data-theme={theme}>
        <header className="retail-header" style={{ justifyContent: 'space-between' }}>
          <div className="retail-header-store">
            <span className="retail-header-name">{l10n.getString('tables-title') || 'Table Management'}</span>
          </div>
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={() => setShowTables(false)}
          >
            &larr; {l10n.getString('back')}
          </button>
        </header>
        <div style={{ flex: 1, overflow: 'auto' }}>
          <TableManagementScreen />
        </div>
      </div>
    );
  }

  // ── Stock Inquiry screen ────────────────────────────────────
  if (showStockInquiry) {
    return (
      <div className="retail-pos" data-theme={theme}>
        <header className="retail-header" style={{ justifyContent: 'space-between' }}>
          <div className="retail-header-store">
            <span className="retail-header-name">{l10n.getString('retail-fn-stok') || 'Stock Inquiry'}</span>
          </div>
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={() => setShowStockInquiry(false)}
          >
            &larr; {l10n.getString('back')}
          </button>
        </header>
        <div style={{ flex: 1, overflow: 'auto' }}>
          <ProductLookupScreen onAddProduct={(p) => handleAdd({
            sku: p.sku, name: p.name, category: p.category,
            price: p.price, barcode: p.barcode ?? null,
            in_stock: p.inStock, stock_qty: p.stockQty ?? null,
            tax_rate_ids: [], created_at: '', price_updated_at: '',
          })} />
        </div>
      </div>
    );
  }

  return (
    <div className="retail-pos" data-theme={theme}>
      {/* ── Header ──────────────────────────── */}
      <header className="retail-header">
        <div className="retail-header-store">
          {storeSettings.logo && (
            <img src={`data:image/png;base64,${storeSettings.logo}`} alt="" className="retail-header-logo" style={{ height: 32, marginRight: 8 }} />
          )}
          <div>
            <span className="retail-header-name">{storeSettings.name || l10n.getString('retail-store-name-fallback')}</span>
            {storeSettings.branch && <span className="retail-header-branch"> &middot; {storeSettings.branch}</span>}
            <span className="retail-header-address">{storeSettings.address || ''}</span>
          </div>
        </div>
        <div className="retail-header-right">
          {shiftLoading ? (
            <span className="retail-shift-badge">{l10n.getString('loading')}</span>
          ) : activeShift ? (
            <span className="retail-shift-badge">
              {l10n.getString('retail-shift-label')} &middot; {formatMoney({ minor_units: activeShift.totalSalesMinor, currency: storeSettings.currency })}
            </span>
          ) : (
            <span className="retail-shift-badge" style={{ opacity: 0.6 }}>{l10n.getString('retail-no-shift')}</span>
          )}
          <div className="retail-header-cashier">
            <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
              <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
            </svg>
            <span>{session?.display_name ?? ''}</span>
          </div>
          <span className="retail-header-clock">
            <span className="retail-header-date">{dateStr}</span>
            <span>{timeStr}</span>
            {shiftDuration && <span className="retail-header-duration">{shiftDuration}</span>}
          </span>
        </div>
      </header>

      {/* ── Low-stock banner ──────────────── */}
      {lowStockCount > 0 && (
        <div className="retail-low-stock-banner">
          <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
            <path d="M10 2a1 1 0 011 1v8a1 1 0 11-2 0V3a1 1 0 011-1zM10 16a1 1 0 100-2 1 1 0 000 2z"/>
          </svg>
          <span>{l10n.getString('retail-low-stock-banner', { count: lowStockCount }) || `${lowStockCount} product${lowStockCount > 1 ? 's' : ''} low on stock`}</span>
        </div>
      )}

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
                aria-label={catLabels.get(cat.id) ?? cat.name}
                aria-pressed={activeCategory === cat.id}
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
              placeholder={l10n.getString('retail-search-placeholder')}
            />
            {searchQuery && (
              <button className="retail-search-clear" onClick={() => setSearchQuery('')} aria-label={l10n.getString('retail-search-clear-aria')}>
                &times;
              </button>
            )}
          </div>

          {isEnabled(FEATURES.USB_SCALE) && (
            <ScaleIndicator
              weighTarget={weighTarget}
              onWeighAdd={handleWeighAdd}
              onClearWeighTarget={() => setWeighTarget(null)}
            />
          )}

          {/* ── Recent products ──────────────── */}
          {recentProducts.length > 0 && !searchQuery.trim() && !activeCategory && (
            <div className="retail-recent-strip">
              <span className="retail-recent-label">{l10n.getString('retail-recent-label')}</span>
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
              {searchQuery.trim() ? (l10n.getString('retail-no-products-match') || 'No products match your search') : (l10n.getString('retail-no-products') || 'No products')}
            </div>
          ) : (
            <div className="retail-grid">
                {pagedProducts.map((p) => <ProductCard
                  key={p.sku}
                  product={p}
                  catHue={catHue}
                  formatMoney={formatMoney}
                  handleAdd={handleAdd}
                  handleOpenQtyPicker={handleOpenQtyPicker}
                  scaleEnabled={isEnabled(FEATURES.USB_SCALE)}
                  onSetWeighTarget={handleSetWeighTarget}
                />)}
            </div>
          )}
          {totalPages > 1 && (
            <div className="retail-page-nav" role="navigation" aria-label={l10n.getString('retail-page-nav-aria') || 'Product pages'}>
              <button className="retail-page-btn" disabled={productPage === 0} onClick={() => setProductPage((p) => p - 1)} aria-label={l10n.getString('retail-page-prev-aria') || 'Previous page'}>{'<'}</button>
              <span className="retail-page-info" aria-current="true">{productPage + 1} / {totalPages}</span>
              <button className="retail-page-btn" disabled={productPage >= totalPages - 1} onClick={() => setProductPage((p) => p + 1)} aria-label={l10n.getString('retail-page-next-aria') || 'Next page'}>{'>'}</button>
            </div>
          )}
          <div className="retail-sku-bar">
            <span className="retail-sku-label">{l10n.getString('retail-sku-label')}</span>
            <input
              ref={skuInputRef}
              className="retail-sku-input"
              type="text"
              value={skuInput}
              onChange={(e) => setSkuInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleSkuSubmit(); }}
              placeholder={l10n.getString('retail-sku-placeholder')}
            />
            <button
              style={{
                padding: '4px 12px', background: '#1a3a5c', color: '#fff',
                border: 'none', cursor: 'pointer', fontWeight: 700, fontSize: 12,
              }}
              onClick={handleSkuSubmit}
            >
              {l10n.getString('retail-sku-go')}
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
            <span>{l10n.getString('cart-title')}</span>
            <span>{l10n.getString('retail-cart-items', { count: lineCount }) || `${lineCount} item${lineCount !== 1 ? 's' : ''}`}</span>
          </div>
          {lines.length === 0 ? (
            <div className="retail-cart-empty">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M6 2 4 6v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6l-2-4H6z" />
                <path d="M4 6h16" />
                <path d="M9 10V8a3 3 0 0 1 6 0v2" />
              </svg>
              <span>{l10n.getString('pos-cart-empty')}</span>
            </div>
          ) : (
            <>
              <div className="retail-cart-table">
                <table className="retail-cart-table-inner">
                  <thead>
                    <tr>
                      <th style={{ width: 40 }}>{l10n.getString('retail-cart-header-col')}</th>
                      <th>{l10n.getString('retail-cart-header-item')}</th>
                      <th style={{ width: 56 }}>{l10n.getString('retail-cart-header-qty')}</th>
                      <th style={{ width: 72 }}>{l10n.getString('retail-cart-header-price')}</th>
                      <th style={{ width: 80 }}>{l10n.getString('retail-cart-header-subtotal')}</th>
                      <th style={{ width: 24 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {lines.map((line, idx) => (
                      <tr key={line.id}>
                        <td className="retail-cart-line-sku">{idx + 1}</td>
                        <td>
                          <div style={{ fontWeight: 600, fontSize: 11 }}>{line.name ?? line.sku}</div>
                          {isEnabled(FEATURES.SERIAL_TRACKING) && trackSerialMap[line.sku] && (
                            <input
                              type="text"
                              className="retail-cart-serial-input"
                              value={serialNumbers[line.id] ?? ''}
                              onChange={(e) => handleSerialChange(line.id, e.target.value)}
                              placeholder="Serial #"
                              aria-label={`Serial number for ${line.name ?? line.sku}`}
                              style={{
                                marginTop: 4, padding: '2px 4px', fontSize: 10,
                                width: '100%', boxSizing: 'border-box',
                                border: '1px solid #ccc', borderRadius: 2,
                              }}
                            />
                          )}
                        </td>
                        <td>
                          <span className="retail-cart-line-qty">
                            <button
                              className="retail-cart-qty-btn"
                              onClick={() => updateQty(line.id, Math.max(1, line.qty - 1))}
                              aria-label={l10n.getString('retail-cart-qty-decrease-aria') || `Decrease quantity of ${line.sku}`}
                            >
                              &minus;
                            </button>
                            <span className="retail-cart-qty-value">{line.qty}</span>
                            <button
                              className="retail-cart-qty-btn"
                              onClick={() => handleIncreaseQty(line)}
                              aria-label={l10n.getString('retail-cart-qty-increase-aria') || `Increase quantity of ${line.sku}`}
                            >
                              +
                            </button>
                          </span>
                        </td>
                        <td className="retail-cart-line-unit">
                          {formatMoney(line.unit_price)}
                          {isManager && (
                            <button
                              type="button"
                              className="retail-cart-line-override"
                              onClick={() => {
                                setOverrideTarget({ id: line.id as LineId, name: line.name ?? line.sku, unit_price: line.unit_price });
                                ensureCart(line.unit_price.currency);
                              }}
                              aria-label={`Override price for ${line.name ?? line.sku}`}
                            >
                              Override
                            </button>
                          )}
                        </td>
                        <td className="retail-cart-line-subtotal">{formatMoney({ minor_units: line.unit_price.minor_units * line.qty, currency: line.unit_price.currency })}</td>
                          <td>
                            <button className="retail-cart-remove-btn" onClick={() => handleRemoveLine(line.id, { sku: line.sku, name: line.name ?? '', category: line.category ?? '', unit_price: line.unit_price })} aria-label={l10n.getString('retail-cart-remove-aria') || `Remove ${line.sku} from cart`}>
                              &times;
                            </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* ── Undo bar ───────────── */}
              {undoBarExit.shouldRender && (
                <div
                  className={`retail-undo-bar${undoBarExit.exiting ? ' retail-undo-bar--exiting' : ''}`}
                  role="status"
                  aria-live="polite"
                >
                  <span className="retail-undo-bar-label">{l10n.getString('retail-undo-items-removed', { count: undoStack.length }) || `${undoStack.length} item${undoStack.length > 1 ? 's' : ''} removed`}</span>
                  <button className="retail-undo-bar-btn" onClick={handleUndoRemove}>{l10n.getString('pos-cart-undo')}</button>
                  <button className="retail-undo-bar-dismiss" onClick={handleDismissUndo} aria-label={l10n.getString('pos-cart-undo-dismiss-aria')}>&times;</button>
                </div>
              )}

              <div className="retail-cart-totals">
                <div className="retail-total-row">
                  <span>{l10n.getString('pos-cart-subtotal')}</span>
                  <span>{subtotal ? formatMoney(subtotal) : '—'}</span>
                </div>
                {discountPercent > 0 && discountAmount && (
                  <div className="retail-total-row">
                    <span>{l10n.getString('retail-total-discount', { percent: discountPercent }) || `Discount ${discountPercent}%`}</span>
                    <span style={{ color: '#c00' }}>&minus;{formatMoney(discountAmount)}</span>
                  </div>
                )}
                {cartTax > 0 && (
                  <div className="retail-total-row">
                    <span>{l10n.getString('retail-total-tax')}</span>
                    <span>{formatMoney({ minor_units: cartTax, currency: subtotal?.currency ?? 'IDR' })}</span>
                  </div>
                )}
                <div className="retail-total-row retail-total-row--grand">
                  <span>{l10n.getString('cart-total-label')}</span>
                  <span>{total ? formatMoney(total) : '—'}</span>
                </div>
              </div>
              {selectedCustomer && (
                <div className="retail-customer-badge">
                  <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
                    <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
                  </svg>
                  <span>{selectedCustomer.name}</span>
                </div>
              )}
              <div className="retail-cart-actions">
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--pay"
                  onClick={handlePay}
                  disabled={lines.length === 0 || !activeShift}
                  aria-label={l10n.getString('sale-pay-button')}
                >
                  {l10n.getString('sale-pay-button')}
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--discount"
                  onClick={() => setShowDiscount(true)}
                  disabled={lines.length === 0}
                  aria-label={l10n.getString('retail-discount-button')}
                >
                  {l10n.getString('retail-discount-button')}
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--hold"
                  onClick={heldCartId ? handleResume : handleHold}
                  disabled={!heldCartId && lines.length === 0}
                  aria-label={heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
                >
                  {heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
                </button>
                <button
                  className="retail-cart-action-btn retail-cart-action-btn--void"
                  onClick={handleRequestClear}
                  disabled={lines.length === 0}
                  aria-label={l10n.getString('pos-cart-clear')}
                >
                  {l10n.getString('pos-cart-clear')}
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
                  {l10n.getString('retail-credit-reminders', { count: creditSales.length }) || `Credit Reminders (${creditSales.length})`}
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* ── Function bar (bottom) ──────────── */}
      <div className="retail-fn-bar" role="toolbar" aria-label={l10n.getString('retail-fn-bar-aria') || 'Function bar'}>
        <button className="retail-fn-btn" onClick={handlePay} disabled={lines.length === 0}>
          <span className="retail-fn-key">F1</span> {l10n.getString('sale-pay-button')}
        </button>
        <button className="retail-fn-btn" onClick={handleRequestClear} disabled={lines.length === 0}>
          <span className="retail-fn-key">F2</span> {l10n.getString('retail-fn-void')}
        </button>
        <button className="retail-fn-btn" onClick={() => setShowDiscount(true)} disabled={lines.length === 0}>
          <span className="retail-fn-key">F3</span> {l10n.getString('retail-fn-diskon')}
        </button>
        <button className="retail-fn-btn" onClick={heldCartId ? handleResume : handleHold} disabled={!heldCartId && lines.length === 0}>
          <span className="retail-fn-key">F4</span> {heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
        </button>
        <button className="retail-fn-btn" onClick={() => skuInputRef.current?.focus()}>
          <span className="retail-fn-key">F5</span> {l10n.getString('retail-fn-cari')}
        </button>
        <button className="retail-fn-btn" onClick={() => setShowSalesHistory(true)}>
          <span className="retail-fn-key">F6</span> {l10n.getString('retail-fn-history')}
        </button>
        <button className="retail-fn-btn" onClick={() => setShowCustomerSearch(true)}>
          <span className="retail-fn-key">F7</span> {l10n.getString('retail-fn-pelanggan')}
        </button>
        <button className="retail-fn-btn" onClick={() => setShowStockInquiry(true)}>
          <span className="retail-fn-key">F8</span> {l10n.getString('retail-fn-stok')}
        </button>
        <button
          className="retail-fn-btn"
          onClick={() => activeShift ? setShowCloseShift(true) : setShowOpenShift(true)}
        >
          <span className="retail-fn-key">F9</span> {activeShift ? l10n.getString('pos-shift-close-btn') : l10n.getString('pos-shift-open-btn')} {l10n.getString('retail-fn-shift')}
        </button>
        <button className="retail-fn-btn" onClick={() => setShowOptions(true)} disabled={session?.role_name === 'cashier'} style={session?.role_name === 'cashier' ? { opacity: 0.4, cursor: 'not-allowed' } : undefined}>
          <span className="retail-fn-key">F10</span> {l10n.getString('retail-fn-options')}
        </button>
        {isEnabled(FEATURES.QUICK_RETURN) && (
          <button className="retail-fn-btn" onClick={() => setShowQuickReturn(true)}>
            <span className="retail-fn-key">F11</span> {l10n.getString('retail-fn-quick-return') || 'Quick Return'}
          </button>
        )}
        <button className="retail-fn-btn" onClick={() => setShowKds(true)}>
          <span className="retail-fn-key">F12</span> {l10n.getString('kds-title') || 'KDS'}
        </button>
        {isEnabled(FEATURES.TABLE_MANAGEMENT) && (
          <button className="retail-fn-btn" onClick={() => setShowTables(true)}>
            🪑 {l10n.getString('tables-title') || 'Tables'}
          </button>
        )}
      </div>

      {/* ── Open Shift modal ────────────────── */}
      {retailOpenShiftExit.shouldRender && (
        <div
          className={`retail-shift-overlay${retailOpenShiftExit.exiting ? ' retail-shift-overlay--exiting' : ''}`}
          role="button"
          tabIndex={0}
          aria-label="Close"
          onClick={() => retailOpenShiftExit.requestClose()}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); retailOpenShiftExit.requestClose(); } }}
        >
          <div className={`retail-shift-modal${retailOpenShiftExit.exiting ? ' retail-shift-modal--exiting' : ''}`} role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('pos-open-shift-title')}</h3>
            <label htmlFor="retail-opening">{l10n.getString('retail-open-shift-opening-label')}</label>
            <input
              id="retail-opening"
              type="number"
              min="0"
              value={openingBalance}
              onChange={(e) => setOpeningBalance(e.target.value)}
            />
            <div className="retail-shift-modal-actions">
              <button onClick={() => retailOpenShiftExit.requestClose()} disabled={openingShift}>{l10n.getString('cancel')}</button>
              <button className="retail-shift-confirm-btn" onClick={handleOpenShift} disabled={openingShift}>
                {openingShift ? l10n.getString('retail-open-shift-opening') : l10n.getString('pos-shift-open-btn')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Close Shift modal ───────────────── */}
      {retailCloseShiftExit.shouldRender && activeShift && (
        <div
          className={`retail-shift-overlay${retailCloseShiftExit.exiting ? ' retail-shift-overlay--exiting' : ''}`}
          role="button"
          tabIndex={0}
          aria-label="Close"
          onClick={() => retailCloseShiftExit.requestClose()}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); retailCloseShiftExit.requestClose(); } }}
        >
          <div className={`retail-shift-modal${retailCloseShiftExit.exiting ? ' retail-shift-modal--exiting' : ''}`} role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('pos-close-shift-title')}</h3>
            {closeShiftError && <div className="retail-shift-error">{closeShiftError}</div>}
            <div style={{ fontSize: 12, color: '#555', marginBottom: 10 }}>
              {l10n.getString('pos-close-shift-opened')}: {new Date(activeShift.openedAt).toLocaleString()}
            </div>
            <label htmlFor="retail-closing">{l10n.getString('pos-close-shift-counted-label')}</label>
            <input
              id="retail-closing"
              type="number"
              min="0"
              value={closingBalance}
              onChange={(e) => setClosingBalance(e.target.value)}
            />
            <label htmlFor="retail-notes" style={{ marginTop: 8 }}>{l10n.getString('pos-shift-notes')}</label>
            <textarea
              id="retail-notes"
              rows={2}
              value={shiftNotes}
              onChange={(e) => setShiftNotes(e.target.value)}
            />
            <div className="retail-shift-modal-actions">
              <button onClick={() => retailCloseShiftExit.requestClose()} disabled={closingShift}>{l10n.getString('cancel')}</button>
              <button className="retail-shift-confirm-btn" onClick={handleCloseShift} disabled={closingShift}>
                {closingShift ? l10n.getString('loading') : l10n.getString('pos-shift-close-btn')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Closed Shift Summary ────────────── */}
      {(retailShiftSummaryExit.shouldRender && closedShiftSummary) && (
        <div className={`retail-shift-overlay${retailShiftSummaryExit.exiting ? ' retail-shift-overlay--exiting' : ''}`}>
          <div className={`retail-shift-modal${retailShiftSummaryExit.exiting ? ' retail-shift-modal--exiting' : ''}`}>
            <h3>{l10n.getString('pos-shift-closed-title')}</h3>
            <div style={{ fontSize: 13, lineHeight: 1.8 }}>
              <div>{l10n.getString('pos-shift-total-sales')}: {formatMoney({ minor_units: closedShiftSummary.totalSalesMinor, currency: storeSettings.currency })}</div>
              <div>{l10n.getString('retail-shift-closed-cash-sales')} {formatMoney({ minor_units: closedShiftSummary.totalCashMinor, currency: storeSettings.currency })}</div>
              <div>{l10n.getString('pos-shift-expected-cash')}: {closedShiftSummary.expectedCashMinor != null ? formatMoney({ minor_units: closedShiftSummary.expectedCashMinor, currency: storeSettings.currency }) : '—'}</div>
              <div>{l10n.getString('pos-shift-difference')}: {closedShiftSummary.cashDifferenceMinor != null ? formatMoney({ minor_units: closedShiftSummary.cashDifferenceMinor, currency: storeSettings.currency }) : '—'}</div>
            </div>
            <div className="retail-shift-modal-actions">
              <button
                className="retail-shift-confirm-btn"
                onClick={() => retailShiftSummaryExit.requestClose()}
              >{l10n.getString('pos-shift-summary-done')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Credit list overlay ─────────────── */}
      {showCreditList && (
        <div className="retail-shift-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowCreditList(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowCreditList(false); } }}>
          <div className="retail-shift-modal" role="presentation" onClick={(e) => e.stopPropagation()} style={{ maxHeight: '70vh', overflowY: 'auto', width: 480 }}>
            <h3>{l10n.getString('retail-credit-reminders-title')}</h3>
            {creditSales.length === 0 ? (
              <div style={{ padding: 16, textAlign: 'center', color: '#888' }}>{l10n.getString('retail-credit-no-outstanding')}</div>
            ) : (
              <table style={{ width: '100%', fontSize: 12, borderCollapse: 'collapse' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid #ccc' }}>
                    <th style={{ textAlign: 'left', padding: 4 }}>{l10n.getString('retail-credit-col-customer')}</th>
                    <th style={{ textAlign: 'right', padding: 4 }}>{l10n.getString('retail-credit-col-amount')}</th>
                    <th style={{ textAlign: 'center', padding: 4 }}>{l10n.getString('retail-credit-col-date')}</th>
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
                          {settlingId === c.saleId ? '…' : l10n.getString('retail-credit-settle')}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <div className="retail-shift-modal-actions">
              <button className="retail-shift-confirm-btn" onClick={() => setShowCreditList(false)}>{l10n.getString('close')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Clear confirm modal ────────────── */}
      {showClearConfirm && (
        <div className="retail-shift-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowClearConfirm(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowClearConfirm(false); } }}>
          <div className="retail-shift-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('retail-clear-cart-title')}</h3>
            <p style={{ fontSize: 13, margin: '0 0 16px', color: '#555' }}>
              {l10n.getString('retail-clear-cart-confirm', { count: lineCount }) || `Remove all ${lineCount} item${lineCount !== 1 ? 's' : ''} from the cart?`}
            </p>
            <div className="retail-shift-modal-actions">
              <button onClick={() => setShowClearConfirm(false)}>{l10n.getString('cancel')}</button>
              <button className="retail-shift-confirm-btn" onClick={handleConfirmClear}>{l10n.getString('retail-clear-cart-clear')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Discount modal ──────────────────── */}
      {retailDiscountExit.shouldRender && (
        <div
          className={`retail-discount-overlay${retailDiscountExit.exiting ? ' retail-discount-overlay--exiting' : ''}`}
          role="button"
          tabIndex={0}
          aria-label="Close"
          onClick={() => retailDiscountExit.requestClose()}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); retailDiscountExit.requestClose(); } }}
        >
          <div className={`retail-discount-modal${retailDiscountExit.exiting ? ' retail-discount-modal--exiting' : ''}`} role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('retail-discount-title')}</h3>
            <div className="retail-discount-tabs">
              <button
                className={`retail-discount-tab${discountTab === 'pct' ? ' retail-discount-tab--active' : ''}`}
                onClick={() => setDiscountTab('pct')}
              >
                {l10n.getString('retail-discount-pct-tab')}
              </button>
              <button
                className={`retail-discount-tab${discountTab === 'rp' ? ' retail-discount-tab--active' : ''}`}
                onClick={() => setDiscountTab('rp')}
              >
                {l10n.getString('retail-discount-rp-tab')}
              </button>
            </div>
            {discountTab === 'pct' ? (
              <>
                <label htmlFor="discount-pct">{l10n.getString('retail-discount-pct-label')}</label>
                <input
                  id="discount-pct"
                  type="number"
                  min="0"
                  max="100"
                  value={discountInput}
                  onChange={(e) => setDiscountInput(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Enter') handleApplyDiscount(); }}
                />
              </>
            ) : (
              <>
                <label htmlFor="discount-rp">{l10n.getString('retail-discount-rp-label')}</label>
                <input
                  id="discount-rp"
                  type="number"
                  min="0"
                  value={discountRpInput}
                  onChange={(e) => setDiscountRpInput(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Enter') handleApplyDiscountRp(); }}
                />
              </>
            )}
            <div className="retail-discount-actions">
              <button onClick={() => { retailDiscountExit.requestClose(); setDiscountInput(''); setDiscountRpInput(''); }}>{l10n.getString('cancel')}</button>
              {discountTab === 'pct' ? (
                <button onClick={handleApplyDiscount}>{l10n.getString('pos-cart-apply')}</button>
              ) : (
                <button onClick={handleApplyDiscountRp}>{l10n.getString('pos-cart-apply')}</button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* ── Customer search modal ──────────── */}
      {showCustomerSearch && (
        <div className="retail-customer-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowCustomerSearch(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowCustomerSearch(false); } }}>
          <div className="retail-customer-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('retail-customer-search-title')}</h3>
            <input
              className="retail-customer-search-input"
              type="text"
              placeholder={l10n.getString('retail-customer-search-placeholder')}
              value={customerSearchQuery}
              onChange={(e) => setCustomerSearchQuery(e.target.value)}
            />
            <div className="retail-customer-search-list">
              {loadingCustomers ? (
                <div className="retail-customer-search-loading">{l10n.getString('retail-customer-search-loading')}</div>
              ) : customerSearchResults.length === 0 ? (
                <div className="retail-customer-search-empty">{l10n.getString('retail-customer-search-empty')}</div>
              ) : (
                customerSearchResults.map((c) => (
                  <button
                    key={c.id}
                    className={`retail-customer-search-item${selectedCustomer?.id === c.id ? ' retail-customer-search-item--selected' : ''}`}
                    onClick={() => {
                      setSelectedCustomer(c);
                      setShowCustomerSearch(false);
                      setCustomerSearchQuery('');
                    }}
                  >
                    <span className="retail-customer-search-item-name">{c.name}</span>
                    {(c.phone || c.email) && (
                      <span className="retail-customer-search-item-detail">{c.phone || c.email}</span>
                    )}
                  </button>
                ))
              )}
            </div>
            <div className="retail-customer-modal-actions">
              {selectedCustomer && (
                <button
                  className="retail-customer-clear-btn"
                  onClick={() => {
                    setSelectedCustomer(null);
                    setShowCustomerSearch(false);
                    setCustomerSearchQuery('');
                  }}
                >
                  {l10n.getString('retail-customer-clear')}
                </button>
              )}
              <button className="retail-customer-close-btn" onClick={() => setShowCustomerSearch(false)}>{l10n.getString('close')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Quantity picker modal ──────────── */}
      {showQtyPicker && pendingProduct && (
        <div className="retail-qty-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowQtyPicker(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowQtyPicker(false); } }}>
          <div className="retail-qty-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
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
              />
              <button
                className="retail-qty-btn"
                onClick={() => setQtyInput((v) => String((parseInt(v, 10) || 1) + 1))}
              >
                +
              </button>
            </div>
            <div className="retail-qty-numpad">
              {[1,2,3,4,5,6,7,8,9,'',0,'⌫'].map((k) => (
                k === '' ? <span key="spacer" /> : (
                  <button
                    key={String(k)}
                    className="retail-qty-num-btn"
                    onClick={() => {
                      if (k === '⌫') setQtyInput((v) => v.length > 1 ? v.slice(0, -1) : '1');
                      else setQtyInput((v) => String(Math.max(1, parseInt(v + String(k), 10) || 1)));
                    }}
                  >
                    {k}
                  </button>
                )
              ))}
            </div>
            <div className="retail-qty-total">
              {l10n.getString('retail-qty-total')} {formatMoney({
                minor_units: pendingProduct.price.minor_units * Math.max(1, parseInt(qtyInput, 10) || 1),
                currency: pendingProduct.price.currency,
              })}
            </div>
            <div className="retail-qty-actions">
              <button className="retail-qty-cancel" onClick={() => setShowQtyPicker(false)}>{l10n.getString('cancel')}</button>
              <button className="retail-qty-confirm" onClick={handleConfirmQty}>{l10n.getString('retail-qty-add')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Held carts list modal ──────────── */}
      {showHeldCartsList && (
        <div className="retail-held-carts-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowHeldCartsList(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowHeldCartsList(false); } }}>
          <div className="retail-held-carts-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('retail-held-carts-title')}</h3>
            {heldCartsList.length === 0 ? (
              <p className="retail-held-carts-empty">{l10n.getString('retail-held-carts-empty')}</p>
            ) : (
              <div className="retail-held-carts-list">
                {heldCartsList.map((c) => (
                  <div key={c.id} className="retail-held-cart-row">
                    <div className="retail-held-cart-info" role="button" tabIndex={0} aria-label="Resume cart" onClick={() => handleResumeCart(c.id)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleResumeCart(c.id); } }}>
                      <span className="retail-held-cart-label">{c.label}</span>
                      <span className="retail-held-cart-meta">
                        {c.item_count} {l10n.getString('retail-cart-items', { count: c.item_count })} &middot; {formatMoney({ minor_units: c.total_minor, currency: c.currency })}
                      </span>
                    </div>
                    <button className="retail-held-cart-delete" onClick={() => handleDeleteHeldCart(c.id)} aria-label={l10n.getString('retail-held-cart-delete-aria')}>
                      &times;
                    </button>
                  </div>
                ))}
              </div>
            )}
            <div className="retail-held-carts-actions">
              <button onClick={() => setShowHeldCartsList(false)}>{l10n.getString('close')}</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Shortcuts overlay ──────────────── */}
      {showShortcuts && (
        <div className="retail-shortcuts-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => setShowShortcuts(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowShortcuts(false); } }}>
          <div className="retail-shortcuts-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3 className="retail-shortcuts-heading">{l10n.getString('retail-shortcuts-title')}</h3>
            <div className="retail-shortcuts-grid">
              <span className="retail-shortcuts-key">F1</span><span>{l10n.getString('retail-shortcut-pay')}</span>
              <span className="retail-shortcuts-key">F2</span><span>{l10n.getString('retail-shortcut-clear')}</span>
              <span className="retail-shortcuts-key">F3</span><span>{l10n.getString('retail-shortcut-discount')}</span>
              <span className="retail-shortcuts-key">F4</span><span>{l10n.getString('retail-shortcut-hold')}</span>
              <span className="retail-shortcuts-key">F5</span><span>{l10n.getString('retail-shortcut-sku')}</span>
              <span className="retail-shortcuts-key">F6</span><span>{l10n.getString('retail-fn-history')}</span>
              <span className="retail-shortcuts-key">F7</span><span>{l10n.getString('retail-fn-pelanggan')}</span>
              <span className="retail-shortcuts-key">F8</span><span>{l10n.getString('retail-fn-stok')}</span>
              <span className="retail-shortcuts-key">F9</span><span>{l10n.getString('retail-shortcut-shift')}</span>
              <span className="retail-shortcuts-key">F10</span><span>{l10n.getString('retail-shortcut-options')}</span>
              <span className="retail-shortcuts-key">F11 / ?</span><span>{l10n.getString('retail-shortcut-list')}</span>
              <span className="retail-shortcuts-key">F12</span><span>{l10n.getString('kds-title') || 'KDS'}</span>
              <span className="retail-shortcuts-key">Esc</span><span>{l10n.getString('retail-shortcut-close')}</span>
            </div>
            <button className="retail-shortcuts-close" onClick={() => setShowShortcuts(false)}>{l10n.getString('close')}</button>
          </div>
        </div>
      )}

      {/* ── Price Override modal ───────────── */}
      {overrideTarget && (
        <PriceOverrideModal
          open
          lineDescription={`${overrideTarget.name} — ${formatMoney(overrideTarget.unit_price)}`}
          currentPrice={overrideTarget.unit_price}
          onConfirm={handleOverrideConfirm}
          onClose={() => setOverrideTarget(null)}
        />
      )}

      {/* ── Quick Return modal ──────────────── */}
      {showQuickReturn && (
        <div className="retail-shift-overlay" role="button" tabIndex={0} aria-label="Close" onClick={() => { setShowQuickReturn(false); setQuickReturnBarcode(''); }} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowQuickReturn(false); setQuickReturnBarcode(''); } }}>
          <div className="retail-shift-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
            <h3>{l10n.getString('retail-quick-return-title') || 'Quick Return'}</h3>
            <p style={{ fontSize: 12, color: '#555', marginBottom: 8 }}>
              {l10n.getString('retail-quick-return-desc') || 'Scan or enter the receipt barcode to look up a sale for return.'}
            </p>
            <input
              type="text"
              className="retail-sku-input"
              style={{ width: '100%', boxSizing: 'border-box', marginBottom: 8 }}
              value={quickReturnBarcode}
              onChange={(e) => setQuickReturnBarcode(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleQuickReturnSubmit(); }}
              placeholder={l10n.getString('retail-quick-return-placeholder') || 'Receipt barcode'}
              aria-label={l10n.getString('retail-quick-return-aria') || 'Receipt barcode input'}
            />
            <div className="retail-shift-modal-actions">
              <button onClick={() => { setShowQuickReturn(false); setQuickReturnBarcode(''); }} disabled={quickReturnLoading}>
                {l10n.getString('cancel')}
              </button>
              <button className="retail-shift-confirm-btn" onClick={handleQuickReturnSubmit} disabled={quickReturnLoading || !quickReturnBarcode.trim()}>
                {quickReturnLoading ? l10n.getString('loading') : (l10n.getString('retail-quick-return-lookup') || 'Look Up')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Quick Return Refund modal ───────── */}
      {showQuickReturnRefund && quickReturnSale && (
        <RefundModal
          open
          sale={quickReturnSale}
          onClose={handleQuickReturnRefundDone}
          onRefunded={handleQuickReturnRefundDone}
        />
      )}

      {/* ── Scan flash overlay ─────────────── */}
      {scanFlash && <div className="retail-scan-flash" />}
    </div>
  );
}

// ── Product card with single-tap / long-press ─────────────────

const PRICE_VOLATILITY_MS = 24 * 60 * 60 * 1000; // 24 h

function isPriceRecent(p: ProductDto): boolean {
  if (!p.price_updated_at) return false;
  const elapsed = Date.now() - new Date(p.price_updated_at).getTime();
  return elapsed >= 0 && elapsed < PRICE_VOLATILITY_MS;
}

function ProductCard({ product, catHue, formatMoney, handleAdd, handleOpenQtyPicker, scaleEnabled, onSetWeighTarget }: {
  product: ProductDto;
  catHue: (catId: string | null) => number;
  formatMoney: (m: Money) => string;
  handleAdd: (p: ProductDto) => void;
  handleOpenQtyPicker: (p: ProductDto) => void;
  scaleEnabled: boolean;
  onSetWeighTarget: (p: ProductDto) => void;
}) {
  const isOutOfStock = !product.in_stock || (product.stock_qty != null && product.stock_qty <= 0);
  const priceRecent = useMemo(() => isPriceRecent(product), [product.price_updated_at]);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isLongPress = useRef(false);

  const handlePointerDown = useCallback(() => {
    if (isOutOfStock) return;
    isLongPress.current = false;
    longPressTimer.current = setTimeout(() => {
      isLongPress.current = true;
      handleOpenQtyPicker(product);
    }, 400);
  }, [product, isOutOfStock, handleOpenQtyPicker]);

  const handlePointerUp = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
    if (!isLongPress.current && !isOutOfStock) handleAdd(product);
  }, [product, isOutOfStock, handleAdd]);

  const handlePointerLeave = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
  }, []);

  return (
    <button
      className={`retail-product-btn${isOutOfStock ? ' retail-product-btn--out-of-stock' : ''}`}
      style={{ '--cat-hue': catHue(product.category) } as React.CSSProperties}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerLeave}
      aria-label={`${product.name} ${formatMoney(product.price)}${isOutOfStock ? ' (out of stock)' : ''}`}
      aria-disabled={isOutOfStock}
    >
      {product.stock_qty != null && product.stock_qty > 0 && (
        <span className={`retail-product-stock-badge retail-stock-${product.stock_qty <= 5 ? 'low' : product.stock_qty <= 10 ? 'medium' : 'high'}`}>{product.stock_qty}</span>
      )}
      {priceRecent && <span className="retail-price-volatility-hint" title="Price changed recently" />}
      <span className="retail-product-name">{product.name}</span>
      <span className="retail-product-price">{formatMoney(product.price)}</span>
      {scaleEnabled && (
        <span
          className="retail-product-weigh-btn"
          onPointerDown={(e) => e.stopPropagation()}
          onPointerUp={(e) => { e.stopPropagation(); e.preventDefault(); onSetWeighTarget(product); }}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); e.preventDefault(); onSetWeighTarget(product); } }}
          aria-label={`Weigh ${product.name}`}
        >
          ⚖
        </span>
      )}
    </button>
  );
}
