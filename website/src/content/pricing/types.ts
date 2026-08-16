/**
 * Pricing content is keyed on the REAL tier enum used by the schema and the
 * client (`free` / `trial` / `pro` / `premium` / `enterprise` — see
 * apps/license-server/pb_schema.json and crates/oz-core/src/subscription.rs).
 * The trial card is not a Paddle product (it is the offline 90-day trial).
 *
 * Each paid tier has TWO Paddle prices: a USD price for the global audience
 * (`en` locale) and an IDR price for Indonesia (`id` locale). The content
 * files are per-locale, so `priceId` always holds the price id for THAT
 * locale's currency — the checkout button never needs to convert. On the
 * server, both ids map to the same tier_key via PADDLE_PRICE_TIERS (see
 * apps/license-server/paddle_webhook.go). `currency` documents which is
 * which and can drive display/toggle logic later.
 */
export type TierKey = 'trial' | 'pro' | 'premium' | 'enterprise';

export type Currency = 'USD' | 'IDR';

export interface PricingTier {
  id: string;
  tierKey: TierKey;
  name: string;
  price: string;
  period: string;
  description: string;
  cta: string;
  /** Currency this locale charges in (en → USD, id → IDR). */
  currency: Currency;
  highlight?: boolean;
  /**
   * Paddle v2 price id for this locale's currency (real `pri_…` when the
   * product is live; `pri_placeholder_…` otherwise — see
   * CheckoutButton.isPlaceholderPriceId). Empty = no checkout (trial / custom).
   */
  priceId?: string;
  features: { label: string; included: boolean }[];
}

export interface FeatureRow {
  label: string;
  values: Record<TierKey, boolean | string | number>;
}
