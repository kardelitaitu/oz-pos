// Pure eligibility filter for the POS promotions picker (PROMO-5/PROMO-3).
// Kept out of PromotionsModal.tsx so the component file stays a
// fast-refresh-clean component module.
import type { Promotion } from '@/api/promotions';

/**
 * Why a promotion is (or is not) applicable to the current cart.
 * With the checkout promotion integration (PROMO-3) every engine kind is
 * selectable: the picker accumulates promotion ids and the backend engine
 * applies them against the post-tax sale at checkout (percentage, fixed
 * amount, and buy-x-get-y alike), so there is no longer a
 * `not-applicable-type` bucket — only the min-order gate (and the
 * percentage value backstop) remain.
 */
export type PromotionEligibility =
  | { promo: Promotion; kind: 'eligible' }
  | { promo: Promotion; kind: 'below-min-order' }
  | { promo: Promotion; kind: 'invalid-value' };

/**
 * Pure eligibility filter (exported for tests): promotions whose minimum
 * order is met are eligible; everything else is shown with the reason it
 * is not applicable. The 1..=100 value bound stays as a client-side
 * backstop for percentage promotions, duplicating the backend validation.
 */
export function evaluatePromotionEligibility(
  promos: Promotion[],
  subtotalMinor: number,
): PromotionEligibility[] {
  return promos.map((promo) => {
    if (promo.promo_type === 'percentage' && (promo.value_minor < 1 || promo.value_minor > 100)) {
      return { promo, kind: 'invalid-value' as const };
    }
    if (subtotalMinor < promo.min_order_minor) {
      return { promo, kind: 'below-min-order' as const };
    }
    return { promo, kind: 'eligible' as const };
  });
}
