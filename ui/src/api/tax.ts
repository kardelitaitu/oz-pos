// ── Tax Rates ──────────────────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface TaxRateDto {
  id: string;
  name: string;
  rate_bps: number;
  is_default: boolean;
  is_inclusive: boolean;
  display_rate: string;
  created_at: string;
  updated_at: string;
}

export interface CreateTaxRateArgs {
  name: string;
  rateBps: number;
  isDefault: boolean;
  isInclusive: boolean;
}

export interface UpdateTaxRateArgs {
  id: string;
  name: string;
  rateBps: number;
  isDefault: boolean;
  isInclusive: boolean;
}

export interface CategoryTaxRateRow {
  category_id: string;
  tax_rate_ids: string[];
}

export interface SetCategoryTaxRatesArgs {
  categoryId: string;
  taxRateIds: string[];
}

export const listTaxRates = (): Promise<TaxRateDto[]> =>
  invoke<TaxRateDto[]>('list_tax_rates');

export const createTaxRate = (args: CreateTaxRateArgs): Promise<TaxRateDto> =>
  invoke<TaxRateDto>('create_tax_rate', { args });

export const updateTaxRate = (args: UpdateTaxRateArgs): Promise<TaxRateDto> =>
  invoke<TaxRateDto>('update_tax_rate', { args });

export const deleteTaxRate = (id: string): Promise<void> =>
  invoke('delete_tax_rate', { id });

export const listCategoryTaxRates = (): Promise<CategoryTaxRateRow[]> =>
  invoke<CategoryTaxRateRow[]>('list_category_tax_rates');

export const setCategoryTaxRates = (args: SetCategoryTaxRatesArgs): Promise<void> =>
  invoke<void>('set_category_tax_rates', {
    args: {
      category_id: args.categoryId,
      tax_rate_ids: args.taxRateIds,
    },
  });
