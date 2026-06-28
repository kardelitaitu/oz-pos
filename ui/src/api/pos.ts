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

export const completeSale = (cartId: CartId): Promise<CompleteSaleResult> =>
  invoke<CompleteSaleResult>('complete_sale', { args: { cartId } });

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
  storeName: string;
  storeAddress: string;
  storeTaxId?: string;
  date: string;
  receiptNumber: string;
  items: LineItemDto[];
  subtotal: MoneyDto;
  tax?: MoneyDto;
  total: MoneyDto;
  payments: PaymentDto[];
  footer?: string;
  /** "narrow" (58mm) or "standard" (80mm). Defaults to standard. */
  paperWidth?: string;
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

// ── Currencies ─────────────────────────────────────────────────────

export interface CurrencyInfo {
  code: string;
  exponent: number;
}

export const getCurrencyInfo = (code: string): Promise<CurrencyInfo> =>
  invoke<CurrencyInfo>('currency_info', { code });
