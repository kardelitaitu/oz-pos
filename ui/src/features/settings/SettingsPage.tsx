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
import { getVersion } from '@/api/system';
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
import { useToast } from '@/frontend/shared/Toast';
import Tooltip from '@/frontend/shell/Tooltip';
import { useTheme } from '@/frontend/shell/ThemeProvider';
import { AppearanceSettings } from './AppearanceSettings';
import FeatureToggleScreen from './FeatureToggleScreen';
import DataManagementScreen from './DataManagementScreen';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import { MultiStoreDashboardScreen } from '@/features/stores';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';
import ShiftManagementScreen from '@/features/shifts/ShiftManagementScreen';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';
import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';
import PromotionManagementScreen from '@/features/promotions/PromotionManagementScreen';
import LicenseSettings from './LicenseSettings';
import './SettingsPage.css';

// ── Sidebar nav item type ─────────────────────────────────────────

interface SettingsNavItem {
  key: string;
  label: string;
  icon: React.ReactNode;
}

const NAV_ITEMS: SettingsNavItem[] = [
  {
    key: 'general',
    label: 'General',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="3" y="3" width="7" height="7" />
        <rect x="14" y="3" width="7" height="7" />
        <rect x="3" y="14" width="7" height="7" />
        <rect x="14" y="14" width="7" height="7" />
      </svg>
    ),
  },
  {
    key: 'appearance',
    label: 'Appearance',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
      </svg>
    ),
  },
  {
    key: 'receipt',
    label: 'Receipt',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
      </svg>
    ),
  },
  {
    key: 'sync',
    label: 'Cloud Sync',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
  {
    key: 'about',
    label: 'About',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="16" x2="12" y2="12" />
        <line x1="12" y1="8" x2="12.01" y2="8" />
      </svg>
    ),
  },
  {
    key: 'features',
    label: 'Features',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M13 2 3 14h9l-1 8 10-12h-9z" />
      </svg>
    ),
  },
  {
    key: 'license',
    label: 'License',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
  {
    key: 'data',
    label: 'Data',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <ellipse cx="12" cy="5" rx="9" ry="3" />
        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
      </svg>
    ),
  },
  {
    key: 'staff',
    label: 'Staff',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
        <circle cx="9" cy="7" r="4" />
        <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
        <path d="M16 3.13a4 4 0 0 1 0 7.75" />
      </svg>
    ),
  },
  {
    key: 'terminals',
    label: 'Terminals',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="2" y="3" width="20" height="14" rx="2" />
        <line x1="8" y1="21" x2="16" y2="21" />
        <line x1="12" y1="17" x2="12" y2="21" />
        <path d="M7 7l3 3-3 3" />
      </svg>
    ),
  },
  {
    key: 'stores',
    label: 'Stores',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
    ),
  },
  {
    key: 'audit',
    label: 'Audit Log',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
      </svg>
    ),
  },
  {
    key: 'offline',
    label: 'Offline Queue',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
  {
    key: 'shifts',
    label: 'Shifts',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        <line x1="12" y1="1" x2="12" y2="23" />
      </svg>
    ),
  },
  {
    key: 'tax',
    label: 'Tax Rates',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        <line x1="12" y1="1" x2="12" y2="23" />
      </svg>
    ),
  },
  {
    key: 'exchange',
    label: 'Exchange Rates',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        <line x1="12" y1="1" x2="12" y2="23" />
      </svg>
    ),
  },
  {
    key: 'promotions',
    label: 'Promotions',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
      </svg>
    ),
  },
];

// ── Category groupings (accordion) ──────────────────────────────

interface SettingsCategory {
  label: string;
  keys: string[];
}

const CATEGORIES: SettingsCategory[] = [
  { label: 'Business', keys: ['general', 'appearance'] },
  { label: 'Operations', keys: ['receipt', 'sync'] },
  { label: 'System', keys: ['about', 'license', 'features', 'data'] },
  { label: 'Management', keys: ['staff', 'terminals', 'stores', 'audit', 'offline', 'shifts', 'tax', 'exchange', 'promotions'] },
];

// ── Clock helper ──────────────────────────────────────────────────

function useClock(): string {
  const [clock, setClock] = useState(() =>
    new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
  );
  useEffect(() => {
    const id = setInterval(
      () => setClock(new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })),
      60_000,
    );
    return () => clearInterval(id);
  }, []);
  return clock;
}

