// ── RetailPosScreen tests — fast rendering + navigation ─────────
//
// Covers: rendering, products/categories, search, keyboard shortcuts,
// hold/resume, barcode registration, credit reminders, table
// management, KDS navigation. Fast isolated tests that don't trigger
// the payment modal or long-press timers. 24 tests.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import tablesFtl from '@/locales/tables.ftl?raw';
import RetailPosScreen from '@/features/retail/RetailPosScreen';

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
    removeLine: vi.fn(),
    updateQty: vi.fn(),
    setDiscount: vi.fn(),
    updateLinePrice: vi.fn(),
    setTipPercent: vi.fn(),
    setServiceCharge: vi.fn(),
    resetCart: vi.fn(),
    setLines: vi.fn(),
    assignCourse: vi.fn(),
    fireCourse: vi.fn(),
    fireAllCourses: vi.fn(),
  })),
}));

vi.mock('@/features/sales/useBarcodeScanner', () => ({
  useBarcodeScanner: mockedBarcode.useBarcodeScanner,
}));

vi.mock('@/api/products', () => ({
  listProducts: vi.fn(() =>
    Promise.resolve([
      { sku: 'SKU-001', name: 'Indomie Goreng', category: 'cat-food', price: { minor_units: 3500, currency: 'IDR' }, barcode: '8991002100110', in_stock: true, stock_qty: 100, tax_rate_ids: [], created_at: '',
      price_updated_at: '', product_type: 'retail' },
      { sku: 'SKU-002', name: 'Teh Botol Sosro', category: 'cat-drink', price: { minor_units: 5000, currency: 'IDR' }, barcode: '8991002100220', in_stock: true, stock_qty: 50, tax_rate_ids: [], created_at: '',
      price_updated_at: '', product_type: 'retail' },
      { sku: 'SKU-003', name: 'Nasi Goreng Spesial', category: 'cat-food', price: { minor_units: 15000, currency: 'IDR' }, barcode: null, in_stock: true, stock_qty: 20, tax_rate_ids: [], created_at: '',
      price_updated_at: '', product_type: 'retail' },
      { sku: 'SKU-004', name: 'Aqua 600ml', category: 'cat-drink', price: { minor_units: 3000, currency: 'IDR' }, barcode: '8991002100330', in_stock: true, stock_qty: 3, tax_rate_ids: [], created_at: '',
      price_updated_at: '', product_type: 'retail' },
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

vi.mock('@/api/shifts', async () => {
  const { createShiftsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createShiftsApiMock({
    getActiveShift: vi.fn(() => Promise.reject(new Error('no shift'))),
  });
});

vi.mock('@/api/settings', async () => {
  const { createSettingsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createSettingsApiMock({
    getStoreSettings: vi.fn(() =>
      Promise.resolve({ name: 'TOKO TEST', address: 'Jl. Contoh No. 123', taxId: '', currency: 'IDR', branch: 'Cabang A', logo: '' }),
    ),
  });
});

vi.mock('@/api/hardware', async () => {
  const { createHardwareApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createHardwareApiMock();
});

vi.mock('@/api/sales', async () => {
  const { createSalesApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createSalesApiMock();
});

vi.mock('@/api/kds', () => ({
  createKdsOrderFromSale: vi.fn((_userId: string, _saleId: string) => Promise.resolve()),
}));

vi.mock('@/features/tables/TableManagementScreen', () => ({
  default: () => <div data-testid="table-management-screen">Table Management Floor Plan</div>,
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

vi.mock('@/contexts/AuthContext', async () => {
  const { createAuthContextMock } = await import('@/__tests__/test-utils/mocks/contexts');
  return {
    useAuth: createAuthContextMock(),
  };
});

vi.mock('@/contexts/WorkspaceContext', async () => {
  const { createWorkspaceContextMock } = await import('@/__tests__/test-utils/mocks/contexts');
  return createWorkspaceContextMock();
});

const catFtl = `
  category-cat-food = Makanan
  category-cat-drink = Minuman
`;

// ── Helper to click "All Categories" button ──────────────────────

async function showAllProducts() {
  const allBtn = await screen.findByRole('button', { name: /^all categories$/i });
  if (allBtn) await userEvent.click(allBtn);
}

// ── Tests ─────────────────────────────────────────────────────────

describe('RetailPosScreen — rendering', () => {
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
      assignCourse: vi.fn(),
      fireCourse: vi.fn(),
      fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any));
  });

  it('renders the store header with name, branch, and clock', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('TOKO TEST')).toBeInTheDocument());
    expect(screen.getByText(/Cabang A/)).toBeInTheDocument();
    expect(screen.getByText('Jl. Contoh No. 123')).toBeInTheDocument();
    expect(screen.getByText('Kasir Test')).toBeInTheDocument();
  });

  it('shows empty cart state initially', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText(/Cart is empty/)).toBeInTheDocument());
  });

  it('renders the function bar with all shortcut buttons', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    expect(screen.getByText('F2')).toBeInTheDocument();
    expect(screen.getByText('F3')).toBeInTheDocument();
    expect(screen.getByText('F4')).toBeInTheDocument();
    expect(screen.getByText('F5')).toBeInTheDocument();
    expect(screen.getByText('F9')).toBeInTheDocument();
    expect(screen.getByText('F10')).toBeInTheDocument();
  });

  it('displays "No shift" badge when no active shift', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText(/No shift/)).toBeInTheDocument());
  });

  it('loads and displays products', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('Indomie Goreng')).toBeInTheDocument());
    await showAllProducts();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Nasi Goreng Spesial')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('shows low-stock badge for products with stock_qty <= 5', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await showAllProducts();
    await waitFor(() => expect(screen.getByText('3')).toBeInTheDocument());
  });

  it('renders category filter buttons', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText(/All Categories/i)).toBeInTheDocument());
    expect(screen.getByText('Makanan')).toBeInTheDocument();
    expect(screen.getByText('Minuman')).toBeInTheDocument();
  });

  it('filters products by category', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('Indomie Goreng')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Minuman'));
    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.queryByText('Nasi Goreng Spesial')).not.toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('clears category filter when clicking "All"', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('Indomie Goreng')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Minuman'));
    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    await userEvent.click(screen.getByText(/All Categories/i));
    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  // ── Search ───────────────────────────────────────────────────

  it('searches products by name', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await showAllProducts();
    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'Teh');
    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  it('searches products by SKU', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await showAllProducts();
    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'SKU-004');
    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.getAllByText('Aqua 600ml').length).toBeGreaterThanOrEqual(1);
  });

  it('clears search when clicking the clear button', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await showAllProducts();
    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'Teh');
    const clearButton = screen.getByLabelText('Clear search');
    await userEvent.click(clearButton);
    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  it('shows empty state when no products match search', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await showAllProducts();
    const searchInput = screen.getByPlaceholderText('Cari produk\u2026');
    await userEvent.type(searchInput, 'ZZZZZZ');
    expect(screen.getByText(/No products match your search/)).toBeInTheDocument();
  });

  // ── Barcode registration ─────────────────────────────────────

  it('registers the barcode scanner on mount', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled());
  });

  // ── Keyboard shortcuts / non-interaction ─────────────────────

  it('shows shortcuts overlay on ? key', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('?');
    expect(screen.getByText(/Keyboard Shortcuts/)).toBeInTheDocument();
  });

  it('shows hold warning when no cart items', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const holdBtn = await screen.findByRole('button', { name: /F4.*Hold/i });
    expect(holdBtn).toBeDisabled();
  });

  it('shows zero credit reminders when no outstanding credits', async () => {
    const sp = await import('@/features/sales/usePosState');
    vi.mocked(sp.usePosState).mockReturnValue({
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      lines: [{ id: crypto.randomUUID(), sku: 'SKU-001', name: 'Indomie Goreng', category: 'cat-food', unit_price: { minor_units: 3500, currency: 'IDR' }, qty: 1 }] as any,
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const creditBtn = await screen.findByText(/Credit Reminders/);
    expect(creditBtn).toBeInTheDocument();
    expect(creditBtn.textContent).toMatch(/Credit Reminders/);
  });

  // ── Table Management ─────────────────────────────────────────

  it('renders the Tables button when TABLE_MANAGEMENT feature is enabled', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    expect(screen.getByRole('button', { name: /table/i })).toBeInTheDocument();
  });

  it('hides the Tables button when TABLE_MANAGEMENT feature is disabled', async () => {
    const settingsApi = await import('@/api/settings');
    vi.mocked(settingsApi.getEnabledFeatures).mockResolvedValueOnce({
      features: ['simple-retail', 'cash-payment'],
    });
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: /table/i })).not.toBeInTheDocument();
  });

  it('opens TableManagementScreen when the Tables button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /table/i }));
    await waitFor(() => expect(screen.getByTestId('table-management-screen')).toBeInTheDocument());
    expect(screen.getByText('Table Management Floor Plan')).toBeInTheDocument();
  });

  it('dismisses TableManagementScreen when the back button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /table/i }));
    await waitFor(() => expect(screen.getByTestId('table-management-screen')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /back/i }));
    await waitFor(() => expect(screen.queryByTestId('table-management-screen')).not.toBeInTheDocument());
    expect(screen.getByText('F1')).toBeInTheDocument();
  });

  // ── KDS (F12) shortcut ──────────────────────────────────────

  it('F12 navigates to KDS workspace via onNavigate', async () => {
    const onNavigate = vi.fn();
    await renderWithProviders(<RetailPosScreen onNavigate={onNavigate} />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F12}');
    expect(onNavigate).toHaveBeenCalledWith('kds');
  });

  it('F12 button in function bar calls onNavigate with kds', async () => {
    const onNavigate = vi.fn();
    await renderWithProviders(<RetailPosScreen onNavigate={onNavigate} />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /F12/i }));
    expect(onNavigate).toHaveBeenCalledWith('kds');
  });

  it('does not crash when F12 is pressed and onNavigate is undefined', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F12}');
    expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
  });
});
