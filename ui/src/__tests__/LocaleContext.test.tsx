/**
 * Tests for `LocaleProvider` / `LocaleContext` — locale resolution and
 * persistence.
 *
 * The provider resolves the initial locale from localStorage, then the
 * browser language, then falls back to 'id'; `setLocale` persists the
 * choice and swaps the Fluent bundle. Tested via renderHook with a
 * controlled navigator + localStorage.
 */

import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { LocaleProvider, LocaleContext } from '@/i18n/LocaleContext';
import { useContext } from 'react';

const STORAGE_KEY = 'oz-pos-locale';

function wrapper({ children }: { children: ReactNode }) {
  return <LocaleProvider>{children}</LocaleProvider>;
}

describe('LocaleProvider', () => {
  const originalLanguage = navigator.language;
  const originalLanguages = navigator.languages;

  beforeEach(() => {
    localStorage.clear();
    Object.defineProperty(navigator, 'language', { value: 'en-US', configurable: true });
    Object.defineProperty(navigator, 'languages', { value: ['en-US'], configurable: true });
  });

  afterEach(() => {
    Object.defineProperty(navigator, 'language', { value: originalLanguage, configurable: true });
    Object.defineProperty(navigator, 'languages', { value: originalLanguages, configurable: true });
  });

  it('matches the browser language when nothing is stored', () => {
    // en-US → 'en'.
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.locale).toBe('en');
  });

  it('matches a secondary language from navigator.languages', () => {
    Object.defineProperty(navigator, 'language', { value: 'fr-FR', configurable: true });
    Object.defineProperty(navigator, 'languages', { value: ['fr-FR', 'id-ID'], configurable: true });
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.locale).toBe('id');
  });

  it('restores a stored locale with priority over the browser', () => {
    localStorage.setItem(STORAGE_KEY, 'id');
    Object.defineProperty(navigator, 'language', { value: 'en-US', configurable: true });
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.locale).toBe('id');
  });

  it('falls back to id when the browser language does not match', () => {
    Object.defineProperty(navigator, 'language', { value: 'fr-FR', configurable: true });
    Object.defineProperty(navigator, 'languages', { value: ['fr-FR', 'de-DE'], configurable: true });
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.locale).toBe('id');
  });

  it('ignores a corrupt stored value and falls through to the browser', () => {
    localStorage.setItem(STORAGE_KEY, 'zz');
    Object.defineProperty(navigator, 'language', { value: 'en-GB', configurable: true });
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.locale).toBe('en');
  });

  it('setLocale persists the choice and exposes it', () => {
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    act(() => result.current.setLocale('id'));
    expect(result.current.locale).toBe('id');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('id');
  });

  it('setLocale swaps the Fluent bundle', async () => {
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    act(() => result.current.setLocale('id'));
    await waitFor(() => {
      // After the bundle swap, a known shared id resolves.
      expect(result.current.getLocaleLabel('id')).toBe('locale-id');
    });
  });

  it('exposes available locales and the label helper', () => {
    const { result } = renderHook(() => useContext(LocaleContext), { wrapper });
    expect(result.current.availableLocales).toEqual(['en', 'id']);
    expect(result.current.getLocaleLabel('en')).toBe('locale-en');
    expect(result.current.getLocaleLabel('id')).toBe('locale-id');
  });
});
