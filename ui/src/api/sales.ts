// ── Sales: POS cart, history, void, discounts, held carts, refunds, dashboard, printing ──

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CartId, LineId, Money } from '@/types/domain';

// ── Cart operations ────────────────────────────────────────────────

/** Arguments for starting a new sale. */
export interface StartSaleArgs {
  currency: string;
}

/** Result of starting a new sale, containing the new cart identifier. */
export interface StartSaleResult {
  cartId: CartId;
  /** ADR-19 §5.1: the deduction location locked at cart-start time. */
  deductionLocationId?: string;
}

/** Arguments for adding a line item to a cart. */
export interface AddLineArgs {
  cartId: CartId;
  sku: string;
  qty: number;
  unitPriceMinor: number;
}

/** Result of adding a line item to a cart. */
export interface AddLineResult {
  lineId: LineId;
  lineTotal: Money | null;
}

/** A serial number captured at checkout for a tracked product. */
export interface SerialNumberArg {
  sku: string;
  serial: string;
}

/** A single payment split for multi-method payments. */
export interface PaymentSplitArg {
  method: string;
  amountMinor: number;
  gatewayReference?: string;
  gatewayStatus?: string;
  gatewayResponse?: string;
}

export interface CompleteSaleArgs {
  cartId: CartId;
  paymentMethod: string;
  tenderedMinor: number | null;
  userId?: string;
  /** Optional customer id to link this sale for loyalty tracking. */
  customerId?: string;
  /** Optional customer name (for credit sales). */
  customerName?: string;
  /** Optional payment splits for multi-method payments. */
  paymentSplits?: PaymentSplitArg[];
  /** Optional serial numbers captured at checkout for track_serial products. */
  serialNumbers?: SerialNumberArg[];
}

/** Result of completing a sale. */
export interface CompleteSaleResult {
  saleId: string;
  total: Money | null;
  lineCount: number;
}

/** Arguments for setting a discount on a cart. */
export interface SetCartDiscountArgs {
  cartId: string;
  percent: number;
  label?: string;
  userId: string;
}

/** Start a new sale and return the new cart identifier. */
export const startSale = (args: StartSaleArgs): Promise<StartSaleResult> =>
  invoke<StartSaleResult>('start_sale', { args });

/** ADR #7: Start a new sale in the store resolved from a session token. */
export const startSaleScoped = (sessionToken: string, args: StartSaleArgs): Promise<StartSaleResult> =>
  invoke<StartSaleResult>('start_sale_scoped', { sessionToken, args });

/** ADR-19: Info about the deduction location locked on a cart. */
export interface DeductionLocationInfo {
  locationId: string;
  locationName: string;
  /** ISO-8601 timestamp of the last manager override, or null. */
  overriddenAt?: string;
}

/** ADR-19 §5.1: Get the deduction location info for an active cart. */
export const getCartDeductionLocation = (cartId: string): Promise<DeductionLocationInfo | null> =>
  invoke<DeductionLocationInfo | null>('get_cart_deduction_location', { cartId });

/** Add a line item to a cart. */
export const addLine = (args: AddLineArgs): Promise<AddLineResult> =>
  invoke<AddLineResult>('add_line', { args });

/** ADR #7: Add a line to a cart in the store resolved from a session token. */
export const addLineScoped = (sessionToken: string, args: AddLineArgs): Promise<AddLineResult> =>
  invoke<AddLineResult>('add_line_scoped', { sessionToken, args });

/** Complete a sale with the given payment details and return the sale record. */
export const completeSale = (args: CompleteSaleArgs): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale', { args });

/** Arguments for completing a sale scoped to the session store. `userId` is read from session, not args. ADR #7. */
export interface CompleteSaleScopedArgs {
  cartId: string;
  paymentMethod: string;
  tenderedMinor: number | null;
  customerId?: string;
  paymentSplits?: PaymentSplitArg[];
  customerName?: string;
  serialNumbers?: SerialNumberArg[];
}

export const completeSaleScoped = (sessionToken: string, args: CompleteSaleScopedArgs): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale_scoped', { sessionToken, args });

// ── Shortfall Resolution — complete_sale_with_resolved_shortfalls ──

export interface LocationAllocation {
  locationId: string;
  qty: number;
}

export interface ResolvedShortfall {
  sku: string;
  allocations: LocationAllocation[];
}

export interface CartLineData {
  sku: string;
  qty: number;
  unitPriceMinor: number;
}

