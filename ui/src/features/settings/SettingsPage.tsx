import { useEffect, useState, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  getUserPreferences,
  setUserPreferences,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
} from '@/api/settings';
import { setDecimalSep } from '@/utils/storage';
import { useAuth } from '@/contexts/AuthContext';
import {
  listCurrencies,
  getDefaultCurrency,
  setDefaultCurrency,
  type CurrencyDto,
} from '@/api/currency';
import {
  getSyncSettings,
  updateSyncSettings,
  syncRun,
  type SyncSettingsDto,
  type SyncAttemptResult,
} from '@/api/offline';
import {
  getBrandSettings,
  setBrandPrimaryColour,
  setBrandStoreName as setBrandStoreNameApi,
} from '@/api/branding';
import { useBrand } from '@/contexts/BrandContext';
import { deriveAccentPalette, applyAccentPalette } from '@/utils/color';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { LanguageSelector } from '@/i18n/LanguageSelector';
import { useToast } from '@/hooks/useToast';
import { AppearanceSettings } from './AppearanceSettings';
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

  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { refreshBrandSettings } = useBrand();

  const [receipt, setReceipt] = useState<ReceiptSettingsDto>({
    showCurrency: false,
    decimalSeparator: 'dot',
    showTax: true,
    footer: '',
    paperWidth: 'standard',
    showTableNumber: false,
    marginTop: 0,
    marginBottom: 0,
    marginLeft: 0,
    marginRight: 0,
  });

  const [store, setStore] = useState<StoreSettingsDto>({
    name: '',
    address: '',
    taxId: '',
    currency: 'IDR',
    branch: '',
    logo: '',
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
  const { session } = useAuth();
  const userId = session?.user_id ?? 'default';
  const [displayCardSize, setDisplayCardSize] = useState(0);
  const [displayFontSize, setDisplayFontSize] = useState(0);
  const [displayFontSmoothing, setDisplayFontSmoothing] = useState('antialiased');
  const [brandColour, setBrandColour] = useState('#10b981');
  const [brandStoreName, setBrandStoreName] = useState('');

  // Sync font-smoothing to <html> whenever it changes
  useEffect(() => {
    document.documentElement.setAttribute('data-font-smoothing', displayFontSmoothing);
  }, [displayFontSmoothing]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [r, s, currenciesData, defaultCurrencyData, syncData, prefs, brand] = await Promise.all([
          getReceiptSettings(),
          getStoreSettings(),
          listCurrencies(),
          getDefaultCurrency(),
          getSyncSettings(),
          getUserPreferences(userId),
          getBrandSettings(),
        ]);
        if (!cancelled) {
          setReceipt(r);
          setStore(s);
          setDecimalSep(r.decimalSeparator);
          setCurrencies(currenciesData);
          setDefaultCurrencyState(defaultCurrencyData ?? 'USD');
          setSync(syncData);
          setSyncServerUrl(syncData.serverUrl ?? '');
          const cs = prefs['cardsize'];
          if (cs !== undefined) setDisplayCardSize(Math.min(4, Math.max(0, parseInt(cs, 10) || 0)));
          const fs = prefs['fontsize'];
          if (fs !== undefined) setDisplayFontSize(Math.min(4, Math.max(0, parseInt(fs, 10) || 0)));
          if (prefs['font-smoothing'] !== undefined) setDisplayFontSmoothing(prefs['font-smoothing']);
          setBrandColour(brand.primary_colour);
          setBrandStoreName(brand.store_name);
          const palette = deriveAccentPalette(brand.primary_colour);
          applyAccentPalette(palette);
        }
      } catch {
        // IPC unavailable — use defaults.
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [userId]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaved(false);
    try {
      await Promise.all([
        setReceiptSettings(receipt, session?.user_id ?? ''),
        setStoreSettings(store, session?.user_id ?? ''),
        setDefaultCurrency({ code: defaultCurrency }),
        setUserPreferences(userId, [
          { key: 'cardsize', value: String(displayCardSize) },
          { key: 'fontsize', value: String(displayFontSize) },
          { key: 'font-smoothing', value: displayFontSmoothing },
        ]),
        updateSyncSettings({
          serverUrl: syncServerUrl || null,
          apiKey: syncApiKey || null,
          enabled: sync.enabled,
        }),
        setBrandPrimaryColour(brandColour),
        setBrandStoreNameApi(brandStoreName),
      ]);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      setSyncApiKey('');
      refreshBrandSettings();
    } catch {
      addToast(l10n.getString('settings-save-error'), 'error');
    } finally {
      setSaving(false);
    }
  }, [receipt, store, defaultCurrency, userId, displayCardSize, displayFontSize, displayFontSmoothing, brandColour, brandStoreName, addToast, l10n, refreshBrandSettings]);

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
          <label className="settings-field" htmlFor="settings-field-store-name">
            {l10n.getString('settings-field-store-name')}
            <Localized id="settings-store-name-placeholder" attrs={{ placeholder: true }}>
              <input
                className="settings-input"
                type="text"
                id="settings-field-store-name"
                placeholder="OZ-POS Store"
                value={store.name}
                onChange={(e) => setStore({ ...store, name: e.target.value })}
              />
            </Localized>
          </label>

          <label className="settings-field" htmlFor="settings-field-address">
            {l10n.getString('settings-field-address')}
            <Localized id="settings-address-placeholder" attrs={{ placeholder: true }}>
              <input
                className="settings-input"
                type="text"
                id="settings-field-address"
                placeholder="123 Main Street"
                value={store.address}
                onChange={(e) => setStore({ ...store, address: e.target.value })}
              />
            </Localized>
          </label>

          <label className="settings-field" htmlFor="settings-field-tax-id">
            {l10n.getString('settings-field-tax-id')}
            <Localized id="settings-tax-id-placeholder" attrs={{ placeholder: true }}>
              <input
                className="settings-input"
                type="text"
                id="settings-field-tax-id"
                placeholder="12-3456789"
                value={store.taxId}
                onChange={(e) => setStore({ ...store, taxId: e.target.value })}
              />
            </Localized>
          </label>

          <div className="settings-field">
            <Localized id="settings-field-language">
              <span className="settings-label"><span>Language</span></span>
            </Localized>
            <LanguageSelector />
          </div>
        </div>
      </Card>

      {/* ── Currency section ────────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-currency"><h2 className="settings-section-title">Currency</h2></Localized>}
      >
        <div className="settings-form">
          <label className="settings-field" htmlFor="settings-field-default-currency">
            <Localized id="settings-field-default-currency">
              <span className="settings-label">Default currency</span>
            </Localized>
            <select
              className="settings-select"
              id="settings-field-default-currency"
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

      {/* ── Display section ──────────────────────── */}
      <Card
        shadow="sm"
        header={
          <Localized id="settings-section-display">
            <h2 className="settings-section-title">Display</h2>
          </Localized>
        }
      >
        <div className="settings-form">
          <label className="settings-field" htmlFor="settings-field-card-size">
            <Localized id="settings-field-card-size">
              <span className="settings-label">Menu Card Size</span>
            </Localized>
            <div className="settings-size-controls">
              <button
                type="button"
                className="settings-size-btn"
                disabled={displayCardSize <= 0}
                onClick={() => setDisplayCardSize((s) => Math.max(0, s - 1))}
                aria-label="Decrease card size"
              >
                &minus;
              </button>
              <span className="settings-size-value">{displayCardSize}</span>
              <button
                type="button"
                className="settings-size-btn"
                disabled={displayCardSize >= 4}
                onClick={() => setDisplayCardSize((s) => Math.min(4, s + 1))}
                aria-label="Increase card size"
              >
                +
              </button>
            </div>
          </label>

          <label className="settings-field" htmlFor="settings-field-font-size">
            <Localized id="settings-field-font-size">
              <span className="settings-label">Font Size</span>
            </Localized>
            <div className="settings-size-controls">
              <button
                type="button"
                className="settings-size-btn"
                disabled={displayFontSize <= 0}
                onClick={() => setDisplayFontSize((s) => Math.max(0, s - 1))}
                aria-label="Decrease font size"
              >
                &minus;
              </button>
              <span className="settings-size-value">{displayFontSize}</span>
              <button
                type="button"
                className="settings-size-btn"
                disabled={displayFontSize >= 4}
                onClick={() => setDisplayFontSize((s) => Math.min(4, s + 1))}
                aria-label="Increase font size"
              >
                +
              </button>
            </div>
          </label>

          <div className="settings-field">
            <Localized id="settings-field-font-smoothing">
              <span className="settings-label">Font Smoothing</span>
            </Localized>
            <select
              className="settings-select"
              id="settings-field-font-smoothing"
              value={displayFontSmoothing}
              onChange={(e) => setDisplayFontSmoothing(e.target.value)}
              aria-label={l10n.getString('settings-field-font-smoothing')}
            >
              <Localized id="settings-font-smoothing-antialiased">
                <option value="antialiased">Antialiased (crisp)</option>
              </Localized>
              <Localized id="settings-font-smoothing-subpixel">
                <option value="subpixel">Subpixel (smooth)</option>
              </Localized>
            </select>
          </div>
        </div>
      </Card>

      {/* ── Appearance section ──────────────────── */}
      <AppearanceSettings
        embedded
        colour={brandColour}
        storeName={brandStoreName}
        onColourChange={(c) => {
          setBrandColour(c);
          const palette = deriveAccentPalette(c);
          applyAccentPalette(palette);
        }}
        onStoreNameChange={setBrandStoreName}
      />

      {/* ── Receipt section ───────────────────────── */}
      <Card
        shadow="sm"
        header={<Localized id="settings-section-receipt"><h2 className="settings-section-title">Receipt</h2></Localized>}
      >
        <div className="settings-form">
          {/* Show currency */}
          <label className="settings-toggle" htmlFor="settings-toggle-show-currency">
            <Localized id="settings-toggle-show-currency-aria" attrs={{ 'aria-label': true }}>
              <input
                type="checkbox"
                id="settings-toggle-show-currency"
                checked={receipt.showCurrency}
                onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
                aria-label="Show currency symbol on amounts"
              />
            </Localized>
            <Localized id="settings-toggle-show-currency">
              <span>Show currency symbol on amounts</span>
            </Localized>
          </label>

          {/* Decimal separator */}
          <label className="settings-field" htmlFor="settings-field-decimal-separator">
            {l10n.getString('settings-field-decimal-separator')}
            <select
              className="settings-select"
              id="settings-field-decimal-separator"
              value={receipt.decimalSeparator}
              onChange={(e) => {
                setReceipt({ ...receipt, decimalSeparator: e.target.value });
                setDecimalSep(e.target.value);
              }}
            >
              <Localized id="settings-decimal-separator-dot">
                <option value="dot">1.00 (dot)</option>
              </Localized>
              <Localized id="settings-decimal-separator-comma">
                <option value="comma">1,00 (comma)</option>
              </Localized>
              <Localized id="settings-decimal-separator-none">
                <option value="none">1 (none)</option>
              </Localized>
            </select>
          </label>

          {/* Show tax */}
          <label className="settings-toggle" htmlFor="settings-toggle-show-tax">
            <Localized id="settings-toggle-show-tax-aria" attrs={{ 'aria-label': true }}>
              <input
                type="checkbox"
                id="settings-toggle-show-tax"
                checked={receipt.showTax}
                onChange={(e) => setReceipt({ ...receipt, showTax: e.target.checked })}
                aria-label="Show tax line on receipts"
              />
            </Localized>
            <Localized id="settings-toggle-show-tax">
              <span>Show tax line on receipts</span>
            </Localized>
          </label>

          {/* Paper width */}
          <label className="settings-field" htmlFor="settings-field-paper-width">
            {l10n.getString('settings-field-paper-width')}
            <select
              className="settings-select"
              id="settings-field-paper-width"
              value={receipt.paperWidth}
              onChange={(e) => setReceipt({ ...receipt, paperWidth: e.target.value })}
            >
              <Localized id="settings-paper-width-standard">
                <option value="standard">80 mm (standard)</option>
              </Localized>
              <Localized id="settings-paper-width-narrow">
                <option value="narrow">58 mm (narrow)</option>
              </Localized>
            </select>
          </label>

          {/* Footer */}
          <label className="settings-field" htmlFor="settings-field-receipt-footer">
            {l10n.getString('settings-field-footer')}
            <Localized id="settings-footer-placeholder" attrs={{ placeholder: true }}>
              <input
                className="settings-input"
                type="text"
                id="settings-field-receipt-footer"
                placeholder="Thank you for shopping!"
                value={receipt.footer}
                onChange={(e) => setReceipt({ ...receipt, footer: e.target.value })}
              />
            </Localized>
          </label>

          {/* Show table number */}
          <label className="settings-toggle" htmlFor="settings-toggle-show-table-number">
            <Localized id="settings-toggle-show-table-number-aria" attrs={{ 'aria-label': true }}>
              <input
                type="checkbox"
                id="settings-toggle-show-table-number"
                checked={receipt.showTableNumber}
                onChange={(e) => setReceipt({ ...receipt, showTableNumber: e.target.checked })}
                aria-label="Show table number on cart and receipts"
              />
            </Localized>
            <Localized id="settings-toggle-show-table-number">
              <span>Show table number on cart and receipts</span>
            </Localized>
          </label>

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
            <label className="settings-field" htmlFor="settings-field-server-url">
              {l10n.getString('settings-sync-server-url')}
              <Localized id="settings-server-url-placeholder" attrs={{ placeholder: true }}>
                <input
                  className="settings-input"
                  type="url"
                  id="settings-field-server-url"
                  placeholder="https://api.example.com"
                  value={syncServerUrl}
                  onChange={(e) => setSyncServerUrl(e.target.value)}
                />
              </Localized>
            </label>

            <label className="settings-field" htmlFor="settings-field-api-key">
              {l10n.getString('settings-sync-api-key')}
              <Localized id="settings-api-key-placeholder" attrs={{ placeholder: true }}>
                <input
                  className="settings-input"
                  type="password"
                  id="settings-field-api-key"
                  placeholder="Enter API key"
                  value={syncApiKey}
                  onChange={(e) => setSyncApiKey(e.target.value)}
                />
              </Localized>
            </label>

            <label className="settings-toggle" htmlFor="settings-toggle-sync-enabled">
              <Localized id="settings-sync-enabled-aria" attrs={{ 'aria-label': true }}>
                <input
                  type="checkbox"
                  id="settings-toggle-sync-enabled"
                  checked={sync.enabled}
                  onChange={(e) => setSync({ ...sync, enabled: e.target.checked })}
                  aria-label="Enable Cloud Sync"
                />
              </Localized>
              <Localized id="settings-sync-enabled">
                <span>Enable Cloud Sync</span>
              </Localized>
            </label>
          </div>
        ) : (
          <div className="settings-form">
            <label className="settings-field" htmlFor="settings-field-server-url">
              {l10n.getString('settings-sync-server-url')}
              <Localized id="settings-server-url-placeholder" attrs={{ placeholder: true }}>
                <input
                  className="settings-input"
                  type="url"
                  id="settings-field-server-url"
                  placeholder="https://api.example.com"
                  value={syncServerUrl}
                  onChange={(e) => setSyncServerUrl(e.target.value)}
                />
              </Localized>
            </label>

            <label className="settings-field" htmlFor="settings-field-api-key">
              {l10n.getString('settings-sync-api-key')}
              <Localized id={sync.hasApiKey ? 'settings-api-key-masked' : 'settings-api-key-placeholder'} attrs={{ placeholder: true }}>
                <input
                  className="settings-input"
                  type="password"
                  id="settings-field-api-key"
                  placeholder={sync.hasApiKey ? '••••••••' : 'Enter API key'}
                  value={syncApiKey}
                  onChange={(e) => setSyncApiKey(e.target.value)}
                />
              </Localized>
            </label>

            <label className="settings-toggle" htmlFor="settings-toggle-sync-enabled">
              <Localized id="settings-sync-enabled-aria" attrs={{ 'aria-label': true }}>
                <input
                  type="checkbox"
                  id="settings-toggle-sync-enabled"
                  checked={sync.enabled}
                  onChange={(e) => setSync({ ...sync, enabled: e.target.checked })}
                  aria-label="Enable Cloud Sync"
                />
              </Localized>
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
                    const result = await syncRun();
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

      {/* ── Page-level Save ───────────────────────── */}
      <div className="settings-page-actions">
        <Localized id="settings-btn-save-aria" attrs={{ 'aria-label': true }} vars={{ state: saved ? 'saved' : 'save' }}>
          <Button
            variant="primary"
            loading={saving}
            onClick={handleSave}
            aria-label={saved ? 'Saved!' : 'Save settings'}
          >
            <Localized id={saved ? 'settings-saved' : 'settings-btn-save'}>
              <span>{saved ? 'Saved!' : 'Save'}</span>
            </Localized>
          </Button>
        </Localized>
      </div>
    </div>
  );
}
