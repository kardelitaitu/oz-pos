/** One currency's summed revenue for a report period. */
export interface CurrencyTotal {
  currency: string;
  total_minor: number;
}

/**
 * Sum revenue rows per currency, preserving first-seen order. REP-02: the
 * backend groups by currency, so a multi-currency period arrives as multiple
 * rows — minor units are NEVER summed across currencies. A period that spans
 * USD and IDR must render two totals, not one collapsed number formatted as
 * the first row's currency.
 */
export function sumRevenueByCurrency(
  rows: ReadonlyArray<{ total_minor: number; currency: string }>,
): CurrencyTotal[] {
  const totals = new Map<string, number>();
  for (const row of rows) {
    totals.set(row.currency, (totals.get(row.currency) ?? 0) + row.total_minor);
  }
  return Array.from(totals, ([currency, total_minor]) => ({ currency, total_minor }));
}

/** One currency's summed gross profit and COGS for a report period. */
export interface GrossProfitTotal {
  currency: string;
  gross_profit_minor: number;
  cogs_minor: number;
}

/**
 * Sum gross profit and COGS per currency for daily revenue rows (the only
 * granularity that carries HPP figures). Rows without the fields (weekly /
 * monthly) contribute zero — never summed across currencies.
 */
export function sumGrossProfitByCurrency(
  rows: ReadonlyArray<{
    currency: string;
    gross_profit_minor?: number;
    cogs_minor?: number;
  }>,
): GrossProfitTotal[] {
  const profit = new Map<string, number>();
  const cogs = new Map<string, number>();
  for (const row of rows) {
    profit.set(row.currency, (profit.get(row.currency) ?? 0) + (row.gross_profit_minor ?? 0));
    cogs.set(row.currency, (cogs.get(row.currency) ?? 0) + (row.cogs_minor ?? 0));
  }
  return Array.from(profit, ([currency, gross_profit_minor]) => ({
    currency,
    gross_profit_minor,
    cogs_minor: cogs.get(currency) ?? 0,
  }));
}
