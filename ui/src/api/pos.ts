// The ONLY place in the UI that calls `invoke()`. Components consume
// hooks that call these wrappers; never import `@tauri-apps/api/core`
// directly from a component.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CartId, LineId, Money, Sku } from '@/types/domain';

export interface StartSaleArgs {
  /** ISO-4217 currency code. Empty string defaults to "USD". */
  currency: string;
}

export interface StartSaleResult {
  cartId: CartId;
}

export interface AddLineArgs {
  cartId: CartId;
  sku: Sku;
  qty: number;
  unitPriceMinor: number;
}

export interface AddLineResult {
  lineId: LineId;
  /** `null` if the line total overflowed i64. */
  lineTotal: Money | null;
}

export interface CompleteSaleArgs {
  cartId: CartId;
  paymentMethod: string;
  tenderedMinor: number | null;
}

export interface CompleteSaleResult {
  saleId: string;
  total: Money | null;
  lineCount: number;
}

export interface PingResult {
  // ping returns "pong" as a string
}

export const ping = (): Promise<string> => invoke<string>('ping');

export interface VersionInfo {
  name: string;
  version: string;
  rustVersion: string;
  target: string;
}

export const getVersion = (): Promise<VersionInfo> =>
  invoke<VersionInfo>('version');

export const startSale = (args: StartSaleArgs): Promise<StartSaleResult> =>
  invoke<StartSaleResult>('start_sale', { args });

export const addLine = (args: AddLineArgs): Promise<AddLineResult> =>
  invoke<AddLineResult>('add_line', { args });

export const completeSale = (args: CompleteSaleArgs): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale', { args });

// ── Hardware ──────────────────────────────────────────────────────

export interface OpenCashDrawerArgs {
  deviceId?: string;
}

export interface OpenCashDrawerResult {
  opened: boolean;
}

export const openCashDrawer = (
  args: OpenCashDrawerArgs = {},
): Promise<OpenCashDrawerResult> =>
  invoke<OpenCashDrawerResult>('open_cash_drawer', { args });

export interface PrintReceiptArgs {
  body: string;
}

export interface PrintReceiptResult {
  printedLines: number;
}

export const printReceipt = (
  args: PrintReceiptArgs,
): Promise<PrintReceiptResult> =>
  invoke<PrintReceiptResult>('print_receipt', { args });

// ── Structured sales receipt ─────────────────────────────────────────

export interface LineItemDto {
  name: string;
  quantity: number;
  unitPrice: MoneyDto;
  totalPrice: MoneyDto;
}

export interface PaymentDto {
  method: string;
  amount: MoneyDto;
  change: MoneyDto | null;
}

export interface MoneyDto {
  minorUnits: number;
  currency: string;
}

export interface PrintSalesReceiptArgs {
  date: string;
  receiptNumber: string;
  items: LineItemDto[];
  subtotal: MoneyDto;
  tax?: MoneyDto;
  total: MoneyDto;
  payments: PaymentDto[];
}

export interface PrintSalesReceiptResult {
  printed: boolean;
}

export const printSalesReceipt = (
  args: PrintSalesReceiptArgs,
): Promise<PrintSalesReceiptResult> =>
  invoke<PrintSalesReceiptResult>('print_sales_receipt', { args });

/// Listen for `receipt:printed` events emitted by the backend.
export const onReceiptPrinted = (
  handler: (lines: number) => void,
): Promise<UnlistenFn> =>
  listen<{ lines: number }>('receipt:printed', (e) =>
    handler(e.payload.lines),
  );

// ── Receipt Settings ──────────────────────────────────────────────────

export interface ReceiptSettingsDto {
  showCurrency: boolean;
  decimalSeparator: string;
  showTax: boolean;
  footer: string;
  paperWidth: string;
}

export const getReceiptSettings = (): Promise<ReceiptSettingsDto> =>
  invoke<ReceiptSettingsDto>('get_receipt_settings');

