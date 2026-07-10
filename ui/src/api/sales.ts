// ── Sales: POS cart, history, void, discounts, held carts, refunds, dashboard, printing ──

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CartId, LineId, Money } from '@/types/domain';

// ── Cart operations ────────────────────────────────────────────────

export interface StartSaleArgs {
  currency: string;
}

export interface StartSaleResult {
  cartId: CartId;
}

export interface AddLineArgs {
  cartId: CartId;
  sku: string;
  qty: number;
  unitPriceMinor: number;
}

export interface AddLineResult {
  lineId: LineId;
  lineTotal: Money | null;
}

export interface SerialNumberArg {
  sku: string;
  serial: string;
}

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

export interface CompleteSaleResult {
  saleId: string;
  total: Money | null;
  lineCount: number;
}

export interface SetCartDiscountArgs {
  cartId: string;
  percent: number;
  label?: string;
  userId: string;
}

export const startSale = (args: StartSaleArgs): Promise<StartSaleResult> =>
  invoke<StartSaleResult>('start_sale', { args });

export const addLine = (args: AddLineArgs): Promise<AddLineResult> =>
  invoke<AddLineResult>('add_line', { args });

export const completeSale = (args: CompleteSaleArgs): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale', { args });

export const getProductTrackSerial = (sku: string): Promise<boolean> =>
  invoke<boolean>('get_product_track_serial', { sku });

export const setCartDiscount = (args: SetCartDiscountArgs): Promise<void> =>
  invoke<void>('set_cart_discount', { args });

export interface OverrideLinePriceArgs {
  cartId: string;
  lineId: string;
  newPriceMinor: number;
  userId: string;
}

export const overrideLinePrice = (args: OverrideLinePriceArgs): Promise<void> =>
  invoke<void>('override_line_price', { args });

// ── Sales History ─────────────────────────────────────────────────

export interface SaleListItem {
  id: string;
  total: Money;
  lineCount: number;
  status: string;
  paymentMethod: string | null;
  userId: string | null;
  createdAt: string;
}

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

export const listSales = (): Promise<SaleListItem[]> =>
  invoke<SaleListItem[]>('list_sales');

/** ADR #7: List sales scoped to the store resolved from a session token. */
export const listSalesScoped = (sessionToken: string): Promise<SaleListItem[]> =>
  invoke<SaleListItem[]>('list_sales_scoped', { sessionToken });

export const getSale = (id: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('get_sale', { id });

// ── Void Sale ─────────────────────────────────────────────────────

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

export const voidSale = (args: VoidSaleArgs): Promise<VoidSaleResult> =>
  invoke<VoidSaleResult>('void_sale', { args });

// ── Hold Order ────────────────────────────────────────────────────

export interface HoldCartArgs {
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
  bill_type?: string;
  customer_name?: string;
}

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

export const holdCart = (args: HoldCartArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('hold_cart', { args });

export const listHeldCarts = (): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_held_carts');

export const listOpenBills = (): Promise<HeldCartRow[]> =>
  invoke<HeldCartRow[]>('list_open_bills');

export const getHeldCart = (id: string): Promise<HeldCartFull | null> =>
  invoke<HeldCartFull | null>('get_held_cart', { id });

export const deleteHeldCart = (id: string): Promise<void> =>
  invoke('delete_held_cart', { id });

// ── Refunds ───────────────────────────────────────────────────────

export interface RefundLineArg {
  saleLineId: string;
  sku: string;
  qty: number;
  unitPriceMinor: number;
  currency: string;
  lineTotalMinor: number;
}

export interface ProcessRefundArgs {
  saleId: string;
  reason: string;
  note?: string | null;
  userId: string;
  lines: RefundLineArg[];
}

export interface ProcessRefundResult {
  refundId: string;
  totalMinor: number;
}

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

export interface RefundLineDto {
  id: string;
  refundId: string;
  saleLineId: string;
  sku: string;
  qty: number;
  unitPrice: Money;
  lineTotal: Money;
}

export const lookupSaleByReceiptBarcode = (barcode: string): Promise<SaleDetail | null> =>
  invoke<SaleDetail | null>('lookup_sale_by_receipt_barcode', { barcode });

export const processRefund = (args: ProcessRefundArgs): Promise<ProcessRefundResult> =>
  invoke<ProcessRefundResult>('process_refund', { args });

export const listRefunds = (saleId: string): Promise<RefundDto[]> =>
  invoke<RefundDto[]>('list_refunds', { saleId });

// ── Dashboard & Reports ───────────────────────────────────────────

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

export interface PaymentBreakdown {
  method: string;
  count: number;
  total: number;
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

export const exportDailySummary = (): Promise<DailySummaryRow[]> =>
  invoke<DailySummaryRow[]>('export_daily_summary');

export const exportSalesByHour = (): Promise<SalesByHourRow[]> =>
  invoke<SalesByHourRow[]>('export_sales_by_hour');

export const exportEodReport = (): Promise<EodReport> =>
  invoke<EodReport>('export_eod_report');

// ── Receipt Printing ──────────────────────────────────────────────

export interface LineItemDto {
  name: string;
  quantity: number;
  unitPrice: MoneyDto;
  totalPrice: MoneyDto;
  taxAmount?: MoneyDto;
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
  tableNumber?: string;
}

export interface PrintSalesReceiptResult {
  printed: boolean;
}

export const printSalesReceipt = (args: PrintSalesReceiptArgs): Promise<PrintSalesReceiptResult> =>
  invoke<PrintSalesReceiptResult>('print_sales_receipt', { args });

export const onReceiptPrinted = (handler: (lines: number) => void): Promise<UnlistenFn> =>
  listen<{ lines: number }>('receipt:printed', (e) => handler(e.payload.lines));
