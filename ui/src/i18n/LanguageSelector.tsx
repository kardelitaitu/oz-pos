import { useContext } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { LocaleContext } from './LocaleContext';
import type { LocaleCode } from './index';

/**
 * Dropdown to switch between supported application languages.
 * Reads available locales from LocaleContext and persists the choice
 * to localStorage.
 *
 * @param hideLabel - When true, the label is omitted (for use in settings
 *   horizontal layout where the parent provides its own label).
 */
import SettingsSelect from '@/features/settings/SettingsSelect';

export function LanguageSelector({ hideLabel }: { hideLabel?: boolean }) {
  const { l10n } = useLocalization();
  const { locale, setLocale, availableLocales, getLocaleLabel } = useContext(LocaleContext);

  const options = availableLocales.map((code) => ({
    value: code,
    label: l10n.getString(getLocaleLabel(code)),
  }));

  return (
    <>
      {!hideLabel && (
        <Localized id="language-selector-label">
          <label htmlFor="language-select" className="settings-label">Language</label>
        </Localized>
      )}
      <SettingsSelect
        id="language-select"
        value={locale}
        onChange={(v) => setLocale(v as LocaleCode)}
        options={options}
        ariaLabel={l10n.getString('language-selector-select-aria')}
      />
    </>
  );
}