export const setReceiptSettings = (
  args: ReceiptSettingsDto,
): Promise<void> => invoke<void>('set_receipt_settings', { args });

// ── Store Settings ────────────────────────────────────────────────────

export interface StoreSettingsDto {
  name: string;
  address: string;
  taxId: string;
}

export const getStoreSettings = (): Promise<StoreSettingsDto> =>
  invoke<StoreSettingsDto>('get_store_settings');

export const setStoreSettings = (
  args: StoreSettingsDto,
): Promise<void> => invoke<void>('set_store_settings', { args });

// ── Setup Wizard ────────────────────────────────────────────────────

export interface CompleteSetupArgs {
  /** Store preset name (e.g. "simple-retail", "restaurant"). */
  preset: string;
  /** Enabled feature keys (kebab-case, e.g. "cash-payment"). */
  features: string[];
}

export interface SetupStatus {
  /** Whether the setup wizard has been completed. */
  completed: boolean;
  /** The store preset name, if set. */
  preset: string | null;
}

/**
 * Persist the chosen preset and enabled features, then mark setup
 * as complete.
 */
export const completeSetup = (
  args: CompleteSetupArgs,
): Promise<void> => invoke<void>('complete_setup', { args });

/**
 * Check whether the setup wizard has already been completed.
 * The app calls this on mount to decide which screen to show.
 */
export const getSetupStatus = (): Promise<SetupStatus> =>
  invoke<SetupStatus>('get_setup_status');

// ── Products ────────────────────────────────────────────────────────

export interface ProductDto {
  sku: string;
  name: string;
  category: string | null;
  price: { minor_units: number; currency: string };
  barcode: string | null;
  in_stock: boolean;
  stock_qty: number | null;
  tax_rate_ids: string[];
}

/**
 * Fetch all products from the database.
 * Returns an array of product DTOs with category names and stock status.
 */
export const listProducts = (): Promise<ProductDto[]> =>
  invoke<ProductDto[]>('list_products');

// ── Product CRUD ─────────────────────────────────────────────────────

export interface CreateProductArgs {
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  initialStock: number;
  taxRateIds: string[];
}

export interface UpdateProductArgs {
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  taxRateIds: string[];
}

export const createProduct = (
  args: CreateProductArgs,
): Promise<{ sku: string }> => invoke('create_product', { args });

export const updateProduct = (
  args: UpdateProductArgs,
): Promise<{ sku: string }> => invoke('update_product', { args });

export const deleteProduct = (
  sku: string,
): Promise<void> => invoke('delete_product', { args: { sku } });

// ── Inventory Adjustment ─────────────────────────────────────────────

export interface AdjustStockArgs {
  sku: string;
  delta: number;
  reason: string;
}

/**
 * Adjust stock for a product by SKU.
 * Positive delta = restock, negative delta = removal.
 * Returns the new quantity.
 */
export const adjustStock = (
  args: AdjustStockArgs,
): Promise<number> =>
  invoke<number>('adjust_stock', { args });

// ── Sales History ─────────────────────────────────────────────────────

export interface SaleListItem {
  id: string;
  total: Money;
  lineCount: number;
  status: string;
  paymentMethod: string | null;
  createdAt: string;
}

export interface SaleLineDto {
  sku: string;
  name: string;
  qty: number;
  unit_price: Money;
  total_minor: number;
}

export interface SaleDetail {
  id: string;
  total: Money;
  lineCount: number;
  status: string;
  paymentMethod: string | null;
  tenderedMinor: number | null;
  createdAt: string;
  lines: SaleLineDto[];
}

export const listSales = (): Promise<SaleListItem[]> =>
  invoke<SaleListItem[]>('list_sales');

