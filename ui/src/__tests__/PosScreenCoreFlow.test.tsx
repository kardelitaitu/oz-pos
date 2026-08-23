// ── PosScreen TDD: Core Sale Flow (Red → Green → Refactor) ─────────────
//
// Target: Exercise the critical path: add product → cart → pay → complete
// Current coverage: ~52% statements, ~55% branches
// Goal: Push to >80% by covering uncovered branches in this flow

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import settingsFtl from '@/locales/settings.ftl?raw';
import PosScreen from '@/features/sales/PosScreen';
import * as shiftsApi from '@/api/shifts';
import * as settingsApi from '@/api/settings';
import * as salesApi from '@/api/sales';
import * as productsApi from '@/api/products';
import * as bundlesApi from '@/api/bundles';
import type * as HardwareModule from '@/api/hardware';
import { mockedBarcode } from '@/__tests__/test-utils/mocks/barcodeScanner';

// ── Shared mock setup (same as integration test) ────────────────────────

vi.mock('@/api/hardware', async () => {
  const actual = await vi.importActual<typeof HardwareModule>('@/api/hardware');
  return {
    ...actual,
    listDisplays: vi.fn(() => Promise.resolve([])),
    displayShow: vi.fn(() => Promise.resolve()),
    displayClear: vi.fn(() => Promise.resolve()),
  };
});

vi.mock('@/features/sales/useBarcodeScanner', async () => {
  const { createBarcodeScannerModuleMock } =
    await import('@/__tests__/test-utils/mocks/barcodeScanner');
  return createBarcodeScannerModuleMock();
});

const mockLookupByBarcode = vi.hoisted(() => vi.fn((_code: string) => Promise.resolve(null)));

vi.mock('@/api/products', () => ({
  lookupByBarcode: mockLookupByBarcode,
  lookupByBarcodeScoped: vi.fn((_sessionToken: string, code: string) =>
    mockLookupByBarcode(code),
  ),
  lookupProductBySku: vi.fn((sku: string) => {
    const products: Record<string, unknown> = {
      'ITEM-001': {
        sku: 'ITEM-001',
        name: 'Test Item',
        category: 'Test',
        price: { minor_units: 700, currency: 'USD' },
        barcode: 'BARCODE-001',
        in_stock: true,
        stock_qty: 100,
        tax_rate_ids: [],
        product_type: 'standard',
        created_at: '',
        price_updated_at: '',
      },
      'ITEM-002': {
        sku: 'ITEM-002',
        name: 'Second Item',
        category: 'Test',
        price: { minor_units: 300, currency: 'USD' },
        barcode: 'BARCODE-002',
        in_stock: true,
        stock_qty: 50,
        tax_rate_ids: [],
        product_type: 'standard',
        created_at: '',
        price_updated_at: '',
      },
    };
    return Promise.resolve(products[sku] ?? null);
  }),
  listProducts: vi.fn(() => Promise.resolve([
    {
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 700, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    },
    {
      sku: 'ITEM-002',
      name: 'Second Item',
      category: 'Test',
      price: { minor_units: 300, currency: 'USD' },
      barcode: 'BARCODE-002',
      in_stock: true,
      stock_qty: 50,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    },
  ])),
  listCategories: vi.fn(() => Promise.resolve([])),
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
  deleteCategory: vi.fn(),
}));

vi.mock('@/api/bundles', () => ({
  lookupBundleBySku: vi.fn(() => Promise.resolve(null)),
  listBundles: vi.fn(() => Promise.resolve([])),
  getBundle: vi.fn(() => Promise.resolve(null)),
  createBundle: vi.fn(),
  updateBundle: vi.fn(),
  deleteBundle: vi.fn(),
}));