export interface CompleteSaleWithResolvedShortfallsArgs {
  cartId: string;
  paymentMethod: string;
  tenderedMinor: number | null;
  customerId?: string;
  paymentSplits?: PaymentSplitArg[];
  customerName?: string;
  serialNumbers?: SerialNumberArg[];
  /** Cart line data reconstructed from the in-memory cart (original was deleted). */
  lines: CartLineData[];
  /** Total sale amount in minor units. */
  totalMinor: number;
  /** ISO-4217 currency code. */
  currency: string;
  /** Discount percentage (0-100). */
  discountPercent: number;
  /** Optional discount label. */
  discountLabel?: string;
  /** Cashier-resolved shortfalls: per-SKU allocation to specific locations. */
  resolutions: ResolvedShortfall[];
}

/** Complete a sale with cashier-resolved shortfalls (split fulfillment).
 *  This is the second command after a PartialStockResult is returned. */
export const completeSaleWithResolvedShortfalls = (
  sessionToken: string,
  args: CompleteSaleWithResolvedShortfallsArgs
): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale_with_resolved_shortfalls_scoped', { sessionToken, args });

// Export front-end types for the StockShortfallDialog
// PartialStockResult, Shortfall, LocationStock are returned by the backend
// and consumed by the front-end dialog; they are defined in the Rust side
// and serialized via JSON. The front-end types mirror the Rust definitions:
export interface LocationStock {
  locationId: string;
  locationName: string;
  qtyAvailable: number;
}

export interface Shortfall {
  sku: string;
  productName: string;
  requestedQty: number;
  primaryQtyAvailable: number;
  deficit: number;
  primaryLocationId: string;
  alternatives: LocationStock[];
}

export interface PartialStockResult {
  requiresResolution: boolean;
  shortfalls: Shortfall[];
}

/** ADR-19 §17: Record a manager override of the deduction location on an active cart. */
export const overrideCartDeductionLocation = (sessionToken: string, cartId: string): Promise<void> =>
  invoke<void>('override_cart_deduction_location_scoped', { sessionToken, cartId });

/** Check whether a product is configured for serial number tracking. */
export const getProductTrackSerial = (sku: string): Promise<boolean> =>
  invoke<boolean>('get_product_track_serial', { sku });

/** Apply a percentage-based discount to a cart. */
export const setCartDiscount = (args: SetCartDiscountArgs): Promise<void> =>
  invoke<void>('set_cart_discount', { args });

/** ADR #7: Scoped cart discount — `userId` is read from session. */
export interface SetCartDiscountScopedArgs {
  cartId: string;
  percent: number;
  label?: string | null;
}

export const setCartDiscountScoped = (sessionToken: string, args: SetCartDiscountScopedArgs): Promise<void> =>
  invoke<void>('set_cart_discount_scoped', { sessionToken, args });

/** Arguments for overriding the price of a specific line in a cart. */
export interface OverrideLinePriceArgs {
  cartId: string;
  lineId: string;
  newPriceMinor: number;
  userId: string;
}

/** Override the unit price of a line item in a cart. */
export const overrideLinePrice = (args: OverrideLinePriceArgs): Promise<void> =>
  invoke<void>('override_line_price', { args });

/** ADR #7: Scoped line price override — `userId` is read from session. */
export const overrideLinePriceScoped = (sessionToken: string, cartId: string, lineId: string, newPriceMinor: number): Promise<void> =>
  invoke<void>('override_line_price_scoped', { sessionToken, args: { cartId, lineId, newPriceMinor } });

// ── Sales History ─────────────────────────────────────────────────

/** A summary row for a sale in the sales history list. */
export interface SaleListItem {
  id: string;
  total: Money;
  lineCount: number;
  status: string;
  paymentMethod: string | null;
  userId: string | null;
  createdAt: string;
}

/** A line item within a sale detail. */
export interface SaleLineDto {
  id: string;
  sku: string;
  name: string;
  qty: number;
  unit_price: Money;
  total_minor: number;
  tax_amount: Money | null;
  tax_rate_id: string | null;
}

/** Full detail of a completed sale, including line items. */
export interface SaleDetail {
  id: string;
  total: Money;
  subtotal: Money;
  taxTotal: Money;
  lineCount: number;
  status: string;
  paymentMethod: string | null;
  tenderedMinor: number | null;
  userId: string | null;
  createdAt: string;
  lines: SaleLineDto[];
}

/** List all completed sales. */
export const listSales = (): Promise<SaleListItem[]> =>
  invoke<SaleListItem[]>('list_sales');

/** ADR #7: List sales scoped to the store resolved from a session token. */
export const listSalesScoped = (sessionToken: string): Promise<SaleListItem[]> =>
  invoke<SaleListItem[]>('list_sales_scoped', { sessionToken });

