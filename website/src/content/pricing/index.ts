import { pricing as enPricing, featureRows as enRows } from './en';
import { pricing as idPricing, featureRows as idRows } from './id';

export function pricingFor(locale: string) {
  return locale === 'id' ? idPricing : enPricing;
}

export function featureRowsFor(locale: string) {
  return locale === 'id' ? idRows : enRows;
}

export type { PricingTier, FeatureRow, TierKey } from './types';
