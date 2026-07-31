import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { usePosState } from '@/features/sales/usePosState';
import { useBarcodeScanner } from '@/features/sales/useBarcodeScanner';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { requiredLocalized } from '@/frontend/shared';
import { useLocalization } from '@fluent/react';
import { useExitAnimation } from '@/hooks/useExitAnimation';
import { useSwipe } from '@/hooks/useSwipe';
import PaymentModal from '@/features/sales/PaymentModal';
import ItemModifierModal from '@/features/sales/components/ItemModifierModal';
import { overrideLinePriceScoped, startSaleScoped, getProductTrackSerial, lookupSaleByReceiptBarcodeScoped } from '@/api/sales';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { useFeatures, FEATURES } from '@/hooks/useFeatures';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listProductsScoped, listCategories, lookupProductBySkuScoped, lookupByBarcodeScoped, type ProductDto, type CategoryDto } from '@/api/products';
import { listCustomers, type CustomerDto } from '@/api/customers';
import { getActiveShiftScoped, openShiftScoped, closeShiftScoped, type ShiftDto } from '@/api/shifts';
import { holdCartScoped, listHeldCartsScoped, getHeldCartScoped, deleteHeldCartScoped, type HeldCartRow, type SaleDetail } from '@/api/sales';
import { getStoreSettingsScoped, listCreditSales, settleCreditScoped, type StoreSettingsDto, type CreditSaleDto } from '@/api/settings';
import { computeCartTax, type CartLineTaxInput } from '@/api/tax';
import { type CartId, type CartLine, type CourseId, type LineId, type Money, type Product, type Sku } from '@/types/domain';
import { useSound } from '@/frontend/shared/useSound';
import { useOptionalTheme } from '@/frontend/shell/ThemeProvider';
import RetailFnBar from './RetailFnBar';
import RetailHeader from './RetailHeader';
import RetailCartPanel from './RetailCartPanel';
import { RETAIL_CART_WIDTH_MIN, RETAIL_CART_WIDTH_DEFAULT, RETAIL_CART_WIDTH_MAX_CAP, clampRetailCartWidth } from './RetailCartPanel.constants';
import RetailProductGrid, { type SortField, type SortOrder } from './RetailProductGrid';
import { SalesHistoryView, TableManagementView, StockInquiryView } from './RetailSubViews';
import RetailModals from './RetailModals';
import RetailReminderPopup from './RetailReminderPopup';
import './RetailPosScreen.css';

function toProduct(p: ProductDto): Product {
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
    productType: p.product_type as Product['productType'],
  };
}

// ── Retail sample product fallback (when IPC is unavailable) ──────
const RETAIL_SAMPLE_PRODUCTS: ProductDto[] = [
  { sku: 'CPU-R7-7800X3D', name: 'AMD Ryzen 7 7800X3D 8-Core',  category: 'cat-cpu',       price: { minor_units: 6250000, currency: 'IDR' }, barcode: '730143314930', in_stock: true,  stock_qty: 15, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'CPU-I7-14700K',  name: 'Intel Core i7-14700K 20-Core', category: 'cat-cpu',       price: { minor_units: 6450000, currency: 'IDR' }, barcode: '503203727850', in_stock: true,  stock_qty: 10, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'CPU-R5-7600',    name: 'AMD Ryzen 5 7600 6-Core',     category: 'cat-cpu',       price: { minor_units: 3150000, currency: 'IDR' }, barcode: '730143314503', in_stock: true,  stock_qty: 25, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'GPU-RTX4070TS',  name: 'ASUS TUF RTX 4070 Ti Super 16G',category: 'cat-gpu',     price: { minor_units: 14850000, currency: 'IDR' },barcode: '195553554890', in_stock: true,  stock_qty: 8,  product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'GPU-RX7800XT',   name: 'Sapphire PULSE RX 7800 XT 16G',category: 'cat-gpu',     price: { minor_units: 8450000, currency: 'IDR' }, barcode: '489517350567', in_stock: true,  stock_qty: 12, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'GPU-RTX4060',    name: 'MSI Ventus 2X RTX 4060 8GB',   category: 'cat-gpu',     price: { minor_units: 4750000, currency: 'IDR' }, barcode: '824142323456', in_stock: true,  stock_qty: 20, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'RAM-D5-32GB-CR', name: 'Corsair Vengeance DDR5 32GB 6K',category: 'cat-ram',     price: { minor_units: 1850000, currency: 'IDR' }, barcode: '840006698765', in_stock: true,  stock_qty: 30, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'RAM-D5-64GB-GS', name: 'G.Skill Trident Z5 RGB 64GB D5',category: 'cat-ram',     price: { minor_units: 3450000, currency: 'IDR' }, barcode: '848354041234', in_stock: true,  stock_qty: 14, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'RAM-D4-16GB-KF', name: 'Kingston Fury Beast 16GB DDR4', category: 'cat-ram',     price: { minor_units: 680000,  currency: 'IDR' }, barcode: '740617319800', in_stock: true,  stock_qty: 45, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'SSD-990PRO-2TB', name: 'Samsung 990 PRO 2TB NVMe SSD',  category: 'cat-storage', price: { minor_units: 2750000, currency: 'IDR' }, barcode: '887276722340', in_stock: true,  stock_qty: 22, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'SSD-P3P-1TB',    name: 'Crucial P3 Plus 1TB M.2 NVMe',  category: 'cat-storage', price: { minor_units: 1150000, currency: 'IDR' }, barcode: '649528918900', in_stock: true,  stock_qty: 35, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'HDD-ST-4TB',     name: 'Seagate BarraCuda 4TB 3.5" HDD',category: 'cat-storage', price: { minor_units: 1350000, currency: 'IDR' }, barcode: '763649112340', in_stock: true,  stock_qty: 18, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'MB-B650-ROG',    name: 'ASUS ROG Strix B650-A Gaming',  category: 'cat-mb',      price: { minor_units: 3650000, currency: 'IDR' }, barcode: '195553948760', in_stock: true,  stock_qty: 9,  product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'MB-Z790-MSI',    name: 'MSI MAG Z790 Tomahawk WiFi',    category: 'cat-mb',      price: { minor_units: 4250000, currency: 'IDR' }, barcode: '824142301230', in_stock: true,  stock_qty: 7,  product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'PSU-RM850X',     name: 'Corsair RM850x 850W Gold Modular',category:'cat-psu',     price: { minor_units: 2150000, currency: 'IDR' }, barcode: '840006601234', in_stock: true,  stock_qty: 16, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'COOL-PA120',     name: 'Thermalright Peerless Assassin 120',category:'cat-cooling',price:{ minor_units: 580000,  currency: 'IDR' }, barcode: '784562098120', in_stock: true,  stock_qty: 40, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'COOL-KRAKEN360', name: 'NZXT Kraken Elite 360 RGB AIO', category: 'cat-cooling', price: { minor_units: 4450000, currency: 'IDR' }, barcode: '815671018900', in_stock: true,  stock_qty: 5,  product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'PASTE-MX6',      name: 'Arctic MX-6 Thermal Paste 4g',  category: 'cat-cooling', price: { minor_units: 125000,  currency: 'IDR' }, barcode: '872767004500', in_stock: true,  stock_qty: 60, product_type: 'retail', tax_rate_ids: [], created_at: '', price_updated_at: '' },
];

