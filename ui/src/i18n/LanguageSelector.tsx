import { useContext } from 'react';
import { LocaleContext } from './LocaleContext';
import type { LocaleCode } from './index';

export function LanguageSelector() {
  const { locale, setLocale, availableLocales, getLocaleLabel } = useContext(LocaleContext);

  return (
    <div>
      <label htmlFor="language-select">Language</label>
      <select
        id="language-select"
        value={locale}
        onChange={(e) => setLocale(e.target.value as LocaleCode)}
        aria-label="Select language"
      >
        {availableLocales.map((code) => (
          <option key={code} value={code}>
            {getLocaleLabel(code)}
          </option>
        ))}
      </select>
    </div>
  );
}
