import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
} from '@/api/settings';
import {
  listCurrencies,
  getDefaultCurrency,
  setDefaultCurrency,
  type CurrencyDto,
} from '@/api/currency';
import {
  getSyncSettings,
  updateSyncSettings,
  triggerSync,
  type SyncSettingsDto,
  type SyncAttemptResult,
} from '@/api/offline';
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

  const [sync, setSync] = useState<SyncSettingsDto>({
    serverUrl: null,
    hasApiKey: false,
    enabled: false,
  });
  const [syncServerUrl, setSyncServerUrl] = useState('');
  const [syncApiKey, setSyncApiKey] = useState('');
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncAttemptResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [r, s, currenciesData, defaultCurrencyData, syncData] = await Promise.all([
          getReceiptSettings(),
          getStoreSettings(),
          listCurrencies(),
          getDefaultCurrency(),
          getSyncSettings(),
        ]);
        if (!cancelled) {
          setReceipt(r);
          setStore(s);
          setCurrencies(currenciesData);
          setDefaultCurrencyState(defaultCurrencyData ?? 'USD');
          setSync(syncData);
          setSyncServerUrl(syncData.serverUrl ?? '');
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
        updateSyncSettings({
          serverUrl: syncServerUrl || null,
          apiKey: syncApiKey || null,
          enabled: sync.enabled,
        }),
      ]);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      setSyncApiKey('');
    } catch {
      // Toast or error state could go here.
    } finally {
      setSaving(false);
    }
  }, [receipt, store, defaultCurrency]);

  if (loading) {
    return <div className="settings-page"><Localized id="settings-loading"><p>Loading settings&hellip;</p></Localized></div>;
  }

  return (
    <div className="settings-page">
      <Localized id="settings-page-title">
        <h1 className="settings-title">Settings</h1>
      </Localized>

      {/* ── Store section ────────────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-store"><h2 className="settings-section-title">Store</h2></Localized>}
      >
        <div className="settings-form">
          <label className="settings-field">
            <Localized id="settings-field-store-name">
              <span className="settings-label">Store name</span>
            </Localized>
            <input
              className="settings-input"
              type="text"
              placeholder="OZ-POS Store"
              value={store.name}
              onChange={(e) => setStore({ ...store, name: e.target.value })}
            />
          </label>

          <label className="settings-field">
            <Localized id="settings-field-address">
              <span className="settings-label">Address</span>
            </Localized>
            <input
              className="settings-input"
              type="text"
              placeholder="123 Main Street"
              value={store.address}
              onChange={(e) => setStore({ ...store, address: e.target.value })}
            />
          </label>

          <label className="settings-field">
            <Localized id="settings-field-tax-id">
              <span className="settings-label">Tax / VAT ID</span>
            </Localized>
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
        header={<Localized id="settings-section-currency"><h2 className="settings-section-title">Currency</h2></Localized>}
      >
        <div className="settings-form">
          <label className="settings-field">
            <Localized id="settings-field-default-currency">
              <span className="settings-label">Default currency</span>
            </Localized>
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
        header={<Localized id="settings-section-receipt"><h2 className="settings-section-title">Receipt</h2></Localized>}
      >
        <div className="settings-form">
          {/* Show currency */}
          <label className="settings-toggle">
            <input
              type="checkbox"
              checked={receipt.showCurrency}
              onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
            />
            <Localized id="settings-toggle-show-currency">
              <span>Show currency symbol on amounts</span>
            </Localized>
          </label>

          {/* Decimal separator */}
          <label className="settings-field">
            <Localized id="settings-field-decimal-separator">
              <span className="settings-label">Decimal separator</span>
            </Localized>
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
            <Localized id="settings-toggle-show-tax">
              <span>Show tax line on receipts</span>
            </Localized>
          </label>

          {/* Paper width */}
          <label className="settings-field">
            <Localized id="settings-field-paper-width">
              <span className="settings-label">Paper width</span>
            </Localized>
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
            <Localized id="settings-field-footer">
              <span className="settings-label">Receipt footer</span>
            </Localized>
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
              <Localized id={saved ? 'settings-saved' : 'settings-btn-save'}>
                <span>{saved ? 'Saved!' : 'Save'}</span>
              </Localized>
            </Button>
          </div>
        </div>
      </Card>

      {/* ── Cloud Sync section ────────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-sync"><h2 className="settings-section-title">Cloud Sync</h2></Localized>}
      >
        {sync.serverUrl === null && !sync.enabled ? (
          <div className="settings-form">
            <p className="settings-hint">
              <Localized id="settings-sync-not-configured">
                <span>Sync is not configured. Enter a server URL and enable sync.</span>
              </Localized>
            </p>
            <label className="settings-field">
              <Localized id="settings-sync-server-url">
                <span className="settings-label">Server URL</span>
              </Localized>
              <input
                className="settings-input"
                type="url"
                placeholder="https://api.example.com"
                value={syncServerUrl}
                onChange={(e) => setSyncServerUrl(e.target.value)}
              />
            </label>

            <label className="settings-field">
              <Localized id="settings-sync-api-key">
                <span className="settings-label">API Key</span>
              </Localized>
              <input
                className="settings-input"
                type="password"
                placeholder="Enter API key"
                value={syncApiKey}
                onChange={(e) => setSyncApiKey(e.target.value)}
              />
            </label>

            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={sync.enabled}
                onChange={(e) => setSync({ ...sync, enabled: e.target.checked })}
              />
              <Localized id="settings-sync-enabled">
                <span>Enable Cloud Sync</span>
              </Localized>
            </label>
          </div>
        ) : (
          <div className="settings-form">
            <label className="settings-field">
              <Localized id="settings-sync-server-url">
                <span className="settings-label">Server URL</span>
              </Localized>
              <input
                className="settings-input"
                type="url"
                placeholder="https://api.example.com"
                value={syncServerUrl}
                onChange={(e) => setSyncServerUrl(e.target.value)}
              />
            </label>

            <label className="settings-field">
              <Localized id="settings-sync-api-key">
                <span className="settings-label">API Key</span>
              </Localized>
              <input
                className="settings-input"
                type="password"
                placeholder={sync.hasApiKey ? '••••••••' : 'Enter API key'}
                value={syncApiKey}
                onChange={(e) => setSyncApiKey(e.target.value)}
              />
            </label>

            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={sync.enabled}
                onChange={(e) => setSync({ ...sync, enabled: e.target.checked })}
              />
              <Localized id="settings-sync-enabled">
                <span>Enable Cloud Sync</span>
              </Localized>
            </label>

            <div className="settings-actions">
              <Button
                variant="secondary"
                loading={syncing}
                onClick={async () => {
                  setSyncing(true);
                  try {
                    const result = await triggerSync();
                    setSyncResult(result);
                  } catch {
                    setSyncResult({ synced: 0, failed: 0, error: 'Sync failed' });
                  } finally {
                    setSyncing(false);
                  }
                }}
              >
                <Localized id={syncing ? 'settings-sync-syncing' : 'settings-sync-sync-now'}>
                  <span>{syncing ? 'Syncing…' : 'Sync Now'}</span>
                </Localized>
              </Button>
            </div>

            {syncResult && (
              <p className="settings-hint">
                <Localized
                  id="settings-sync-result"
                  vars={{ synced: syncResult.synced, failed: syncResult.failed }}
                >
                  <span>Last sync: {syncResult.synced} synced, {syncResult.failed} failed</span>
                </Localized>
              </p>
            )}
          </div>
        )}
      </Card>
    </div>
  );
}