export const getSale = (id: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('get_sale', { id });

// ── Dashboard ────────────────────────────────────────────────────────

export interface DailySummaryRow {
  sale_id: string;
  total_minor: number;
  currency: string;
  line_count: number;
  status: string;
  created_at: string;
}

export interface SalesByHourRow {
  hour: number;
  total_minor: number;
  sale_count: number;
}

export const exportDailySummary = (): Promise<DailySummaryRow[]> =>
  invoke<DailySummaryRow[]>('export_daily_summary');

export const exportSalesByHour = (): Promise<SalesByHourRow[]> =>
  invoke<SalesByHourRow[]>('export_sales_by_hour');

// ── Tax Rates ──────────────────────────────────────────────────────

export interface TaxRateDto {
  id: string;
  name: string;
  rate_bps: number;
  is_default: boolean;
  display_rate: string;
  created_at: string;
  updated_at: string;
}

export interface CreateTaxRateArgs {
  name: string;
  rateBps: number;
  isDefault: boolean;
}

export interface UpdateTaxRateArgs {
  id: string;
  name: string;
  rateBps: number;
  isDefault: boolean;
}

export const listTaxRates = (): Promise<TaxRateDto[]> =>
  invoke<TaxRateDto[]>('list_tax_rates');

export const createTaxRate = (
  args: CreateTaxRateArgs,
): Promise<TaxRateDto> =>
  invoke<TaxRateDto>('create_tax_rate', { args });

export const updateTaxRate = (
  args: UpdateTaxRateArgs,
): Promise<TaxRateDto> =>
  invoke<TaxRateDto>('update_tax_rate', { args });

export const deleteTaxRate = (
  id: string,
): Promise<void> => invoke('delete_tax_rate', { id });

// ── Barcode Scanner ─────────────────────────────────────────────────

export interface ScannerInfo {
  id: string;
}

export interface BarcodeScannedPayload {
  code: string;
  symbology: string;
}

/** List all registered barcode scanners (USB, serial, mock). */
export const listScanners = (): Promise<ScannerInfo[]> =>
  invoke<ScannerInfo[]>('list_scanners');

/** Start the background scanner polling task for the given scanner id. */
export const startScanner = (scannerId: string): Promise<void> =>
  invoke('start_scanner', { scannerId });

/** Stop the active scanner polling task. */
export const stopScanner = (): Promise<void> => invoke('stop_scanner');

/** Listen for `barcode:scanned` events emitted by the backend. */
export const onBarcodeScanned = (
  handler: (payload: BarcodeScannedPayload) => void,
): Promise<UnlistenFn> =>
  listen<BarcodeScannedPayload>('barcode:scanned', (e) =>
    handler(e.payload),
  );

/** Listen for `barcode:error` events emitted by the backend. */
export const onBarcodeError = (
  handler: (error: string) => void,
): Promise<UnlistenFn> =>
  listen<{ error: string }>('barcode:error', (e) =>
    handler(e.payload.error),
  );

/** Look up a product by barcode. Returns `null` when not found. */
export const lookupByBarcode = (
  barcode: string,
): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_by_barcode', { barcode });

// ── Feature Flags ────────────────────────────────────────────────────

export interface EnabledFeaturesResult {
  /** Kebab-case feature keys (e.g. "cash-payment", "barcode-scanning"). */
  features: string[];
}

/**
 * Fetch the list of currently-enabled feature keys from the backend.
 *
 * The front-end calls this on mount to decide which nav items and UI
 * elements to show/hide based on the store's active feature flags.
 */
export const getEnabledFeatures = (): Promise<EnabledFeaturesResult> =>
  invoke<EnabledFeaturesResult>('get_enabled_features');

// ── Auth ──────────────────────────────────────────────────────────────

export interface StaffLoginArgs {
  username: string;
  pin: string;
}

export interface LoginSessionDto {
  user_id: string;
  display_name: string;
  role_name: string;
  role_id: string;
}

export interface StaffLoginResult {
  session: LoginSessionDto;
}

/**
 * Authenticate a staff member by username and PIN.
 * Returns session info including user id, display name, and role.
 */
export const staffLogin = (
  args: StaffLoginArgs,
): Promise<StaffLoginResult> =>
  invoke<StaffLoginResult>('staff_login', { args });

// ── Staff Management ─────────────────────────────────────────────────

export interface StaffMemberDto {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  role_name: string;
  is_active: boolean;
}

export interface RoleDto {
  id: string;
  name: string;
  description: string;
}

export interface CreateStaffArgs {
  username: string;
  pin: string;
  display_name: string;
  role_id: string;
}

export interface UpdateStaffArgs {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  is_active: boolean;
}

export const listStaff = (): Promise<StaffMemberDto[]> =>
  invoke<StaffMemberDto[]>('list_staff');

export const listRoles = (): Promise<RoleDto[]> =>
  invoke<RoleDto[]>('list_roles');

export const createStaff = (
  args: CreateStaffArgs,
): Promise<StaffMemberDto> =>
  invoke<StaffMemberDto>('create_staff', { args });

export const updateStaff = (
  args: UpdateStaffArgs,
): Promise<StaffMemberDto> =>
  invoke<StaffMemberDto>('update_staff', { args });

// ── Customers ─────────────────────────────────────────────────────────

export interface CustomerDto {
  id: string;
  name: string;
  email: string | null;
  phone: string | null;
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface CreateCustomerArgs {
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

export interface UpdateCustomerArgs {
  id: string;
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

export const listCustomers = (): Promise<CustomerDto[]> =>
  invoke<CustomerDto[]>('list_customers');

export const getCustomer = (id: string): Promise<CustomerDto | null> =>
  invoke<CustomerDto | null>('get_customer', { id });

export const createCustomer = (
  args: CreateCustomerArgs,
): Promise<CustomerDto> =>
  invoke<CustomerDto>('create_customer', { args });

export const updateCustomer = (
  args: UpdateCustomerArgs,
): Promise<CustomerDto> =>
  invoke<CustomerDto>('update_customer', { args });

export const deleteCustomer = (
  id: string,
): Promise<void> => invoke('delete_customer', { id });

// ── Categories ────────────────────────────────────────────────────────

export interface CategoryDto {
  id: string;
  name: string;
  colour: string;
}

export interface CreateCategoryArgs {
  id: string;
  name: string;
  colour: string;
}

/** Fetch all categories from the database. */
export const listCategories = (): Promise<CategoryDto[]> =>
  invoke<CategoryDto[]>('list_categories');

/** Create a new category. */
export const createCategory = (
  args: CreateCategoryArgs,
): Promise<{ id: string }> => invoke('create_category', { args });

/** Delete a category by id. */
export const deleteCategory = (
  id: string,
): Promise<void> => invoke('delete_category', { args: { id } });

// ── Currencies ─────────────────────────────────────────────────────

export interface CurrencyInfo {
  code: string;
  exponent: number;
}

export interface CurrencyDto {
  code: string;
  name: string;
  minor_exponent: number;
  symbol: string;
}

export interface SetDefaultCurrencyArgs {
  code: string;
}

export const getCurrencyInfo = (code: string): Promise<CurrencyInfo> =>
  invoke<CurrencyInfo>('currency_info', { code });

export const listCurrencies = (): Promise<CurrencyDto[]> =>
  invoke<CurrencyDto[]>('list_currencies');

export const getDefaultCurrency = (): Promise<string | null> =>
  invoke<string | null>('get_default_currency');

export const setDefaultCurrency = (args: SetDefaultCurrencyArgs): Promise<void> =>
  invoke<void>('set_default_currency', { args });

// ── Exchange Rates ───────────────────────────────────────────────────

export interface ExchangeRateDto {
  id: string;
  from_currency: string;
  to_currency: string;
  rate: number;
  source: string;
  effective_date: string;
  created_at: string;
}

export interface CreateExchangeRateArgs {
  fromCurrency: string;
  toCurrency: string;
  rate: number;
  source?: string;
  effectiveDate?: string;
}

export const listExchangeRates = (): Promise<ExchangeRateDto[]> =>
  invoke<ExchangeRateDto[]>('list_exchange_rates');

export const createExchangeRate = (args: CreateExchangeRateArgs): Promise<ExchangeRateDto> =>
  invoke<ExchangeRateDto>('create_exchange_rate', { args });

export const deleteExchangeRate = (id: string): Promise<void> =>
  invoke<void>('delete_exchange_rate', { id });

// ── Audit Log ─────────────────────────────────────────────────────────

export interface AuditEntryDto {
  id: string;
  user_id: string;
  action: string;
  target_type: string | null;
  target_id: string | null;
  details: string;
  outcome: string;
  created_at: string;
}

/**
 * Fetch audit log entries in reverse chronological order.
 * Supports pagination via `limit` and `offset`.
 */
export const listAuditLog = (
  limit: number = 100,
  offset: number = 0,
): Promise<AuditEntryDto[]> =>
  invoke<AuditEntryDto[]>('list_audit_log', {
    args: { limit, offset },
  });

// ── EOD Report ───────────────────────────────────────────────────────

export interface PaymentBreakdown {
  method: string;
  count: number;
  total: number;
}

export interface SalesByHourRow {
  hour: number;
  total_minor: number;
  sale_count: number;
}

export interface EodReport {
  total_sales: number;
  total_revenue: number;
  currency: string;
  payment_breakdown: PaymentBreakdown[];
  void_count: number;
  void_total: number;
  discount_count: number;
  discount_total: number;
  hourly_breakdown: SalesByHourRow[];
}

/**
 * Fetch the End-of-Day report for today.
 * Returns a comprehensive summary of sales, payments, voids, and discounts.
 */
export const exportEodReport = (): Promise<EodReport> =>
  invoke<EodReport>('export_eod_report');

// ── Discount ─────────────────────────────────────────────────────────

export interface SetCartDiscountArgs {
  cartId: string;
  percent: number;
  label?: string;
}

/**
 * Set or clear a cart-level percentage discount.
 * Pass percent=0 to clear. Label is optional.
 */
export const setCartDiscount = (
  args: SetCartDiscountArgs,
): Promise<void> =>
  invoke<void>('set_cart_discount', { args });

// ── Hold Order ───────────────────────────────────────────────────────

export interface HoldCartArgs {
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
}

export interface HeldCartRow {
  id: string;
  label: string;
  item_count: number;
  total_minor: number;
  currency: string;
  created_at: string;
}

export interface HeldCartFull {
  id: string;
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
  created_at: string;
}

/**
 * Park the current cart as a held order.
 */
export const holdCart = (
  args: HoldCartArgs,
): Promise<{ id: string }> =>
  invoke<{ id: string }>('hold_cart', { args });

/**
 * List all held orders, most recent first.
 */
export const listHeldCarts = (): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_held_carts');

/**
 * Get the full held cart data by id.
 */
export const getHeldCart = (
  id: string,
): Promise<HeldCartFull | null> =>
  invoke<HeldCartFull | null>('get_held_cart', { id });

/**
 * Delete a held cart by id.
 */
export const deleteHeldCart = (
  id: string,
): Promise<void> => invoke('delete_held_cart', { id });

// ── Void Sale ─────────────────────────────────────────────────────────

export interface VoidSaleArgs {
  saleId: string;
  userId: string;
  reason: string;
}

export interface VoidSaleResult {
  id: string;
  status: string;
  total: Money;
  line_count: number;
  created_at: string;
}

/**
 * Void an active sale by id. Restores stock for all line items
 * and creates an immutable audit log entry.
 */
export const voidSale = (
  args: VoidSaleArgs,
): Promise<VoidSaleResult> =>
  invoke<VoidSaleResult>('void_sale', { args });
