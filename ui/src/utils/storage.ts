/**
 * localStorage keys used across the POS frontend.
 * Centralised so we never scatter magic strings.
 */
export const STORAGE_KEYS = {
  CART_WIDTH: 'pos-cart-width',
  LOCKED_CART: 'pos-locked-cart',
  LOCALE: 'oz-pos-locale',
  DECIMAL_SEP: 'oz-pos-decimal-sep',
} as const;

/** Decimal separator mode for monetary display. */
export type DecimalSep = 'dot' | 'comma' | 'none';

const VALID_DECIMAL_SEPS: readonly string[] = ['dot', 'comma', 'none'];

/** Read the persisted decimal separator, falling back to `'dot'`. */
export function getDecimalSep(): DecimalSep {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP);
    if (raw && VALID_DECIMAL_SEPS.includes(raw)) return raw as DecimalSep;
  } catch { /* localStorage unavailable */ }
  return 'dot';
}

/** Persist the decimal separator to localStorage. */
export function setDecimalSep(value: string): void {
  if (!VALID_DECIMAL_SEPS.includes(value)) return;
  try {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, value);
  } catch { /* quota exceeded — ignore */ }
}
