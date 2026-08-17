// ── Upgrade CTA helpers (C2.2 in-app upgrade triggers) ───────────────

/**
 * Pricing-page anchor for each tier's upgrade target. The in-app gates
 * deep-link to the matching card on the website pricing page.
 */
export type UpgradeTarget = 'plus' | 'pro' | 'premium';

/** Website pricing URL for the given locale + tier anchor. */
export function upgradePricingUrl(locale: string, target: UpgradeTarget): string {
  return `https://oz-pos.adikaradwiatmaja.workers.dev/${locale}/pricing/#${target}`;
}

/** Open the pricing page for an upgrade target in a new tab. */
export function openUpgradePricing(locale: string, target: UpgradeTarget): void {
  window.open(upgradePricingUrl(locale, target), '_blank', 'noopener,noreferrer');
}
