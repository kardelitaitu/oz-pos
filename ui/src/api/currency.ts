// ── Currency: Exchange rates, currency list ───────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** Basic currency info including its minor unit exponent. */
export interface CurrencyInfo {
  code: string;
  exponent: number;
}

/** A full currency definition. */
export interface CurrencyDto {
  code: string;
  name: string;
  minor_exponent: number;
  symbol: string;
}

/** Arguments for setting the default currency. */
export interface SetDefaultCurrencyArgs {
  code: string;
}

/** Get currency info (code and exponent) for a given currency code. */
export const getCurrencyInfo = (code: string): Promise<CurrencyInfo> =>
  loggedInvoke<CurrencyInfo>('currency_info', { code });

/** List all available currencies resolved from a session token. ADR #7. */
export const listCurrenciesScoped = (sessionToken: string): Promise<CurrencyDto[]> =>
  loggedInvoke<CurrencyDto[]>('list_currencies_scoped', { sessionToken });

/** Get the store's default currency code. */
export const getDefaultCurrency = (): Promise<string | null> =>
  loggedInvoke<string | null>('get_default_currency');

/** Set the store's default currency. */
export const setDefaultCurrency = (args: SetDefaultCurrencyArgs): Promise<void> =>
  loggedInvoke<void>('set_default_currency', { args });

/** ADR #7: Get the default currency in the store resolved from a session token. */
export const getDefaultCurrencyScoped = (sessionToken: string): Promise<string | null> =>
  loggedInvoke<string | null>('get_default_currency_scoped', { sessionToken });

/** ADR #7: Set the default currency in the store resolved from a session token. */
export const setDefaultCurrencyScoped = (
  sessionToken: string,
  args: SetDefaultCurrencyArgs,
): Promise<void> =>
  loggedInvoke<void>('set_default_currency_scoped', { sessionToken, args });

// ── Exchange Rates ────────────────────────────────────────────────

/** An exchange rate between two currencies. */
export interface ExchangeRateDto {
  id: string;
  from_currency: string;
  to_currency: string;
  /** Fixed-point rate in millionths: `rate_millionths / 1_000_000`. */
  rate_millionths: number;
  source: string;
  effective_date: string;
  created_at: string;
}

/** Arguments for creating a new exchange rate.
 *
 * Field names intentionally match the Rust DTO exactly. The value is
 * fixed-point millionths rather than a floating-point decimal.
 */
export interface CreateExchangeRateArgs {
  from_currency: string;
  to_currency: string;
  rate_millionths: number;
  source?: string;
  effective_date?: string;
}

/** Convert a validated fixed-point rate to a display/calculation decimal. */
export function exchangeRateToDecimal(rate: Pick<ExchangeRateDto, 'rate_millionths'>): number {
  return rate.rate_millionths / 1_000_000;
}

/** Arguments for {@link convertMinorUnits}. */
export interface ConvertMinorUnitsArgs {
  /** Amount in base-currency minor units (safe integer). */
  baseMinor: number;
  /** ISO-4217 exponent of the base currency. */
  baseExponent: number;
  /** Stored fixed-point rate at 6-decimal scale, strictly positive. */
  rateMillionths: number;
  /** ISO-4217 exponent of the charge currency. */
  chargeExponent: number;
  /**
   * When true the stored rate is `charge → base` and the conversion uses
   * its reciprocal (the PaymentModal's inverse-pair fallback). When false
   * the stored rate is `base → charge` directly.
   */
  inverse?: boolean;
}

/**
 * Convert minor units across currencies with EXACT decimal arithmetic
 * (MONEY-01, 2026-08-31).
 *
 * The previous PaymentModal path divided to major units, multiplied by a
 * binary-float rate, and scaled back — every product landing exactly on
 * the .5 minor-unit boundary mis-rounded (0.03 USD @ 149.5 → 4.485 →
 * float 448.49999… → 448 instead of 449). This helper keeps the whole
 * computation in BigInt: `chargeMinor = baseMinor × rate × 10^chargeExp
 * / (10^baseExp × 10^6)` (or the reciprocal when `inverse`), rounded
 * half-up toward +Infinity — the same tie rule `Math.round` applies to
 * positives, now applied to the EXACT value rather than its float
 * approximation. Results beyond 2^53 lose precision on the final
 * `Number()` conversion; POS amounts never approach that range.
 */
