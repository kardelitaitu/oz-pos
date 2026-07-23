/* eslint-disable react-refresh/only-export-components */
import { createContext, useState, useCallback, useMemo } from 'react';

import type { ReactNode } from 'react';
import type { FluentBundle } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { getBundle, getAvailableLocales, getLocaleLabel } from './index';
import type { LocaleCode } from './index';

/** Shape of the locale context exposed to consumers. */
export interface LocaleContextValue {
  locale: LocaleCode;
  setLocale: (code: LocaleCode) => void;
  availableLocales: LocaleCode[];
  getLocaleLabel: (code: LocaleCode) => string;
}

/** React context that carries the current locale and setter. */
export const LocaleContext = createContext<LocaleContextValue>({
  locale: 'id',
  setLocale: () => {},
  availableLocales: ['id'],
  getLocaleLabel: () => '',
});

interface LocaleProviderProps {
  children: ReactNode;
}

const STORAGE_KEY = 'oz-pos-locale';

/** Supported locale codes, ordered for lookup. */
const SUPPORTED_LOCALES: LocaleCode[] = ['en', 'id', 'th'];

/**
 * Resolve the initial locale for this session.
 *
 * 1. If the user previously chose a locale, restore it from localStorage.
 * 2. Otherwise, try to match the browser's preferred language(s) against
 *    the supported locales.
 * 3. Fall back to Indonesian (`'id'`) as the application default when no
 *    match is found.
 */
function resolveInitialLocale(): LocaleCode {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'en' || stored === 'id' || stored === 'th') return stored;

  if (typeof navigator !== 'undefined') {
    const languages = [navigator.language, ...(navigator.languages ?? [])];
    for (const lang of languages) {
      if (!lang) continue;
      const normalized = lang.toLowerCase();
      for (const code of SUPPORTED_LOCALES) {
        if (normalized.startsWith(code)) return code;
      }
    }
  }

  return 'id';
}

/**
 * Provides locale state and the Fluent localisation provider to the
 * component tree. Persists the user's choice to localStorage and
 * initialises from the stored value, the browser language, or the
 * application default (`'id'`).
 */
export function LocaleProvider({ children }: LocaleProviderProps) {
  const [locale, setLocaleState] = useState<LocaleCode>(resolveInitialLocale);

  const [bundle, setBundle] = useState<FluentBundle>(() => getBundle(locale));

  const setLocale = useCallback((code: LocaleCode) => {
    setLocaleState(code);
    setBundle(getBundle(code));
    localStorage.setItem(STORAGE_KEY, code);
  }, []);

  const l10n = useMemo(() => new ReactLocalization([bundle]), [bundle]);

  const value: LocaleContextValue = {
    locale,
    setLocale,
    availableLocales: getAvailableLocales(),
    getLocaleLabel,
  };

  return (
    <LocaleContext.Provider value={value}>
      <LocalizationProvider l10n={l10n}>
        {children}
      </LocalizationProvider>
    </LocaleContext.Provider>
  );
}
