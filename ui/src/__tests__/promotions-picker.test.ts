// Unit tests for the POS promotions picker eligibility filter (PROMO-5).
import { describe, expect, it } from 'vitest';
import { evaluatePromotionEligibility } from '../features/sales/promotionEligibility';
import type { Promotion } from '../api/promotions';

function promo(overrides: Partial<Promotion>): Promotion {
  return {
    id: 'p1',
    name: 'Test Promo',
    description: '',
    promo_type: 'percentage',
    value_minor: 10,
    min_qty: null,
    trigger_sku: null,
    reward_sku: null,
    reward_qty: null,
    starts_at: null,
    ends_at: null,
    min_order_minor: 0,
    category_id: null,
    active: true,
    created_at: '2026-01-01T00:00:00.000Z',
    updated_at: '2026-01-01T00:00:00.000Z',
    ...overrides,
  };
}

describe('evaluatePromotionEligibility', () => {
  it('marks an in-scope percentage promotion eligible', () => {
    const result = evaluatePromotionEligibility([promo({})], 50_000);
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({ kind: 'eligible' });
  });

  it('marks fixed_amount and buy_x_get_y as not applicable in this checkout', () => {
    const result = evaluatePromotionEligibility(
      [promo({ promo_type: 'fixed_amount' }), promo({ promo_type: 'buy_x_get_y' })],
      50_000,
    );
    expect(result.map((r) => r.kind)).toEqual(['not-applicable-type', 'not-applicable-type']);
  });

  it('rejects percentage promotions whose minimum order exceeds the subtotal', () => {
    const result = evaluatePromotionEligibility([promo({ min_order_minor: 100_000 })], 50_000);
    expect(result[0]).toMatchObject({ kind: 'below-min-order' });
  });

  it('allows a percentage promotion exactly at its minimum order', () => {
    const result = evaluatePromotionEligibility([promo({ min_order_minor: 50_000 })], 50_000);
    expect(result[0]).toMatchObject({ kind: 'eligible' });
  });

  it('guards against out-of-range percentage values (PROMO-1 backstop)', () => {
    const result = evaluatePromotionEligibility(
      [promo({ value_minor: 0 }), promo({ value_minor: 150 })],
      50_000,
    );
    expect(result.map((r) => r.kind)).toEqual(['not-applicable-type', 'not-applicable-type']);
  });
});