vi.mock('@/api/shifts', async () => {
  const { createShiftsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createShiftsApiMock();
});

vi.mock('@/api/settings', async () => {
  const { createSettingsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createSettingsApiMock();
});

vi.mock('@/api/sales', async () => {
  const { createSalesApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return {
    ...createSalesApiMock(),
    createKdsOrderFromSaleScoped: vi.fn(() => Promise.resolve({})),
    createKdsOrderFromSale: vi.fn(() => Promise.resolve({})),
  };
});

vi.mock('@/api/tax', () => ({
  computeCartTax: vi.fn(() => Promise.resolve({ taxMinor: 0, hasExclusive: false })),
}));

vi.mock('@/utils/interaction', () => ({
  triggerInteraction: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', async () => {
  const { createAuthContextMock } = await import('@/__tests__/test-utils/mocks/contexts');
  return {
    useAuth: createAuthContextMock({ isManager: true }),
  };
});

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    activeWorkspace: 'store-pos',
    setActiveWorkspace: vi.fn(),
    activeInstance: null,
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
    sessionToken: 'mock-session-token',
    swapSessionToken: vi.fn(),
    terminalId: '',
  }),
  useWorkspaceScope: () => ({
    storeId: 'default',
    instanceId: 'default',
    typeKey: 'store-pos',
  }),
  WorkspaceProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/hooks/useFeatures', () => ({
  useFeatures: () => ({
    enabled: new Set<string>(),
    loading: false,
    isEnabled: () => true,
    loaded: true,
    filterRoutes: (routes: string[]) => routes,
    error: null,
  }),
  FEATURES: {
    KITCHEN_DISPLAY: 'kitchen-display',
    TABLE_MANAGEMENT: 'table-management',
    USB_SCALE: 'usb-scale',
    MULTI_LOCATION: 'multi-location',
    QUICK_RETURN: 'quick-return',
    SERIAL_TRACKING: 'serial-tracking',
  } as const,
}));

// ── Test fixtures ──────────────────────────────────────────────────────

function shiftFixture(openedAt?: Date) {
  const date = openedAt ?? new Date();
  return {
    id: 'shift-1',
    userId: 'user-1',
    terminalId: null,
    openedAt: date.toISOString(),
    closedAt: null,
    openingBalanceMinor: 0,
    closingBalanceMinor: null,
    expectedCashMinor: null,
    cashDifferenceMinor: null,
    totalSalesMinor: 0,
    totalCashMinor: 0,
    totalCardMinor: 0,
    totalOtherMinor: 0,
    totalVoidsMinor: 0,
    totalRefundsMinor: 0,
    totalPayoutsMinor: 0,
    notes: '',
    status: 'open' as const,
    createdAt: date.toISOString(),
    updatedAt: date.toISOString(),
  };
}

const receiptSettingsFixture = {
  showCurrency: true,
  decimalSeparator: '.',
  showTax: true,
  footer: '',
  paperWidth: '80mm',
  showTableNumber: false,
  marginTop: 0,
  marginBottom: 0,
  marginLeft: 0,
  marginRight: 0,
  taxRoundingMode: 'half_up',
};

