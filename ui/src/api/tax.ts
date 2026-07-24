// ── Tax Rates ──────────────────────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** A tax rate definition in basis points. */
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

/** Arguments for creating a new tax rate. */
export interface CreateTaxRateArgs {
  name: string;
  rateBps: number;
  isDefault: boolean;
  isInclusive: boolean;
}

/** Arguments for updating an existing tax rate. */
export interface UpdateTaxRateArgs {
  id: string;
  name: string;
  rateBps: number;
  isDefault: boolean;
  isInclusive: boolean;
}

/** A product category and its assigned tax rate identifiers. */
export interface CategoryTaxRateRow {
  category_id: string;
  tax_rate_ids: string[];
}

/** Arguments for setting tax rates on a product category. */
export interface SetCategoryTaxRatesArgs {
  categoryId: string;
  taxRateIds: string[];
}

/** A cart line for computing tax in a live preview. */
export interface CartLineTaxInput {
  sku: string;
  qty: number;
  unit_price_minor: number;
}

/** Compute total tax for a set of cart lines (live preview) using the scoped variant (ADR #7). */
export const computeCartTax = (
  sessionToken: string | null,
  lines: CartLineTaxInput[],
  currency: string,
): Promise<number> =>
  sessionToken
    ? loggedInvoke<number>('compute_cart_tax_scoped', { sessionToken, lines, currency })
    : Promise.resolve(0);

/** List all tax rates. */
export const listTaxRates = (): Promise<TaxRateDto[]> =>
  loggedInvoke<TaxRateDto[]>('list_tax_rates');

/** Create a new tax rate. */
export const createTaxRate = (args: CreateTaxRateArgs): Promise<TaxRateDto> =>
  loggedInvoke<TaxRateDto>('create_tax_rate', { args });

/** Update an existing tax rate. */
export const updateTaxRate = (args: UpdateTaxRateArgs): Promise<TaxRateDto> =>
  loggedInvoke<TaxRateDto>('update_tax_rate', { args });

/** Delete a tax rate by its identifier. */
export const deleteTaxRate = (id: string): Promise<void> =>
  loggedInvoke('delete_tax_rate', { id });

/** List all category-to-tax-rate assignments. */
export const listCategoryTaxRates = (): Promise<CategoryTaxRateRow[]> =>
  loggedInvoke<CategoryTaxRateRow[]>('list_category_tax_rates');

/** Set the tax rates assigned to a product category. */
export const setCategoryTaxRates = (args: SetCategoryTaxRatesArgs): Promise<void> =>
  loggedInvoke<void>('set_category_tax_rates', {
    args: {
      category_id: args.categoryId,
      tax_rate_ids: args.taxRateIds,
    },
  });
