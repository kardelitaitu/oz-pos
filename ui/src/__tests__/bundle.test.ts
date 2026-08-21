import { describe, expect, it } from 'vitest';
import { normalizeBundleId, detectBundleId, hasBundleId } from '@/utils/bundle';

describe('normalizeBundleId', () => {
  it('recognizes restaurant_starter case-insensitively', () => {
    for (const raw of ['restaurant_starter', 'RESTAURANT_STARTER', ' Restaurant_Starter ']) {
      expect(normalizeBundleId(raw)).toBe('restaurant_starter');
    }
  });

  it('normalizes unknowns and absence to the empty string', () => {
    for (const raw of ['fancy_bundle', 'kafe', '', null, undefined, 'restaurant-starter']) {
      expect(normalizeBundleId(raw)).toBe('');
    }
  });
});

describe('detectBundleId', () => {
  it('reads the ?bundle= param', () => {
    expect(detectBundleId('?bundle=restaurant_starter')).toBe('restaurant_starter');
    expect(detectBundleId('?v=kafe&bundle=restaurant_starter')).toBe('restaurant_starter');
    expect(detectBundleId('?bundle=restaurant_starter&utm_source=ads')).toBe('restaurant_starter');
  });

  it('returns empty when the param is absent or unknown', () => {
    expect(detectBundleId('')).toBe('');
    expect(detectBundleId('?utm_source=ads')).toBe('');
    expect(detectBundleId('?bundle=fancy_bundle')).toBe('');
  });
});

describe('hasBundleId', () => {
  it('is true only for a detected bundle', () => {
    expect(hasBundleId('?bundle=restaurant_starter')).toBe(true);
    expect(hasBundleId('?bundle=fancy_bundle')).toBe(false);
    expect(hasBundleId('')).toBe(false);
  });
});
