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

// ── Exchange Rates ────────────────────────────────────────────────

/** An exchange rate between two currencies. */
export interface ExchangeRateDto {
  id: string;
  from_currency: string;
  to_currency: string;
  rate: number;
  source: string;
  effective_date: string;
  created_at: string;
}

/** Arguments for creating a new exchange rate. */
export interface CreateExchangeRateArgs {
  fromCurrency: string;
  toCurrency: string;
  rate: number;
  source?: string;
  effectiveDate?: string;
}

/** List all exchange rates. */
export const listExchangeRates = (): Promise<ExchangeRateDto[]> =>
  loggedInvoke<ExchangeRateDto[]>('list_exchange_rates');

/** Create a new exchange rate. */
export const createExchangeRate = (args: CreateExchangeRateArgs): Promise<ExchangeRateDto> =>
  loggedInvoke<ExchangeRateDto>('create_exchange_rate', { args });

/** Delete an exchange rate by its identifier. */
export const deleteExchangeRate = (id: string): Promise<void> =>
  loggedInvoke<void>('delete_exchange_rate', { id });
