// Domain types mirrored from `oz-core` (Rust).
//
// Newtypes use `& { readonly __brand }` so the TypeScript type system
// refuses to mix up `CartId` and `Sku`. Convert at the IPC boundary
// (`pos.ts` is the only place that talks to Rust).

/** Branded string type for cart identifiers. */
export type CartId = string & { readonly __brand: 'CartId' };
/** Branded string type for cart line identifiers. */
export type LineId = string & { readonly __brand: 'LineId' };
/** Branded string type for stock-keeping unit codes. */
export type Sku = string & { readonly __brand: 'Sku' };

import { getDecimalSep } from '@/utils/storage';

/** Money in minor units, paired with a 3-letter ISO-4217 code. */
export interface Money {
  readonly minor_units: number;
  readonly currency: string;
}

/** Course identifier for restaurant order coursing. */
export type CourseId = 'appetizer' | 'main' | 'dessert' | 'drinks';

/** Coursing status — items on hold wait to be fired to the kitchen. */
export type CoursingStatus = 'hold' | 'fired';

/** All defined course types with their display labels. */
export const COURSES: { id: CourseId; label: string; emoji: string }[] = [
  { id: 'appetizer', label: 'Appetizer', emoji: '🥗' },
  { id: 'main', label: 'Main Course', emoji: '🍽️' },
  { id: 'dessert', label: 'Dessert', emoji: '🍰' },
  { id: 'drinks', label: 'Drinks', emoji: '🥤' },
];

/** Label for a given course ID. */
export function courseLabel(courseId: CourseId): string {
  return COURSES.find((c) => c.id === courseId)?.label ?? courseId;
}

/** Emoji for a given course ID. */
export function courseEmoji(courseId: CourseId): string {
  return COURSES.find((c) => c.id === courseId)?.emoji ?? '🍽️';
}

/** A single line in a shopping cart. */
export interface CartLine {
  readonly id: LineId;
  readonly sku: Sku;
  /** Display name of the product (set at add time, may be absent from IPC). */
  readonly name?: string;
  /** Product category used to render a course chip on the cart line. */
  readonly category?: string;
  readonly qty: number;
  readonly unit_price: Money;
  /** Course assignment for restaurant coursing (undefined = not applicable). */
  readonly courseId?: CourseId;
  /** Coursing status — hold (not yet sent to kitchen) or fired. */
  readonly coursingStatus?: CoursingStatus;
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
  /** Product type: "retail" | "restaurant" | "both" | "service". */
  readonly productType: 'retail' | 'restaurant' | 'both' | 'service';
}

/** Mirrors `AppError` in `apps/desktop-client/src/error.rs`. */
export type AppError =
  | { kind: 'core'; subKind: string; message: string }
  | { kind: 'hardware'; subKind: string; message: string }
  | { kind: 'invalid'; message: string }
  | { kind: 'internal'; message: string };

/** Type guard that checks whether an unknown value is an AppError. */
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