// Additional FTL strings for the core flow
const testCoreFtl = `
pos-shift-loading = Loading shift…
pos-shift-elapsed = { \$h }h { \$m }m
pos-shift-no-active = No active shift
pos-shift-open-btn = Open Shift
pos-shift-open-aria = Open shift
pos-shift-close-btn = Close Shift
pos-shift-close-aria = Close shift
pos-cart-lock = Lock
pos-cart-open-bill = Open Bill
pos-cart-open-bill-aria = Open Bill
pos-cart-open-bills = Open Bills
pos-cart-open-bills-aria = Open Bills
pos-cart-clear = Clear
pos-cart-clear-aria = Clear cart
pos-cart-pay = Charge
pos-cart-charge-aria = Charge
pos-cart-empty = Cart is empty
pos-cart-empty-subtitle = Tap a menu item to start the order
pos-cart-panel-title = Current Sale
pos-cart-panel-title-order = Current Order
pos-cart-panel-aria = Cart
pos-cart-line-aria = { \$sku }, { \$qty } × { \$amount }
pos-cart-line-override-aria = Override price for { \$name }
pos-cart-line-decrease-aria = Decrease quantity for { \$sku }
pos-cart-line-increase-aria = Increase quantity for { \$sku }
pos-cart-line-remove-aria = Remove { \$sku }
pos-cart-line-swipe-remove-aria = Remove { \$sku }
pos-cart-deduction-badge-aria = Deduction location: { \$name }
pos-cart-deducting-label = Deducting: { \$name }
pos-dismiss-error-aria = Dismiss error
pos-close-shift-cart-error = Cart is not empty. Close or complete the sale first.
pos-close-shift-failed = Failed to close shift
pos-toast-receipt-settings-failed = Failed to load receipt settings
pos-bundle-expanded = Bundle "{ \$name }" added — { \$count } items
pos-no-barcode-match = No product or bundle matches this barcode
pos-scanner-error = Scanner error: { \$detail }
pos-close-shift-title = Close Shift
pos-close-shift-closing = Closing…
pos-close-shift-confirm = Close Shift
pos-close-shift-opening = Opening…
pos-close-shift-opening-balance = Opening Balance
pos-close-shift-opening-balance-placeholder = Enter opening balance
pos-open-shift-title = Open Shift
pos-open-shift-opening-balance = Opening Balance
pos-open-shift-opening-balance-placeholder = Enter opening balance
pos-close-shift-notes-label = Notes (optional)
pos-close-shift-notes-placeholder = Any notes about this shift…
pos-close-shift-notes-aria = Shift notes
pos-close-shift-summary-label = { \$label }
pos-close-shift-summary-value = { \$value }
pos-shift-closed-title = Shift Closed
pos-shift-total-sales = Total Sales
pos-shift-cash-sales = Cash Sales
pos-shift-card-sales = Card Sales
pos-shift-expected-cash = Expected Cash
pos-shift-counted = Counted
pos-shift-difference = Difference
pos-shift-over = Over
pos-shift-short = Short
pos-shift-notes = Notes
pos-shift-summary-done = Done
tables-title = Table Management
retail-fn-history = Sales History
retail-fn-stok = Stock Inquiry
kds-title = Kitchen Display
settings-page-title = Settings
pos-cart-subtotal = Subtotal
pos-cart-options-expand-aria = Show discount, tip, service charge
pos-cart-options-collapse-aria = Hide discount, tip, service charge
payment-done-title = Sale Complete
payment-done-receipt = Receipt printed
payment-change-label = Change due
payment-done-note = Receipt printed
pos-cart-discount-label = Discount ({ \$label })
pos-cart-discount-remove-aria = Remove discount
pos-cart-add-discount = + Add Discount
pos-cart-pct-placeholder = %
pos-cart-label-placeholder = Label (optional)
pos-cart-discount-pct-aria = Discount percentage
pos-cart-discount-label-aria = Discount label
pos-cart-apply = Apply
pos-cart-cancel = Cancel
pos-cart-discount-cancel-aria = Cancel discount
pos-cart-tip-label = Add Tip
pos-cart-tip-aria = Tip percentage
pos-cart-tip-segment-zero-aria = No tip
pos-cart-tip-segment-aria = { \$percent }% tip
pos-cart-tip-line = Tip ({ \$percent }%)
pos-cart-service-toggle-aria = Toggle service charge
pos-cart-service-toggle-label = Add { \$percent }% service charge
pos-cart-service-line = Service ({ \$percent }%)
pos-cart-clear-aria = Clear cart
pos-cart-open-bill-aria = Open Bill
pos-cart-open-bills-aria = Open Bills
pos-cart-undo-btn = Undo
pos-cart-undo-dismiss-aria = Dismiss undo
payment-title = Complete Order
payment-amount-tendered = Amount Tendered
payment-complete = Complete
payment-cancel = Cancel
payment-print = Print
payment-skip = Skip
sale-complete = Sale Complete
payment-method-cash = Cash
payment-method-card = Card
payment-method-qris = QRIS
payment-method-credit = Credit
payment-method-open-bill = Open Bill
payment-qr-reference-placeholder = QR reference
payment-qr-upgrade = QRIS payments are a Plus feature. Upgrade to Plus to accept QRIS.
payment-customer-placeholder = e.g. John Doe
payment-loyalty-points-label = Points
pos-cart-table-label = Table
pos-cart-table-aria = Table number
pos-cart-table-placeholder = Table number
pos-cart-discount-form = Discount form
pos-cart-discount-btn = + Add Discount
pos-cart-discount-apply = Apply
pos-cart-discount-cancel = Cancel
pos-cart-discount-clear = ×
payment-shortfall-cancelled = Payment cancelled
payment-shortfall-title = Shortfall
payment-shortfall-message = Tendered amount is less than total
payment-shortfall-retry = Retry
payment-shortfall-cancel = Cancel
`;

