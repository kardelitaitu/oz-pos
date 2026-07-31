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

/** Reference counts for a tax rate (TAX-03) — used by the delete confirmation UI. */
export interface TaxRateDependencyCounts {
  products: number;
  categories: number;
  sale_lines: number;
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

/** List all tax rates for the store resolved from a session token. ADR #7. */
export const listTaxRatesScoped = (sessionToken: string): Promise<TaxRateDto[]> =>
  loggedInvoke<TaxRateDto[]>('list_tax_rates_scoped', { sessionToken });

/** Create a tax rate in the store resolved from a session token. ADR #7. */
export const createTaxRateScoped = (
  sessionToken: string,
  args: CreateTaxRateArgs,
): Promise<TaxRateDto> =>
  loggedInvoke<TaxRateDto>('create_tax_rate_scoped', { sessionToken, args });

/** Update a tax rate in the store resolved from a session token. ADR #7. */
export const updateTaxRateScoped = (
  sessionToken: string,
  args: UpdateTaxRateArgs,
): Promise<TaxRateDto> =>
  loggedInvoke<TaxRateDto>('update_tax_rate_scoped', { sessionToken, args });

/** Delete a tax rate in the store resolved from a session token. ADR #7. */
export const deleteTaxRateScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke('delete_tax_rate_scoped', { sessionToken, id });

/** Get reference counts for a tax rate in the session store (TAX-03). */
export const getTaxRateDependencyCountsScoped = (
  sessionToken: string,
  id: string,
): Promise<TaxRateDependencyCounts> =>
  loggedInvoke<TaxRateDependencyCounts>('get_tax_rate_dependency_counts_scoped', {
    sessionToken,
    id,
  });

/** List category-to-tax-rate assignments for a session store. ADR #7. */
export const listCategoryTaxRatesScoped = (sessionToken: string): Promise<CategoryTaxRateRow[]> =>
  loggedInvoke<CategoryTaxRateRow[]>('list_category_tax_rates_scoped', { sessionToken });

/** Set category tax rates in the store resolved from a session token. ADR #7. */
export const setCategoryTaxRatesScoped = (
  sessionToken: string,
  args: SetCategoryTaxRatesArgs,
): Promise<void> =>
  loggedInvoke<void>('set_category_tax_rates_scoped', {
    sessionToken,
    args: {
      category_id: args.categoryId,
      tax_rate_ids: args.taxRateIds,
    },
  });
