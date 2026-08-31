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
  rows: ReadonlyArray<{ total_minor: number; currency: string }> | null | undefined,
): CurrencyTotal[] {
  if (!rows || rows.length === 0) {
    return [];
  }
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
 * Sum gross profit and COGS per currency for revenue rows carrying HPP
 * figures (daily, weekly, and monthly). Rows without the fields contribute
 * zero — never summed across currencies.
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

/** One currency's refund and net-revenue totals for a report period. */
export interface NetRevenueTotal {
  currency: string;
  refund_minor: number;
  net_revenue_minor: number;
}

/**
 * Sum refunds and net revenue per currency (REP-04). Refunds are already
 * attributed to their own period by the backend; this only aggregates the
 * rows of the selected range. Rows without the fields contribute zero —
 * never summed across currencies. Net can be negative when a period
 * contains only refunds.
 */
export function sumNetRevenueByCurrency(
  rows: ReadonlyArray<{
    currency: string;
    refund_minor?: number;
    net_revenue_minor?: number;
  }>,
): NetRevenueTotal[] {
  const refunds = new Map<string, number>();
  const net = new Map<string, number>();
  for (const row of rows) {
    refunds.set(row.currency, (refunds.get(row.currency) ?? 0) + (row.refund_minor ?? 0));
    net.set(row.currency, (net.get(row.currency) ?? 0) + (row.net_revenue_minor ?? 0));
  }
  return Array.from(refunds, ([currency, refund_minor]) => ({
    currency,
    refund_minor,
    net_revenue_minor: net.get(currency) ?? 0,
  }));
}
