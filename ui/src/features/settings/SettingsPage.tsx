import { useEffect, useState, useCallback } from 'react';
import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  listCurrencies,
  getDefaultCurrency,
  setDefaultCurrency,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
  type CurrencyDto,
} from '@/api/pos';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './SettingsPage.css';

/**
 * Settings (Preferences) screen.
 *
 * Sections:
 * - Store: name, address, tax ID
 * - Receipt display: currency prefix, decimal separator, tax line, footer, paper width
 *
 * On mount, loads current settings from the backend.
 * The "Save" button persists all changes at once.
 */
export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const [receipt, setReceipt] = useState<ReceiptSettingsDto>({
    showCurrency: false,
    decimalSeparator: 'dot',
    showTax: true,
    footer: '',
    paperWidth: 'standard',
  });

  const [store, setStore] = useState<StoreSettingsDto>({
    name: '',
    address: '',
    taxId: '',
  });

  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [defaultCurrency, setDefaultCurrencyState] = useState<string>('USD');

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [r, s, currenciesData, defaultCurrencyData] = await Promise.all([
          getReceiptSettings(),
          getStoreSettings(),
          listCurrencies(),
          getDefaultCurrency(),
        ]);
        if (!cancelled) {
          setReceipt(r);
          setStore(s);
          setCurrencies(currenciesData);
          setDefaultCurrencyState(defaultCurrencyData ?? 'USD');
        }
      } catch {
        // IPC unavailable — use defaults.
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaved(false);
    try {
      await Promise.all([
        setReceiptSettings(receipt),
        setStoreSettings(store),
        setDefaultCurrency({ code: defaultCurrency }),
      ]);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch {
      // Toast or error state could go here.
    } finally {
      setSaving(false);
    }
  }, [receipt, store, defaultCurrency]);

  if (loading) {
    return <div className="settings-page"><p>Loading settings…</p></div>;
  }

  return (
    <div className="settings-page">
      <h1 className="settings-title">Settings</h1>

      {/* ── Store section ────────────────────────── */}
      <Card
        shadow="sm"
        header={<h2 className="settings-section-title">Store</h2>}
      >
        <div className="settings-form">
          <label className="settings-field">
            <span className="settings-label">Store name</span>
            <input
              className="settings-input"
              type="text"
              placeholder="OZ-POS Store"
              value={store.name}
              onChange={(e) => setStore({ ...store, name: e.target.value })}
            />
          </label>

          <label className="settings-field">
            <span className="settings-label">Address</span>
            <input
              className="settings-input"
              type="text"
              placeholder="123 Main Street"
              value={store.address}
              onChange={(e) => setStore({ ...store, address: e.target.value })}
            />
          </label>

          <label className="settings-field">
            <span className="settings-label">Tax / VAT ID</span>
            <input
              className="settings-input"
              type="text"
              placeholder="12-3456789"
              value={store.taxId}
              onChange={(e) => setStore({ ...store, taxId: e.target.value })}
            />
          </label>
        </div>
      </Card>

      {/* ── Currency section ────────────────────── */}
      <Card
        shadow="sm"
        header={<h2 className="settings-section-title">Currency</h2>}
      >
        <div className="settings-form">
          <label className="settings-field">
            <span className="settings-label">Default currency</span>
            <select
              className="settings-select"
              value={defaultCurrency}
              onChange={(e) => setDefaultCurrencyState(e.target.value)}
            >
              {currencies.map((c) => (
                <option key={c.code} value={c.code}>
                  {c.code} — {c.name} ({c.symbol})
                </option>
              ))}
            </select>
          </label>
        </div>
      </Card>

      {/* ── Receipt section ───────────────────────── */}
      <Card
        shadow="sm"
        header={<h2 className="settings-section-title">Receipt</h2>}
      >
        <div className="settings-form">
          {/* Show currency */}
          <label className="settings-toggle">
            <input
              type="checkbox"
              checked={receipt.showCurrency}
              onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
            />
            <span>Show currency symbol on amounts</span>
          </label>

          {/* Decimal separator */}
          <label className="settings-field">
            <span className="settings-label">Decimal separator</span>
            <select
              className="settings-select"
              value={receipt.decimalSeparator}
              onChange={(e) => setReceipt({ ...receipt, decimalSeparator: e.target.value })}
            >
              <option value="dot">1.00 (dot)</option>
              <option value="comma">1,00 (comma)</option>
              <option value="none">1 (none)</option>
            </select>
          </label>

          {/* Show tax */}
          <label className="settings-toggle">
            <input
              type="checkbox"
              checked={receipt.showTax}
              onChange={(e) => setReceipt({ ...receipt, showTax: e.target.checked })}
            />
            <span>Show tax line on receipts</span>
          </label>

          {/* Paper width */}
          <label className="settings-field">
            <span className="settings-label">Paper width</span>
            <select
              className="settings-select"
              value={receipt.paperWidth}
              onChange={(e) => setReceipt({ ...receipt, paperWidth: e.target.value })}
            >
              <option value="standard">80 mm (standard)</option>
              <option value="narrow">58 mm (narrow)</option>
            </select>
          </label>

          {/* Footer */}
          <label className="settings-field">
            <span className="settings-label">Receipt footer</span>
            <input
              className="settings-input"
              type="text"
              placeholder="Thank you for shopping!"
              value={receipt.footer}
              onChange={(e) => setReceipt({ ...receipt, footer: e.target.value })}
            />
          </label>

          {/* Save */}
          <div className="settings-actions">
            <Button
              variant="primary"
              loading={saving}
              onClick={handleSave}
            >
              {saved ? 'Saved!' : 'Save'}
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
