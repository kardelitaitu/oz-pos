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
  holdCartScoped?: ReturnType<typeof vi.fn>;
  listHeldCartsScoped?: ReturnType<typeof vi.fn>;
  getHeldCartScoped?: ReturnType<typeof vi.fn>;
  deleteHeldCartScoped?: ReturnType<typeof vi.fn>;
  startSaleScoped?: ReturnType<typeof vi.fn>;
  addLineScoped?: ReturnType<typeof vi.fn>;
  setCartDiscountScoped?: ReturnType<typeof vi.fn>;
  completeSaleScoped?: ReturnType<typeof vi.fn>;
  listSalesScoped?: ReturnType<typeof vi.fn>;
  getSaleScoped?: ReturnType<typeof vi.fn>;
  voidSaleScoped?: ReturnType<typeof vi.fn>;
  processRefundScoped?: ReturnType<typeof vi.fn>;
  listRefundsScoped?: ReturnType<typeof vi.fn>;
  exportDailySummaryScoped?: ReturnType<typeof vi.fn>;
  exportSalesByHourScoped?: ReturnType<typeof vi.fn>;
  exportEodReportScoped?: ReturnType<typeof vi.fn>;
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
    holdCartScoped: vi.fn((_token: string) => Promise.resolve({ id: 'held-1' })),
    listHeldCartsScoped: vi.fn((_token: string) => Promise.resolve([])),
    getHeldCartScoped: vi.fn((_token: string, _id: string) => Promise.resolve(null)),
    deleteHeldCartScoped: vi.fn((_token: string, _id: string) => Promise.resolve()),
    startSaleScoped: vi.fn((_token: string) => Promise.resolve({ cartId: 'cart-1' })),
    addLineScoped: vi.fn((_token: string) => Promise.resolve({ lineId: 'line-added-1', lineTotal: null })),
    setCartDiscountScoped: vi.fn((_token: string) => Promise.resolve()),
    completeSaleScoped: vi.fn((_token: string) => Promise.resolve({ saleId: 'sale-1', total: { minor_units: 3500, currency: 'IDR' }, lineCount: 1 })),
    listSalesScoped: vi.fn((_token: string) => Promise.resolve([])),
    getSaleScoped: vi.fn((_token: string, _id: string) => Promise.resolve(null)),
    voidSaleScoped: vi.fn((_token: string, _saleId: string, _reason: string) => Promise.resolve()),
    processRefundScoped: vi.fn((_token: string) => Promise.resolve({ refundId: 'refund-1', totalMinor: 0 })),
    listRefundsScoped: vi.fn((_token: string, _saleId: string) => Promise.resolve([])),
    exportDailySummaryScoped: vi.fn((_token: string) => Promise.resolve([])),
    exportSalesByHourScoped: vi.fn((_token: string) => Promise.resolve([])),
    exportEodReportScoped: vi.fn((_token: string) => Promise.resolve(null)),
    ...overrides,
  };
}

// ── settings ──────────────────────────────────────────────────────

export interface SettingsApiOverrides {
  getStoreSettings?: ReturnType<typeof vi.fn>;
  getReceiptSettings?: ReturnType<typeof vi.fn>;
  getCreditSettings?: ReturnType<typeof vi.fn>;
  getEnabledFeatures?: ReturnType<typeof vi.fn>;
  getStoreSettingsScoped?: ReturnType<typeof vi.fn>;
  setReceiptSettingsScoped?: ReturnType<typeof vi.fn>;
  setStoreSettingsScoped?: ReturnType<typeof vi.fn>;
  setCreditSettingsScoped?: ReturnType<typeof vi.fn>;
  listCreditSalesScoped?: ReturnType<typeof vi.fn>;
  settleCreditScoped?: ReturnType<typeof vi.fn>;
  setHardwareSettingsScoped?: ReturnType<typeof vi.fn>;
  setUserPreferencesScoped?: ReturnType<typeof vi.fn>;
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
    // @deprecated kept for backward compat; new callers should use getUserPreferencesScoped
    getUserPreferences: vi.fn(),
    getUserPreferencesScoped: vi.fn((_token: string) => Promise.resolve({})),
    getStoreSettingsScoped: vi.fn((_token: string) =>
      Promise.resolve({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' }),
    ),
    getReceiptSettingsScoped: vi.fn((_token: string) => Promise.resolve({
      showCurrency: true, decimalSeparator: 'dot', showTax: true,
      footer: '', paperWidth: 'standard', showTableNumber: false,
      marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
    })),
    setReceiptSettingsScoped: vi.fn((_token: string) => Promise.resolve()),
    setStoreSettingsScoped: vi.fn((_token: string) => Promise.resolve()),
    setCreditSettingsScoped: vi.fn((_token: string) => Promise.resolve()),
    listCreditSalesScoped: vi.fn((_token: string) => Promise.resolve([])),
    settleCreditScoped: vi.fn((_token: string, _saleId: string) => Promise.resolve()),
    setHardwareSettingsScoped: vi.fn((_token: string) => Promise.resolve()),
    setUserPreferencesScoped: vi.fn((_token: string) => Promise.resolve()),
    setUserPreferences: vi.fn(),
    ...overrides,
  };
}

// ── shifts ────────────────────────────────────────────────────────

export interface ShiftsApiOverrides {
  getActiveShift?: ReturnType<typeof vi.fn>;
  openShift?: ReturnType<typeof vi.fn>;
  getActiveShiftScoped?: ReturnType<typeof vi.fn>;
  openShiftScoped?: ReturnType<typeof vi.fn>;
  closeShiftScoped?: ReturnType<typeof vi.fn>;
  listShiftsScoped?: ReturnType<typeof vi.fn>;
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
    getActiveShiftScoped: vi.fn((_token: string) => Promise.resolve(defaultShift)),
    openShiftScoped: vi.fn((_token: string, _openingBalanceMinor: number) => Promise.resolve({ ...defaultShift, openingBalanceMinor: 100000 })),
    closeShiftScoped: vi.fn((_token: string, _id: string, _closingBalanceMinor: number) => Promise.resolve()),
    listShiftsScoped: vi.fn((_token: string) => Promise.resolve([])),
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
  listProductsScoped?: ReturnType<typeof vi.fn>;
  listCategoriesScoped?: ReturnType<typeof vi.fn>;
  lookupByBarcodeScoped?: ReturnType<typeof vi.fn>;
  lookupProductBySkuScoped?: ReturnType<typeof vi.fn>;
  createProductScoped?: ReturnType<typeof vi.fn>;
  updateProductScoped?: ReturnType<typeof vi.fn>;
  deleteProductScoped?: ReturnType<typeof vi.fn>;
  adjustStockScoped?: ReturnType<typeof vi.fn>;
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
    listProductsScoped: vi.fn((_token: string) => Promise.resolve([])),
    listCategoriesScoped: vi.fn((_token: string) => Promise.resolve([])),
    lookupByBarcodeScoped: vi.fn((_token: string, _barcode: string) => Promise.resolve(null)),
    lookupProductBySkuScoped: vi.fn((_token: string, _sku: string) => Promise.resolve(null)),
    createProductScoped: vi.fn((_token: string) => Promise.resolve({ sku: 'new-sku' })),
    updateProductScoped: vi.fn((_token: string) => Promise.resolve({ sku: 'updated' })),
    deleteProductScoped: vi.fn((_token: string, _sku: string) => Promise.resolve()),
    adjustStockScoped: vi.fn((_token: string) => Promise.resolve(0)),
    ...overrides,
  };
}