function useDate(): string {
  const [date, setDate] = useState(() =>
    new Date().toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' }),
  );
  useEffect(() => {
    const id = setInterval(
      () => setDate(new Date().toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' })),
      60_000,
    );
    return () => clearInterval(id);
  }, []);
  return date;
}

// ── Component ─────────────────────────────────────────────────────

/** Settings hub — sidebar-driven navigation across general, appearance, features, data management, staff, terminals, multi-store, audit, offline queue, shifts, tax, currency, and promotions. */
export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const [appVersion, setAppVersion] = useState('');

  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { refreshBrandSettings, settings: brandSettings } = useBrand();
  const { theme, toggleTheme } = useTheme();

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

  // ── Sidebar navigation state ─────────────────────────────────
  const [activeSection, setActiveSection] = useState('general');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() =>
    localStorage.getItem('settings-sidebar-collapsed') === 'true',
  );

  useEffect(() => {
    localStorage.setItem('settings-sidebar-collapsed', String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  // ── Accordion: single expanded category (persisted) ──────────
  const [expandedCategory, setExpandedCategory] = useState<string | null>(() => {
    const stored = localStorage.getItem('settings-sidebar-expanded');
    if (stored) return stored;
    // Default: expand the first category containing the default section
    return 'Business';
  });

  useEffect(() => {
    if (expandedCategory) {
      localStorage.setItem('settings-sidebar-expanded', expandedCategory);
    } else {
      localStorage.removeItem('settings-sidebar-expanded');
    }
  }, [expandedCategory]);

  const toggleCategory = useCallback((label: string) => {
    setExpandedCategory((prev) => (prev === label ? null : label));
  }, []);

  const clock = useClock();
  const today = useDate();

  // Sync font-smoothing to <html> whenever it changes
  useEffect(() => {
    document.documentElement.setAttribute('data-font-smoothing', displayFontSmoothing);
  }, [displayFontSmoothing]);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const [r, s, currenciesData, defaultCurrencyData, syncData, prefs, brand, ver] = await Promise.all([
        getReceiptSettings(),
        getStoreSettings(),
        listCurrencies(),
        getDefaultCurrency(),
        getSyncSettings(),
        getUserPreferences(userId),
        getBrandSettings(),
        getVersion(),
      ]);
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
      setAppVersion(ver.version);
      const palette = deriveAccentPalette(brand.primary_colour);
      applyAccentPalette(palette);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : 'Failed to load settings');
    } finally {
      setLoading(false);
    }
  }, [userId]);

  useEffect(() => { load(); }, [load]);

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
      addToast({ message: l10n.getString('settings-save-error'), type: 'error' });
    } finally {
      setSaving(false);
    }
  }, [receipt, store, defaultCurrency, userId, session?.user_id, sync.enabled, syncServerUrl, syncApiKey, displayCardSize, displayFontSize, displayFontSmoothing, brandColour, brandStoreName, addToast, l10n, refreshBrandSettings]);

  // ── Loading / Error states ───────────────────────────────────

  if (loading) {
    return <div className="settings-page" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Localized id="settings-loading"><p>Loading settings&hellip;</p></Localized></div>;
  }

  if (loadError) {
    return (
      <div className="settings-page" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div className="settings-error" role="alert">
          <p>{loadError}</p>
          <Button variant="secondary" onClick={() => { setLoadError(null); setLoading(true); load(); }}>
            <Localized id="settings-retry"><span>Retry</span></Localized>
          </Button>
        </div>
      </div>
    );
  }

  // ── Render section content ───────────────────────────────────

  function renderSection(key: string) {
    switch (key) {
      case 'general':
        return (
          <>
            {/* ── Store section ──────────────────────── */}
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

            {/* ── Currency section ──────────────────── */}
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
          </>
        );

      case 'appearance':
        return (
          <>
            {/* ── Display section ────────────────────── */}
            <Card
              shadow="sm"
              header={<Localized id="settings-section-display"><h2 className="settings-section-title">Display</h2></Localized>}
            >
              <div className="settings-form">
                <div className="settings-field">
                  <Localized id="settings-field-card-size">
                    <span className="settings-label">Menu Card Size</span>
                  </Localized>
                  <div className="settings-size-controls">
                    <Localized id="settings-card-size-decrease-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="settings-size-btn"
                        disabled={displayCardSize <= 0}
                        onClick={() => setDisplayCardSize((s) => Math.max(0, s - 1))}
                        aria-label="Decrease card size"
                      >
                        &minus;
                      </button>
                    </Localized>
                    <span className="settings-size-value">{displayCardSize}</span>
                    <Localized id="settings-card-size-increase-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="settings-size-btn"
                        disabled={displayCardSize >= 4}
                        onClick={() => setDisplayCardSize((s) => Math.min(4, s + 1))}
                        aria-label="Increase card size"
                      >
                        +
                      </button>
                    </Localized>
                  </div>
                </div>

                <div className="settings-field">
                  <Localized id="settings-field-font-size">
                    <span className="settings-label">Font Size</span>
                  </Localized>
                  <div className="settings-size-controls">
                    <Localized id="settings-font-size-decrease-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="settings-size-btn"
                        disabled={displayFontSize <= 0}
                        onClick={() => setDisplayFontSize((s) => Math.max(0, s - 1))}
                        aria-label="Decrease font size"
                      >
                        &minus;
                      </button>
                    </Localized>
                    <span className="settings-size-value">{displayFontSize}</span>
                    <Localized id="settings-font-size-increase-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="settings-size-btn"
                        disabled={displayFontSize >= 4}
                        onClick={() => setDisplayFontSize((s) => Math.min(4, s + 1))}
                        aria-label="Increase font size"
                      >
                        +
                      </button>
                    </Localized>
                  </div>
                </div>

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

            {/* ── Appearance section ────────────────── */}
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
          </>
        );

      case 'receipt':
        return (
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
        );

      case 'sync':
        return (
          <Card
            shadow="sm"
            header={<Localized id="settings-section-sync"><h2 className="settings-section-title">Cloud Sync</h2></Localized>}
          >
            <div className="settings-form">
              {sync.serverUrl === null && !sync.enabled && (
                <p className="settings-hint">
                  <Localized id="settings-sync-not-configured">
                    <span>Sync is not configured. Enter a server URL and enable sync.</span>
                  </Localized>
                </p>
              )}

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

              {(sync.serverUrl !== null || sync.enabled) && (
                <>
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
                    <div className="settings-sync-result-block">
                      <p className="settings-hint">
                        <Localized
                          id="settings-sync-result"
                          vars={{ synced: syncResult.synced, failed: syncResult.failed }}
                        >
                          <span>Last sync: {syncResult.synced} synced, {syncResult.failed} failed</span>
                        </Localized>
                      </p>
                      {syncResult.error && (
                        <p className="settings-hint settings-hint--error">{syncResult.error}</p>
                      )}
                    </div>
                  )}
                </>
              )}
            </div>
          </Card>
        );

      case 'about':
        return (
          <Card
            shadow="sm"
            header={<Localized id="settings-system-license-header"><h2 className="settings-section-title">System &amp; License Ownership</h2></Localized>}
          >
            <div className="settings-form settings-license-section">
              <div className="settings-license-row">
                <Localized id="settings-software-edition"><span className="settings-license-label">Software Edition</span></Localized>
                <Localized id="settings-app-version" vars={{ version: appVersion }}>
                  <span className="settings-license-value">OZ-POS Enterprise v{appVersion}</span>
                </Localized>
              </div>
              <div className="settings-license-row">
                <Localized id="settings-license-type"><span className="settings-license-label">License Type</span></Localized>
                <Localized id="settings-license-type-value">
                  <span className="settings-license-value settings-license-value--warning">Proprietary Commercial License</span>
                </Localized>
              </div>
              <div className="settings-license-row">
                <Localized id="settings-copyright-notice"><span className="settings-license-label">Copyright Notice</span></Localized>
                <Localized id="settings-copyright-notice-value">
                  <span className="settings-license-value settings-license-value--medium">&copy; 2024-2026 OZ-POS Contributors. All Rights Reserved.</span>
                </Localized>
              </div>
              <div className="settings-license-row settings-license-row--last">
                <Localized id="settings-commercial-contact"><span className="settings-license-label">Commercial Contact</span></Localized>
                <span className="settings-license-value settings-license-value--mono">adikaradwiatmaja@gmail.com</span>
              </div>
            </div>
          </Card>
        );

      case 'license':
        return <LicenseSettings />;

      case 'features':
        return <FeatureToggleScreen />;

      case 'data':
        return <DataManagementScreen />;

      case 'staff':
        return <StaffManagementScreen />;

      case 'terminals':
        return <TerminalManagementScreen />;

      case 'stores':
        return <MultiStoreDashboardScreen />;

      case 'audit':
        return <AuditLogScreen />;

      case 'offline':
        return <OfflineQueueScreen />;

      case 'shifts':
        return <ShiftManagementScreen />;

      case 'tax':
        return <TaxConfigurationScreen />;

      case 'exchange':
        return <ExchangeRateScreen />;

      case 'promotions':
        return <PromotionManagementScreen />;

      default:
        return null;
    }
  }

  // ── Main render ──────────────────────────────────────────────

  return (
    <div className="settings-page">
      {/* ── Top bar ────────────────────────────────────── */}
      <header className="settings-topbar">
        <div className="settings-topbar-left">
          <Tooltip content={brandSettings.store_name || 'OZ-POS'}>
            <div className="settings-topbar-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="3" width="18" height="14" rx="2" />
                <line x1="3" y1="10" x2="21" y2="10" />
                <line x1="7" y1="15" x2="9" y2="15" />
                <line x1="15" y1="15" x2="17" y2="15" />
              </svg>
            </div>
          </Tooltip>
          <span className="settings-topbar-name">Settings</span>
        </div>
        <div className="settings-topbar-right">
          <span className="settings-topbar-clock" aria-label={`${today}, ${clock}`}>
            {today} {clock}
          </span>
          <div className="settings-save-bar">
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
      </header>

      {/* ── Body ──────────────────────────────────────────── */}
      <div className="settings-body">
        {/* ── Sidebar ────────────────────────────────── */}
        <aside
          className={`settings-sidebar${sidebarCollapsed ? ' collapsed' : ''}`}
          aria-label="Settings navigation"
        >
          <div className="settings-sidebar-header">
            <button
              type="button"
              className="settings-sidebar-toggle"
              onClick={() => setSidebarCollapsed((p) => !p)}
              aria-label={sidebarCollapsed ? 'Expand settings sidebar' : 'Collapse settings sidebar'}
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                width="16"
                height="16"
                aria-hidden="true"
              >
                <polyline points={sidebarCollapsed ? '9 18 15 12 9 6' : '15 18 9 12 15 6'} />
              </svg>
            </button>
          </div>

          <nav className="settings-sidebar-nav">
            {CATEGORIES.map((cat) => {
              const isExpanded = expandedCategory === cat.label;
              const hasActive = cat.keys.includes(activeSection);
              return (
                <div key={cat.label} className="settings-sidebar-section">
                  <button
                    type="button"
                    className={`settings-sidebar-section-header${hasActive ? ' settings-sidebar-section-header--active' : ''}`}
                    onClick={() => toggleCategory(cat.label)}
                    aria-expanded={isExpanded}
                  >
                    <span className="settings-sidebar-section-label">{cat.label}</span>
                    <svg
                      className={`settings-sidebar-chevron${isExpanded ? '' : ' collapsed'}`}
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      width="12"
                      height="12"
                      aria-hidden="true"
                    >
                      <polyline points="9 18 15 12 9 6" />
                    </svg>
                  </button>
                  {isExpanded && (
                    <div className="settings-sidebar-section-items">
                      {cat.keys.map((key) => {
                        const item = NAV_ITEMS.find((n) => n.key === key)!;
                        return (
                          <Tooltip key={key} content={item.label} showDelay={800}>
                            <button
                              type="button"
                              className={`settings-nav-item${activeSection === key ? ' settings-nav-item--active' : ''}`}
                              onClick={() => setActiveSection(key)}
                              aria-current={activeSection === key ? 'page' : undefined}
                              aria-label={item.label}
                            >
                              <span className="settings-nav-icon">{item.icon}</span>
                              <span className="settings-nav-label">{item.label}</span>
                            </button>
                          </Tooltip>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </nav>


        </aside>

        {/* ── Main content ──────────────────────────────── */}
        <main className="settings-content">
          {renderSection(activeSection)}
        </main>
      </div>

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="settings-footer">
        <span className="settings-footer-left">
          <button
            type="button"
            className="settings-footer-theme-toggle"
            onClick={toggleTheme}
            aria-label={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
          >
            {theme === 'light' ? (
              /* Moon icon (click to go dark) */
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
            ) : (
              /* Sun icon (click to go light) */
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
            )}
          </button>
          OZ-POS Enterprise v{appVersion}
        </span>
        <span className="settings-footer-right">
          <Localized id="settings-license-type-value">
            <span>Proprietary Commercial License</span>
          </Localized>
        </span>
      </footer>
    </div>
  );
}