// ── Test helpers ───────────────────────────────────────────────────────

function stripIsolates(text: string): string {
  return text.replace(/[\u2068\u2069]/g, '');
}

async function renderPosScreenWithShift(openedAt?: Date) {
  vi.mocked(shiftsApi.getActiveShiftScoped).mockResolvedValueOnce(shiftFixture(openedAt));
  vi.mocked(settingsApi.getReceiptSettingsScoped).mockResolvedValueOnce(receiptSettingsFixture);
  vi.mocked(salesApi.startSaleScoped).mockResolvedValue({
    cartId: 'test-cart-1',
    deductionLocationId: 'loc-store-inventory',
  });
  vi.mocked(salesApi.getCartDeductionLocation).mockResolvedValue({
    locationId: 'loc-store-inventory',
    locationName: 'Store Inventory',
  });
  // Mock completeSale for payment flow
  vi.mocked(salesApi.completeSaleScoped).mockResolvedValue({
    saleId: 'sale-1',
    total: { minor_units: 1000, currency: 'USD' },
    lineCount: 1,
  });
  // Mock finalizeSale
  vi.mocked(salesApi.finalizeSale).mockResolvedValue({});
  vi.mocked(salesApi.finalizeSaleScoped).mockResolvedValue({});
  // Mock createKdsOrderFromSaleScoped
  vi.mocked(salesApi.createKdsOrderFromSaleScoped).mockResolvedValue({});
  vi.mocked(salesApi.createKdsOrderFromSale).mockResolvedValue({});
  // Mock getSale for receipt
  vi.mocked(salesApi.getSale).mockResolvedValue({
    id: 'sale-1',
    subtotal: { minor_units: 700, currency: 'USD' },
    total: { minor_units: 1000, currency: 'USD' },
    taxTotal: { minor_units: 0, currency: 'USD' },
    lines: [{
      id: 'line-1',
      sku: 'ITEM-001',
      name: 'Test Item',
      qty: 1,
      unit_price: { minor_units: 700, currency: 'USD' },
      tax_amount: { minor_units: 0, currency: 'USD' },
    }],
    created_at: new Date().toISOString(),
  });
  vi.mocked(salesApi.printSalesReceipt).mockResolvedValue({ printed: true });

  return renderWithProviders(
    <PosScreen />,
    salesFtl,
    productsFtl,
    inventoryFtl,
    settingsFtl,
    testCoreFtl,
  );
}

// ── TEST: Core Sale Flow (RED - should fail initially) ─────────────────

