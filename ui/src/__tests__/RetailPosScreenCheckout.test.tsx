// ── RetailPosScreen checkout + navigation tests ───────────────────
//
// Covers: payment modal, full checkout flow with cash payment, F6
// Sales History, F8 Stock Inquiry. These tests involve heavier
// component loading (PaymentModal) and are the most time-consuming.
// Split from RetailPosScreen.test.tsx to enable parallel execution. 8 tests.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
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

describe('RetailPosScreen — checkout & navigation', () => {
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

  // ── Payment modal ────────────────────────────────────────────

  it('opens payment modal when Pay is clicked with items and active shift', async () => {
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
    } as any);
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
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const payBtn = await screen.findByRole('button', { name: /^pay$/i });
    await userEvent.click(payBtn);
    await waitFor(() => expect(screen.getByText(/Payment/)).toBeInTheDocument());
  });

  it('completes full checkout flow with cash payment: add items → pay → tender → complete sale', async () => {
    const posState = await import('@/features/sales/usePosState');
    const addProduct = vi.fn();
    const resetCart = vi.fn();
    vi.mocked(posState.usePosState).mockReturnValue({
      lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'Indomie Goreng', category: 'cat-food', qty: 1, unit_price: { minor_units: 3500, currency: 'IDR' } }],
      total: { minor_units: 3500, currency: 'IDR' },
      subtotal: { minor_units: 3500, currency: 'IDR' },
      discountPercent: 0, discountLabel: '', discountAmount: null,
      tipPercent: 0, tipAmount: null,
      serviceChargeEnabled: false, serviceChargePercent: 0, serviceChargeAmount: null,
      addProduct, removeLine: vi.fn(), updateQty: vi.fn(),
      setDiscount: vi.fn(), resetCart,
      updateLinePrice: vi.fn(), setTipPercent: vi.fn(), setServiceCharge: vi.fn(), setLines: vi.fn(),
      assignCourse: vi.fn(), fireCourse: vi.fn(), fireAllCourses: vi.fn(),
    } as any);
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
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    const payBtn = await screen.findByRole('button', { name: /^pay$/i });
    await userEvent.click(payBtn);
    await waitFor(() => expect(screen.getByText(/^Complete$/)).toBeInTheDocument());
    const exactBtn = Array.from(document.querySelectorAll('.payment-quick-btn')).find(
      (btn) => btn.textContent?.includes('Exact'),
    )!;
    await userEvent.click(exactBtn);
    expect(screen.getByText(/Change/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Complete$/i }));
    await waitFor(() => expect(screen.getByText(/Sale Complete/i)).toBeInTheDocument(), { timeout: 5000 });
    expect(salesApi.completeSale).toHaveBeenCalledWith(
      expect.objectContaining({ paymentMethod: 'CASH', tenderedMinor: 3500 }),
    );
    expect(salesApi.printSalesReceipt).toHaveBeenCalled();
  });

  // ── F6 Sales History shortcut ─────────────────────────────────

  it('opens SalesHistoryScreen when F6 is pressed', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F6}');
    await waitFor(() => expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument());
    expect(screen.getByText('Sales History')).toBeInTheDocument();
  });

  it('opens SalesHistoryScreen when the F6 button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /F6/i }));
    await waitFor(() => expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument());
  });

  it('dismisses SalesHistoryScreen when the back button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F6}');
    await waitFor(() => expect(screen.getByTestId('sales-history-screen')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /back/i }));
    await waitFor(() => expect(screen.queryByTestId('sales-history-screen')).not.toBeInTheDocument());
    expect(screen.getByText('F1')).toBeInTheDocument();
  });

  // ── F8 Stock Inquiry shortcut ─────────────────────────────────

  it('opens ProductLookupScreen when F8 is pressed', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F8}');
    await waitFor(() => expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument());
    expect(screen.getByText('Stock Inquiry')).toBeInTheDocument();
  });

  it('opens ProductLookupScreen when the F8 button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /F8/i }));
    await waitFor(() => expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument());
  });

  it('dismisses ProductLookupScreen when the back button is clicked', async () => {
    await renderWithProviders(<RetailPosScreen />, salesFtl, productsFtl, tablesFtl, catFtl);
    await waitFor(() => expect(screen.getByText('F1')).toBeInTheDocument());
    await userEvent.keyboard('{F8}');
    await waitFor(() => expect(screen.getByTestId('stock-inquiry-screen')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /back/i }));
    await waitFor(() => expect(screen.queryByTestId('stock-inquiry-screen')).not.toBeInTheDocument());
    expect(screen.getByText('F1')).toBeInTheDocument();
  });
});
