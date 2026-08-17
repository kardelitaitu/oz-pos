import { describe, expect, it } from 'vitest';
import { t } from '../index';

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
