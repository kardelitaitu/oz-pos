/**
 * Pricing content is keyed on the REAL tier enum used by the schema and the
 * client (`free` / `trial` / `pro` / `premium` / `enterprise` — see
 * apps/license-server/pb_schema.json and crates/oz-core/src/subscription.rs).
 * The trial card is not a Paddle product (it is the offline 90-day trial);
 * `priceId` is a Paddle v2 price id placeholder for the paid tiers.
 */
export type TierKey = 'trial' | 'pro' | 'premium' | 'enterprise';

export interface PricingTier {
  id: string;
  tierKey: TierKey;
  name: string;
  price: string;
  period: string;
  description: string;
  cta: string;
  highlight?: boolean;
  /** Paddle v2 price id (placeholder `pri_…`). Empty = no checkout (trial / custom). */
  priceId?: string;
  features: { label: string; included: boolean }[];
}

export interface FeatureRow {
  label: string;
  values: Record<TierKey, boolean | string | number>;
}
