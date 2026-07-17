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

/**
 * Provides locale state and the Fluent localisation provider to the
 * component tree. Persists the user's choice to localStorage and
 * initialises from the stored value or defaults to `'id'`.
 */
export function LocaleProvider({ children }: LocaleProviderProps) {
  const [locale, setLocaleState] = useState<LocaleCode>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'en' || stored === 'id') return stored;
    return 'id';
  });

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
