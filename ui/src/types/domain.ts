// Domain types mirrored from `oz-core` (Rust).
//
// Newtypes use `& { readonly __brand }` so the TypeScript type system
// refuses to mix up `CartId` and `Sku`. Convert at the IPC boundary
// (`pos.ts` is the only place that talks to Rust).

export type CartId = string & { readonly __brand: 'CartId' };
export type LineId = string & { readonly __brand: 'LineId' };
export type Sku = string & { readonly __brand: 'Sku' };

/** Money in minor units, paired with a 3-letter ISO-4217 code. */
export interface Money {
  readonly minor_units: number;
  readonly currency: string;
}

/** Single line in a cart. */
export interface CartLine {
  readonly id: LineId;
  readonly sku: Sku;
  readonly qty: number;
  readonly unit_price: Money;
}

/** Mirrors `AppError` in `src-tauri/src/error.rs`. */
export type AppError =
  | { kind: 'core'; message: string }
  | { kind: 'hardware'; message: string }
  | { kind: 'invalid'; message: string }
  | { kind: 'internal'; message: string };

export const isAppError = (e: unknown): e is AppError =>
  typeof e === 'object' && e !== null && 'kind' in e && 'message' in e;

/** Format `Money` for display. Locales will be added once we have
 *  the real Fluent IDs. */
export const formatMoney = (m: Money, locale: string = 'en-US'): string => {
  const fmt = new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: m.currency,
  });
  // ISO-4217 minor-unit exponent: USD/EUR/IDR = 2, JPY/KRW = 0, KWD = 3.
  // This is duplicated from `oz_core::Currency::minor_unit_exponent`.
  const known: Record<string, number> = {
    JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
    KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
  };
  const exp = known[m.currency] ?? 2;
  const major = m.minor_units / 10 ** exp;
  return fmt.format(major);
};