const RETAIL_SAMPLE_CATEGORIES: CategoryDto[] = [
  { id: 'cat-cpu', name: 'Processors (CPU)', colour: '#e74c3c', icon: 'cpu-1' },
  { id: 'cat-gpu', name: 'Graphics Cards (GPU)', colour: '#2ecc71', icon: 'gpu-1' },
  { id: 'cat-ram', name: 'Memory (RAM)', colour: '#9b59b6', icon: 'ram-1' },
  { id: 'cat-storage', name: 'Storage (SSD/HDD)', colour: '#3498db', icon: 'hdd-1' },
  { id: 'cat-mb', name: 'Motherboards', colour: '#f39c12', icon: 'mb-1' },
  { id: 'cat-psu', name: 'Power Supply', colour: '#1abc9c', icon: 'psu-1' },
  { id: 'cat-cooling', name: 'Cooling & Cases', colour: '#34495e', icon: 'cool-1' },
];

interface RetailPosScreenProps {
  onNavigate?: (route: string) => void;
}

/** Retail POS sales screen — product lookup on the left, cart panel on the right with resizable width and barcode scanning support. */
export default function RetailPosScreen({ onNavigate }: RetailPosScreenProps) {
  const { l10n } = useLocalization();
  const l10nRef = useRef(l10n);
  l10nRef.current = l10n;
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { addToast } = useToast();
  const { session, isManager } = useAuth();
  const { sessionToken: rawToken, setActiveWorkspace } = useWorkspace();
  const sessionToken = rawToken || '';
  const userId = session?.user_id ?? '';

  // ── Screen-reader announcements (declared early — used by add handlers) ──
  const announceRef = useRef<HTMLDivElement>(null);
  const announceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const announce = useCallback((msg: string) => {
    if (announceTimerRef.current) clearTimeout(announceTimerRef.current);
    if (announceRef.current) {
      announceRef.current.textContent = msg;
      announceTimerRef.current = setTimeout(() => {
        if (announceRef.current) announceRef.current.textContent = '';
      }, 3000);
    }
  }, []);

  const {
    lines, total, subtotal, discountPercent, discountLabel, discountAmount,
    addProduct, removeLine, updateQty, updateLinePrice, assignCourse, setDiscount, resetCart,
  } = usePosState();

  const lineCount = lines.reduce((a, l) => a + l.qty, 0);
  // Ref for stock-check callbacks to avoid stale closure on rapid sequential adds
  const linesRef = useRef(lines);
  linesRef.current = lines;

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
    // Clean up pending set: remove SKUs no longer in the cart
    for (const sku of pendingTrackFetchRef.current) {
      if (!uniqueSkus.includes(sku as Sku)) pendingTrackFetchRef.current.delete(sku);
    }
    for (const sku of uniqueSkus) {
      if (trackSerialMap[sku] === undefined && !pendingTrackFetchRef.current.has(sku)) {
        pendingTrackFetchRef.current.add(sku);
        getProductTrackSerial(sku).then((track) => {
          setTrackSerialMap((prev) => ({ ...prev, [sku]: track }));
        }).catch(() => { /* serial track lookup is best-effort */ });
      }
    }
  }, [lines, trackSerialMap]);

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
      const sale = await lookupSaleByReceiptBarcodeScoped(sessionToken, barcode);
      if (sale) {
        setQuickReturnSale(sale);
        setShowQuickReturn(false);
        setShowQuickReturnRefund(true);
        setQuickReturnBarcode('');
      } else {
        addToast({ message: requiredLocalized(l10n, 'retail-quick-return-not-found'), type: 'error' });
        playError();
      }
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-quick-return-error'), type: 'error' });
      playError();
    } finally {
      setQuickReturnLoading(false);
    }
  }, [quickReturnBarcode, addToast, l10n, playError, sessionToken]);

  const handleQuickReturnRefundDone = useCallback(() => {
    setShowQuickReturnRefund(false);
    setQuickReturnSale(null);
  }, []);

  // P0-1 (audit docs/2026-07-28-retail-pos-theming-audit.md): replace the
  // shadow useState that read a per-component localStorage key and matched
  // the OS dark-mode preference on mount but never updated afterwards.
  // useOptionalTheme()?.theme returns Theme | undefined — Theme when
  // AppProviders' ThemeProvider wraps, undefined for unwrapped renders
  // (React strips undefined from JSX attributes; CSS falls back to :root
  // via cascade).
  // Implicitly also closes the P0-3 storage-key shadow, the P2-6 dead
  // setter, and the P2-7 missing-useTheme import by deleting the
  // shadow state entirely.
  const theme = useOptionalTheme()?.theme;


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
  const [undoStack, setUndoStack] = useState<{ sku: Sku; name: string; category: string; unit_price: Money; qty: number }[]>([]);

  const handleRemoveLine = useCallback((id: string, line: { sku: Sku; name: string; category: string; unit_price: Money; qty: number }) => {
    removeLine(id as LineId);
    setUndoStack((prev) => [line, ...prev].slice(0, MAX_UNDO));
  }, [removeLine]);

  const handleUndoRemove = useCallback(() => {
    if (undoStack.length === 0) return;
    const item = undoStack[0]!;
    addProduct({ sku: item.sku, name: item.name, category: item.category, productType: 'retail', price: item.unit_price, barcode: null, inStock: true, stockQty: null }, item.qty);
    announce(requiredLocalized(l10nRef.current, 'retail-added-to-cart', { name: item.name }));
    setUndoStack((prev) => prev.slice(1));
  }, [undoStack, addProduct, announce]);

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
      const inCart = linesRef.current.filter((l) => l.sku === pendingProduct.sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + qty > pendingProduct.stock_qty) {
        addToast({ message: requiredLocalized(l10n, 'retail-toast-insufficient-stock', { name: pendingProduct.name }), type: 'warning' });
        return;
      }
    }
    addProduct(toProduct(pendingProduct), qty);
    announce(requiredLocalized(l10nRef.current, 'retail-added-to-cart', { name: pendingProduct.name }));
    setShowQtyPicker(false);
    setPendingProduct(null);
  }, [pendingProduct, qtyInput, addProduct, addToast, l10n, announce]);

  // ── Keyboard shortcut overlay ────────────────────────────────────
  const [showShortcuts, setShowShortcuts] = useState(false);

  // ── Barcode scan flash ───────────────────────────────────────────
  const [scanFlash, setScanFlash] = useState(false);
  const scanFlashTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clean up the scan-flash and announce timers on unmount
  useEffect(() => {
    return () => {
      if (scanFlashTimerRef.current) clearTimeout(scanFlashTimerRef.current);
      if (announceTimerRef.current) clearTimeout(announceTimerRef.current);
    };
  }, []);

  // ── Confirm clear cart ────────────────────────────────────────────
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const handleRequestClear = useCallback(() => {
    if (lines.length === 0) return;
    setShowClearConfirm(true);
  }, [lines.length]);

  // ── Scroll preservation ──────────────────────────────────────
  const [savedScrollTop, setSavedScrollTop] = useState(0);

  // Save scroll position before swapping to a sub-view; restore after remount.
  // The real scroll container is .retail-grid (overflow-y: auto); .retail-main
  // itself is overflow-y: hidden, so read from the grid element. The data-testid
  // doubles as the scroll-container contract used by the scroll-preservation test
  // — keep it in sync with RetailProductGrid's grid wrapper (intentional coupling).
  const getScrollContainer = useCallback(() => {
    return retailPosRef.current?.querySelector<HTMLElement>('[data-testid="product-grid-scroll"]') ?? null;
  }, []);
  const saveScroll = useCallback(() => {
    const el = getScrollContainer();
    if (el) setSavedScrollTop(el.scrollTop);
  }, [getScrollContainer]);
  const goToSubView = useCallback((setter: (v: boolean) => void) => {
    saveScroll();
    setter(true);
  }, [saveScroll]);

  // ── Products & Categories ────────────────────────────────────

  const [products, setProducts] = useState<ProductDto[]>([]);
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [productsLoading, setProductsLoading] = useState(true);
  const [categoriesLoading, setCategoriesLoading] = useState(true);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [filterLowStock, setFilterLowStock] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const loadProductsAndCategories = useCallback((token: string) => {
    // Abort any previous in-flight request to prevent race condition
    if (loadProductsAbortRef.current) {
      loadProductsAbortRef.current.abort();
    }
    const controller = new AbortController();
    loadProductsAbortRef.current = controller;
    setProductsLoading(true);
    setCategoriesLoading(true);
    setLoadError(null);
    listProductsScoped(token)
      .then((prods) => { if (!controller.signal.aborted) setProducts(prods); })
      .catch(() => {
        if (controller.signal.aborted) return;
        setProducts(RETAIL_SAMPLE_PRODUCTS);
        setLoadError(requiredLocalized(l10nRef.current, 'retail-load-error'));
      })
      .finally(() => { if (!controller.signal.aborted) setProductsLoading(false); });
    listCategories()
      .then((cats) => {
        if (controller.signal.aborted) return;
        setCategories(cats && cats.length > 0 ? cats : RETAIL_SAMPLE_CATEGORIES);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setCategories(RETAIL_SAMPLE_CATEGORIES);
        setLoadError(requiredLocalized(l10nRef.current, 'retail-load-error'));
      })
      .finally(() => {
        if (!controller.signal.aborted) setCategoriesLoading(false);
      });
    return () => { controller.abort(); };
  }, []);

  useEffect(() => loadProductsAndCategories(sessionToken), [sessionToken, loadProductsAndCategories]);

  const handleRetryLoad = useCallback(() => {
    loadProductsAndCategories(sessionToken);
  }, [sessionToken, loadProductsAndCategories]);

  // Tracks the latest AbortController so handleRetryLoad and rapid
  // sessionToken changes abort the previous in-flight fetch,
  // preventing stale data from overwriting fresh results.
  const loadProductsAbortRef = useRef<AbortController | null>(null);

  const [searchQuery, setSearchQuery] = useState('');

  const allLabel = requiredLocalized(l10n, 'product-lookup-all-categories');
  const catLabels = useMemo(() => {
    const m = new Map<string, string>();
    categories.forEach((c) => {
      const catId = `category-${c.id}`;
      const label = l10nRef.current.getString(catId);
      m.set(c.id, label !== catId ? label : c.name);
    });
    return m;
  }, [categories]); // l10n via ref

  const lowStockCount = useMemo(
    () => products.filter((p) => {
      if (p.stock_qty == null || p.stock_qty <= 0) return false;
      const threshold = p.low_stock_threshold ?? 5;
      return p.stock_qty <= threshold;
    }).length,
    [products],
  );

  const [productPage, setProductPage] = useState(0);
  const PAGE_SIZE = 50;

  const [editingProduct, setEditingProduct] = useState<ProductDto | null>(null);
  const [isAddCategoryOpen, setIsAddCategoryOpen] = useState(false);
  const [isAddProductOpen, setIsAddProductOpen] = useState(false);

  const handleEditProduct = useCallback((p: ProductDto) => {
    setEditingProduct(p);
  }, []);

  const handleSaveProductEdit = useCallback((updatedProduct: ProductDto) => {
    setProducts((prev) =>
      prev.map((p) => (p.sku === updatedProduct.sku ? updatedProduct : p)),
    );
    setEditingProduct(null);
  }, [setProducts]);

  const handleSaveNewCategory = useCallback((newCat: CategoryDto) => {
    setCategories((prev) => [...prev, newCat]);
    setActiveCategory(newCat.id);
  }, [setCategories]);

  const handleSaveNewProduct = useCallback((newProd: ProductDto) => {
    setProducts((prev) => [newProd, ...prev]);
  }, [setProducts]);

  const filteredProducts = useMemo(() => {
    let list = products.filter((p) => p.product_type === 'retail');
    if (activeCategory) {
      const activeCatObj = categories.find((c) => c.id === activeCategory);
      list = list.filter(
        (p) =>
          p.category === activeCategory ||
          (activeCatObj && p.category === activeCatObj.name) ||
          p.category?.toLowerCase() === activeCategory.toLowerCase(),
      );
    }
    if (filterLowStock) {
      list = list.filter((p) => {
        if (p.stock_qty == null || p.stock_qty <= 0) return false;
        const threshold = p.low_stock_threshold ?? 5;
        return p.stock_qty <= threshold;
      });
    }
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      list = list.filter((p) => p.name.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q));
    }
    return list;
  }, [products, activeCategory, searchQuery, categories, filterLowStock]);

  const [sortField, setSortField] = useState<SortField>('sku');
  const [sortOrder, setSortOrder] = useState<SortOrder>('asc');

  const handleSort = useCallback((field: SortField) => {
    if (sortField === field) {
      setSortOrder((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortOrder('asc');
    }
  }, [sortField]);

  const sortedProducts = useMemo(() => {
    const list = [...filteredProducts];
    list.sort((a, b) => {
      let comp = 0;
      if (sortField === 'sku') {
        comp = a.sku.localeCompare(b.sku);
      } else if (sortField === 'name') {
        comp = a.name.localeCompare(b.name);
      } else if (sortField === 'stock') {
        const stockA = a.stock_qty ?? 0;
        const stockB = b.stock_qty ?? 0;
        comp = stockA - stockB;
      } else if (sortField === 'price') {
        comp = a.price.minor_units - b.price.minor_units;
      }
      return sortOrder === 'asc' ? comp : -comp;
    });
    return list;
  }, [filteredProducts, sortField, sortOrder]);

  const totalPages = Math.max(1, Math.ceil(sortedProducts.length / PAGE_SIZE));
  const pagedProducts = useMemo(
    () => sortedProducts.slice(productPage * PAGE_SIZE, (productPage + 1) * PAGE_SIZE),
    [sortedProducts, productPage],
  );

  // Reset page when filter changes
  useEffect(() => { setProductPage(0); }, [activeCategory, searchQuery, filterLowStock]);

  const catHue = useCallback((catId: string | null) => {
    if (!catId) return 210;
    let h = 0;
    for (let i = 0; i < catId.length; i++) h = (h * 31 + catId.charCodeAt(i)) | 0;
    return Math.abs(h) % 360;
  }, []);

  const handleAdd = useCallback((p: ProductDto) => {
    if (p.stock_qty != null) {
      const inCart = linesRef.current.filter((l) => l.sku === p.sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + 1 > p.stock_qty) {
        addToast({ message: requiredLocalized(l10n, 'retail-toast-insufficient-stock', { name: p.name }), type: 'warning' });
        return;
      }
    }
    addProduct(toProduct(p));
    announce(requiredLocalized(l10nRef.current, 'retail-added-to-cart', { name: p.name }));
  }, [addProduct, addToast, l10n, announce]);

  const handleWeighAdd = useCallback((sku: Sku, weightGrams: number) => {
    const product = products.find((p) => p.sku === sku);
    if (!product) return;
    const qty = Math.max(1, Math.round(weightGrams));
    if (product.stock_qty != null) {
      const inCart = linesRef.current.filter((l) => l.sku === sku).reduce((s, l) => s + l.qty, 0);
      if (inCart + qty > product.stock_qty) {
        addToast({ message: requiredLocalized(l10n, 'retail-toast-insufficient-stock', { name: product.name }), type: 'warning' });
        return;
      }
    }
    addProduct(toProduct(product), qty);
    announce(requiredLocalized(l10nRef.current, 'retail-added-to-cart', { name: product.name }));
    setWeighTarget(null);
    addToast({ message: requiredLocalized(l10n, 'scale-weigh-added', { name: product.name, weight: qty }), type: 'success' });
  }, [products, addProduct, addToast, l10n, announce]);

  const handleSetWeighTarget = useCallback((p: ProductDto) => {
    if (weighTarget?.sku === p.sku) return;
    setWeighTarget({ sku: p.sku as Sku, name: p.name });
    addToast({ message: requiredLocalized(l10n, 'scale-target-set', { name: p.name }), type: 'info' });
  }, [weighTarget, addToast, l10n]);

  /** Stock-aware cart qty increase — checks stock_qty before incrementing. */
  const handleIncreaseQty = useCallback((line: { sku: string; id: LineId; qty: number }) => {
    const product = products.find((p) => p.sku === line.sku);
    if (product?.stock_qty != null) {
      const otherLinesQty = linesRef.current
        .filter((l) => l.sku === line.sku && l.id !== line.id)
        .reduce((s, l) => s + l.qty, 0);
      if (otherLinesQty + line.qty + 1 > product.stock_qty) {
        addToast({ message: requiredLocalized(l10n, 'retail-toast-insufficient-stock', { name: product.name }), type: 'warning' });
        return;
      }
    }
    updateQty(line.id, line.qty + 1);
  }, [products, updateQty, addToast, l10n]);

  /** Open the modifier editor for a cart line. */
  const handleEditModifiers = useCallback((line: CartLine) => {
    setModifierLine(line);
  }, []);

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
      const found = await lookupProductBySkuScoped(sessionToken, val);
      if (found) { handleAdd(found); return; }
    } catch { /* unreachable */ }
    addToast({ message: requiredLocalized(l10n, 'retail-sku-not-found', { sku: val }), type: 'warning' });
  }, [skuInput, handleAdd, addToast, l10n, sessionToken]);

  const handleBarcode = useCallback(async (payload: { code: string }) => {

    const list = productsRef.current;
    const found = list.find((x) => x.barcode === payload.code);
    if (found) { handleAdd(found); setScanFlash(true); playBeep(); if (scanFlashTimerRef.current) clearTimeout(scanFlashTimerRef.current); scanFlashTimerRef.current = setTimeout(() => { setScanFlash(false); scanFlashTimerRef.current = null; }, 300); return; }
    try {
      const p = await lookupByBarcodeScoped(sessionToken, payload.code);
      if (p) { handleAdd(p); setScanFlash(true); playBeep(); if (scanFlashTimerRef.current) clearTimeout(scanFlashTimerRef.current); scanFlashTimerRef.current = setTimeout(() => { setScanFlash(false); scanFlashTimerRef.current = null; }, 300); return; }
    } catch { /* unreachable */ }
    playError();
    addToast({ message: requiredLocalized(l10n, 'pos-no-barcode-match'), type: 'warning' });
  }, [handleAdd, addToast, l10n, playBeep, playError, sessionToken]);

  useBarcodeScanner({ onProductFound: handleBarcode });

  // ── Store settings ──────────────────────────────────────────

  const [storeSettings, setStoreSettings] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  useEffect(() => {
    let mounted = true;
    getStoreSettingsScoped(sessionToken).then((s) => { if (mounted) setStoreSettings(s); }).catch(() => { if (mounted) addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-failed-settings'), type: 'error' }); });
    return () => { mounted = false; };
  }, [addToast, sessionToken]);

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
    getActiveShiftScoped(sessionToken)
      .then((s) => setActiveShift(s))
      .catch(() => setActiveShift(null))
      .finally(() => setShiftLoading(false));
  }, [sessionToken]);

  const handleOpenShift = useCallback(async () => {
    const val = Math.round(parseFloat(openingBalance) * 100);
    if (Number.isNaN(val) || val < 0) return;
    setOpeningShift(true);
    try {
      const s = await openShiftScoped(sessionToken, val);
      setActiveShift(s);
      setShowOpenShift(false);
      setOpeningBalance('');
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-failed-open-shift'), type: 'error' });
    } finally {
      setOpeningShift(false);
    }
  }, [openingBalance, addToast, l10n, sessionToken]);

  const handleCloseShift = useCallback(async () => {
    if (!activeShift) return;
    const val = Math.round(parseFloat(closingBalance) * 100);
    if (Number.isNaN(val) || val < 0) return;
    setClosingShift(true);
    setCloseShiftError(null);
    try {
      const s = await closeShiftScoped(sessionToken, activeShift.id, val, shiftNotes || null);
      setClosedShiftSummary(s);
      setActiveShift(null);
    } catch (e) {
      setCloseShiftError((e instanceof Error ? e.message : String(e)) ?? (requiredLocalized(l10n, 'pos-close-shift-failed')));
    } finally {
      setClosingShift(false);
    }
  }, [activeShift, closingBalance, shiftNotes, l10n, sessionToken]);

  // ── Live tax preview ────────────────────────────────────────

  const [cartTax, setCartTax] = useState<number>(0);

  useEffect(() => {
    const controller = new AbortController();
    if (lines.length === 0 || !subtotal) {
      setCartTax(0);
      return () => { controller.abort(); };
    }
    const currency = subtotal.currency;
    const taxLines: CartLineTaxInput[] = lines.map((l) => ({
      sku: String(l.sku),
      qty: l.qty,
      unit_price_minor: l.unit_price.minor_units,
    }));
    computeCartTax(sessionToken, taxLines, currency)
      .then((tax) => { if (!controller.signal.aborted) setCartTax(tax); })
      .catch(() => { if (!controller.signal.aborted) setCartTax(0); });
    return () => { controller.abort(); };
  }, [lines, subtotal, sessionToken]);

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
    if (Number.isNaN(rp) || rp <= 0 || !subtotal || subtotal.minor_units === 0) return;
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

  // ── Modifier modal state ──────────────────────────────────────
  const [modifierLine, setModifierLine] = useState<CartLine | null>(null);

  // Moved below [selectedCustomer, setSelectedCustomer] so it can reset
  // discount and customer on cart clear (was previously defined before
  // those states existed, causing a TDZ bug when they were added to deps).
  const handleConfirmClear = useCallback(() => {
    setCartId(null);
    resetCart();
    setDiscount(0, '');
    setUndoStack([]);
    setSerialNumbers({});
    setSelectedCustomer(null);
    setModifierLine(null);
    setShowClearConfirm(false);
  }, [resetCart, setDiscount, setSelectedCustomer, setModifierLine]);

  const ensureCart = useCallback(async (currency: string): Promise<CartId | null> => {
    if (cartId) return cartId;
    try {
      const { cartId: newCartId } = await startSaleScoped(sessionToken, { currency });
      setCartId(newCartId);
      return newCartId;
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-failed-cart'), type: 'error' });
      return null;
    }
  }, [cartId, addToast, l10n, sessionToken]);

  const handleOverrideConfirm = useCallback(async (newPriceMinor: number) => {
    if (!overrideTarget) return;
    const cId = cartId;
    if (!cId) {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-no-cart'), type: 'error' });
      setOverrideTarget(null);
      return;
    }
    try {
      await overrideLinePriceScoped(sessionToken, cId, overrideTarget.id, newPriceMinor);
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
  }, [overrideTarget, cartId, addToast, l10n, updateLinePrice, sessionToken]);

  const allCustomersRef = useRef<CustomerDto[]>([]);
  const customerSearchQueryRef = useRef(customerSearchQuery);
  customerSearchQueryRef.current = customerSearchQuery;

  // Fetch customer list when the modal opens; filter locally on keystrokes
  useEffect(() => {
    if (!showCustomerSearch) { setCustomerSearchResults([]); return; }
    let cancelled = false;
    setLoadingCustomers(true);
    listCustomers()
      .then((customers) => {
        if (cancelled) return;
        allCustomersRef.current = customers;
        const q = customerSearchQueryRef.current.trim().toLowerCase();
        setCustomerSearchResults(
          !q ? customers : customers.filter(
            (c) =>
              c.name.toLowerCase().includes(q) ||
              (c.phone && c.phone.includes(q)) ||
              (c.email && c.email.toLowerCase().includes(q)),
          ),
        );
      })
      .catch(() => { if (cancelled) return; addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-customers-failed'), type: 'error' }); setCustomerSearchResults([]); })
      .finally(() => { if (cancelled) return; setLoadingCustomers(false); });
    return () => { cancelled = true; };
  }, [showCustomerSearch, addToast]); // l10n via ref — fetch only when modal opens

  // Filter cached customers locally on keystroke — avoids redundant API calls
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
  }, [customerSearchQuery, showCustomerSearch]);

  // ── Payment modal ────────────────────────────────────────────

  const [showPayment, setShowPayment] = useState(false);

  // P7-1: Swipe left on cart panel → open payment modal (tablet flow)
  const cartSwipe = useSwipe({
    onSwipeLeft: () => {
      if (!activeShift) { return; }
      if (!total) return;
      setShowPayment(true);
    },
  });

  const handlePay = useCallback(() => {
    if (!activeShift) { addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-open-shift-first'), type: 'warning' }); return; }
    setShowPayment(true);
  }, [activeShift, addToast]); // l10n via ref

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
      const { id } = await holdCartScoped(sessionToken, {
        label: `Hold #${Date.now()}`,
        cart_data: cartData,
        item_count: lines.length,
        total_minor: subtotal.minor_units,
        currency: subtotal.currency,
        bill_type: 'hold',
      });
      setHeldCartId(id);
      resetCart();
      addToast({ message: requiredLocalized(l10n, 'retail-toast-order-held'), type: 'success' });
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-failed-hold'), type: 'error' });
    }
  }, [lines, discountPercent, discountLabel, subtotal, resetCart, addToast, l10n, sessionToken]);

  const handleResumeCart = useCallback(async (cartId: string) => {
    try {
      const full = await getHeldCartScoped(sessionToken, cartId);
      if (!full) return;
      const cleanupCorrupt = async () => {
        await deleteHeldCartScoped(sessionToken, cartId).catch(() => {});
        setHeldCartId(null);
        addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-corrupt-cart'), type: 'error' });
      };
      let data: Record<string, unknown>;
      try {
        data = JSON.parse(full.cart_data);
      } catch {
        await cleanupCorrupt();
        return;
      }
      if (!Array.isArray(data['lines'])) {
        await cleanupCorrupt();
        return;
      }
      const rawLines = data['lines'] as { sku: string; name: string; category: string; qty: number; unit_price: Money }[];
      // Validate each line has required fields; skip corrupt ones
      let hasCorruptLines = false;
      for (const l of rawLines) {
        if (!l.sku || !l.unit_price || typeof l.unit_price.minor_units !== 'number') {
          hasCorruptLines = true;
          continue;
        }
        const qty = Number.isFinite(l.qty) && l.qty > 0 ? Math.round(l.qty) : 1;
        addProduct({ sku: l.sku as Sku, name: l.name, category: l.category ?? '', price: l.unit_price, barcode: null, inStock: true, stockQty: null, productType: 'retail' }, qty);
      }
      if (hasCorruptLines) {
        addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-corrupt-cart'), type: 'error' });
      }
      if (data['discountPercent']) setDiscount(data['discountPercent'] as number, (data['discountLabel'] as string) ?? '');
      await deleteHeldCartScoped(sessionToken, cartId);
      setHeldCartId(null);
      setShowHeldCartsList(false);
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-failed-resume'), type: 'error' });
    }
  }, [addProduct, setDiscount, addToast, l10n, sessionToken]);

  const handleResume = useCallback(async () => {
    try {
      const carts = await listHeldCartsScoped(sessionToken);
      const held = carts.filter((c) => c.bill_type === 'hold');
      if (held.length === 0) return;
      if (held.length === 1) {
        await handleResumeCart(held[0]!.id);
        return;
      }
      setHeldCartsList(held);
      setShowHeldCartsList(true);
    } catch {
      addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-failed-load-held'), type: 'error' });
    }
  }, [handleResumeCart, addToast, sessionToken]);

  // ── Held cart delete confirm (P1-3) ────────────────────────────
  const [deleteHeldTarget, setDeleteHeldTarget] = useState<HeldCartRow | null>(null);
  const retailDeleteHeldExit = useExitAnimation(!!deleteHeldTarget, () => setDeleteHeldTarget(null));

  const handleDeleteHeldCart = useCallback(async (cartId: string) => {
    try {
      await deleteHeldCartScoped(sessionToken, cartId);
      setHeldCartsList((prev) => prev.filter((c) => c.id !== cartId));
      if (heldCartId === cartId) setHeldCartId(null);
      addToast({ type: 'success', message: requiredLocalized(l10nRef.current, 'retail-toast-held-cart-deleted') });
    } catch {
      addToast({ type: 'error', message: requiredLocalized(l10nRef.current, 'retail-toast-failed-delete-held') });
    } finally {
      setDeleteHeldTarget(null);
    }
  }, [sessionToken, heldCartId, addToast]); // l10n via ref

  // ── Load persisted held carts on mount ───────────────────────

  useEffect(() => {
    let mounted = true;
    listHeldCartsScoped(sessionToken)
      .then((carts) => {
        if (!mounted) return;
        const held = carts.find((c) => c.bill_type === 'hold');
        if (held) setHeldCartId(held.id);
      })
      .catch(() => { if (mounted) addToast({ message: requiredLocalized(l10nRef.current, 'retail-toast-failed-load-held'), type: 'error' }); });
    return () => { mounted = false; };
  }, [sessionToken, addToast]); // l10n via ref — stable dep chain

  // ── Options / Sub-views ────────────────────────────────

  const [showSalesHistory, setShowSalesHistory] = useState(false);
  const [showStockInquiry, setShowStockInquiry] = useState(false);
  const [showTables, setShowTables] = useState(false);

  const inSubView = showSalesHistory || showTables || showStockInquiry;
  // Restore scroll only when returning from a sub-view (element remounts),
  // not while inside one — the effect keyed on inSubView flips to false on return.
  useEffect(() => {
    if (inSubView) return;
    if (savedScrollTop > 0) {
      const el = getScrollContainer();
      if (el) {
        el.scrollTop = savedScrollTop;
        setSavedScrollTop(0);
      }
    }
  }, [inSubView, savedScrollTop, getScrollContainer]);

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
      await settleCreditScoped(sessionToken, saleId);
      setCreditSales((prev) => prev.filter((c) => c.saleId !== saleId));
      addToast({ message: requiredLocalized(l10n, 'retail-toast-credit-settled'), type: 'success' });
    } catch {
      addToast({ message: requiredLocalized(l10n, 'retail-toast-failed-settle'), type: 'error' });
    } finally {
      setSettlingId(null);
    }
  }, [addToast, l10n, sessionToken]);

  // ── Retail modal exit animations ───────────────────────────────
  const retailCustomerExit = useExitAnimation(showCustomerSearch, () => { setShowCustomerSearch(false); setCustomerSearchQuery(''); });
  const retailQtyExit = useExitAnimation(showQtyPicker && !!pendingProduct, () => { setShowQtyPicker(false); setPendingProduct(null); });
  const retailHeldCartsExit = useExitAnimation(showHeldCartsList, () => setShowHeldCartsList(false));
  const retailShortcutsExit = useExitAnimation(showShortcuts, () => setShowShortcuts(false));
  const retailQuickReturnExit = useExitAnimation(showQuickReturn, () => { setShowQuickReturn(false); setQuickReturnBarcode(''); });
  const retailClearConfirmExit = useExitAnimation(showClearConfirm, () => setShowClearConfirm(false));
  const retailCreditListExit = useExitAnimation(showCreditList, () => setShowCreditList(false));

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

  const handleOpenSettings = useCallback(() => {
    if (onNavigate) {
      onNavigate('settings');
    } else {
      setActiveWorkspace('admin');
    }
  }, [onNavigate, setActiveWorkspace]);

  // ── Keyboard shortcuts ────────────────────────────────────────

  const isAnyModalOpen = useCallback(
    () => document.querySelector('[aria-modal="true"]') !== null,
    []);

  const isAnyOverlayOpen = useCallback(() =>
    showCustomerSearch || showHeldCartsList || showQuickReturn ||
    showDiscount || showQtyPicker || showShortcuts || showCreditList || showClearConfirm ||
    showOpenShift || (showCloseShift && !closedShiftSummary) || !!closedShiftSummary ||
    !!editingProduct || isAddCategoryOpen || isAddProductOpen || !!deleteHeldTarget,
  [showCustomerSearch, showHeldCartsList, showQuickReturn, showDiscount, showQtyPicker,
    showShortcuts, showCreditList, showClearConfirm, showOpenShift, showCloseShift,
    closedShiftSummary, editingProduct, isAddCategoryOpen, isAddProductOpen, deleteHeldTarget]);

  useEffect(() => {

    const handler = (e: KeyboardEvent) => {
      // Guard: block all hotkeys while any aria-modal is open (e.g. WorkspaceSettingsModal)
      if (isAnyModalOpen()) return;

      // Escape: handled per-modal by useFocusTrap (Phase A gave all overlays
      // aria-modal="true", so isAnyModalOpen() above already returned).

      // Guard: block hotkeys while local overlays/dialogs are visible
      if (isAnyOverlayOpen()) return;
      switch (e.key) {
        case 'F1': if (e.cancelable) e.preventDefault(); handlePay(); break;
        case 'F2': if (e.cancelable) e.preventDefault(); if (lines.length > 0) handleRequestClear(); break;
        case 'F3': if (e.cancelable) e.preventDefault(); if (lines.length > 0) setShowDiscount(true); break;
        case 'F4': if (e.cancelable) e.preventDefault(); if (heldCartId) handleResume(); else handleHold(); break;
        case 'F5': if (e.cancelable) e.preventDefault(); skuInputRef.current?.focus(); break;
        case 'F6': if (e.cancelable) e.preventDefault(); goToSubView(setShowSalesHistory); break;
        case 'F7': if (e.cancelable) e.preventDefault(); setShowCustomerSearch(true); break;
        case 'F8': if (e.cancelable) e.preventDefault(); goToSubView(setShowStockInquiry); break;
        case 'F9': if (e.cancelable) e.preventDefault(); if (activeShift) setShowCloseShift(true); else setShowOpenShift(true); break;
        // F10 is handled globally by AppShell.tsx — opens the WorkspaceSettingsModal.
        // The button-based settings navigation (onOpenSettings) still works via RetailFnBar.
        case 'F11': if (e.cancelable) e.preventDefault(); setShowQuickReturn(true); break;
        case '?': setShowShortcuts((v) => !v); break;
        case 'F12': if (e.cancelable) e.preventDefault(); onNavigate?.('kds'); break;
        case 'l': if (e.ctrlKey && document.activeElement?.tagName !== 'INPUT') { e.preventDefault(); setFilterLowStock((prev) => !prev); } break;
        case 'k': if (e.ctrlKey && document.activeElement?.tagName !== 'INPUT') { e.preventDefault(); setShowCreditList(true); } break;
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  // handleOpenSettings deliberately excluded — F10 removed (handled by AppShell).
  }, [isAnyModalOpen, isAnyOverlayOpen, showPayment, showOpenShift, showCloseShift, showDiscount, showQtyPicker, showShortcuts, showCustomerSearch, showClearConfirm, showCreditList, showSalesHistory, showStockInquiry, showTables, showHeldCartsList, showQuickReturn, closedShiftSummary, editingProduct, isAddCategoryOpen, isAddProductOpen, handlePay, lines.length, handleRequestClear, handleHold, handleResume, heldCartId, activeShift, session, addToast, onNavigate, goToSubView]);

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
        {...(sessionToken ? { sessionToken } : {})}
        selectedCustomer={selectedCustomer}
        {...(isEnabled(FEATURES.SERIAL_TRACKING) ? { serialNumbers } : {})}
        onCustomerChange={(c) => setSelectedCustomer(c)}
        tenderPresets={tenderPresets}
        onComplete={() => { setShowPayment(false); resetCart(); setSelectedCustomer(null); playSuccess(); addToast({ message: requiredLocalized(l10n, 'retail-toast-sale-complete'), type: 'success' }); }}
        onClose={() => setShowPayment(false)}
      />
    );
  }

  // ── Sales History screen ────────────────────────────────────
  if (showSalesHistory) {
    return <SalesHistoryView theme={theme} onBack={() => setShowSalesHistory(false)} />;
  }

  // ── Table Management screen ────────────────────────────────
  if (showTables) {
    return <TableManagementView theme={theme} onBack={() => setShowTables(false)} />;
  }

  // ── Stock Inquiry screen ────────────────────────────────────
  if (showStockInquiry) {
    return <StockInquiryView theme={theme} onBack={() => setShowStockInquiry(false)} onAddProduct={handleAdd} />;
  }

  return (
    <>
    <div className="retail-pos" data-theme={theme}>
      {/* ── Skip-to-content link ─────────────── */}
      <a href="#retail-main" className="retail-skip-link">
        {requiredLocalized(l10n, 'retail-skip-to-main')}
      </a>

      {/* ── Header ──────────────────────────── */}
      <RetailHeader
        storeSettings={storeSettings}
        shiftLoading={shiftLoading}
        activeShift={activeShift}
        displayName={session?.display_name ?? ''}
        dateStr={dateStr}
        timeStr={timeStr}
        shiftDuration={shiftDuration}
        onWorkspacePicker={goToWorkspacePicker}
      />


      {/* ── Main area ───────────────────────── */}
      <div id="retail-main" className="retail-main" ref={retailPosRef}>
        {/* Screen-reader announcement (visually hidden) */}
        <div ref={announceRef} className="retail-sr-only" data-testid="retail-sr-announce" role="status" aria-live="polite" aria-atomic="true" />
        {/* Left: product grid */}
        {filterLowStock && (
          <div className="retail-filter-indicator" role="status" aria-label={requiredLocalized(l10n, 'retail-filter-indicator-aria')}>
            <svg viewBox="0 0 20 20" fill="currentColor" width="12" height="12" aria-hidden="true">
              <path fillRule="evenodd" d="M3 3a1 1 0 011 0v12a1 1 0 11-2 0V4a1 1 0 011-1zm7.707 3.293a1 1 0 010 1.414L9.414 9H17a1 1 0 110 2H9.414l1.293 1.293a1 1 0 01-1.414 1.414l-3-3a1 1 0 010-1.414l3-3a1 1 0 011.414 0z" clipRule="evenodd" />
            </svg>
            <span>{requiredLocalized(l10n, 'retail-filtered-low-stock', { count: lowStockCount })}</span>
          </div>
        )}
        {/* ── Error banner ──────────────── */}
        {loadError && (
          <div className="retail-load-error" role="alert">
            <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
              <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
            </svg>
            <span className="retail-load-error-text">{loadError}</span>
            <button
              type="button"
              className="retail-load-error-retry"
              onClick={handleRetryLoad}
              aria-label={requiredLocalized(l10n, 'retail-load-error-retry-aria')}
            >
              {requiredLocalized(l10n, 'retry')}
            </button>
            <button
              type="button"
              className="retail-load-error-dismiss"
              onClick={() => setLoadError(null)}
              aria-label={requiredLocalized(l10n, 'dismiss')}
            >
              <svg viewBox="0 0 20 20" fill="currentColor" width="12" height="12" aria-hidden="true">
                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
              </svg>
            </button>
          </div>
        )}
        <RetailProductGrid
          data={{
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
          }}
          actions={{
            onSetActiveCategory: setActiveCategory,
            onSetSearchQuery: setSearchQuery,
            onSort: handleSort,
            onSetProductPage: setProductPage,
            onAddProduct: handleAdd,
            onEditProduct: handleEditProduct,
            onOpenQtyPicker: handleOpenQtyPicker,
            onSetWeighTarget: handleSetWeighTarget,
            onClearWeighTarget: () => setWeighTarget(null),
            onAddCategory: () => setIsAddCategoryOpen(true),
            onAddNewProduct: () => setIsAddProductOpen(true),
            onSkuInputChange: setSkuInput,
            onSkuSubmit: handleSkuSubmit,
            onWeighAdd: handleWeighAdd,
          }}
          isScaleEnabled={isEnabled(FEATURES.USB_SCALE)}
          catHue={catHue}
          skuInputRef={skuInputRef}
        />

        {/* ── Resize handle ────────────────── */}
        <RetailCartPanel
          lines={lines}
          lineCount={lineCount}
          selectedCustomer={selectedCustomer}
          totals={{
            subtotal,
            total,
            discountPercent,
            discountAmount,
            cartTax,
          }}
          retailCartWidth={retailCartWidth}
          serialNumbers={serialNumbers}
          trackSerialMap={trackSerialMap}
          overrideTarget={overrideTarget}
          undoStack={undoStack}
          undoBarExit={{
            shouldRender: undoBarExit.shouldRender,
            exiting: undoBarExit.exiting,
            requestClose: undoBarExit.requestClose,
          }}
          isSerialTracking={isEnabled(FEATURES.SERIAL_TRACKING)}
          isManager={isManager}
          activeShift={!!activeShift}
          heldCartId={heldCartId}
          cartWidthMin={RETAIL_CART_WIDTH_MIN}
          cartWidthMaxCap={RETAIL_CART_WIDTH_MAX_CAP}
          onResizeWidth={setRetailCartWidth}
          onStartResize={startResize}
          cartSwipe={cartSwipe as Record<string, unknown>}
          showCourseSelector={true}
          lineActions={{
            onRemoveLine: handleRemoveLine,
            onIncreaseQty: handleIncreaseQty,
            onUpdateQty: updateQty,
            onSerialChange: handleSerialChange,
            onSetOverrideTarget: setOverrideTarget,
            onAssignCourse: (lineId, courseId) => { assignCourse(lineId, courseId as CourseId); },
            onEditModifiers: handleEditModifiers,
          }}
          panelActions={{
            onPay: handlePay,
            onShowDiscount: () => setShowDiscount(true),
            onHoldResume: heldCartId ? handleResume : handleHold,
            onRequestClear: handleRequestClear,
            onShowCreditList: () => setShowCreditList(true),
            onLoadCreditSales: loadCreditSales,
          }}
          onUndoRemove={handleUndoRemove}
          onDismissUndo={handleDismissUndo}
          onEnsureCart={ensureCart}
        />
      </div>

      {/* ── Function bar (bottom) ──────────── */}
      <RetailFnBar
        linesLength={lines.length}
        heldCartId={heldCartId}
        activeShift={!!activeShift}
        onPay={handlePay}
        onRequestClear={() => { if (!isAnyOverlayOpen()) handleRequestClear(); }}
        onShowDiscount={() => { if (!isAnyOverlayOpen()) setShowDiscount(true); }}
        onHoldResume={heldCartId ? handleResume : handleHold}
        onShowSalesHistory={() => goToSubView(setShowSalesHistory)}
        onShowCustomerSearch={() => { if (!isAnyOverlayOpen()) setShowCustomerSearch(true); }}
        onShowStockInquiry={() => goToSubView(setShowStockInquiry)}
        onToggleShift={() => { if (!isAnyOverlayOpen()) { if (activeShift) setShowCloseShift(true); else setShowOpenShift(true); } }}
        onOpenSettings={handleOpenSettings}
        onShowQuickReturn={() => { if (!isAnyOverlayOpen()) setShowQuickReturn(true); }}
        onShowTables={() => goToSubView(setShowTables)}
        onNavigateKds={() => onNavigate?.('kds')}
        skuInputRef={skuInputRef}
      />
      <RetailReminderPopup
        lowStockCount={lowStockCount}
        creditCount={creditSales.length}
        heldCartCount={heldCartsList.length}
        lowStockActive={filterLowStock}
        onClickLowStock={() => setFilterLowStock((prev) => !prev)}
        onClickCredit={() => { if (!isAnyOverlayOpen()) setShowCreditList(true); }}
        onClickHeldCarts={() => { if (!isAnyOverlayOpen()) setShowHeldCartsList(true); }}
      />
      <RetailModals
        shift={{
          activeShift,
          openShiftExit: { shouldRender: retailOpenShiftExit.shouldRender, exiting: retailOpenShiftExit.exiting, requestClose: retailOpenShiftExit.requestClose },
          closeShiftExit: { shouldRender: retailCloseShiftExit.shouldRender, exiting: retailCloseShiftExit.exiting, requestClose: retailCloseShiftExit.requestClose },
          shiftSummaryExit: { shouldRender: retailShiftSummaryExit.shouldRender, exiting: retailShiftSummaryExit.exiting, requestClose: retailShiftSummaryExit.requestClose },
          closedShiftSummary,
          openingBalance,
          closingBalance,
          shiftNotes,
          openingShift,
          closingShift,
          closeShiftError,
          storeSettings: { currency: storeSettings.currency },
          onOpeningBalanceChange: setOpeningBalance,
          onClosingBalanceChange: setClosingBalance,
          onShiftNotesChange: setShiftNotes,
          onOpenShift: handleOpenShift,
          onCloseShift: handleCloseShift,
        }}
        discount={{
          exit: { shouldRender: retailDiscountExit.shouldRender, exiting: retailDiscountExit.exiting, requestClose: retailDiscountExit.requestClose },
          tab: discountTab,
          input: discountInput,
          rpInput: discountRpInput,
          onTabChange: setDiscountTab,
          onInputChange: setDiscountInput,
          onRpInputChange: setDiscountRpInput,
          onApplyPct: handleApplyDiscount,
          onApplyRp: handleApplyDiscountRp,
          onCancel: () => { retailDiscountExit.requestClose(); setDiscountInput(''); setDiscountRpInput(''); },
        }}
        customer={{
          exit: { shouldRender: retailCustomerExit.shouldRender, exiting: retailCustomerExit.exiting, requestClose: retailCustomerExit.requestClose },
          query: customerSearchQuery,
          results: customerSearchResults,
          loading: loadingCustomers,
          selected: selectedCustomer,
          onQueryChange: setCustomerSearchQuery,
          onSelect: (c) => { setSelectedCustomer(c); setShowCustomerSearch(false); setCustomerSearchQuery(''); },
          onClear: () => { setSelectedCustomer(null); setShowCustomerSearch(false); setCustomerSearchQuery(''); },
          onClose: () => retailCustomerExit.requestClose(),
        }}
        qtyPicker={{
          exit: { shouldRender: retailQtyExit.shouldRender, exiting: retailQtyExit.exiting, requestClose: retailQtyExit.requestClose },
          product: pendingProduct ? { name: pendingProduct.name, price: pendingProduct.price } : null,
          input: qtyInput,
          onInputChange: setQtyInput,
          onConfirm: handleConfirmQty,
          onCancel: () => retailQtyExit.requestClose(),
        }}
        heldCarts={{
          exit: { shouldRender: retailHeldCartsExit.shouldRender, exiting: retailHeldCartsExit.exiting, requestClose: retailHeldCartsExit.requestClose },
          list: heldCartsList,
          onResume: handleResumeCart,
          onDelete: (id) => { const row = heldCartsList.find((c) => c.id === id) ?? null; setDeleteHeldTarget(row); },
          onClose: () => retailHeldCartsExit.requestClose(),
        }}
        deleteHeldCartConfirm={{
          exit: { shouldRender: retailDeleteHeldExit.shouldRender, exiting: retailDeleteHeldExit.exiting, requestClose: retailDeleteHeldExit.requestClose },
          label: deleteHeldTarget?.label ?? '',
          onConfirm: () => { if (deleteHeldTarget) handleDeleteHeldCart(deleteHeldTarget.id); },
          onClose: () => retailDeleteHeldExit.requestClose(),
        }}
        credit={{
          exit: { shouldRender: retailCreditListExit.shouldRender, exiting: retailCreditListExit.exiting, requestClose: retailCreditListExit.requestClose },
          sales: creditSales,
          settlingId,
          onSettle: handleSettleCredit,
          onClose: () => retailCreditListExit.requestClose(),
        }}
        quickReturn={{
          exit: { shouldRender: retailQuickReturnExit.shouldRender, exiting: retailQuickReturnExit.exiting, requestClose: retailQuickReturnExit.requestClose },
          barcode: quickReturnBarcode,
          loading: quickReturnLoading,
          onBarcodeChange: setQuickReturnBarcode,
          onSubmit: handleQuickReturnSubmit,
          onClose: () => retailQuickReturnExit.requestClose(),
        }}
        clearConfirm={{
          exit: { shouldRender: retailClearConfirmExit.shouldRender, exiting: retailClearConfirmExit.exiting, requestClose: retailClearConfirmExit.requestClose },
          lineCount,
          onConfirm: handleConfirmClear,
          onClose: () => retailClearConfirmExit.requestClose(),
        }}
        shortcuts={{
          exit: { shouldRender: retailShortcutsExit.shouldRender, exiting: retailShortcutsExit.exiting, requestClose: retailShortcutsExit.requestClose },
          onClose: () => retailShortcutsExit.requestClose(),
        }}
        override={{
          target: overrideTarget,
          onConfirm: handleOverrideConfirm,
          onClose: () => setOverrideTarget(null),
        }}
        editProduct={{
          product: editingProduct,
          isOpen: Boolean(editingProduct),
          onClose: () => setEditingProduct(null),
          onSave: handleSaveProductEdit,
        }}
        addCategory={{
          isOpen: isAddCategoryOpen,
          onClose: () => setIsAddCategoryOpen(false),
          onSave: handleSaveNewCategory,
        }}
        addProduct={{
          categories,
          isOpen: isAddProductOpen,
          onClose: () => setIsAddProductOpen(false),
          onSave: handleSaveNewProduct,
        }}
        showQuickReturnRefund={showQuickReturnRefund}
        quickReturnSale={quickReturnSale}
        quickReturnRefundDone={handleQuickReturnRefundDone}
        scanFlash={scanFlash}
      />

      {/* ── Item modifier modal ──────────── */}
      <ItemModifierModal
        open={!!modifierLine}
        productName={modifierLine?.name ?? modifierLine?.sku ?? ''}
        basePriceMinor={modifierLine?.unit_price.minor_units ?? 0}
        currency={modifierLine?.unit_price.currency ?? 'IDR'}
        groups={[]}
        onConfirm={() => setModifierLine(null)}
        onClose={() => setModifierLine(null)}
      />
    </div>
  </>
  );
}
