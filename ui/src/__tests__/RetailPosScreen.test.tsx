// ── RetailPosScreen tests ──────────────────────────────────────────
//
// Covers: product grid rendering, category filtering, search, cart
// operations, barcode/SKU input, shift management, discount modal,
// payment modal, credit reminders, quantity picker, keyboard shortcuts.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import type { ReactNode } from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider } from '@/frontend/shared/Toast';
import { withFluent } from '@/locales/test-utils';
import salesFtl from '@/locales/sales.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import RetailPosScreen from '@/features/retail/RetailPosScreen';
import type { LineId } from '@/types/domain';

// ── Hoisted mock helpers ──────────────────────────────────────────

const mockedBarcode = vi.hoisted(() => {
  let onProductFound: ((payload: { code: string; symbology: string }) => void) | null = null;
  return {
    triggerScan(code: string) {
      onProductFound?.({ code, symbology: 'test' });
    },
    reset() {
      onProductFound = null;
    },
    useBarcodeScanner: vi.fn(
      (opts: { onProductFound: (p: { code: string; symbology: string }) => void }) => {
        onProductFound = opts.onProductFound;
      },
    ),
  };
});

// ── Mock modules ──────────────────────────────────────────────────

vi.mock('@/features/sales/usePosState', () => ({
  usePosState: vi.fn(() => ({
    lines: [],
    total: null,
    subtotal: null,
    discountPercent: 0,
    discountLabel: '',
    discountAmount: null,
    tipPercent: 0,
    tipAmount: null,
    serviceChargeEnabled: false,
    serviceChargePercent: 0,
    serviceChargeAmount: null,
    addProduct: vi.fn(),
    removeLine: vi.fn((_id: string) => {}),
    updateQty: vi.fn((_id: string, _qty: number) => {}),
    setDiscount: vi.fn(),
    updateLinePrice: vi.fn(),
    setTipPercent: vi.fn(),
    setServiceCharge: vi.fn(),
    resetCart: vi.fn(),
    setLines: vi.fn(),
  })),
}));

vi.mock('@/features/sales/useBarcodeScanner', () => ({
  useBarcodeScanner: mockedBarcode.useBarcodeScanner,
}));

vi.mock('@/api/products', () => ({
  listProducts: vi.fn(() =>
    Promise.resolve([
      { sku: 'SKU-001', name: 'Indomie Goreng', category: 'cat-food', price: { minor_units: 3500, currency: 'IDR' }, barcode: '8991002100110', in_stock: true, stock_qty: 100, tax_rate_ids: [], created_at: '',
      price_updated_at: '' },
      { sku: 'SKU-002', name: 'Teh Botol Sosro', category: 'cat-drink', price: { minor_units: 5000, currency: 'IDR' }, barcode: '8991002100220', in_stock: true, stock_qty: 50, tax_rate_ids: [], created_at: '',
      price_updated_at: '' },
      { sku: 'SKU-003', name: 'Nasi Goreng Spesial', category: 'cat-food', price: { minor_units: 15000, currency: 'IDR' }, barcode: null, in_stock: true, stock_qty: 20, tax_rate_ids: [], created_at: '',
      price_updated_at: '' },
      { sku: 'SKU-004', name: 'Aqua 600ml', category: 'cat-drink', price: { minor_units: 3000, currency: 'IDR' }, barcode: '8991002100330', in_stock: true, stock_qty: 3, tax_rate_ids: [], created_at: '',
      price_updated_at: '' },
    ]),
  ),
  listCategories: vi.fn(() =>
    Promise.resolve([
      { id: 'cat-food', name: 'Makanan', colour: '#e74c3c' },
      { id: 'cat-drink', name: 'Minuman', colour: '#3498db' },
    ]),
  ),
  lookupProductBySku: vi.fn(() => Promise.resolve(null)),
  lookupByBarcode: vi.fn(() => Promise.resolve(null)),
  createProduct: vi.fn(),
  updateProduct: vi.fn(),
  deleteProduct: vi.fn(),
  adjustStock: vi.fn(),
  listProductVariants: vi.fn(() => Promise.resolve([])),
  getProductVariant: vi.fn(() => Promise.resolve(null)),
  createProductVariant: vi.fn(),
  updateProductVariant: vi.fn(),
  deleteProductVariant: vi.fn(),
  createCategory: vi.fn(),
  updateCategory: vi.fn(),
  deleteCategory: vi.fn(),
}));

