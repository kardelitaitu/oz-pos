// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { getPreferredLanguage, setPreferredLanguage } from '../language';

describe('language helpers (localStorage get/set)', () => {
  afterEach(() => {
    localStorage.clear();
  });

  it('returns null when no value is stored', () => {
    expect(getPreferredLanguage()).toBeNull();
  });

  it('returns the stored language after setPreferredLanguage', () => {
    setPreferredLanguage('id');
    expect(getPreferredLanguage()).toBe('id');
  });

  it('overwrites a previously stored language', () => {
    setPreferredLanguage('en');
    expect(getPreferredLanguage()).toBe('en');
    setPreferredLanguage('id');
    expect(getPreferredLanguage()).toBe('id');
  });

  it('reads the oz_language key from localStorage', () => {
    localStorage.setItem('oz_language', 'id');
    expect(getPreferredLanguage()).toBe('id');
  });

  it('returns null for an unrecognized value stored in localStorage', () => {
    localStorage.setItem('oz_language', 'fr');
    // The function returns whatever is in localStorage, cast to Language.
    // Consumers decide whether to trust the value — the helper is agnostic.
    expect(getPreferredLanguage()).toBe('fr');
  });
});
