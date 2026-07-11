import { useContext } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { LocaleContext } from './LocaleContext';
import type { LocaleCode } from './index';

/**
 * Dropdown to switch between supported application languages.
 * Reads available locales from LocaleContext and persists the choice
 * to localStorage.
 */
export function LanguageSelector() {
  const { l10n } = useLocalization();
  const { locale, setLocale, availableLocales, getLocaleLabel } = useContext(LocaleContext);

  return (
    <div>
      <Localized id="language-selector-label">
        <label htmlFor="language-select">Language</label>
      </Localized>
      <select
        id="language-select"
        value={locale}
        onChange={(e) => setLocale(e.target.value as LocaleCode)}
        aria-label={l10n.getString('language-selector-select-aria')}
      >
        {availableLocales.map((code) => (
          <option key={code} value={code}>
            {l10n.getString(getLocaleLabel(code))}
          </option>
        ))}
      </select>
    </div>
  );
}
