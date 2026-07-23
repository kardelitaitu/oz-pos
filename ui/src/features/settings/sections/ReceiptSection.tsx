import { Localized } from '@fluent/react';
import type { ReactLocalization } from '@fluent/react';
import { Card } from '@/components/Card';
import SettingsSelect from '../SettingsSelect';
import type { ReceiptSettingsDto } from '@/api/settings';

export interface ReceiptSectionProps {
  receipt: ReceiptSettingsDto;
  setReceipt: (r: ReceiptSettingsDto) => void;
  setDecimalSep: (v: string) => void;
  markDirty: () => void;
  l10n: ReactLocalization;
}

export default function ReceiptSection({
  receipt,
  setReceipt,
  setDecimalSep,
  markDirty,
  l10n,
}: ReceiptSectionProps) {
  return (
    <Card
      shadow="sm"
      header={<Localized id="settings-section-receipt"><h2 className="settings-section-title">Receipt</h2></Localized>}
    >
      <div className="settings-form">
        {/* Show currency */}
        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="receipt-show-currency" className="settings-label">
            <Localized id="settings-toggle-show-currency">
              <span>Show currency symbol on amounts</span>
            </Localized>
          </label>
          <span className="settings-field-input-wrap">
            <label className="settings-toggle" htmlFor="receipt-show-currency">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="receipt-show-currency"
                  type="checkbox"
                  role="switch"
                  checked={receipt.showCurrency}
                  aria-checked={receipt.showCurrency}
                  onChange={(e) => { setReceipt({ ...receipt, showCurrency: e.target.checked }); markDirty(); }}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </span>
        </div>

        {/* Decimal separator */}
        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- SettingsSelect component */}
          <label htmlFor="settings-field-decimal-separator" className="settings-label">
            {l10n.getString('settings-field-decimal-separator')}
          </label>
          <span className="settings-field-input-wrap">
            <SettingsSelect
              id="settings-field-decimal-separator"
              value={receipt.decimalSeparator}
              onChange={(v) => {
                setReceipt({ ...receipt, decimalSeparator: v });
                setDecimalSep(v);
                markDirty();
              }}
              options={[
                { value: 'dot', label: l10n.getString('settings-decimal-separator-dot') },
                { value: 'comma', label: l10n.getString('settings-decimal-separator-comma') },
                { value: 'none', label: l10n.getString('settings-decimal-separator-none') },
              ]}
            />
          </span>
        </div>

        {/* Show tax */}
        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="receipt-show-tax" className="settings-label">
            <Localized id="settings-toggle-show-tax">
              <span>Show tax line on receipts</span>
            </Localized>
          </label>
          <span className="settings-field-input-wrap">
            <label className="settings-toggle" htmlFor="receipt-show-tax">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="receipt-show-tax"
                  type="checkbox"
                  role="switch"
                  checked={receipt.showTax}
                  aria-checked={receipt.showTax}
                  onChange={(e) => { setReceipt({ ...receipt, showTax: e.target.checked }); markDirty(); }}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </span>
        </div>

        {/* Paper width */}
        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- SettingsSelect component */}
          <label htmlFor="settings-field-paper-width" className="settings-label">
            {l10n.getString('settings-field-paper-width')}
          </label>
          <span className="settings-field-input-wrap">
            <SettingsSelect
              id="settings-field-paper-width"
              value={receipt.paperWidth}
              onChange={(v) => { setReceipt({ ...receipt, paperWidth: v }); markDirty(); }}
              options={[
                { value: 'standard', label: l10n.getString('settings-paper-width-standard') },
                { value: 'narrow', label: l10n.getString('settings-paper-width-narrow') },
              ]}
            />
          </span>
        </div>

        {/* Footer */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-field-receipt-footer" className="settings-label">
            {l10n.getString('settings-field-footer')}
          </label>
          <span className="settings-field-input-wrap">
            <Localized id="settings-footer-placeholder" attrs={{ placeholder: true }}>
              <textarea
                className="settings-input settings-textarea"
                id="settings-field-receipt-footer"
                rows={3}
                maxLength={500}
                placeholder="Thank you for shopping!"
                value={receipt.footer}
                onChange={(e) => { setReceipt({ ...receipt, footer: e.target.value }); markDirty(); }}
              />
            </Localized>
            <span className="settings-hint settings-char-count">
              {receipt.footer.length}/500
            </span>
          </span>
        </div>

        {/* Show table number */}
        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="receipt-show-table-number" className="settings-label">
            <Localized id="settings-toggle-show-table-number">
              <span>Show table number on cart and receipts</span>
            </Localized>
          </label>
          <span className="settings-field-input-wrap">
            <label className="settings-toggle" htmlFor="receipt-show-table-number">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="receipt-show-table-number"
                  type="checkbox"
                  role="switch"
                  checked={receipt.showTableNumber}
                  aria-checked={receipt.showTableNumber}
                  onChange={(e) => { setReceipt({ ...receipt, showTableNumber: e.target.checked }); markDirty(); }}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </span>
        </div>
      </div>
    </Card>
  );
}
