import { createContext, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import type { FluentBundle } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { getBundle, getAvailableLocales, getLocaleLabel } from './index';
import type { LocaleCode } from './index';

export interface LocaleContextValue {
  locale: LocaleCode;
  setLocale: (code: LocaleCode) => void;
  availableLocales: LocaleCode[];
  getLocaleLabel: (code: LocaleCode) => string;
}

export const LocaleContext = createContext<LocaleContextValue>({
  locale: 'en',
  setLocale: () => {},
  availableLocales: ['en'],
  getLocaleLabel: () => '',
});

interface LocaleProviderProps {
  children: ReactNode;
}

const STORAGE_KEY = 'oz-pos-locale';

export function LocaleProvider({ children }: LocaleProviderProps) {
  const [locale, setLocaleState] = useState<LocaleCode>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'en' || stored === 'id') return stored;
    return 'en';
  });

  const [bundle, setBundle] = useState<FluentBundle>(() => getBundle(locale));

  const setLocale = useCallback((code: LocaleCode) => {
    setLocaleState(code);
    setBundle(getBundle(code));
    localStorage.setItem(STORAGE_KEY, code);
  }, []);

  const value: LocaleContextValue = {
    locale,
    setLocale,
    availableLocales: getAvailableLocales(),
    getLocaleLabel,
  };

  return (
    <LocaleContext.Provider value={value}>
      <LocalizationProvider l10n={new ReactLocalization([bundle])}>
        {children}
      </LocalizationProvider>
    </LocaleContext.Provider>
  );
}
