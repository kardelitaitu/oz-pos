// The ONLY place in the UI that calls `invoke()`. Components consume
// hooks that call these wrappers; never import `@tauri-apps/api/core`
// directly from a component.

import { invoke } from '@tauri-apps/api/core';
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