/** Fetch a single sale by its identifier. */
export const getSale = (id: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('get_sale', { id });

/** ADR #7: Fetch a sale by ID from the store resolved from a session token. */
export const getSaleScoped = (sessionToken: string, id: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('get_sale_scoped', { sessionToken, id });

// ── Void Sale ─────────────────────────────────────────────────────

/** Arguments for voiding a completed sale. */
export interface VoidSaleArgs {
  saleId: string;
  userId: string;
  reason: string;
}

/** Result of voiding a sale. */
export interface VoidSaleResult {
  id: string;
  status: string;
  total: Money;
  line_count: number;
  created_at: string;
}

/** Void a completed sale with a reason. */
export const voidSale = (args: VoidSaleArgs): Promise<VoidSaleResult> =>
  invoke<VoidSaleResult>('void_sale', { args });

/** ADR #7: Void a sale in the store resolved from a session token. */
export const voidSaleScoped = (sessionToken: string, saleId: string, reason: string): Promise<VoidSaleResult> =>
  invoke<VoidSaleResult>('void_sale_scoped', { sessionToken, args: { saleId, reason } });

// ── Hold Order ────────────────────────────────────────────────────

/** Arguments for holding (parking) a cart for later retrieval. */
export interface HoldCartArgs {
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
  bill_type?: string;
  customer_name?: string;
}

/** A summary row of a held (parked) cart. */
export interface HeldCartRow {
  id: string;
  label: string;
  item_count: number;
  total_minor: number;
  currency: string;
  created_at: string;
  bill_type: string;
  customer_name: string | null;
}

/** Full detail of a held cart including serialised cart data. */
export interface HeldCartFull {
  id: string;
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
  created_at: string;
  bill_type: string;
  customer_name: string | null;
}

/** Park the current cart for later retrieval. */
export const holdCart = (args: HoldCartArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('hold_cart', { args });

/** ADR #7: Hold a cart in the store resolved from a session token. */
export const holdCartScoped = (sessionToken: string, args: HoldCartArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('hold_cart_scoped', { sessionToken, args });

/** List all held (parked) carts. */
export const listHeldCarts = (): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_held_carts');

/** ADR #7: Scoped held carts listing. */
export const listHeldCartsScoped = (sessionToken: string): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_held_carts_scoped', { sessionToken });

/** List all open bills (table-based held carts). */
export const listOpenBills = (): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_open_bills');

/** ADR #7: Scoped open bills listing. */
export const listOpenBillsScoped = (sessionToken: string): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_open_bills_scoped', { sessionToken });

/** Fetch the full detail of a held cart by its identifier. */
export const getHeldCart = (id: string): Promise<HeldCartFull | null> =>
  invoke<HeldCartFull | null>('get_held_cart', { id });

/** ADR #7: Scoped held cart retrieval. */
export const getHeldCartScoped = (sessionToken: string, id: string): Promise<HeldCartFull | null> =>
  invoke<HeldCartFull | null>('get_held_cart_scoped', { sessionToken, id });

/** Delete a held cart by its identifier. */
export const deleteHeldCart = (id: string): Promise<void> =>
  invoke('delete_held_cart', { id });

/** ADR #7: Scoped held cart deletion. */
export const deleteHeldCartScoped = (sessionToken: string, id: string): Promise<void> =>
  invoke('delete_held_cart_scoped', { sessionToken, id });

// ── Refunds ───────────────────────────────────────────────────────

/** A single line item being refunded. */
export interface RefundLineArg {
  saleLineId: string;
  sku: string;
  qty: number;
  unitPriceMinor: number;
  currency: string;
  lineTotalMinor: number;
}

/** Arguments for processing a refund against a completed sale. */
export interface ProcessRefundArgs {
  saleId: string;
  reason: string;
  note?: string | null;
  userId: string;
  lines: RefundLineArg[];
}

/** Result of processing a refund. */
export interface ProcessRefundResult {
  refundId: string;
  totalMinor: number;
}

/** A processed refund record with its associated line items. */
export interface RefundDto {
  id: string;
  saleId: string;
  total: Money;
  reason: string;
  note: string;
  processedBy: string;
  createdAt: string;
  lines: RefundLineDto[];
}

/** A line item within a refund record. */
export interface RefundLineDto {
  id: string;
  refundId: string;
  saleLineId: string;
  sku: string;
  qty: number;
  unitPrice: Money;
  lineTotal: Money;
}

/** Look up a completed sale by its receipt barcode. */
export const lookupSaleByReceiptBarcode = (barcode: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('lookup_sale_by_receipt_barcode', { barcode });

/** ADR #7: Scoped receipt barcode lookup using session token. */
export const lookupSaleByReceiptBarcodeScoped = (sessionToken: string, barcode: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('lookup_sale_by_receipt_barcode_scoped', { sessionToken, barcode });

/** Process a refund against a completed sale. */
export const processRefund = (args: ProcessRefundArgs): Promise<ProcessRefundResult> =>
  invoke<ProcessRefundResult>('process_refund', { args });

/** Arguments for processing a refund scoped to the session store. `userId` is read from session, not args. ADR #7. */
export interface ProcessRefundScopedArgs {
  saleId: string;
  reason: string;
  note?: string | null;
  lines: RefundLineArg[];
}

export const processRefundScoped = (sessionToken: string, args: ProcessRefundScopedArgs): Promise<ProcessRefundResult> =>
  invoke<ProcessRefundResult>('process_refund_scoped', { sessionToken, args });

/** List all refunds for a given sale. */
export const listRefunds = (saleId: string): Promise<RefundDto[]> =>
  invoke<RefundDto[]>('list_refunds', { saleId });

/** ADR #7: Scoped refund listing using session token. */
export const listRefundsScoped = (sessionToken: string, saleId: string): Promise<RefundDto[]> =>
  invoke<RefundDto[]>('list_refunds_scoped', { sessionToken, saleId });

// ── Dashboard & Reports ───────────────────────────────────────────

/** A single row in the daily sales summary export. */
export interface DailySummaryRow {
  sale_id: string;
  total_minor: number;
  currency: string;
  line_count: number;
  status: string;
  created_at: string;
}

/** A row in the sales-by-hour breakdown. */
export interface SalesByHourRow {
  hour: number;
  total_minor: number;
  sale_count: number;
}

/** A payment method breakdown with count and total. */
export interface PaymentBreakdown {
  method: string;
  count: number;
  total: number;
}

/** End-of-day report aggregating sales, voids, discounts, and hourly breakdown. */
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

/** Export the daily sales summary. */
export const exportDailySummary = (): Promise<DailySummaryRow[]> =>
  invoke<DailySummaryRow[]>('export_daily_summary');

/** ADR #7: Scoped daily summary report for the store resolved from a session token. */
export const exportDailySummaryScoped = (sessionToken: string): Promise<DailySummaryRow[]> =>
  invoke<DailySummaryRow[]>('export_daily_summary_scoped', { sessionToken });

/** Export the sales-by-hour breakdown. */
export const exportSalesByHour = (): Promise<SalesByHourRow[]> =>
  invoke<SalesByHourRow[]>('export_sales_by_hour');

/** ADR #7: Scoped sales-by-hour report for the store resolved from a session token. */
export const exportSalesByHourScoped = (sessionToken: string): Promise<SalesByHourRow[]> =>
  invoke<SalesByHourRow[]>('export_sales_by_hour_scoped', { sessionToken });

/** Export the end-of-day report. */
export const exportEodReport = (): Promise<EodReport> =>
  invoke<EodReport>('export_eod_report');

/** ADR #7: Scoped EOD report for the store resolved from a session token. */
export const exportEodReportScoped = (sessionToken: string): Promise<EodReport> =>
  invoke<EodReport>('export_eod_report_scoped', { sessionToken });

// ── Receipt Printing ──────────────────────────────────────────────

/** A line item for receipt printing. */
export interface LineItemDto {
  name: string;
  quantity: number;
  unitPrice: MoneyDto;
  totalPrice: MoneyDto;
  taxAmount?: MoneyDto;
}

/** A payment entry for receipt printing. */
export interface PaymentDto {
  method: string;
  amount: MoneyDto;
  change: MoneyDto | null;
}

/** Monetary value representation for receipt printing. */
export interface MoneyDto {
  minorUnits: number;
  currency: string;
}

/** Arguments for printing a sales receipt. */
export interface PrintSalesReceiptArgs {
  date: string;
  receiptNumber: string;
  items: LineItemDto[];
  subtotal: MoneyDto;
  tax?: MoneyDto;
  total: MoneyDto;
  payments: PaymentDto[];
  tableNumber?: string;
}

/** Result of a receipt print request. */
export interface PrintSalesReceiptResult {
  printed: boolean;
}

/** Print a formatted sales receipt. */
export const printSalesReceipt = (args: PrintSalesReceiptArgs): Promise<PrintSalesReceiptResult> =>
  invoke<PrintSalesReceiptResult>('print_sales_receipt', { args });

/** Subscribe to receipt-printed events from the backend. Returns an unsubscribe function. */
export const onReceiptPrinted = (handler: (lines: number) => void): Promise<UnlistenFn> =>
  listen<{ lines: number }>('receipt:printed', (e) => handler(e.payload.lines));
