// ── Add-on Marketplace (C4.3) ──────────────────────────────────────
//
// Defines the available add-ons, their metadata, and the marketplace
// API. Add-ons extend tier capabilities (e.g. "advanced_analytics"
// adds analytics to Plus without upgrading to Pro).


/** A single add-on in the marketplace catalog. */
export interface AddonDefinition {
  /** Machine-readable identifier matching the `addons` array in the license payload. */
  id: string;
  /** Localized display name key (use with Fluent). */
  nameKey: string;
  /** Localized description key (use with Fluent). */
  descriptionKey: string;
  /** Monthly price in USD (display only — actual billing via Paddle). */
  priceUsd: number;
  /** The tier(s) that benefit from this add-on. Empty = all tiers. */
  targetTiers: string[];
  /** Paddle price ID for checkout. */
  paddlePriceId: string;
  /** Feature icon (CSS class or emoji). */
  icon: string;
}

/**
 * The marketplace catalog. Prices and Paddle price IDs are placeholders —
 * replace with real values before launch. The catalog is static because
 * add-ons change infrequently and the UI needs to render instantly.
 */
export const ADDON_CATALOG: AddonDefinition[] = [
  {
    id: 'advanced_analytics',
    nameKey: 'addon-analytics-name',
    descriptionKey: 'addon-analytics-desc',
    priceUsd: 2.99,
    targetTiers: ['plus'],
    paddlePriceId: 'pri_addon_analytics_monthly',
    icon: '📊',
  },
  {
    id: 'priority_support',
    nameKey: 'addon-support-name',
    descriptionKey: 'addon-support-desc',
    priceUsd: 4.99,
    targetTiers: [],
    paddlePriceId: 'pri_addon_support_monthly',
    icon: '🎯',
  },
  {
    id: 'extra_storage',
    nameKey: 'addon-storage-name',
    descriptionKey: 'addon-storage-desc',
    priceUsd: 1.99,
    targetTiers: ['plus', 'pro'],
    paddlePriceId: 'pri_addon_storage_monthly',
    icon: '☁️',
  },
  {
    id: 'custom_hal',
    nameKey: 'addon-hal-name',
    descriptionKey: 'addon-hal-desc',
    priceUsd: 9.99,
    targetTiers: ['premium', 'enterprise'],
    paddlePriceId: 'pri_addon_hal_monthly',
    icon: '🔌',
  },
];

/** Look up an add-on by ID from the catalog. */
export function getAddonById(id: string): AddonDefinition | undefined {
  return ADDON_CATALOG.find((a) => a.id === id);
}

/** Filter the catalog to add-ons relevant for a given tier. */
export function getAddonsForTier(tier: string): AddonDefinition[] {
  return ADDON_CATALOG.filter(
    (a) => a.targetTiers.length === 0 || a.targetTiers.includes(tier),
  );
}

/** Check if a tenant already has a specific add-on. */
export function tenantHasAddon(addons: string[], addonId: string): boolean {
  return addons.some((a) => a.toLowerCase() === addonId.toLowerCase());
}
