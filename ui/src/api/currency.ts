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

/** List all available currencies. */
export const listCurrencies = (): Promise<CurrencyDto[]> =>
  loggedInvoke<CurrencyDto[]>('list_currencies');

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

/** Format a fixed-point rate without exposing trailing binary-float noise. */
export function formatExchangeRate(rate: Pick<ExchangeRateDto, 'rate_millionths'>): string {
  const decimal = exchangeRateToDecimal(rate);
  if (!Number.isFinite(decimal)) return '—';
  return decimal.toFixed(6).replace(/0+$/, '').replace(/\.$/, '') || '0';
}

/** List all exchange rates. */
export const listExchangeRates = (): Promise<ExchangeRateDto[]> =>
  loggedInvoke<ExchangeRateDto[]>('list_exchange_rates');

/** ADR #7: List all exchange rates in the store resolved from a session token. */
export const listExchangeRatesScoped = (sessionToken: string): Promise<ExchangeRateDto[]> =>
  loggedInvoke<ExchangeRateDto[]>('list_exchange_rates_scoped', { sessionToken });

/** Create a new exchange rate. */
export const createExchangeRate = (args: CreateExchangeRateArgs): Promise<ExchangeRateDto> =>
  loggedInvoke<ExchangeRateDto>('create_exchange_rate', { args });

/** ADR #7: Create an exchange rate in the store resolved from a session token. */
export const createExchangeRateScoped = (
  sessionToken: string,
  args: CreateExchangeRateArgs,
): Promise<ExchangeRateDto> =>
  loggedInvoke<ExchangeRateDto>('create_exchange_rate_scoped', { sessionToken, args });

/** Delete an exchange rate by its identifier. */
export const deleteExchangeRate = (id: string): Promise<void> =>
  loggedInvoke<void>('delete_exchange_rate', { id });

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
