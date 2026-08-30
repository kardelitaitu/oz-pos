// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { pricingFor, featureRowsFor } from '../../content/pricing';
import { pricing as enPricing } from '../../content/pricing/en';
import { pricing as idPricing } from '../../content/pricing/id';
import type { TierKey, BillingPeriod } from '../../content/pricing/types';

/**
 * Pricing-content invariants. These pin the data contract the pricing page
 * and the account dashboard subscribe section both consume — if the content
 * drifts (a tier dropped, a price id missing, an IDR tier without its USD
 * sibling), the checkout flow breaks silently. Property-style: assert the
 * invariant over the whole set, not one example.
 */

const LOCALES = ['en', 'id'] as const;
const PERIODS: BillingPeriod[] = ['monthly', 'yearly'];
const PAID_TIERS: TierKey[] = ['plus', 'pro', 'premium'];

describe('pricingFor selector', () => {
  it('maps en → USD content and id → IDR content', () => {
    expect(pricingFor('en')).toBe(enPricing);
    expect(pricingFor('id')).toBe(idPricing);
    expect(enPricing).not.toBe(idPricing);
  });

  it('returns the same tier lineup in both locales', () => {
    const enKeys = enPricing.map((t) => t.tierKey);
    const idKeys = idPricing.map((t) => t.tierKey);
    expect(idKeys).toEqual(enKeys);
    // The documented lineup: Free · Plus · Pro · Premium · Enterprise.
    expect(enKeys).toEqual(['free', 'plus', 'pro', 'premium', 'enterprise']);
  });
});

describe('tier shape invariants', () => {
  it('every tier has a currency matching its locale', () => {
    for (const t of enPricing) expect(t.currency).toBe('USD');
    for (const t of idPricing) expect(t.currency).toBe('IDR');
  });

  it('every tier has both billing periods with a display price', () => {
    for (const locale of LOCALES) {
      const pricing = locale === 'en' ? enPricing : idPricing;
      for (const tier of pricing) {
        for (const period of PERIODS) {
          expect(tier.prices[period], `${locale} ${tier.tierKey} ${period}`).toBeDefined();
          expect(tier.prices[period].price).toBeTruthy();
        }
      }
    }
  });

  it('the free tier has no price id (no checkout)', () => {
    for (const locale of LOCALES) {
      const free = (locale === 'en' ? enPricing : idPricing).find((t) => t.tierKey === 'free');
      expect(free?.prices.monthly.priceId).toBeUndefined();
      expect(free?.prices.yearly.priceId).toBeUndefined();
    }
  });

  it('every paid tier has a price id for the yearly period (checkout requires it)', () => {
    for (const locale of LOCALES) {
      const pricing = locale === 'en' ? enPricing : idPricing;
      for (const tier of pricing) {
        if (PAID_TIERS.includes(tier.tierKey)) {
          expect(tier.prices.yearly.priceId, `${locale} ${tier.tierKey} yearly`).toBeTruthy();
        }
      }
    }
  });

  it('enterprise has no price id (contact-sales only)', () => {
    for (const locale of LOCALES) {
      const ent = (locale === 'en' ? enPricing : idPricing).find((t) => t.tierKey === 'enterprise');
      expect(ent?.prices.yearly.priceId).toBeUndefined();
      expect(ent?.prices.monthly.priceId).toBeUndefined();
    }
  });

  it('exactly one tier is highlighted (Most Popular)', () => {
    for (const locale of LOCALES) {
      const highlighted = (locale === 'en' ? enPricing : idPricing).filter((t) => t.highlight);
      expect(highlighted).toHaveLength(1);
      expect(highlighted[0].tierKey).toBe('pro');
    }
  });
});

describe('locale parity invariants', () => {
  it('paid tiers carry the same price id suffix shape in both locales (both bill Paddle USD)', () => {
    // Per types.ts: both locales charge the same Paddle price ids today
    // (Paddle has no IDR billing currency) — the id locale only differs in
    // the displayed Rp amount.
    for (const tierKey of PAID_TIERS) {
      const en = enPricing.find((t) => t.tierKey === tierKey)!;
      const id = idPricing.find((t) => t.tierKey === tierKey)!;
      expect(id.prices.yearly.priceId).toBe(en.prices.yearly.priceId);
      expect(id.prices.monthly.priceId).toBe(en.prices.monthly.priceId);
    }
  });

  it('the bundle option, when present, exists in both locales', () => {
    for (const tierKey of PAID_TIERS) {
      const en = enPricing.find((t) => t.tierKey === tierKey)!;
      const id = idPricing.find((t) => t.tierKey === tierKey)!;
      expect(Boolean(id.bundle)).toBe(Boolean(en.bundle));
    }
  });
});

describe('featureRowsFor', () => {
  it('returns a rows array for both locales', () => {
    for (const locale of LOCALES) {
      const rows = featureRowsFor(locale);
      expect(rows.length).toBeGreaterThan(0);
    }
  });
});