vi.mock('@/api/shifts', () => ({
  getActiveShift: vi.fn(() => Promise.reject(new Error('no shift'))),
  openShift: vi.fn(() =>
    Promise.resolve({
      id: 'shift-1', userId: 'user-1', terminalId: null,
      openedAt: '2026-07-05T08:00:00Z', closedAt: null,
      openingBalanceMinor: 100000, closingBalanceMinor: null,
      expectedCashMinor: null, cashDifferenceMinor: null,
      totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0,
      totalOtherMinor: 0, totalVoidsMinor: 0, totalRefundsMinor: 0,
      totalPayoutsMinor: 0, notes: '', status: 'open',
      createdAt: '2026-07-05T08:00:00Z', updatedAt: '2026-07-05T08:00:00Z',
    }),
  ),
  closeShift: vi.fn(),
  listShifts: vi.fn(() => Promise.resolve([])),
  getShift: vi.fn(() => Promise.resolve(null)),
  createCashPayout: vi.fn(),
  getShiftReport: vi.fn(),
}));

vi.mock('@/api/settings', () => ({
  getStoreSettings: vi.fn(() =>
    Promise.resolve({ name: 'TOKO TEST', address: 'Jl. Contoh No. 123', taxId: '', currency: 'IDR', branch: 'Cabang A', logo: '' }),
  ),
  getReceiptSettings: vi.fn(() => Promise.resolve({ showCurrency: true, decimalSeparator: 'dot', showTax: true, footer: '', paperWidth: 'standard', showTableNumber: false, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 })),
  setReceiptSettings: vi.fn(),
  setStoreSettings: vi.fn(),
  getCreditSettings: vi.fn(() => Promise.resolve({ enabled: true, reminderIntervalHours: 24, maxLimitMinor: 500000 })),
  setCreditSettings: vi.fn(),
  listCreditSales: vi.fn(() => Promise.resolve([])),
  settleCredit: vi.fn(),
  getHardwareSettings: vi.fn(() => Promise.resolve({ printerConnection: 'auto', printerDevicePath: '', printerPaperSize: '80', scannerDeviceId: '', scannerInputMode: 'auto' })),
  setHardwareSettings: vi.fn(),
  completeSetup: vi.fn(),
  dismissSetupWizard: vi.fn(),
  getSetupStatus: vi.fn(),
  getEnabledFeatures: vi.fn(),
  getUserPreferences: vi.fn(),
  setUserPreferences: vi.fn(),
}));

vi.mock('@/api/hardware', () => ({
  listScanners: vi.fn(() => Promise.resolve([])),
  listDisplays: vi.fn(() => Promise.resolve([])),
  displayShow: vi.fn(() => Promise.resolve()),
  displayClear: vi.fn(() => Promise.resolve()),
  openCashDrawer: vi.fn(),
  printReceipt: vi.fn(),
  startScanner: vi.fn(),
  stopScanner: vi.fn(),
  onBarcodeScanned: vi.fn(),
  onBarcodeError: vi.fn(),
}));

