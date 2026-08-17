/**
 * Pricing content is keyed on the REAL tier enum used by the schema and the
 * client (`free` / `trial` / `pro` / `premium` / `enterprise` — see
 * apps/license-server/pb_schema.json and crates/oz-core/src/subscription.rs).
 * The trial card is not a Paddle product (it is the offline 90-day trial).
 *
 * Paddle does not support IDR as a billing currency, so both locales
 * charge the SAME USD price ids: `en` shows USD, `id` shows an Rp display
 * figure that is an approximation of the USD amount (the checkout always
 * bills in USD — see src/content/pricing/id.ts). On the server the price
 * ids map to the tier_key via PADDLE_PRICE_TIERS (see
 * apps/license-server/paddle_webhook.go). `currency` documents the display
 * currency and can drive a future local-provider (e.g. Midtrans/Xendit)
 * billing path.
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
