// ── Currency: Exchange rates, currency list ───────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface CurrencyInfo {
  code: string;
  exponent: number;
}

export interface CurrencyDto {
  code: string;
  name: string;
  minor_exponent: number;
  symbol: string;
}

export interface SetDefaultCurrencyArgs {
  code: string;
}

export const getCurrencyInfo = (code: string): Promise<CurrencyInfo> =>
  invoke<CurrencyInfo>('currency_info', { code });

export const listCurrencies = (): Promise<CurrencyDto[]> =>
  invoke<CurrencyDto[]>('list_currencies');

export const getDefaultCurrency = (): Promise<string | null> =>
  invoke<string | null>('get_default_currency');

export const setDefaultCurrency = (args: SetDefaultCurrencyArgs): Promise<void> =>
  invoke<void>('set_default_currency', { args });

// ── Exchange Rates ────────────────────────────────────────────────

export interface ExchangeRateDto {
  id: string;
  from_currency: string;
  to_currency: string;
  rate: number;
  source: string;
  effective_date: string;
  created_at: string;
}

export interface CreateExchangeRateArgs {
  fromCurrency: string;
  toCurrency: string;
  rate: number;
  source?: string;
  effectiveDate?: string;
}

export const listExchangeRates = (): Promise<ExchangeRateDto[]> =>
  invoke<ExchangeRateDto[]>('list_exchange_rates');

export const createExchangeRate = (args: CreateExchangeRateArgs): Promise<ExchangeRateDto> =>
  invoke<ExchangeRateDto>('create_exchange_rate', { args });

export const deleteExchangeRate = (id: string): Promise<void> =>
  invoke<void>('delete_exchange_rate', { id });