describe('PosScreen — Core Sale Flow (TDD)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    mockedBarcode.reset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('adds product to cart, opens payment, completes cash sale, resets cart', async () => {
    // 1. Render with active shift
    await renderPosScreenWithShift();

    // Wait for shift to load - shows "0m" for elapsed time
    await waitFor(() => {
      expect(screen.getByText('0m')).toBeInTheDocument();
    });

    // 2. Add product via barcode scan (simulates clicking product card)
    // First, scan a product barcode
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 700, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    });

    // Debug: check barcode scanner is registered
    await waitFor(() => {
      console.log('Barcode scanner mock:', mockedBarcode.useBarcodeScanner);
      console.log('Barcode scanner calls:', mockedBarcode.useBarcodeScanner.mock.calls);
    }, { timeout: 2000 });

    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    // 3. Verify product added to cart (check cart panel specifically)
    await waitFor(() => {
      const cartLine = screen.getByTestId('cart-panel-line-item');
      expect(cartLine).toBeInTheDocument();
      expect(cartLine.textContent).toContain('Test Item');
    });

    // 4. Verify subtotal shows $7.00 (format uses comma for decimal in some locales)
    await waitFor(() => {
      // Subtotal is in the options toggle button - find by class
      const optionsToggle = document.querySelector('.pos-cart-options-toggle');
      expect(optionsToggle).toBeInTheDocument();
      expect(optionsToggle?.textContent).toContain('$');
      expect(optionsToggle?.textContent).toContain('7');
    });

    // 5. Click Pay button (Charge) - use the one in the cart footer
    const payButtons = screen.getAllByRole('button', { name: /charge/i });
    await userEvent.click(payButtons[payButtons.length - 1]);

    // 6. Verify PaymentModal opens - debug
    await waitFor(() => {
      console.log('After charge click:', document.body.innerHTML?.includes('Complete Order'));
      console.log('showPayment state check:', document.body.innerHTML?.includes('payment-overlay'));
    }, { timeout: 2000 });

    await waitFor(() => {
      expect(screen.getByText('Complete Order')).toBeInTheDocument();
    });

    // 7. Enter amount tendered ($10.00)
    const tenderedInput = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(tenderedInput, '10.00');

    // 8. Click Complete
    const completeBtn = screen.getByRole('button', { name: /^complete$/i });
    await userEvent.click(completeBtn);

    // 9. Wait for ReceiptPreview to appear, then click Skip
    await waitFor(() => {
      expect(screen.getByText('Skip')).toBeInTheDocument();
    });

    const skipBtn = screen.getByRole('button', { name: /skip/i });
    await userEvent.click(skipBtn);

    // 10. Verify sale completes - cart should be reset to empty
    await waitFor(() => {
      expect(screen.getByText('Cart is empty')).toBeInTheDocument();
    }, { timeout: 5000 });

    // 11. Verify shift is still open
    expect(screen.getByText('0m')).toBeInTheDocument();

    // 10. Verify cart is reset (empty state)
    await waitFor(() => {
      expect(screen.getByText('Cart is empty')).toBeInTheDocument();
    });

    // 11. Verify shift is still open
    expect(screen.getByText('0m')).toBeInTheDocument();
  });

  it('adds multiple products, shows correct subtotal, opens payment', async () => {
    await renderPosScreenWithShift();

    await waitFor(() => {
      expect(screen.getByText('0m')).toBeInTheDocument();
    });

    // Add first product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 700, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    });

    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    // Verify product added to cart
    await waitFor(() => {
      const cartLine = screen.getByTestId('cart-panel-line-item');
      expect(cartLine).toBeInTheDocument();
      expect(cartLine.textContent).toContain('Test Item');
    });

    // Add second product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-002',
      name: 'Second Item',
      category: 'Test',
      price: { minor_units: 300, currency: 'USD' },
      barcode: 'BARCODE-002',
      in_stock: true,
      stock_qty: 50,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    });

    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-002');
    });

    // Verify second product added
    await waitFor(() => {
      const cartLines = screen.getAllByTestId('cart-panel-line-item');
      expect(cartLines.length).toBe(2);
      expect(cartLines[1].textContent).toContain('Second Item');
    });

    // Verify subtotal is $10.00 (700 + 300 = 1000 minor units)
    await waitFor(() => {
      const optionsToggle = document.querySelector('.pos-cart-options-toggle');
      expect(optionsToggle).toBeInTheDocument();
      expect(optionsToggle?.textContent).toContain('$');
      expect(optionsToggle?.textContent).toContain('10');
    });

    // Open payment
    const payButtons = screen.getAllByRole('button', { name: /charge/i });
    await userEvent.click(payButtons[payButtons.length - 1]);

    await waitFor(() => {
      expect(screen.getByText('Complete Order')).toBeInTheDocument();
    });

    // Verify total in payment modal matches
    await waitFor(() => {
      console.log('Payment modal body:', document.body.innerHTML?.slice(0, 10000));
    }, { timeout: 2000 });

    await waitFor(() => {
      expect(screen.getByText((content) => content.includes('$') && content.includes('10'))).toBeInTheDocument();
    });
  });

  it('rejects adding product when no shift is open', async () => {
    // Render without shift
    vi.mocked(shiftsApi.getActiveShiftScoped).mockResolvedValueOnce(null);
    vi.mocked(settingsApi.getReceiptSettingsScoped).mockResolvedValueOnce(receiptSettingsFixture);

    await renderWithProviders(
      <PosScreen />,
      salesFtl,
      productsFtl,
      inventoryFtl,
      settingsFtl,
      testCoreFtl,
    );

    // Try to scan a product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 700, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      tax_rate_ids: [],
      product_type: 'standard',
      created_at: '',
      price_updated_at: '',
    });

    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    // Should show warning toast
    await waitFor(() => {
      expect(screen.getByText(/open a shift first/i)).toBeInTheDocument();
    });

    // Cart should remain empty
    expect(screen.getByText('Cart is empty')).toBeInTheDocument();
  });
});