export function convertMinorUnits({
  baseMinor,
  baseExponent,
  rateMillionths,
  chargeExponent,
  inverse = false,
}: ConvertMinorUnitsArgs): number {
  if (!Number.isInteger(rateMillionths) || rateMillionths <= 0) {
    throw new RangeError(`convertMinorUnits: rateMillionths must be a positive integer, got ${rateMillionths}`);
  }
  const bm = BigInt(baseMinor);
  const rm = BigInt(rateMillionths);
  const scaleBase = 10n ** BigInt(baseExponent);
  const scaleCharge = 10n ** BigInt(chargeExponent);
  let num: bigint;
  let den: bigint;
  if (inverse) {
    // charge = base / (rm/1e6) = base × 1e6 / rm
    num = bm * 1_000_000n * scaleCharge;
    den = rm * scaleBase;
  } else {
    num = bm * rm * scaleCharge;
    den = scaleBase * 1_000_000n;
  }
  // Half-up toward +Infinity: floor-divide, then bump when the remainder
  // is at least half the denominator. `den` is always positive here.
  let q = num / den;
  let r = num % den;
  if (r < 0n) {
    q -= 1n;
    r += den;
  }
  if (2n * r >= den) {
    q += 1n;
  }
  return Number(q);
}

/**
 * Exact reciprocal of a fixed-point rate, re-encoded at the same
 * 6-decimal scale (MONEY-01). `round_half_up(1e12 / rateMillionths)` —
 * replaces the float round-trip `Math.round((1 / (rm/1e6)) * 1e6)` when
 * an inverse-pair rate must be persisted as `tender_rate_millionths`.
 */
export function reciprocalMillionths(rateMillionths: number): number {
  if (!Number.isInteger(rateMillionths) || rateMillionths <= 0) {
    throw new RangeError(`reciprocalMillionths: must be a positive integer, got ${rateMillionths}`);
  }
  const num = 1_000_000_000_000n; // 1e6 × 1e6
  const den = BigInt(rateMillionths);
  let q = num / den;
  const r = num % den;
  if (2n * r >= den) {
    q += 1n;
  }
  return Number(q);
}

/** Format a fixed-point rate without exposing trailing binary-float noise. */
export function formatExchangeRate(rate: Pick<ExchangeRateDto, 'rate_millionths'>): string {
  const decimal = exchangeRateToDecimal(rate);
  if (!Number.isFinite(decimal)) return '—';
  return decimal.toFixed(6).replace(/0+$/, '').replace(/\.$/, '') || '0';
}

/** List all exchange rates in the store resolved from a session token. ADR #7. */
export const listExchangeRatesScoped = (sessionToken: string): Promise<ExchangeRateDto[]> =>
  loggedInvoke<ExchangeRateDto[]>('list_exchange_rates_scoped', { sessionToken });

/**
 * The CURRENT rate for every pair (CUR-11), bounded — one row per
 * (from, to) instead of the full history. For overview/converter
 * consumers like the PaymentModal currency picker; the rate-history
 * editor keeps `listExchangeRatesScoped`.
 */
export const listLatestExchangeRatesScoped = (sessionToken: string): Promise<ExchangeRateDto[]> =>
  loggedInvoke<ExchangeRateDto[]>('list_latest_exchange_rates_scoped', { sessionToken });

/** ADR #7: Create an exchange rate in the store resolved from a session token. */
export const createExchangeRateScoped = (
  sessionToken: string,
  args: CreateExchangeRateArgs,
): Promise<ExchangeRateDto> =>
  loggedInvoke<ExchangeRateDto>('create_exchange_rate_scoped', { sessionToken, args });

/** ADR #7: Delete an exchange rate in the store resolved from a session token. */
export const deleteExchangeRateScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke<void>('delete_exchange_rate_scoped', { sessionToken, id });

/** Arguments for the latest-rate query (CUR-04). */
export interface GetLatestExchangeRateArgs {
  fromCurrency: string;
  toCurrency: string;
  effectiveDate?: string;
}

/** ADR #7: Return the latest exchange rate for a pair effective on/before the date.
 *
 *  CUR-04: checkout must use this instead of `find()`-ing the full history
 *  list, so a rate is selected by effective date rather than list order. */
export const getLatestExchangeRateScoped = (
  sessionToken: string,
  args: GetLatestExchangeRateArgs,
): Promise<ExchangeRateDto | null> =>
  loggedInvoke<ExchangeRateDto | null>('get_latest_exchange_rate_scoped', {
    sessionToken,
    fromCurrency: args.fromCurrency,
    toCurrency: args.toCurrency,
    ...(args.effectiveDate ? { effectiveDate: args.effectiveDate } : {}),
  });