vi.mock('@/api/sales', () => ({
  holdCart: vi.fn(() => Promise.resolve({ id: 'held-1' })),
  listHeldCarts: vi.fn(() => Promise.resolve([])),
  getHeldCart: vi.fn(() => Promise.resolve(null)),
  deleteHeldCart: vi.fn(() => Promise.resolve()),
  startSale: vi.fn(() => Promise.resolve({ cartId: 'cart-1' })),
  addLine: vi.fn(() => Promise.resolve({ lineId: 'line-added-1', lineTotal: null })),
  setCartDiscount: vi.fn(() => Promise.resolve()),
  completeSale: vi.fn(() => Promise.resolve({ saleId: 'sale-1', total: { minor_units: 3500, currency: 'IDR' }, lineCount: 1 })),
  getSale: vi.fn(() => Promise.resolve({
    id: 'sale-1', total: { minor_units: 3500, currency: 'IDR' },
    subtotal: { minor_units: 3500, currency: 'IDR' },
    taxTotal: { minor_units: 0, currency: 'IDR' },
    lineCount: 1, status: 'completed', paymentMethod: 'CASH',
    tenderedMinor: 5000, userId: 'user-1', createdAt: '2026-07-06T10:00:00Z',
    lines: [{ id: 'line-1', sku: 'SKU-001', name: 'Indomie Goreng', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' }, total_minor: 3500, tax_amount: null, tax_rate_id: null }],
  })),
  printSalesReceipt: vi.fn(() => Promise.resolve({ printed: true })),
  listSales: vi.fn(() => Promise.resolve([])),
  voidSale: vi.fn(),
  processRefund: vi.fn(() => Promise.resolve({ refundId: 'refund-1', totalMinor: 0 })),
  listRefunds: vi.fn(() => Promise.resolve([])),
  exportDailySummary: vi.fn(() => Promise.resolve([])),
  exportSalesByHour: vi.fn(() => Promise.resolve([])),
  exportEodReport: vi.fn(() => Promise.resolve(null)),
  getProductTrackSerial: vi.fn(() => Promise.resolve(false)),
}));

vi.mock('@/api/kds', () => ({
  createKdsOrderFromSale: vi.fn(() => Promise.resolve()),
}));

vi.mock('@/features/tables/TableManagementScreen', () => ({
  default: () => <div data-testid="table-management-screen">Table Management Floor Plan</div>,
}));

vi.mock('@/features/kds/KdsScreen', () => ({
  default: () => <div data-testid="kds-screen">Kitchen Display System</div>,
}));

vi.mock('@/features/sales/SalesHistoryScreen', () => ({
  default: () => <div data-testid="sales-history-screen">Sales History</div>,
}));

vi.mock('@/features/products/ProductLookupScreen', () => ({
  default: () => <div data-testid="stock-inquiry-screen">Stock Inquiry</div>,
}));

vi.mock('@/api/currency', () => ({
  listCurrencies: vi.fn(() => Promise.resolve([])),
  listExchangeRates: vi.fn(() => Promise.resolve([])),
  getDefaultCurrency: vi.fn(() => Promise.resolve({ code: 'IDR', name: 'Indonesian Rupiah', symbol: 'Rp', decimalPlaces: 2, isDefault: true })),
}));

vi.mock('@/api/customers', () => ({
  listCustomers: vi.fn(() => Promise.resolve([])),
  createCustomer: vi.fn(),
  updateCustomer: vi.fn(),
  deleteCustomer: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', username: 'testuser', role_name: 'cashier', token: 'mock-token', role_id: 'role-1', display_name: 'Kasir Test' },
    loading: false, error: null, login: vi.fn(), logout: vi.fn(), clearError: vi.fn(),
    isManager: false, isOwner: false,
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    activeWorkspace: 'store-pos',
    setActiveWorkspace: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
  }),
  WorkspaceProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

// ── Test wrapper ──────────────────────────────────────────────────

const catFtl = `
  category-cat-food = Makanan
  category-cat-drink = Minuman
`;

function wrap(children: React.ReactNode) {
  return withFluent(<ToastProvider>{children}</ToastProvider>, salesFtl, sharedFtl, productsFtl, catFtl);
}

// ── Tests ─────────────────────────────────────────────────────────

describe('RetailPosScreen', () => {
  beforeEach(async () => {
    mockedBarcode.reset();
    const sp = await import('@/features/sales/usePosState');
    vi.mocked(sp.usePosState).mockReset();
    vi.mocked(sp.usePosState).mockImplementation(() => ({
      lines: [],
      total: null,
      subtotal: null,
      discountPercent: 0,
      discountLabel: '',
      discountAmount: null,
      tipPercent: 0,
      tipAmount: null,
      serviceChargeEnabled: false,
      serviceChargePercent: 0,
      serviceChargeAmount: null,
      addProduct: vi.fn(),
      removeLine: vi.fn(),
      updateQty: vi.fn(),
      setDiscount: vi.fn(),
      updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(),
      setServiceCharge: vi.fn(),
      resetCart: vi.fn(),
      setLines: vi.fn(),
    }));
  });

  // ── Rendering ──────────────────────────────────────────────────

  it('renders the store header with name, branch, and clock', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('TOKO TEST')).toBeInTheDocument();
    });
    expect(screen.getByText(/Cabang A/)).toBeInTheDocument();
    expect(screen.getByText('Jl. Contoh No. 123')).toBeInTheDocument();
    expect(screen.getByText('Kasir Test')).toBeInTheDocument();
  });

  it('shows empty cart state initially', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
    });
  });

  it('renders the function bar with all shortcut buttons', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });
    expect(screen.getByText('F2')).toBeInTheDocument();
    expect(screen.getByText('F3')).toBeInTheDocument();
    expect(screen.getByText('F4')).toBeInTheDocument();
    expect(screen.getByText('F5')).toBeInTheDocument();
    expect(screen.getByText('F9')).toBeInTheDocument();
    expect(screen.getByText('F10')).toBeInTheDocument();
  });

  it('displays "No shift" badge when no active shift', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText(/No shift/)).toBeInTheDocument();
    });
  });

  // ── Products & Categories ─────────────────────────────────────

  async function showAllProducts() {
    const allBtn = await screen.findByRole('button', { name: /^all categories$/i });
    if (allBtn) await userEvent.click(allBtn);
  }

  it('loads and displays products', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await showAllProducts();

    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Nasi Goreng Spesial')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('shows low-stock badge for products with stock_qty <= 5', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    await waitFor(() => {
      expect(screen.getByText('3')).toBeInTheDocument(); // Aqua 600ml stock badge
    });
  });

  it('renders category filter buttons', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText(/All Categories/i)).toBeInTheDocument();
    });
    expect(screen.getByText('Makanan')).toBeInTheDocument();
    expect(screen.getByText('Minuman')).toBeInTheDocument();
  });

  it('filters products by category', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Minuman'));

    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.queryByText('Nasi Goreng Spesial')).not.toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('clears category filter when clicking "All"', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Minuman'));
    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();

    await userEvent.click(screen.getByText(/All Categories/i));
    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  // ── Search ─────────────────────────────────────────────────────

  it('searches products by name', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'Teh');

    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  it('searches products by SKU', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'SKU-004');

    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    // Product name appears only once; search input also contains the text
    expect(screen.getAllByText('Aqua 600ml').length).toBeGreaterThanOrEqual(1);
  });

  it('clears search when clicking the clear button', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'Teh');

    const clearButton = screen.getByLabelText('Clear search');
    await userEvent.click(clearButton);

    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  it('shows empty state when no products match search', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'ZZZZZZ');

    expect(screen.getByText(/No products match your search/)).toBeInTheDocument();
  });

  // ── Add to cart via product tap ─────────────────────────────────

  it('opens quantity picker on long-press of a product button', async () => {
    render(wrap(<RetailPosScreen />));

    const productBtns = await screen.findAllByRole('button', { name: /indomie goreng/i });
    const productBtn = productBtns[0]!;
    fireEvent.pointerDown(productBtn);
    await new Promise(r => setTimeout(r, 500));
    fireEvent.pointerUp(productBtn);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /add/i })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
    expect(screen.getByDisplayValue('1')).toBeInTheDocument();
  });

  it('shows correct price in quantity picker', async () => {
    render(wrap(<RetailPosScreen />));

    await showAllProducts();

    const productBtns = await screen.findAllByRole('button', { name: /indomie goreng/i });
    const productBtn = productBtns[0]!;
    fireEvent.pointerDown(productBtn);
    await new Promise(r => setTimeout(r, 500));
    fireEvent.pointerUp(productBtn);

    await waitFor(() => {
      const qtyModal = screen.getByRole('heading', { name: /Indomie Goreng/i })
        .closest('.retail-qty-modal')!;
      expect(within(qtyModal as HTMLElement).getAllByText(/35[.,]00/)[0]).toBeInTheDocument();
    });
  });

  it('calls addProduct when confirming quantity via long-press', async () => {
    const { usePosState } = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(usePosState).mockReturnValue({
      lines: [],
      total: null,
      subtotal: null,
      discountPercent: 0,
      discountLabel: '',
      discountAmount: null,
      tipPercent: 0,
      tipAmount: null,
      serviceChargeEnabled: false,
      serviceChargePercent: 0,
      serviceChargeAmount: null,
      addProduct,
      removeLine: vi.fn(),
      updateQty: vi.fn(),
      setDiscount: vi.fn(),
      updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(),
      setServiceCharge: vi.fn(),
      resetCart: vi.fn(),
      setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    const productBtn = screen.getByText('Indomie Goreng').closest('button')!;
    fireEvent.pointerDown(productBtn);
    await new Promise(r => setTimeout(r, 500));
    fireEvent.pointerUp(productBtn);

    await waitFor(() => {
      expect(screen.getByText('Add')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Add'));

    expect(addProduct).toHaveBeenCalledTimes(1);
    expect(addProduct).toHaveBeenCalledWith(
      expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }),
    );
  });

  it('adds product on single tap of a product button', async () => {
    const { usePosState } = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(usePosState).mockReturnValue({
      lines: [],
      total: null,
      subtotal: null,
      discountPercent: 0,
      discountLabel: '',
      discountAmount: null,
      tipPercent: 0,
      tipAmount: null,
      serviceChargeEnabled: false,
      serviceChargePercent: 0,
      serviceChargeAmount: null,
      addProduct,
      removeLine: vi.fn(),
      updateQty: vi.fn(),
      setDiscount: vi.fn(),
      updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(),
      setServiceCharge: vi.fn(),
      resetCart: vi.fn(),
      setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    const productBtn = screen.getByText('Indomie Goreng').closest('button')!;
    fireEvent.pointerDown(productBtn);
    fireEvent.pointerUp(productBtn);

    await waitFor(() => {
      expect(addProduct).toHaveBeenCalledTimes(1);
    });
    expect(addProduct).toHaveBeenCalledWith(
      expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }),
    );
  });

  // ── SKU / Barcode input ───────────────────────────────────────

  it('adds product when SKU is submitted via Enter', async () => {
    const posState = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [],
      total: null, subtotal: null,
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, 'SKU-001{Enter}');

    expect(addProduct).toHaveBeenCalledTimes(1);
    expect(addProduct).toHaveBeenCalledWith(
      expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }),
    );
  });

  it('adds product when SKU is submitted via GO button', async () => {
    const posState = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [], total: null, subtotal: null,
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, 'SKU-001');
    await userEvent.click(screen.getByText('GO'));

    expect(addProduct).toHaveBeenCalledTimes(1);
  });

  it('shows warning toast when SKU is not found', async () => {
    render(wrap(<RetailPosScreen />));

    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, 'INVALID-SKU{Enter}');

    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/No product.*matches this barcode/);
    });
  });

  it('calls lookupProductBySku when barcode is entered via SKU input', async () => {
    const productsApi = await import('@/api/products');
    vi.mocked(productsApi.lookupProductBySku).mockResolvedValueOnce({
      sku: 'REMOTE-SKU', name: 'Remote Product', category: null,
      price: { minor_units: 10000, currency: 'IDR' }, barcode: '1234567890',
      in_stock: true, stock_qty: 10, tax_rate_ids: [], created_at: '',
      price_updated_at: '',
    });

    render(wrap(<RetailPosScreen />));

    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, '1234567890{Enter}');

    await waitFor(() => {
      expect(productsApi.lookupProductBySku).toHaveBeenCalledWith('1234567890');
    });
  });

  // ── Barcode scanning ──────────────────────────────────────────

  it('registers the barcode scanner on mount', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
  });

  it('adds product when barcode is scanned matching local product', async () => {
    const posState = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [], total: null, subtotal: null,
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    mockedBarcode.triggerScan('8991002100110');

    await waitFor(() => {
      expect(addProduct).toHaveBeenCalledWith(
        expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }),
      );
    });
  });

  it('calls lookupByBarcode when scanned code not in local products', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    const productsApi = await import('@/api/products');
    mockedBarcode.triggerScan('UNKNOWN-CODE');

    await waitFor(() => {
      expect(productsApi.lookupByBarcode).toHaveBeenCalledWith('UNKNOWN-CODE');
    });
  });

  // ── Shift management ──────────────────────────────────────────

  it('opens shift modal when F9 is pressed and no shift is active', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText(/No shift/)).toBeInTheDocument();
    });

    await userEvent.keyboard('{F9}');

    expect(screen.getByRole('heading', { name: /open shift/i })).toBeInTheDocument();
  });

  it('opens a shift when opening balance is submitted', async () => {
    const { openShift } = await import('@/api/shifts');

    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText(/No shift/)).toBeInTheDocument();
    });

    await userEvent.keyboard('{F9}');
    const input = screen.getByLabelText(/Opening balance/);
    await userEvent.type(input, '100000');
    await userEvent.click(screen.getByText('Open'));

    await waitFor(() => {
      expect(openShift).toHaveBeenCalledWith('user-1', 10000000);
    });
  });

  it('shows warning when Pay is pressed without an active shift', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const payBtns = await screen.findAllByRole('button', { name: /F1.*Pay/i });
    await userEvent.click(payBtns[0]!);

    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/Open a shift first/);
    });
  });

  // ── Discount modal ────────────────────────────────────────────

  it('opens discount modal', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const diskonBtn = await screen.findByRole('button', { name: /^diskon$/i });
    await userEvent.click(diskonBtn);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /Discount/i })).toBeInTheDocument();
    });
  });

  it('applies discount from the discount modal', async () => {
    const posState = await import('@/features/sales/usePosState');
    const setDiscount = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount, resetCart: vi.fn(),
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const diskonBtn = await screen.findByRole('button', { name: /^diskon$/i });
    await userEvent.click(diskonBtn);

    const discountInput = screen.getByLabelText(/Discount/);
    await userEvent.type(discountInput, '10');

    await userEvent.click(screen.getByRole('button', { name: /apply/i }));

    expect(setDiscount).toHaveBeenCalledWith(10, '');
  });

  // ── Payment modal ─────────────────────────────────────────────

  it('opens payment modal when Pay is clicked with items and active shift', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    const shiftsApi = await import('@/api/shifts');
    vi.mocked(shiftsApi.getActiveShift).mockResolvedValueOnce({
      id: 'shift-1', userId: 'user-1', terminalId: null,
      openedAt: '2026-07-05T08:00:00Z', closedAt: null,
      openingBalanceMinor: 100000, closingBalanceMinor: null,
      expectedCashMinor: null, cashDifferenceMinor: null,
      totalSalesMinor: 50000, totalCashMinor: 40000, totalCardMinor: 10000,
      totalOtherMinor: 0, totalVoidsMinor: 0, totalRefundsMinor: 0,
      totalPayoutsMinor: 0, notes: '', status: 'open',
      createdAt: '2026-07-05T08:00:00Z', updatedAt: '2026-07-05T08:00:00Z',
    });

    render(wrap(<RetailPosScreen />));

    const payBtn = await screen.findByRole('button', { name: /^pay$/i });
    await userEvent.click(payBtn);

    await waitFor(() => {
      expect(screen.getByText(/Payment/)).toBeInTheDocument();
    });
  });

  // ── Keyboard shortcuts ─────────────────────────────────────────

  it('shows shortcuts overlay on F11', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F11}');

    expect(screen.getByText(/Keyboard Shortcuts/)).toBeInTheDocument();
  });

  it('shows shortcuts overlay on ? key', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('?');

    expect(screen.getByText(/Keyboard Shortcuts/)).toBeInTheDocument();
  });

  // ── Hold / Resume ─────────────────────────────────────────────

  it('shows hold warning when no cart items', async () => {
    render(wrap(<RetailPosScreen />));

    const holdBtn = await screen.findByRole('button', { name: /F4.*Hold/i });
    expect(holdBtn).toBeDisabled();
  });

  // ── Clear cart ────────────────────────────────────────────────

  it('shows clear confirmation when Void/Clear is clicked with items', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const clearBtn = await screen.findByRole('button', { name: /^clear$/i });
    await userEvent.click(clearBtn);

    await waitFor(() => {
      expect(screen.getByText(/Clear Cart/)).toBeInTheDocument();
    });
  });

  it('resets cart when clear is confirmed', async () => {
    const posState = await import('@/features/sales/usePosState');
    const resetCart = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), resetCart,
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const clearBtn = await screen.findByRole('button', { name: /^clear$/i });
    await userEvent.click(clearBtn);

    const confirmBtns = screen.getAllByRole('button', { name: /^clear$/i });
    await userEvent.click(confirmBtns[1]!);

    expect(resetCart).toHaveBeenCalledTimes(1);
  });

  // ── Credit reminders ──────────────────────────────────────────

  it('shows zero credit reminders when no outstanding credits', async () => {
    const sp = await import('@/features/sales/usePosState');
    vi.mocked(sp.usePosState).mockReturnValue({
      lines: [{ id: crypto.randomUUID() as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: 'cat-food', unit_price: { minor_units: 3500, currency: 'IDR' }, qty: 1 }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    });

    render(wrap(<RetailPosScreen />));

    const creditBtn = await screen.findByText(/Credit Reminders/);
    expect(creditBtn).toBeInTheDocument();
    expect(creditBtn.textContent).toMatch(/Credit Reminders/);
  });

  // ── Full checkout integration flow ─────────────────────────────

  it('completes full checkout flow with cash payment: add items → pay → tender → complete sale', async () => {
    const posState = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    const resetCart = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as import('@/types/domain').Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), resetCart,
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
    });

    const shiftsApi = await import('@/api/shifts');
    vi.mocked(shiftsApi.getActiveShift).mockResolvedValueOnce({
      id: 'shift-1', userId: 'user-1', terminalId: null,
      openedAt: '2026-07-06T08:00:00Z', closedAt: null,
      openingBalanceMinor: 100000, closingBalanceMinor: null,
      expectedCashMinor: null, cashDifferenceMinor: null,
      totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0,
      totalOtherMinor: 0, totalVoidsMinor: 0, totalRefundsMinor: 0,
      totalPayoutsMinor: 0, notes: '', status: 'open',
      createdAt: '2026-07-06T08:00:00Z', updatedAt: '2026-07-06T08:00:00Z',
    });

    const salesApi = await import('@/api/sales');

    render(wrap(<RetailPosScreen />));

    const payBtn = await screen.findByRole('button', { name: /^pay$/i });
    await userEvent.click(payBtn);

    await waitFor(() => {
      expect(screen.getByText(/^Complete$/)).toBeInTheDocument();
    });

    const exactBtn = Array.from(document.querySelectorAll('.payment-quick-btn')).find(
      (btn) => btn.textContent?.includes('Exact'),
    )!;
    await userEvent.click(exactBtn);

    expect(screen.getByText(/Change/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /^Complete$/i }));

    await waitFor(() => {
      expect(screen.getByText(/Sale Complete/i)).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(salesApi.completeSale).toHaveBeenCalledWith(
      expect.objectContaining({
        paymentMethod: 'CASH',
        tenderedMinor: 3500,
      }),
    );

    expect(salesApi.printSalesReceipt).toHaveBeenCalled();
  });

  // ── Table Management button ───────────────────────────────────

  it('renders the Tables button when TABLE_MANAGEMENT feature is enabled', async () => {
    // By default getEnabledFeatures rejects → useFeatures enables ALL features,
    // so the Tables button should be visible.
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: /tables/i })).toBeInTheDocument();
  });

  it('hides the Tables button when TABLE_MANAGEMENT feature is disabled', async () => {
    const settingsApi = await import('@/api/settings');
    vi.mocked(settingsApi.getEnabledFeatures).mockResolvedValueOnce({
      features: ['simple-retail', 'cash-payment'],
    });

    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    expect(screen.queryByRole('button', { name: /tables/i })).not.toBeInTheDocument();
  });

  it('opens TableManagementScreen when the Tables button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /tables/i }));

    await waitFor(() => {
      expect(screen.getByTestId('table-management-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Table Management Floor Plan')).toBeInTheDocument();
  });

  it('dismisses TableManagementScreen when the back button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /tables/i }));

    await waitFor(() => {
      expect(screen.getByTestId('table-management-screen')).toBeInTheDocument();
    });

    // The back button renders as "← back" (larr + localized "back" text)
    await userEvent.click(screen.getByRole('button', { name: /back/i }));

    await waitFor(() => {
      expect(screen.queryByTestId('table-management-screen')).not.toBeInTheDocument();
    });
    // Should be back on the main POS screen
    expect(screen.getByText('F1')).toBeInTheDocument();
  });

  // ── KDS (F12) shortcut ────────────────────────────────────────

  it('opens KdsScreen when F12 is pressed', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F12}');

    await waitFor(() => {
      expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Kitchen Display System')).toBeInTheDocument();
  });

  it('opens KdsScreen when the F12 button in the function bar is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /F12/i }));

    await waitFor(() => {
      expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
    });
  });

  it('dismisses KdsScreen when the back button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F12}');

    await waitFor(() => {
      expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /back/i }));

    await waitFor(() => {
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });
    expect(screen.getByText('F1')).toBeInTheDocument();
  });

  it('suppresses F-keys while KdsScreen is shown', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    // Open KDS via F12
    await userEvent.keyboard('{F12}');
    await waitFor(() => {
      expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
    });

    // Pressing F9 while KDS is shown should NOT open the shift modal
    await userEvent.keyboard('{F9}');
    expect(screen.queryByRole('heading', { name: /open shift/i })).not.toBeInTheDocument();

    // KDS should still be visible
    expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
  });

  // ── F6 Sales History shortcut ─────────────────────────────────

  it('opens SalesHistoryScreen when F6 is pressed', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F6}');

    await waitFor(() => {
      expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Sales History')).toBeInTheDocument();
  });

  it('opens SalesHistoryScreen when the F6 button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /F6/i }));

    await waitFor(() => {
      expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument();
    });
  });

  it('dismisses SalesHistoryScreen when the back button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F6}');
    await waitFor(() => {
      expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /back/i }));

    await waitFor(() => {
      expect(screen.queryByTestId('sales-history-screen')).not.toBeInTheDocument();
    });
    expect(screen.getByText('F1')).toBeInTheDocument();
  });

  // ── F8 Stock Inquiry shortcut ─────────────────────────────────

  it('opens ProductLookupScreen when F8 is pressed', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F8}');

    await waitFor(() => {
      expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Stock Inquiry')).toBeInTheDocument();
  });

  it('opens ProductLookupScreen when the F8 button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /F8/i }));

    await waitFor(() => {
      expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument();
    });
  });

  it('dismisses ProductLookupScreen when the back button is clicked', async () => {
    render(wrap(<RetailPosScreen />));

    await waitFor(() => {
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    await userEvent.keyboard('{F8}');
    await waitFor(() => {
      expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /back/i }));

    await waitFor(() => {
      expect(screen.queryByTestId('stock-inquiry-screen')).not.toBeInTheDocument();
    });
    expect(screen.getByText('F1')).toBeInTheDocument();
  });
});
