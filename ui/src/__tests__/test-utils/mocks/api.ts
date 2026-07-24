// ── Shared API mocks ────────────────────────────────────────────────
//
// Factory functions for commonly-mocked API modules. Each factory
// returns a vi.fn()-based mock object that matches the real module's
// exports. Pass overrides to customise return values for specific tests.
//
// Usage:
//   import { createSalesApiMock } from '@/__tests__/test-utils/mocks/api';
//   vi.mock('@/api/sales', () => createSalesApiMock());

import { vi } from 'vitest';

// ── sales ─────────────────────────────────────────────────────────

export interface SalesApiOverrides {
  completeSale?: ReturnType<typeof vi.fn>;
  startSale?: ReturnType<typeof vi.fn>;
  addLine?: ReturnType<typeof vi.fn>;
  holdCart?: ReturnType<typeof vi.fn>;
  printSalesReceipt?: ReturnType<typeof vi.fn>;
  getSale?: ReturnType<typeof vi.fn>;
}

export function createSalesApiMock(overrides: SalesApiOverrides = {}) {
  return {
    holdCart: vi.fn(() => Promise.resolve({ id: 'held-1' })),
    listHeldCarts: vi.fn(() => Promise.resolve([])),
    getHeldCart: vi.fn(() => Promise.resolve(null)),
    deleteHeldCart: vi.fn(() => Promise.resolve()),
    startSale: vi.fn(() => Promise.resolve({ cartId: 'cart-1' })),
    getCartDeductionLocation: vi.fn(() => Promise.resolve(null)),
    addLine: vi.fn(() => Promise.resolve({ lineId: 'line-added-1', lineTotal: null })),
    setCartDiscount: vi.fn(() => Promise.resolve()),
    completeSale: vi.fn(() => Promise.resolve({ saleId: 'sale-1', total: { minor_units: 3500, currency: 'IDR' }, lineCount: 1 })),
    listSales: vi.fn(() => Promise.resolve([])),
    getSale: vi.fn(() => Promise.resolve(null)),
    voidSale: vi.fn(),
    processRefund: vi.fn(() => Promise.resolve({ refundId: 'refund-1', totalMinor: 0 })),
    listRefunds: vi.fn(() => Promise.resolve([])),
    exportDailySummary: vi.fn(() => Promise.resolve([])),
    exportSalesByHour: vi.fn(() => Promise.resolve([])),
    exportEodReport: vi.fn(() => Promise.resolve(null)),
    printSalesReceipt: vi.fn(() => Promise.resolve({ printed: true })),
    onReceiptPrinted: vi.fn(),
    getProductTrackSerial: vi.fn(() => Promise.resolve(false)),
    ...overrides,
  };
}

// ── settings ──────────────────────────────────────────────────────

export interface SettingsApiOverrides {
  getStoreSettings?: ReturnType<typeof vi.fn>;
  getReceiptSettings?: ReturnType<typeof vi.fn>;
  getCreditSettings?: ReturnType<typeof vi.fn>;
  getEnabledFeatures?: ReturnType<typeof vi.fn>;
}

export function createSettingsApiMock(overrides: SettingsApiOverrides = {}) {
  return {
    getStoreSettings: vi.fn(() =>
      Promise.resolve({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' }),
    ),
    getReceiptSettings: vi.fn(() => Promise.resolve({
      showCurrency: true, decimalSeparator: 'dot', showTax: true,
      footer: '', paperWidth: 'standard', showTableNumber: false,
      marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
    })),
    setReceiptSettings: vi.fn(),
    setStoreSettings: vi.fn(),
    getCreditSettings: vi.fn(() => Promise.resolve(
      { enabled: true, reminderIntervalHours: 24, maxLimitMinor: 500000 },
    )),
    setCreditSettings: vi.fn(),
    listCreditSales: vi.fn(() => Promise.resolve([])),
    settleCredit: vi.fn(),
    getHardwareSettings: vi.fn(() => Promise.resolve(
      { printerConnection: 'auto', printerDevicePath: '', printerPaperSize: '80',
        scannerDeviceId: '', scannerInputMode: 'auto' },
    )),
    setHardwareSettings: vi.fn(),
    completeSetup: vi.fn(),
    dismissSetupWizard: vi.fn(),
    getSetupStatus: vi.fn(),
    getEnabledFeatures: vi.fn(),
    getUserPreferences: vi.fn(),
    getUserPreferencesScoped: vi.fn(),
    getStoreSettingsScoped: vi.fn(),
    getReceiptSettingsScoped: vi.fn(),
    setUserPreferences: vi.fn(),
    ...overrides,
  };
}

// ── shifts ────────────────────────────────────────────────────────

export interface ShiftsApiOverrides {
  getActiveShift?: ReturnType<typeof vi.fn>;
  openShift?: ReturnType<typeof vi.fn>;
}

const defaultShift = {
  id: 'shift-1', userId: 'user-1', terminalId: null,
  openedAt: new Date().toISOString(), closedAt: null,
  openingBalanceMinor: 0, closingBalanceMinor: null,
  expectedCashMinor: null, cashDifferenceMinor: null,
  totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0,
  totalOtherMinor: 0, totalVoidsMinor: 0, totalRefundsMinor: 0,
  totalPayoutsMinor: 0, notes: '', status: 'open' as const,
  createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
};

export function createShiftsApiMock(overrides: ShiftsApiOverrides = {}) {
  return {
    getActiveShift: vi.fn(() => Promise.resolve(defaultShift)),
    openShift: vi.fn(() => Promise.resolve({ ...defaultShift, openingBalanceMinor: 100000 })),
    closeShift: vi.fn(),
    listShifts: vi.fn(() => Promise.resolve([])),
    getShift: vi.fn(() => Promise.resolve(null)),
    createCashPayout: vi.fn(),
    getShiftReport: vi.fn(),
    ...overrides,
  };
}

// ── hardware ──────────────────────────────────────────────────────

export function createHardwareApiMock() {
  return {
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
  };
}

// ── products ──────────────────────────────────────────────────────

export interface ProductsApiOverrides {
  listProducts?: ReturnType<typeof vi.fn>;
  listCategories?: ReturnType<typeof vi.fn>;
  lookupByBarcode?: ReturnType<typeof vi.fn>;
  lookupProductBySku?: ReturnType<typeof vi.fn>;
}

export function createProductsApiMock(overrides: ProductsApiOverrides = {}) {
  return {
    listProducts: vi.fn(() => Promise.resolve([])),
    listCategories: vi.fn(() => Promise.resolve([])),
    lookupByBarcode: vi.fn(() => Promise.resolve(null)),
    lookupProductBySku: vi.fn(() => Promise.resolve(null)),
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
    ...overrides,
  };
}
