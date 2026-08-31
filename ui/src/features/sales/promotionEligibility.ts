// Pure eligibility filter for the POS promotions picker (PROMO-5).
// Kept out of PromotionsModal.tsx so the component file stays a
// fast-refresh-clean component module.
import type { Promotion } from '@/api/promotions';

/**
 * Why a promotion is (or is not) applicable to the current cart.
 * Percentage promotions map onto the cart discount pipeline at
 * checkout (`set_cart_discount`), so they are the applicable kind; the
 * other engine kinds stay visible but unselectable until the
 * complete-sale promotion integration lands.
 */
export type PromotionEligibility =
  | { promo: Promotion; kind: 'eligible' }
  | { promo: Promotion; kind: 'not-applicable-type' }
  | { promo: Promotion; kind: 'below-min-order' };

/**
 * Pure eligibility filter (exported for tests): percentage promotions
 * whose minimum order is met are eligible; everything else is shown
 * with the reason it is not applicable. The 1..=100 value bound is a
 * client-side backstop duplicating the backend validation.
 */
export function evaluatePromotionEligibility(
  promos: Promotion[],
  subtotalMinor: number,
): PromotionEligibility[] {
  return promos.map((promo) => {
    if (promo.promo_type !== 'percentage' || promo.value_minor < 1 || promo.value_minor > 100) {
      return { promo, kind: 'not-applicable-type' as const };
    }
    if (subtotalMinor < promo.min_order_minor) {
      return { promo, kind: 'below-min-order' as const };
    }
    return { promo, kind: 'eligible' as const };
  });
}
