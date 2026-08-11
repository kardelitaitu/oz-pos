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
