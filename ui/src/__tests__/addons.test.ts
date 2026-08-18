// ── C4.3: Add-on Marketplace unit tests ───────────────────────────
//
// Tests the addon catalog definition, lookup helpers, and tier-filtering
// logic. Pure functions — no React rendering needed.

import { describe, expect, it } from 'vitest';
import {
  ADDON_CATALOG,
  getAddonById,
  getAddonsForTier,
  tenantHasAddon,
  type AddonDefinition,
} from '@/api/addons';

// ── Catalog validation ────────────────────────────────────────────

describe('ADDON_CATALOG', () => {
  it('contains at least 4 add-ons', () => {
    expect(ADDON_CATALOG.length).toBeGreaterThanOrEqual(4);
  });

  it('every addon has a unique id', () => {
    const ids = ADDON_CATALOG.map((a) => a.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('every addon has a non-empty nameKey', () => {
    for (const addon of ADDON_CATALOG) {
      expect(addon.nameKey).toBeTruthy();
      expect(addon.nameKey.length).toBeGreaterThan(0);
    }
  });

  it('every addon has a non-empty descriptionKey', () => {
    for (const addon of ADDON_CATALOG) {
      expect(addon.descriptionKey).toBeTruthy();
      expect(addon.descriptionKey.length).toBeGreaterThan(0);
    }
  });

  it('every addon has a positive price', () => {
    for (const addon of ADDON_CATALOG) {
      expect(addon.priceUsd).toBeGreaterThan(0);
    }
  });

  it('every addon has a paddlePriceId', () => {
    for (const addon of ADDON_CATALOG) {
      expect(addon.paddlePriceId).toBeTruthy();
      expect(addon.paddlePriceId.length).toBeGreaterThan(0);
    }
  });

  it('every addon has an icon', () => {
    for (const addon of ADDON_CATALOG) {
      expect(addon.icon).toBeTruthy();
    }
  });

  it('targetTiers is an array (may be empty for all-tier addons)', () => {
    for (const addon of ADDON_CATALOG) {
      expect(Array.isArray(addon.targetTiers)).toBe(true);
    }
  });

  it('contains advanced_analytics addon', () => {
    const addon = getAddonById('advanced_analytics');
    expect(addon).toBeDefined();
    expect(addon!.priceUsd).toBe(2.99);
    expect(addon!.targetTiers).toContain('plus');
  });

  it('contains priority_support addon', () => {
    const addon = getAddonById('priority_support');
    expect(addon).toBeDefined();
    expect(addon!.priceUsd).toBe(4.99);
  });

  it('contains extra_storage addon', () => {
    const addon = getAddonById('extra_storage');
    expect(addon).toBeDefined();
    expect(addon!.targetTiers).toContain('plus');
    expect(addon!.targetTiers).toContain('pro');
  });

  it('contains custom_hal addon', () => {
    const addon = getAddonById('custom_hal');
    expect(addon).toBeDefined();
    expect(addon!.targetTiers).toContain('premium');
    expect(addon!.targetTiers).toContain('enterprise');
  });
});

// ── getAddonById ──────────────────────────────────────────────────

describe('getAddonById', () => {
  it('returns the addon for a valid id', () => {
    const addon = getAddonById('advanced_analytics');
    expect(addon).toBeDefined();
    expect(addon!.id).toBe('advanced_analytics');
  });

  it('returns undefined for unknown id', () => {
    expect(getAddonById('nonexistent')).toBeUndefined();
  });

  it('returns undefined for empty string', () => {
    expect(getAddonById('')).toBeUndefined();
  });
});

// ── getAddonsForTier ──────────────────────────────────────────────

describe('getAddonsForTier', () => {
  it('returns all-tier addons for free tier', () => {
    const addons = getAddonsForTier('free');
    // priority_support has empty targetTiers (all tiers)
    expect(addons.some((a) => a.id === 'priority_support')).toBe(true);
  });

  it('returns plus-relevant addons for plus tier', () => {
    const addons = getAddonsForTier('plus');
    expect(addons.some((a) => a.id === 'advanced_analytics')).toBe(true);
    expect(addons.some((a) => a.id === 'extra_storage')).toBe(true);
    expect(addons.some((a) => a.id === 'priority_support')).toBe(true);
    // custom_hal targets premium/enterprise only
    expect(addons.some((a) => a.id === 'custom_hal')).toBe(false);
  });

  it('returns plus-relevant addons for pro tier', () => {
    const addons = getAddonsForTier('pro');
    expect(addons.some((a) => a.id === 'extra_storage')).toBe(true);
    expect(addons.some((a) => a.id === 'priority_support')).toBe(true);
    // advanced_analytics targets plus only
    expect(addons.some((a) => a.id === 'advanced_analytics')).toBe(false);
  });

  it('returns premium-relevant addons for premium tier', () => {
    const addons = getAddonsForTier('premium');
    expect(addons.some((a) => a.id === 'custom_hal')).toBe(true);
    expect(addons.some((a) => a.id === 'priority_support')).toBe(true);
  });

  it('returns all-tier addons for enterprise tier', () => {
    const addons = getAddonsForTier('enterprise');
    expect(addons.some((a) => a.id === 'custom_hal')).toBe(true);
    expect(addons.some((a) => a.id === 'priority_support')).toBe(true);
  });

  it('always includes all-tier addons (empty targetTiers)', () => {
    const allTierAddons = ADDON_CATALOG.filter((a) => a.targetTiers.length === 0);
    for (const tier of ['free', 'plus', 'pro', 'premium', 'enterprise']) {
      const result = getAddonsForTier(tier);
      for (const addon of allTierAddons) {
        expect(result.some((a) => a.id === addon.id)).toBe(true);
      }
    }
  });

  it('never includes tier-specific addons for wrong tiers', () => {
    // custom_hal targets premium/enterprise
    const freeAddons = getAddonsForTier('free');
    expect(freeAddons.some((a) => a.id === 'custom_hal')).toBe(false);

    // advanced_analytics targets plus only
    const proAddons = getAddonsForTier('pro');
    expect(proAddons.some((a) => a.id === 'advanced_analytics')).toBe(false);
  });
});

// ── tenantHasAddon ────────────────────────────────────────────────

describe('tenantHasAddon', () => {
  it('returns true when addon is present', () => {
    expect(tenantHasAddon(['advanced_analytics'], 'advanced_analytics')).toBe(true);
  });

  it('returns true with case-insensitive match', () => {
    expect(tenantHasAddon(['Advanced_Analytics'], 'advanced_analytics')).toBe(true);
    expect(tenantHasAddon(['advanced_analytics'], 'ADVANCED_ANALYTICS')).toBe(true);
  });

  it('returns false when addon is not present', () => {
    expect(tenantHasAddon(['priority_support'], 'advanced_analytics')).toBe(false);
  });

  it('returns false for empty addons list', () => {
    expect(tenantHasAddon([], 'advanced_analytics')).toBe(false);
  });

  it('returns true with multiple addons', () => {
    const addons = ['advanced_analytics', 'priority_support', 'extra_storage'];
    expect(tenantHasAddon(addons, 'priority_support')).toBe(true);
    expect(tenantHasAddon(addons, 'custom_hal')).toBe(false);
  });

  it('returns false for empty addon id', () => {
    expect(tenantHasAddon(['advanced_analytics'], '')).toBe(false);
  });
});
