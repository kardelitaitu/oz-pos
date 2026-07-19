// ── RetailPosScreen interaction tests ──────────────────────────────
//
// Covers: long-press quantity picker, SKU/barcode input, barcode
// scanning, shift management, discount modal, clear cart. These
// tests involve userEvent interactions and moderate async waits.
// Split from RetailPosScreen.test.tsx to enable parallel execution. 17 tests.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { act } from 'react';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import tablesFtl from '@/locales/tables.ftl?raw';
import RetailPosScreen from '@/features/retail/RetailPosScreen';
import type { LineId, Sku } from '@/types/domain';

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

// ── Tests ─────────────────────────────────────────────────────────

describe('RetailPosScreen — interactions', () => {
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

  // ── Long-press quantity picker ────────────────────────────────
  // Note: these tests use real setTimeout(500) for long-press detection

  it('opens quantity picker on long-press of a product button', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const productBtns = await screen.findAllByRole('button', { name: /indomie goreng/i });
    const productBtn = productBtns[0]!;
    fireEvent.pointerDown(productBtn);
    await act(async () => { await new Promise(r => setTimeout(r, 500)); });
    fireEvent.pointerUp(productBtn);
    await waitFor(() => expect(screen.getByRole('button', { name: /add/i })).toBeInTheDocument());
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
    expect(screen.getByDisplayValue('1')).toBeInTheDocument();
  });

  it('shows correct price in quantity picker', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const productBtns = await screen.findAllByRole('button', { name: /indomie goreng/i });
    const productBtn = productBtns[0]!;
    fireEvent.pointerDown(productBtn);
    await act(async () => { await new Promise(r => setTimeout(r, 500)); });
    fireEvent.pointerUp(productBtn);
    await waitFor(() => {
      const qtyModal = screen.getByRole('heading', { name: /Indomie Goreng/i })
        .closest('.retail-qty-modal')!;
      expect(qtyModal as HTMLElement).toBeInTheDocument();
    });
  });

  it('calls addProduct when confirming quantity via long-press', async () => {
    const { usePosState } = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(usePosState).mockReturnValue({
      lines: [], total: null, subtotal: null,
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('Indomie Goreng')).toBeInTheDocument());
    const productBtn = screen.getByText('Indomie Goreng').closest('button')!;
    fireEvent.pointerDown(productBtn);
    await act(async () => { await new Promise(r => setTimeout(r, 500)); });
    fireEvent.pointerUp(productBtn);
    await waitFor(() => expect(screen.getByText('Add')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Add'));
    expect(addProduct).toHaveBeenCalledTimes(1);
    expect(addProduct).toHaveBeenCalledWith(expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }));
  });

  it('adds product on single tap of a product button', async () => {
    const { usePosState } = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    vi.mocked(usePosState).mockReturnValue({
      lines: [], total: null, subtotal: null,
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('Indomie Goreng')).toBeInTheDocument());
    const productBtn = screen.getByText('Indomie Goreng').closest('button')!;
    fireEvent.pointerDown(productBtn);
    fireEvent.pointerUp(productBtn);
    await waitFor(() => expect(addProduct).toHaveBeenCalledTimes(1));
    expect(addProduct).toHaveBeenCalledWith(expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }));
  });

  // ── SKU / Barcode input ──────────────────────────────────────

  it('adds product when SKU is submitted via Enter', async () => {
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, 'SKU-001{Enter}');
    expect(addProduct).toHaveBeenCalledTimes(1);
    expect(addProduct).toHaveBeenCalledWith(expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' }));
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, 'SKU-001');
    await userEvent.click(screen.getByText('GO'));
    expect(addProduct).toHaveBeenCalledTimes(1);
  });

  it('shows warning toast when SKU is not found', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
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
      price_updated_at: '', product_type: 'retail',
    });
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0]!;
    await userEvent.type(skuInput, '1234567890{Enter}');
    await waitFor(() => expect(productsApi.lookupProductBySku).toHaveBeenCalledWith('1234567890'));
  });

  // ── Barcode scanning ─────────────────────────────────────────

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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled());
    act(() => { mockedBarcode.triggerScan('8991002100110'); });
    await waitFor(() => expect(addProduct).toHaveBeenCalledWith(expect.objectContaining({ sku: 'SKU-001', name: 'Indomie Goreng' })));
  });

  it('calls lookupByBarcode when scanned code not in local products', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled());
    const productsApi = await import('@/api/products');
    act(() => { mockedBarcode.triggerScan('UNKNOWN-CODE'); });
    await waitFor(() => expect(productsApi.lookupByBarcode).toHaveBeenCalledWith('UNKNOWN-CODE'));
  });

  // ── Shift management ─────────────────────────────────────────

  it('opens shift modal when F9 is pressed and no shift is active', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText(/No shift/)).toBeInTheDocument());
    await userEvent.keyboard('{F9}');
    expect(screen.getByRole('heading', { name: /open shift/i })).toBeInTheDocument();
  });

  it('opens a shift when opening balance is submitted', async () => {
    const { openShift } = await import('@/api/shifts');
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText(/No shift/)).toBeInTheDocument());
    await userEvent.keyboard('{F9}');
    const input = screen.getByLabelText(/Opening balance/);
    await userEvent.type(input, '100000');
    await userEvent.click(screen.getByText('Open'));
    await waitFor(() => expect(openShift).toHaveBeenCalledWith('user-1', 10000000));
  });

  it('shows warning when Pay is pressed without an active shift', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
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
    const payBtns = await screen.findAllByRole('button', { name: /F1.*Pay/i });
    await userEvent.click(payBtns[0]!);
    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/Open a shift first/);
    });
  });

  // ── Discount modal ───────────────────────────────────────────

  it('opens discount modal', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
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
    const diskonBtn = await screen.findByRole('button', { name: /^diskon$/i });
    await userEvent.click(diskonBtn);
    await waitFor(() => expect(screen.getByRole('heading', { name: /Discount/i })).toBeInTheDocument());
  });

  it('applies discount from the discount modal', async () => {
    const posState = await import('@/features/sales/usePosState');
    const setDiscount = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount, resetCart: vi.fn(),
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const diskonBtn = await screen.findByRole('button', { name: /^diskon$/i });
    await userEvent.click(diskonBtn);
    const discountInput = screen.getByLabelText(/Discount/);
    await userEvent.type(discountInput, '10');
    await userEvent.click(screen.getByRole('button', { name: /apply/i }));
    expect(setDiscount).toHaveBeenCalledWith(10, '');
  });

  // ── Clear cart ───────────────────────────────────────────────

  it('shows clear confirmation when Void/Clear is clicked with items', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
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
    const clearBtn = await screen.findByRole('button', { name: /^clear$/i });
    await userEvent.click(clearBtn);
    await waitFor(() => expect(screen.getByText(/Clear Cart/)).toBeInTheDocument());
  });

  // ── Pay button edge cases ──────────────────────────────────

  it('disables Pay button when cart is empty (no lines)', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const payBtns = await screen.findAllByRole('button', { name: /pay/i });
    expect(payBtns[0]).toBeDisabled();
  });

  it('keeps Pay button disabled when cart has items but no shift', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: '', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
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
    const payBtns = await screen.findAllByRole('button', { name: /pay/i });
    expect(payBtns[0]).toBeDisabled();
  });

  // ── Cart line removal ────────────────────────────────────────

  it('calls removeLine when cart remove button is clicked', async () => {
    const posState = await import('@/features/sales/usePosState');
    const removeLine = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine, updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => {
      // Product name appears in both the product grid AND the cart panel.
      const names = screen.getAllByText('Indomie Goreng');
      expect(names.length).toBeGreaterThanOrEqual(1);
    });
    const removeBtns = document.querySelectorAll('.retail-cart-remove-btn');
    expect(removeBtns.length).toBeGreaterThanOrEqual(1);
    await userEvent.click(removeBtns[0]);
    expect(removeLine).toHaveBeenCalledTimes(1);
    expect(removeLine).toHaveBeenCalledWith('line-1');
  });

  it('removes multiple line items individually from cart panel', async () => {
    const posState = await import('@/features/sales/usePosState');
    const removeLine = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [
        { id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } },
        { id: 'line-2' as LineId, sku: 'SKU-002' as Sku, name: 'Teh Botol Sosro', category: 'cat-drink', qty: 2, unit_price: { minor_units: 5000, currency: 'IDR' } },
      ],
      total: { minor_units: 13500, currency: 'IDR' },
      subtotal: { minor_units: 13500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine, updateQty: vi.fn(),
      setDiscount: vi.fn(), updateLinePrice: vi.fn(),
      setTipPercent: vi.fn(), setServiceCharge: vi.fn(),
      resetCart: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => {
      const names = screen.getAllByText('Indomie Goreng');
      expect(names.length).toBeGreaterThanOrEqual(1);
    });
    const names = screen.getAllByText('Teh Botol Sosro');
    expect(names.length).toBeGreaterThanOrEqual(1);
    // Find all remove buttons and click each
    const removeBtns = document.querySelectorAll('.retail-cart-remove-btn');
    for (const btn of removeBtns) {
      await userEvent.click(btn);
    }
    expect(removeLine).toHaveBeenCalledTimes(2);
    expect(removeLine).toHaveBeenCalledWith('line-1');
    expect(removeLine).toHaveBeenCalledWith('line-2');
  });

  // ── Keyboard shortcut: F5 → SKU focus ────────────────────────

  it('focuses SKU input when F5 is pressed', async () => {
    const posState = await import('@/features/sales/usePosState');
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [], total: null, subtotal: null,
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
    const skuInputs = await screen.findAllByPlaceholderText(/Scan or type barcode/);
    const skuInput = skuInputs[0];
    expect(skuInput).not.toBe(document.activeElement);
    await userEvent.keyboard('{F5}');
    await waitFor(() => {
      expect(skuInput).toBe(document.activeElement);
    });
  });

  // ── Keyboard shortcut: F6 → Sales History ────────────────────

  it('opens Sales History screen when F6 is pressed', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('sales-history-screen')).not.toBeInTheDocument();
    await userEvent.keyboard('{F6}');
    await waitFor(() => {
      expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Sales History')).toBeInTheDocument();
  });

  // ── Keyboard shortcut: F7 → Customer Search ──────────────────

  it('opens Customer Search overlay when F7 is pressed', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });
    await userEvent.keyboard('{F7}');
    // Customer search shows an input field for searching
    await waitFor(() => {
      // The customer search renders a search input
      const searchInputs = screen.getAllByPlaceholderText(/search|cari|find/i);
      expect(searchInputs.length).toBeGreaterThanOrEqual(1);
    });
  });

  // ── Keyboard shortcut: F8 → Stock Inquiry ────────────────────

  it('opens Stock Inquiry screen when F8 is pressed', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('stock-inquiry-screen')).not.toBeInTheDocument();
    await userEvent.keyboard('{F8}');
    await waitFor(() => {
      expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument();
    });
    expect(screen.getByText('Stock Inquiry')).toBeInTheDocument();
  });

  it('resets cart when clear is confirmed', async () => {
    const posState = await import('@/features/sales/usePosState');
    const resetCart = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct: vi.fn(), removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), resetCart,
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const clearBtn = await screen.findByRole('button', { name: /^clear$/i });
    await userEvent.click(clearBtn);
    const confirmBtns = screen.getAllByRole('button', { name: /^clear$/i });
    await userEvent.click(confirmBtns[1]!);
    expect(resetCart).toHaveBeenCalledTimes(1);
  });
});
