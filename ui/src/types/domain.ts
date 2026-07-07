// Domain types mirrored from `oz-core` (Rust).
//
// Newtypes use `& { readonly __brand }` so the TypeScript type system
// refuses to mix up `CartId` and `Sku`. Convert at the IPC boundary
// (`pos.ts` is the only place that talks to Rust).

export type CartId = string & { readonly __brand: 'CartId' };
export type LineId = string & { readonly __brand: 'LineId' };
export type Sku = string & { readonly __brand: 'Sku' };

import { getDecimalSep } from '@/utils/storage';

/** Money in minor units, paired with a 3-letter ISO-4217 code. */
export interface Money {
  readonly minor_units: number;
  readonly currency: string;
}

/** Single line in a cart. */
export interface CartLine {
  readonly id: LineId;
  readonly sku: Sku;
  /** Display name of the product (set at add time, may be absent from IPC). */
  readonly name?: string;
  /** Product category used to render a course chip on the cart line. */
  readonly category?: string;
  readonly qty: number;
  readonly unit_price: Money;
}

/**
 * A product that can be sold in the store.
 * Mirrors the product domain model from the backend.
 */
export interface Product {
  readonly sku: Sku;
  readonly name: string;
  readonly category: string;
  readonly price: Money;
  /** Barcode (EAN-13, UPC-A, etc.) if available. */
  readonly barcode: string | null;
  /** Whether the product is currently in stock. */
  readonly inStock: boolean;
  /** Current stock quantity, or null if tracking is disabled. */
  readonly stockQty: number | null;
  /** ISO-8601 creation timestamp. */
  readonly createdAt?: string;
  /** ISO-8601 timestamp of the last price change. */
  readonly priceUpdatedAt?: string;
}

/** Mirrors `AppError` in `apps/desktop-client/src/error.rs`. */
export type AppError =
  | { kind: 'core'; subKind: string; message: string }
  | { kind: 'hardware'; subKind: string; message: string }
  | { kind: 'invalid'; message: string }
  | { kind: 'internal'; message: string };

export const isAppError = (e: unknown): e is AppError =>
  typeof e === 'object' && e !== null && 'kind' in e;

/** Format `Money` for display. Defaults to Indonesian locale (id-ID).
 *  `decimalSep` overrides the per‑store receipt setting (read from
 *  localStorage when omitted). */
export const formatMoney = (
  m: Money,
  locale: string = 'id-ID',
  decimalSep?: 'dot' | 'comma' | 'none',
): string => {
  const sep = decimalSep ?? getDecimalSep();
  const hideDecimals = sep === 'none';
  const fmt = new Intl.NumberFormat(locale, {
    style: 'decimal',
    minimumFractionDigits: hideDecimals ? 0 : 2,
    maximumFractionDigits: hideDecimals ? 0 : 2,
  });
  // ISO-4217 minor-unit exponent: USD/EUR/IDR = 2, JPY/KRW = 0, KWD = 3.
  const known: Record<string, number> = {
    JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
    KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
  };
  const exp = known[m.currency] ?? 2;
  const major = m.minor_units / 10 ** exp;
  // USD → symbol ($), IDR → code (IDR)
  const prefix = m.currency === 'USD' ? '$' : m.currency;
  return `${prefix} ${fmt.format(major)}`;
};
