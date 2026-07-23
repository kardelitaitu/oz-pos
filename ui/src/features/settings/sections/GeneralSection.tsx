import { Localized } from '@fluent/react';
import type { ReactLocalization } from '@fluent/react';
import { Card } from '@/components/Card';
import { LanguageSelector } from '@/i18n/LanguageSelector';
import SettingsSelect from '../SettingsSelect';
import type { StoreSettingsDto } from '@/api/settings';
import type { CurrencyDto } from '@/api/currency';

export interface GeneralSectionProps {
  store: StoreSettingsDto;
  setStore: (s: StoreSettingsDto) => void;
  markDirty: () => void;
  // eslint-disable-next-line @typescript-eslint/consistent-type-imports -- spread onto <input> elements
  cmInput: React.HTMLAttributes<HTMLInputElement>;
  fieldErrors: Record<string, string>;
  validateField: (field: string, value: string) => void;
  clearFieldError: (field: string) => void;
  currencies: CurrencyDto[];
  defaultCurrency: string;
  setDefaultCurrencyState: (v: string) => void;
  l10n: ReactLocalization;
}

export default function GeneralSection({
  store,
  setStore,
  markDirty,
  cmInput,
  fieldErrors,
  validateField,
  clearFieldError,
  currencies,
  defaultCurrency,
  setDefaultCurrencyState,
  l10n,
}: GeneralSectionProps) {
  return (
    <>
      {/* ── Store section ──────────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-store"><h2 className="settings-section-title">Store</h2></Localized>}
      >
        <div className="settings-form">
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="settings-field-store-name" className="settings-label">
              {l10n.getString('settings-field-store-name')}
            </label>
            <span className="settings-field-input-wrap">
              <Localized id="settings-store-name-placeholder" attrs={{ placeholder: true }}>
                <input
                  className={`settings-input${fieldErrors['store-name'] ? ' settings-input--error' : ''}`} {...cmInput}
                  type="text"
                  id="settings-field-store-name"
                  required
                  maxLength={100}
                  placeholder="OZ-POS Store"
                  value={store.name}
                  onChange={(e) => { setStore({ ...store, name: e.target.value }); clearFieldError('store-name'); markDirty(); }}
                  onBlur={() => validateField('store-name', store.name)}
                />
              </Localized>
              {fieldErrors['store-name'] && (
                <p className="settings-hint settings-hint--error">{fieldErrors['store-name']}</p>
              )}
            </span>
          </div>

          <div className="settings-field settings-field--horizontal">
            <label htmlFor="settings-field-address" className="settings-label">
              {l10n.getString('settings-field-address')}
            </label>
            <span className="settings-field-input-wrap">
              <Localized id="settings-address-placeholder" attrs={{ placeholder: true }}>
                <input
                  className={`settings-input${fieldErrors['address'] ? ' settings-input--error' : ''}`} {...cmInput}
                  type="text"
                  id="settings-field-address"
                  maxLength={200}
                  placeholder="123 Main Street"
                  value={store.address}
                  onChange={(e) => { setStore({ ...store, address: e.target.value }); clearFieldError('address'); markDirty(); }}
                />
              </Localized>
              {fieldErrors['address'] && (
                <p className="settings-hint settings-hint--error">{fieldErrors['address']}</p>
              )}
            </span>
          </div>

          <div className="settings-field settings-field--horizontal">
            <label htmlFor="settings-field-tax-id" className="settings-label">
              {l10n.getString('settings-field-tax-id')}
            </label>
            <span className="settings-field-input-wrap">
              <Localized id="settings-tax-id-placeholder" attrs={{ placeholder: true }}>
                <input
                  className={`settings-input${fieldErrors['tax-id'] ? ' settings-input--error' : ''}`} {...cmInput}
                  type="text"
                  id="settings-field-tax-id"
                  maxLength={20}
                  pattern="[A-Za-z0-9\-./]*"
                  placeholder="12-3456789"
                  title={l10n.getString('settings-tax-id-pattern-hint')}
                  value={store.taxId}
                  onChange={(e) => { setStore({ ...store, taxId: e.target.value }); clearFieldError('tax-id'); markDirty(); }}
                  onBlur={() => validateField('tax-id', store.taxId)}
                />
              </Localized>
              {fieldErrors['tax-id'] && (
                <p className="settings-hint settings-hint--error">{fieldErrors['tax-id']}</p>
              )}
            </span>
          </div>

          <div className="settings-field settings-field--horizontal">
            {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- LanguageSelector component */}
            <label htmlFor="language-select" className="settings-label">
              <Localized id="settings-field-language">
                <span>Language</span>
              </Localized>
            </label>
            <span className="settings-field-input-wrap">
              <LanguageSelector hideLabel />
            </span>
          </div>
        </div>
      </Card>

      {/* ── Currency section ──────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-currency"><h2 className="settings-section-title">Currency</h2></Localized>}
      >
        <div className="settings-form">
          <div className="settings-field settings-field--horizontal">
            {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- SettingsSelect component has hidden native select */}
            <label htmlFor="settings-field-default-currency" className="settings-label">
              <Localized id="settings-field-default-currency">
                <span>Default currency</span>
              </Localized>
            </label>
            <span className="settings-field-input-wrap">
              <SettingsSelect
                id="settings-field-default-currency"
                value={currencies.length > 0 ? defaultCurrency : ''}
                onChange={(v) => { setDefaultCurrencyState(v); markDirty(); }}
                options={currencies.length > 0
                  ? currencies.map((c) => ({
                      value: c.code,
                      label: `${c.code} — ${c.name} (${c.symbol})`,
                    }))
                  : []
                }
                disabled={currencies.length === 0}
                ariaLabel={l10n.getString('settings-field-default-currency')}
                placeholder={currencies.length === 0 ? l10n.getString('settings-currency-loading') : ''}
              />
            </span>
          </div>
        </div>
      </Card>
    </>
  );
}
