// ── revenueTotals helper tests (REP-02) ──────────────────────────
import { describe, it, expect } from 'vitest';
import { sumRevenueByCurrency } from '@/features/reports/revenueTotals';

describe('sumRevenueByCurrency', () => {
  it('sums rows within each currency and preserves first-seen order', () => {
    const totals = sumRevenueByCurrency([
      { total_minor: 10000, currency: 'USD' },
      { total_minor: 20000, currency: 'USD' },
      { total_minor: 500000, currency: 'IDR' },
    ]);
    expect(totals).toEqual([
      { currency: 'USD', total_minor: 30000 },
      { currency: 'IDR', total_minor: 500000 },
    ]);
  });

  it('returns a single total for a single-currency period', () => {
    const totals = sumRevenueByCurrency([
      { total_minor: 100, currency: 'USD' },
      { total_minor: 200, currency: 'USD' },
    ]);
    expect(totals).toEqual([{ currency: 'USD', total_minor: 300 }]);
  });

  it('returns an empty array for an empty period', () => {
    expect(sumRevenueByCurrency([])).toEqual([]);
  });
});
