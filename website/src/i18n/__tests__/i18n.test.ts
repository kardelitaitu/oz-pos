import { describe, expect, it } from 'vitest';
import { t, dict, locales } from '../index';

/**
 * The i18n helper. Full en/id parity is enforced separately by
 * scripts/audit-i18n.mjs (runs in precheck/prebuild) — these pin the
 * lookup semantics.
 */
describe('t()', () => {
  it('resolves dot paths in both locales', () => {
    expect(t('en', 'login.tabEmailCode')).toBe('Email code');
    const idValue = t('id', 'login.tabEmailCode');
    expect(idValue).toBeTruthy();
    expect(idValue).not.toBe('login.tabEmailCode');
  });

  it('falls back to the key for unknown paths instead of throwing', () => {
    expect(t('en', 'nope.missing')).toBe('nope.missing');
    expect(t('en', 'login')).toBe('login'); // non-string values fall back too
  });

  it('falls back to English for unknown locales', () => {
    expect(t('fr', 'login.tabEmailCode')).toBe('Email code');
  });
});

describe('dict()', () => {
  it('returns the full dictionary for a known locale', () => {
    const d = dict('en');
    expect(d).toHaveProperty('nav');
    expect(d).toHaveProperty('hero');
  });

  it('falls back to English for an unknown locale', () => {
    const d = dict('fr');
    expect(d).toEqual(dict('en'));
  });

  it('enables structured access for arrays', () => {
    const d = dict('en') as Record<string, unknown>;
    const features = (d.features as Record<string, unknown>).items as unknown[];
    expect(Array.isArray(features)).toBe(true);
    expect(features.length).toBe(9);
  });
});

describe('locales constant', () => {
  it('contains exactly en and id', () => {
    expect(locales).toEqual(['en', 'id']);
  });
});
