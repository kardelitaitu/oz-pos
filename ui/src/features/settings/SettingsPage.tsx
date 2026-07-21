import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
import { useCurrency } from '@/contexts/CurrencyContext';
import {
  listCurrencies,
  type CurrencyDto,
} from '@/api/currency';
import {
  getSyncSettings,
  updateSyncSettings,
  syncRun,
  syncPull,
  pendingSyncCount,
  testSyncConnection,
  requestSyncToken,
  type SyncSettingsDto,
  type SyncAttemptResult,
  type PullResult,
  type PingResult,
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
import { Skeleton } from '@/components/Skeleton';
import { LanguageSelector } from '@/i18n/LanguageSelector';
import SettingsSelect from './SettingsSelect';
import { useToast } from '@/frontend/shared/Toast';
import { useTheme } from '@/frontend/shell/ThemeProvider';
import { useKeyboardAvoidance } from '@/hooks/useKeyboardAvoidance';
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
import EmailReportSettings from './EmailReportSettings';
import { useContextMenu, ContextMenu } from '@/frontend/shared';
import SettingsNavTree, {
  NAV_ITEMS as NAV_ITEMS_REF,
  CATEGORIES as CATEGORIES_REF,
  CATEGORY_I18N_KEYS as CATEGORY_I18N_KEYS_REF,
  NAV_L10N_KEYS as NAV_L10N_KEYS_REF,
} from './SettingsNavTree';
import './SettingsPage.css';
import './SettingsNavTree.css';

/** Snapshot of initial loaded values for the Revert-to-saved button. */
interface SettingsSnapshot {
  receipt: ReceiptSettingsDto;
  store: StoreSettingsDto;
  defaultCurrency: string;
  sync: SyncSettingsDto;
  syncServerUrl: string;
  displayCardSize: number;
  displayFontSize: number;
  displayFontSmoothing: string;
  brandColour: string;
  brandStoreName: string;
}

// ── Clock helper ──────────────────────────────────────────────────

function useClock(): string {
  const [clock, setClock] = useState(() =>
    new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
  );
  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | undefined;
    // Align the first tick to the next minute boundary so the clock
    // is accurate from the start rather than drifting by mount time.
    const now = new Date();
    const msUntilNextMinute =
      (60 - now.getSeconds()) * 1000 - now.getMilliseconds();
    const timeout = setTimeout(() => {
      const tick = () =>
        setClock(
          new Date().toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit',
          }),
        );
      tick();
      intervalId = setInterval(tick, 60_000);
    }, msUntilNextMinute);
    return () => {
      clearTimeout(timeout);
      if (intervalId) clearInterval(intervalId);
    };
  }, []);
  return clock;
}

/** Return today's formatted date. The date only changes at midnight and
 *  the settings page is not expected to stay open across day boundaries,
 *  so we compute once at mount rather than polling every 60 seconds. */
function getToday(): string {
  return new Date().toLocaleDateString(undefined, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}

// ── Component ─────────────────────────────────────────────────────

// ── Token expiry badge helper ───────────────────────────────────

/** Structured expiry info for a JWT token. The UI uses Fluent keys
 *  to render the badge text, making it localisable. */
interface ExpiryInfo {
  /** Fluent key — one of the `settings-sync-expiry-*` keys. */
  fluentKey: string;
  /** Argument object passed to `l10n.getString()`. */
  fluentArgs: Record<string, number | string>;
  /** Urgency level for the badge colour. */
  tone: 'good' | 'warn' | 'critical';
}

/** Compute a localisable expiry label and urgency colour for a JWT token.
 *  Returns `null` when no `expiresAt` is provided. */
function formatTokenExpiry(expiresAt: string | null): ExpiryInfo | null {
  if (!expiresAt) return null;
  const now = Date.now();
  const expiry = Date.parse(expiresAt);
  if (Number.isNaN(expiry)) {
    return { fluentKey: 'settings-sync-expiry-fallback', fluentArgs: { iso: expiresAt }, tone: 'warn' };
  }
  const diffMs = expiry - now;
  if (diffMs <= 0) {
    return { fluentKey: 'settings-sync-expiry-expired', fluentArgs: {}, tone: 'critical' };
  }
  const mins = Math.floor(diffMs / 60_000);
  const hours = Math.floor(diffMs / 3_600_000);
  const days = Math.floor(diffMs / 86_400_000);
  const tone: 'good' | 'warn' | 'critical' =
    hours < 1 ? 'critical' : hours < 24 ? 'warn' : 'good';
  if (days >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-days', fluentArgs: { count: days }, tone };
  }
  if (hours >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-hours', fluentArgs: { count: hours }, tone };
  }
  if (mins >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-minutes', fluentArgs: { count: mins }, tone };
  }
  return { fluentKey: 'settings-sync-expiry-less-than-minute', fluentArgs: {}, tone: 'critical' };
}

/** Settings hub — sidebar-driven navigation across general, appearance, features, data management, staff, terminals, multi-store, audit, offline queue, shifts, tax, currency, and promotions. */
export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const [appVersion, setAppVersion] = useState('');

  // ── Updater state ─────────────────────────────────────────
  type UpdateCheckState = 'idle' | 'checking' | 'up-to-date' | 'available' | 'installing' | 'error';
  const [updateState, setUpdateState] = useState<UpdateCheckState>('idle');
  const [updateVersion, setUpdateVersion] = useState('');
  const updateInstanceRef = useRef<{ version?: string; downloadAndInstall(): Promise<void> } | null>(null);

  const handleCheckUpdates = useCallback(async () => {
    setUpdateState('checking');
    setUpdateVersion('');
    updateInstanceRef.current = null;
    try {
      const updater = await import('@tauri-apps/plugin-updater');
      const update = await updater.check();
      if (update) {
        setUpdateVersion(update.version ?? '');
        updateInstanceRef.current = update;
        setUpdateState('available');
      } else {
        setUpdateState('up-to-date');
      }
    } catch {
      setUpdateState('error');
    }
  }, []);

  const handleInstallUpdate = useCallback(async () => {
    const instance = updateInstanceRef.current;
    if (!instance) return;
    setUpdateState('installing');
    try {
      await instance.downloadAndInstall();
    } catch {
      setUpdateState('available');
    }
  }, []);

  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const { refreshBrandSettings } = useBrand();
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
  });

  const { currency: ctxCurrency, setCurrency: setCtxCurrency } = useCurrency();
  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [defaultCurrency, setDefaultCurrencyState] = useState<string>(ctxCurrency);

  // Sync local state when context currency changes externally.
  useEffect(() => { setDefaultCurrencyState(ctxCurrency); }, [ctxCurrency]);

  const [sync, setSync] = useState<SyncSettingsDto>({
    serverUrl: null,
    hasApiKey: false,
    enabled: false,
  });
  const [syncServerUrl, setSyncServerUrl] = useState('');
  const [syncApiKey, setSyncApiKey] = useState('');
  const [syncApiKeyVisible, setSyncApiKeyVisible] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncAttemptResult | null>(null);
  const [pullResult, setPullResult] = useState<PullResult | null>(null);
  const [pendingCount, setPendingCount] = useState<number | null>(null);
  const [testing, setTesting] = useState(false);
  const [pingResult, setPingResult] = useState<PingResult | null>(null);
  const [requesting, setRequesting] = useState(false);
  const [tokenExpiresAt, setTokenExpiresAt] = useState<string | null>(null);

  const { session } = useAuth();
  const userId = session?.user_id ?? 'default';

  const [displayCardSize, setDisplayCardSize] = useState(0);
  const [displayFontSize, setDisplayFontSize] = useState(0);
  const [displayFontSmoothing, setDisplayFontSmoothing] = useState('antialiased');
  const [brandColour, setBrandColour] = useState('#10b981');
  const [brandStoreName, setBrandStoreName] = useState('');

  const cm = useContextMenu();

  // P7-4: Keyboard avoidance — scroll inputs into view on mobile
  const { containerRef: settingsKeyboardRef } = useKeyboardAvoidance();

  const cmInput = useMemo(() => ({
    autoComplete: 'off' as const,
    autoCorrect: 'off' as const,
    spellCheck: false as const,
    'data-gramm': 'false' as const,
    onContextMenu: (e: React.MouseEvent<HTMLInputElement>) => cm.open(e, e.currentTarget),
  }), [cm]);

  // ── Navigation state ────────────────────────────────────────────
  const [activeSection, setActiveSection] = useState('general');
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [sectionKey, setSectionKey] = useState(0);
  const [searchQuery, setSearchQuery] = useState('');

  // ── Field validation state ────────────────────────────────
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});

  const validateField = useCallback((field: string, value: string) => {
    setFieldErrors((prev) => {
      const next = { ...prev };
      if (field === 'store-name' && !value.trim()) {
        next[field] = l10n.getString('settings-store-name-required');
      } else if (field === 'tax-id' && value.trim() && !/^[A-Za-z0-9-./]*$/.test(value.trim())) {
        next[field] = l10n.getString('settings-tax-id-pattern-error');
      } else {
        delete next[field];
      }
      return next;
    });
  }, [l10n]);

  const clearFieldError = useCallback((field: string) => {
    setFieldErrors((prev) => {
      if (!prev[field]) return prev;
      const next = { ...prev };
      delete next[field];
      return next;
    });
  }, []);

  /** Navigate to a section. */
  const navigateToSection = useCallback((key: string) => {
    setActiveSection(key);
    setMobileSidebarOpen(false);
    setSectionKey((k) => k + 1);
  }, []);

  // ── Unsaved changes tracking ────────────────────────────────
  const [isDirty, setIsDirty] = useState(false);
  const markDirty = useCallback(() => { setIsDirty(true); }, []);

  // Warn before closing the tab / window when there are unsaved changes.
  useEffect(() => {
    function handleBeforeUnload(e: BeforeUnloadEvent) {
      if (isDirty) {
        e.preventDefault();
        // WebView2 (Windows) and Chromium require returnValue to be set
        // to a non-empty string for the beforeunload dialog to appear.
        // e.preventDefault() alone is insufficient on WebView2.
        e.returnValue = '';
      }
    }
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [isDirty]);

  // ── Accordion state moved to SettingsNavTree.tsx ──────────────

  const clock = useClock();
  const today = getToday();

  // ── Snapshot for Revert-to-saved ──────────────────────────

  const initialSnapshotRef = useRef<SettingsSnapshot | null>(null);

  const handleRevert = useCallback(() => {
    const snap = initialSnapshotRef.current;
    if (!snap) return;
    setReceipt(snap.receipt);
    setStore(snap.store);
    setDecimalSep(snap.receipt.decimalSeparator);
    setDefaultCurrencyState(snap.defaultCurrency);
    setSync(snap.sync);
    setSyncServerUrl(snap.syncServerUrl);
    setDisplayCardSize(snap.displayCardSize);
    setDisplayFontSize(snap.displayFontSize);
    setDisplayFontSmoothing(snap.displayFontSmoothing);
    setBrandColour(snap.brandColour);
    setBrandStoreName(snap.brandStoreName);
    setIsDirty(false);
    setFieldErrors({});
    setSyncResult(null);
    setSyncApiKey('');
    setSyncApiKeyVisible(false);
    setTokenExpiresAt(null);
  }, []);

  // Sync font-smoothing to <html> whenever it changes
  useEffect(() => {
    document.documentElement.setAttribute('data-font-smoothing', displayFontSmoothing);
  }, [displayFontSmoothing]);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    // Use allSettled so a single failing API doesn't block the entire
    // settings page — each successful result is applied independently.
    const results = await Promise.allSettled([
      getReceiptSettings(),
      getStoreSettings(),
      listCurrencies(),
      getSyncSettings(),
      getUserPreferences(userId),
      getBrandSettings(),
      getVersion(),
    ]);
    const [rR, sR, cR, syncR, prefsR, brandR, verR] = results;

    // Local variables capture the newly-loaded values for the snapshot
    // (avoid reading React state, which would add deps and cause loops).
    let snapReceipt: ReceiptSettingsDto | undefined;
    let snapCardSize: number | undefined;
    let snapFontSize: number | undefined;
    let snapFontSmoothing: string | undefined;
    let snapBrandColour: string | undefined;
    let snapStoreName: string | undefined;

    try {
      if (rR.status === 'fulfilled') { snapReceipt = rR.value; setReceipt(rR.value); setDecimalSep(rR.value.decimalSeparator); }
      if (sR.status === 'fulfilled') setStore(sR.value);
      if (cR.status === 'fulfilled') setCurrencies(cR.value);
      if (syncR.status === 'fulfilled') { setSync(syncR.value); setSyncServerUrl(syncR.value.serverUrl ?? ''); }
      if (prefsR.status === 'fulfilled') {
        const p = prefsR.value;
        const cs = p['cardsize'];
        if (cs !== undefined) { snapCardSize = Math.min(4, Math.max(0, parseInt(cs, 10) || 0)); setDisplayCardSize(snapCardSize); }
        const fs = p['fontsize'];
        if (fs !== undefined) { snapFontSize = Math.min(4, Math.max(0, parseInt(fs, 10) || 0)); setDisplayFontSize(snapFontSize); }
        if (p['font-smoothing'] !== undefined) { snapFontSmoothing = p['font-smoothing']; setDisplayFontSmoothing(snapFontSmoothing); }
      }
      if (brandR.status === 'fulfilled') {
        snapBrandColour = brandR.value.primary_colour;
        snapStoreName = brandR.value.store_name;
        setBrandColour(snapBrandColour);
        setBrandStoreName(snapStoreName);
        const palette = deriveAccentPalette(snapBrandColour);
        applyAccentPalette(palette);
      }
      if (verR.status === 'fulfilled') setAppVersion(verR.value.version);

      // Only surface a full-page error when every single API failed.
      if (results.every((r) => r.status === 'rejected')) {
        setLoadError(l10n.getString('settings-load-failed'));
      } else if (results.some((r) => r.status === 'rejected')) {
        // Some APIs failed — page loads partially; warn the user.
        addToast({ message: l10n.getString('settings-load-partial'), type: 'error' });
      }
      // Store snapshot of initial loaded values for revert.
      // Use local variables captured from the try block above (not from
      // React state) to avoid adding these values to the deps array and
      // causing an infinite re-load loop.
      // On retry (initialSnapshotRef.current is set), preserve previous
      // snapshot values for any API that failed to avoid reverting to
      // default/empty state for backend data that is still valid.
      const prev = initialSnapshotRef.current;
      initialSnapshotRef.current = {
        receipt: rR.status === 'fulfilled' ? rR.value : (snapReceipt ?? prev?.receipt ?? receipt),
        store: sR.status === 'fulfilled' ? sR.value : (prev?.store ?? store),
        defaultCurrency: ctxCurrency,
        sync: syncR.status === 'fulfilled' ? syncR.value : (prev?.sync ?? sync),
        syncServerUrl: syncR.status === 'fulfilled' ? (syncR.value.serverUrl ?? '') : (prev?.syncServerUrl ?? syncServerUrl),
        displayCardSize: snapCardSize ?? prev?.displayCardSize ?? displayCardSize,
        displayFontSize: snapFontSize ?? prev?.displayFontSize ?? displayFontSize,
        displayFontSmoothing: snapFontSmoothing ?? prev?.displayFontSmoothing ?? displayFontSmoothing,
        brandColour: snapBrandColour ?? prev?.brandColour ?? brandColour,
        brandStoreName: snapStoreName ?? prev?.brandStoreName ?? brandStoreName,
      };
    } finally {
      setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userId, l10n, addToast]);

  useEffect(() => { load(); }, [load]);

  // Scroll content to top when navigating between sections.
  useEffect(() => {
    const el = document.querySelector<HTMLElement>('.settings-content');
    if (el) el.scrollTop = 0;
  }, [activeSection]);

  const handleSave = async () => {
    setSaving(true);
    setSaved(false);
    // Use allSettled so a single failing save doesn't silently block
    // the others — the user gets a warning about partial failures.
    const results = await Promise.allSettled([
      setReceiptSettings(receipt, session?.user_id ?? ''),
      setStoreSettings(store, session?.user_id ?? ''),
      setCtxCurrency(defaultCurrency),
      setUserPreferences(userId, [
        { key: 'cardsize', value: String(displayCardSize) },
        { key: 'fontsize', value: String(displayFontSize) },
        { key: 'font-smoothing', value: displayFontSmoothing },
      ]),
      updateSyncSettings({
        serverUrl: syncServerUrl || null,
        ...(syncApiKey ? { apiKey: syncApiKey } : {}),
        enabled: sync.enabled,
      }),
      setBrandPrimaryColour(brandColour),
      setBrandStoreNameApi(brandStoreName),
    ]);

    const failed = results.filter((r) => r.status === 'rejected').length;

    // At least one save succeeded — show confirmation and refresh.
    if (failed < results.length) {
      setIsDirty(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      // Persist the sync DTO in React state so the UI immediately
      // reflects the just-saved values (server URL, API key presence,
      // enabled flag). Without this the loaded snapshot stays stale
      // until the next page reload, causing placeholder regressions
      // like "Enter API key" after saving a new key or a blank server
      // URL field after saving a URL.
      if (results[4]?.status === 'fulfilled') {
        if (syncApiKey) {
          // Mirror the token to the shared IPC channel so the
          // Retail Options screen (useCloudSync) can load it.
          invoke('set_setting', {
            key: 'sync.auth_token',
            value: syncApiKey,
            user_id: userId,
          }).catch(() => { /* best-effort */ });
          setSyncApiKey('');
        }
        setSync((prev) => ({
          ...prev,
          serverUrl: syncServerUrl || null,
          hasApiKey: syncApiKey ? true : prev.hasApiKey,
          enabled: sync.enabled,
        }));
      }
      refreshBrandSettings();

      // Update the snapshot so Revert goes to the *saved* state.
      initialSnapshotRef.current = {
        receipt,
        store,
        defaultCurrency,
        sync,
        syncServerUrl,
        displayCardSize,
        displayFontSize,
        displayFontSmoothing,
        brandColour,
        brandStoreName,
      };
    }

    if (failed === results.length) {
      addToast({ message: l10n.getString('settings-save-error'), type: 'error' });
    } else if (failed > 0) {
      addToast({ message: l10n.getString('settings-save-partial'), type: 'error' });
    }

    setSaving(false);
  };

  // ── Sidebar search filtering moved to SettingsNavTree.tsx ─────

  // ── Cloud Sync diagnostics ──────────────────────────────────

  // Load pending offline count when sync section is active
  const refreshPendingCount = useCallback(async () => {
    try {
      const count = await pendingSyncCount();
      setPendingCount(count);
    } catch {
      setPendingCount(null);
    }
  }, []);

  useEffect(() => {
    if (activeSection === 'sync') {
      refreshPendingCount();
    }
  }, [activeSection, refreshPendingCount]);

  // ── Keyboard shortcuts ────────────────────────────────────

  // Keep a ref to the latest handleSave so the keyboard listener
  // never calls a stale closure (avoids re-binding on every render).
  const handleSaveRef = useRef(handleSave);
  handleSaveRef.current = handleSave;

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Ctrl+S / Cmd+S → save (guarded by saving flag)
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (!saving) handleSaveRef.current();
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [saving]);

  // ── Loading / Error states ───────────────────────────────────

  if (loading) {
    return (
      <div className="settings-page">
        <header className="settings-topbar">
          <div className="settings-topbar-left">
            <div className="settings-topbar-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </div>
            <span className="settings-topbar-name"><Localized id="settings-title">Settings</Localized></span>
          </div>
        </header>
        <div className="settings-body">
          <div className="settings-loading">
            <div className="settings-loading-card">
              <Skeleton variant="block" width="40%" height="1.5rem" />
              <Skeleton variant="text" width="100%" />
              <Skeleton variant="text" width="100%" />
              <Skeleton variant="text" width="60%" />
            </div>
            <div className="settings-loading-card">
              <Skeleton variant="block" width="35%" height="1.5rem" />
              <Skeleton variant="text" width="100%" />
              <Skeleton variant="text" width="80%" />
            </div>
            <div className="settings-loading-card">
              <Skeleton variant="block" width="30%" height="1.5rem" />
              <Skeleton variant="text" width="100%" />
              <Skeleton variant="text" width="50%" />
            </div>
          </div>
        </div>
      </div>
    );
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

      case 'appearance':
        return (
          <>
            {/* ── Display section ────────────────────── */}
            <Card
              shadow="sm"
              header={<Localized id="settings-section-display"><h2 className="settings-section-title">Display</h2></Localized>}
            >
              <div className="settings-form">
                <div className="settings-field settings-field--horizontal">
                  <Localized id="settings-field-card-size">
                    <span className="settings-label">Menu Card Size</span>
                  </Localized>
                  <span className="settings-field-input-wrap">
                    <div className="settings-size-controls">
                      <Localized id="settings-card-size-decrease-aria" attrs={{ 'aria-label': true }}>
                        <button
                          type="button"
                          className="settings-size-btn"
                          disabled={displayCardSize <= 0}
                          onClick={() => { setDisplayCardSize((s) => Math.max(0, s - 1)); markDirty(); }}
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
                          onClick={() => { setDisplayCardSize((s) => Math.min(4, s + 1)); markDirty(); }}
                          aria-label="Increase card size"
                        >
                          +
                        </button>
                      </Localized>
                    </div>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  <Localized id="settings-field-font-size">
                    <span className="settings-label">Font Size</span>
                  </Localized>
                  <span className="settings-field-input-wrap">
                    <div className="settings-size-controls">
                      <Localized id="settings-font-size-decrease-aria" attrs={{ 'aria-label': true }}>
                        <button
                          type="button"
                          className="settings-size-btn"
                          disabled={displayFontSize <= 0}
                          onClick={() => { setDisplayFontSize((s) => Math.max(0, s - 1)); markDirty(); }}
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
                          onClick={() => { setDisplayFontSize((s) => Math.min(4, s + 1)); markDirty(); }}
                          aria-label="Increase font size"
                        >
                          +
                        </button>
                      </Localized>
                    </div>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- SettingsSelect component has hidden native select */}
                  <label htmlFor="settings-field-font-smoothing" className="settings-label">
                    <Localized id="settings-field-font-smoothing">
                      <span>Font Smoothing</span>
                    </Localized>
                  </label>
                  <span className="settings-field-input-wrap">
                    <SettingsSelect
                      id="settings-field-font-smoothing"
                      value={displayFontSmoothing}
                      onChange={(v) => { setDisplayFontSmoothing(v); markDirty(); }}
                      options={[
                        { value: 'antialiased', label: l10n.getString('settings-font-smoothing-antialiased') },
                        { value: 'subpixel', label: l10n.getString('settings-font-smoothing-subpixel') },
                      ]}
                      ariaLabel={l10n.getString('settings-field-font-smoothing')}
                    />
                  </span>
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
                markDirty();
              }}
              onStoreNameChange={(name) => { setBrandStoreName(name); markDirty(); }}
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

              <div className="settings-field settings-field--horizontal">
                <label htmlFor="settings-field-server-url" className="settings-label">
                  {l10n.getString('settings-sync-server-url')}
                </label>
                <span className="settings-field-input-wrap">
                  <Localized id="settings-server-url-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="settings-input" {...cmInput}
                      type="url"
                      id="settings-field-server-url"
                      placeholder="https://api.example.com"
                      value={syncServerUrl}
                      onChange={(e) => { setSyncServerUrl(e.target.value); setPingResult(null); setTokenExpiresAt(null); markDirty(); }}
                    />
                  </Localized>
                </span>
              </div>

              <div className="settings-field settings-field--horizontal">
                <label htmlFor="settings-field-api-key" className="settings-label">
                  {l10n.getString('settings-sync-api-key')}
                </label>
                <span className="settings-field-input-wrap">
                  <div className="settings-input-wrap">
                    <Localized id={sync.hasApiKey ? 'settings-api-key-masked' : 'settings-api-key-placeholder'} attrs={{ placeholder: true }}>
                      <input
                        className="settings-input" {...cmInput}
                        type={syncApiKeyVisible ? 'text' : 'password'}
                        id="settings-field-api-key"
                        placeholder={sync.hasApiKey ? '••••••••' : 'Enter API key'}
                        value={syncApiKey}
                        onChange={(e) => { setSyncApiKey(e.target.value); markDirty(); }}
                      />
                    </Localized>
                    {/* Only show the eye toggle when there is text to reveal.
                        When hasApiKey is true the placeholder shows dots, but the
                        actual key value is never loaded from the backend — so
                        toggling visibility on an empty field is misleading. */}
                    {syncApiKey && (
                    <button
                      type="button"
                      className="settings-input-toggle"
                      onClick={() => setSyncApiKeyVisible((v) => !v)}
                      aria-label={l10n.getString(syncApiKeyVisible ? 'settings-api-key-hide-aria' : 'settings-api-key-show-aria')}
                      tabIndex={-1}
                    >
                      {syncApiKeyVisible ? (
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                          <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                          <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                          <line x1="1" y1="1" x2="23" y2="23" />
                        </svg>
                      ) : (
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                          <circle cx="12" cy="12" r="3" />
                        </svg>
                      )}
                    </button>
                    )}
                  </div>
                  <p className="settings-hint">
                    <Localized id="settings-sync-token-hint">
                      <span>Enter a JWT token from the cloud server. Generate one via POST /api/v1/tokens</span>
                    </Localized>
                  </p>
                  <div className="settings-sync-token-actions">
                    <Button
                      variant="ghost"
                      loading={requesting}
                      onClick={async () => {
                        setRequesting(true);
                        try {
                          const result = await requestSyncToken(syncServerUrl || undefined);
                          if (result.ok && result.token) {
                            setSyncApiKey(result.token);
                            setSyncApiKeyVisible(false);
                            setTokenExpiresAt(result.expiresAt ?? null);
                            markDirty();
                            addToast({ message: result.status, type: 'success' });
                          } else {
                            addToast({ message: result.status, type: 'error' });
                          }
                        } catch {
                          addToast({ message: l10n.getString('settings-sync-token-request-failed'), type: 'error' });
                        } finally {
                          setRequesting(false);
                        }
                      }}
                    >
                      <Localized id={requesting ? 'settings-sync-requesting' : 'settings-sync-request-token'}>
                        <span>{requesting ? 'Requesting…' : 'Request Token'}</span>
                      </Localized>
                    </Button>
                  </div>
                  {(() => {
                    const expiry = formatTokenExpiry(tokenExpiresAt);
                    if (!expiry) return null;
                    return (
                      <span className={`settings-sync-expiry-badge settings-sync-expiry-badge--${expiry.tone}`}>
                        {l10n.getString(expiry.fluentKey, expiry.fluentArgs)}
                      </span>
                    );
                  })()}
                </span>
              </div>

              <div className="settings-field settings-field--horizontal">
                {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
                <label htmlFor="sync-enabled" className="settings-label">
                  <Localized id="settings-sync-enabled">
                    <span>Enable Cloud Sync</span>
                  </Localized>
                </label>
                <span className="settings-field-input-wrap">
                  <label className="settings-toggle" htmlFor="sync-enabled">
                    <span className="sr-only">Toggle</span>
                    <span className="settings-toggle-switch">
                      <input
                        id="sync-enabled"
                        type="checkbox"
                        role="switch"
                        checked={sync.enabled}
                        aria-checked={sync.enabled}
                        onChange={(e) => { setSync({ ...sync, enabled: e.target.checked }); markDirty(); }}
                      />
                      <span className="settings-toggle-slider" />
                    </span>
                  </label>
                </span>
              </div>

              {(sync.serverUrl !== null || sync.enabled) && (
                <>
                  {/* ── Status indicator ──────────────────── */}
                  <div className="settings-sync-status">
                    <span
                      className={`settings-sync-dot${syncResult && !syncResult.error ? ' settings-sync-dot--ok' : ''}${syncResult?.error ? ' settings-sync-dot--err' : ''}`}
                      aria-hidden="true"
                    />
                    <span className="settings-sync-status-text">
                      {syncResult === null
                        ? (pingResult
                          ? pingResult.status
                          : l10n.getString('settings-sync-status-idle'))
                        : syncResult.error
                          ? syncResult.error
                          : l10n.getString('settings-sync-status-ok')}
                    </span>
                    {pendingCount !== null && pendingCount > 0 && (
                      <span className="settings-sync-pending-badge">
                        {l10n.getString('settings-sync-pending-count', { count: pendingCount })}
                      </span>
                    )}
                  </div>

                  <div className="settings-actions">
                    <Button
                      variant="ghost"
                      loading={testing}
                      onClick={async () => {
                        setTesting(true);
                        setPingResult(null);
                        try {
                          const result = await testSyncConnection(syncServerUrl || undefined);
                          setPingResult(result);
                          if (result.ok) {
                            addToast({ message: result.status, type: 'success' });
                          } else {
                            addToast({ message: result.status, type: 'error' });
                          }
                        } catch {
                          setPingResult({ ok: false, status: l10n.getString('settings-sync-test-failed'), latencyMs: null });
                          addToast({ message: l10n.getString('settings-sync-test-failed'), type: 'error' });
                        } finally {
                          setTesting(false);
                        }
                      }}
                    >
                      <Localized id={testing ? 'settings-sync-testing' : 'settings-sync-test-connection'}>
                        <span>{testing ? 'Testing…' : 'Test Connection'}</span>
                      </Localized>
                    </Button>
                    <Button
                      variant="secondary"
                      loading={syncing}
                      onClick={async () => {
                        setSyncing(true);
                        setSyncResult(null);
                        try {
                          const result = await syncRun();
                          setSyncResult(result);
                          refreshPendingCount();
                          if (result.error) {
                            addToast({ message: result.error, type: 'error' });
                          } else if (result.synced > 0 || result.failed > 0) {
                            addToast({
                              message: l10n.getString('settings-sync-success', { synced: result.synced, failed: result.failed }),
                              type: 'success',
                            });
                          } else {
                            addToast({
                              message: l10n.getString('settings-sync-nothing'),
                              type: 'info',
                            });
                          }
                        } catch {
                          const errMsg = l10n.getString('settings-sync-error');
                          setSyncResult({ synced: 0, failed: 0, error: errMsg });
                          addToast({ message: errMsg, type: 'error' });
                        } finally {
                          setSyncing(false);
                        }
                      }}
                    >
                      <Localized id={syncing ? 'settings-sync-syncing' : 'settings-sync-sync-now'}>
                        <span>{syncing ? 'Syncing…' : 'Sync Now'}</span>
                      </Localized>
                    </Button>
                    <Button
                      variant="ghost"
                      loading={pulling}
                      onClick={async () => {
                        setPulling(true);
                        setPullResult(null);
                        try {
                          const result = await syncPull();
                          setPullResult(result);
                          if (result.error) {
                            addToast({ message: result.error, type: 'error' });
                          } else if (result.productsPulled > 0 || result.taxRatesPulled > 0 || result.usersPulled > 0) {
                            addToast({
                              message: l10n.getString('settings-sync-pull-toast-success', { products: result.productsPulled, tax_rates: result.taxRatesPulled, users: result.usersPulled }),
                              type: 'success',
                            });
                          } else {
                            addToast({
                              message: l10n.getString('settings-sync-pull-empty'),
                              type: 'info',
                            });
                          }
                        } catch {
                          const errMsg = l10n.getString('settings-sync-error');
                          setPullResult({ productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: errMsg });
                          addToast({ message: errMsg, type: 'error' });
                        } finally {
                          setPulling(false);
                        }
                      }}
                    >
                      <Localized id={pulling ? 'settings-sync-pulling' : 'settings-sync-pull'}>
                        <span>{pulling ? 'Pulling…' : 'Pull from Server'}</span>
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

                  {pullResult && (
                    <div className="settings-sync-result-block">
                      <p className="settings-hint">
                        <Localized
                          id="settings-sync-pull-result"
                          vars={{ products: pullResult.productsPulled, tax_rates: pullResult.taxRatesPulled, users: pullResult.usersPulled }}
                        >
                          <span>Last pull: {pullResult.productsPulled} products, {pullResult.taxRatesPulled} tax rates, {pullResult.usersPulled} users</span>
                        </Localized>
                      </p>
                      {pullResult.error && (
                        <p className="settings-hint settings-hint--error">{pullResult.error}</p>
                      )}
                    </div>
                  )}
                </>
              )}
            </div>
          </Card>
        );

      case 'email':
        return <EmailReportSettings />;

      case 'about':
        return (
          <>
            <Card
              shadow="sm"
              header={<Localized id="settings-system-license-header"><h2 className="settings-section-title">System &amp; License Ownership</h2></Localized>}
            >
              <div className="settings-form">
                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-software-edition"><span>Software Edition</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    <Localized id="settings-app-version" vars={{ version: appVersion }}>
                      <span className="settings-license-value">OZ-POS Enterprise v{appVersion}</span>
                    </Localized>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-license-type"><span>License Type</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    <Localized id="settings-license-type-value">
                      <span className="settings-license-value settings-license-value--warning">Proprietary Commercial License</span>
                    </Localized>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-copyright-notice"><span>Copyright Notice</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    <Localized id="settings-copyright-notice-value">
                      <span className="settings-license-value">&copy; 2024-2026 OZ-POS Contributors. All Rights Reserved.</span>
                    </Localized>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-commercial-contact"><span>Commercial Contact</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    <span className="settings-license-value settings-license-value--mono">adikaradwiatmaja@gmail.com</span>
                  </span>
                </div>
              </div>
            </Card>

            <Card
              shadow="sm"
              header={<Localized id="settings-updates-heading"><h2 className="settings-section-title">Updates</h2></Localized>}
            >
              <div className="settings-form">
                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-current-version"><span>Current Version</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    <span className="settings-license-value">{appVersion}</span>
                  </span>
                </div>

                <div className="settings-field settings-field--horizontal">
                  <span className="settings-label">
                    <Localized id="settings-update-status-label"><span>Status</span></Localized>
                  </span>
                  <span className="settings-field-input-wrap">
                    {updateState === 'up-to-date' && (
                      <span className="settings-license-value settings-license-value--active">
                        <Localized id="settings-up-to-date"><span>Up to date</span></Localized>
                      </span>
                    )}
                    {updateState === 'available' && (
                      <span className="settings-license-value settings-license-value--active">
                        <Localized id="settings-update-available" vars={{ version: updateVersion }}>
                          <span>{updateVersion} available</span>
                        </Localized>
                      </span>
                    )}
                    {updateState === 'error' && (
                      <span className="settings-license-value settings-license-value--warning">
                        <Localized id="settings-update-check-error"><span>Check failed</span></Localized>
                      </span>
                    )}
                    {updateState === 'checking' && (
                      <span className="settings-license-value">
                        <Localized id="settings-checking-for-updates"><span>Checking…</span></Localized>
                      </span>
                    )}
                    {updateState === 'idle' && (
                      <span className="settings-license-value settings-license-value--inactive">
                        <Localized id="settings-update-not-checked"><span>Not checked</span></Localized>
                      </span>
                    )}
                  </span>
                </div>

                <div className="settings-actions">
                  {updateState !== 'installing' && (
                    <Button
                      variant="secondary"
                      onClick={handleCheckUpdates}
                      loading={updateState === 'checking'}
                      disabled={updateState === 'checking'}
                    >
                      <Localized id={
                        updateState === 'error'
                          ? 'settings-update-retry'
                          : 'settings-check-for-updates'
                      }>
                        <span>{updateState === 'error' ? 'Retry' : 'Check for Updates'}</span>
                      </Localized>
                    </Button>
                  )}

                  {updateState === 'available' && (
                    <Button
                      variant="primary"
                      onClick={handleInstallUpdate}
                    >
                      <Localized id="settings-install-update">
                        <span>Install Now</span>
                      </Localized>
                    </Button>
                  )}

                  {updateState === 'installing' && (
                    <>
                      <Button
                        variant="secondary"
                        loading
                        disabled
                      >
                        <Localized id="settings-checking-for-updates">
                          <span>Checking…</span>
                        </Localized>
                      </Button>
                      <Button
                        variant="primary"
                        loading
                        disabled
                      >
                        <Localized id="settings-installing-update">
                          <span>Installing…</span>
                        </Localized>
                      </Button>
                    </>
                  )}
                </div>
              </div>
            </Card>
          </>
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

  // ── Resolve current nav item + category for breadcrumb ─────

  const currentNavItem = NAV_ITEMS_REF.find((n) => n.key === activeSection);
  const currentCategory = CATEGORIES_REF.find((c) => c.keys.includes(activeSection));

  // ── Main render ──────────────────────────────────────────────

  return (
    <div className="settings-page" onContextMenu={(e) => e.preventDefault()}>
      {cm.menu && (
        <ContextMenu
          menu={cm.menu}
          menuRef={cm.menuRef}
          onCopy={cm.handleCopy}
          onPaste={cm.handlePaste}
          onClose={cm.close}
        />
      )}
      {/* ── Top bar ────────────────────────────────────── */}
      <header className="settings-topbar">
        <div className="settings-topbar-left">
          <button
            type="button"
            className="settings-mobile-menu-btn"
            onClick={() => setMobileSidebarOpen((p) => !p)}
            aria-label={mobileSidebarOpen ? l10n.getString('settings-sidebar-collapse-aria') : l10n.getString('settings-sidebar-expand-aria')}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              {mobileSidebarOpen ? (
                <>
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </>
              ) : (
                <>
                  <line x1="3" y1="12" x2="21" y2="12" />
                  <line x1="3" y1="6" x2="21" y2="6" />
                  <line x1="3" y1="18" x2="21" y2="18" />
                </>
              )}
            </svg>
          </button>
          <div className="settings-topbar-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </div>
          <span className="settings-topbar-name">
            <Localized id="settings-title">Settings</Localized>
          </span>
        </div>
        <div className="settings-topbar-center">
          <div className="settings-topbar-search">
            <svg className="settings-topbar-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              id="settings-search-input"
              name="settings-search"
              className="settings-topbar-search-input"
              type="text"
              placeholder="Search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              aria-label={l10n.getString('settings-sidebar-search-aria')}
              {...cmInput}
            />
            {searchQuery && (
              <button
                type="button"
                className="settings-topbar-search-clear"
                onClick={() => setSearchQuery('')}
                aria-label={l10n.getString('settings-sidebar-search-clear-aria')}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            )}
          </div>
        </div>
        <div className="settings-topbar-right">
          <span className="settings-topbar-clock" aria-label={`${today}, ${clock}`}>
            {today} {clock}
          </span>
          <div className="settings-save-bar">
            {/* Revert button is always rendered but invisible when not dirty.
                This reserves layout space and prevents the clock and save
                button from shifting on appearance/disappearance. */}
            <span
              className={`settings-save-dot${isDirty && !saving && !saved ? '' : ' settings-save-dot--hidden'}`}
              aria-hidden="true"
            />
            <Localized id="settings-btn-revert-aria" attrs={{ 'aria-label': true }}>
              <button
                type="button"
                className={`settings-btn-revert${isDirty && !saving && !saved ? '' : ' settings-btn-revert--hidden'}`}
                onClick={handleRevert}
                aria-label="Revert changes"
                tabIndex={isDirty && !saving && !saved ? undefined : -1}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                  <polyline points="1 4 1 10 7 10" />
                  <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
                </svg>
                <Localized id="settings-btn-revert">
                  <span>Revert</span>
                </Localized>
              </button>
            </Localized>
            <Localized id="settings-btn-save-aria" attrs={{ 'aria-label': true }} vars={{ state: saved ? 'saved' : 'save' }}>
              <Button
                variant="primary"
                loading={saving}
                onClick={handleSave}
              >
                {saved && !saving ? (
                  <span className="settings-saved-checkmark">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                    <Localized id="settings-saved"><span>Saved!</span></Localized>
                  </span>
                ) : (
                  <Localized id="settings-btn-save"><span>Save</span></Localized>
                )}
              </Button>
            </Localized>
          </div>
        </div>
      </header>

      {/* ── Body ──────────────────────────────────────────── */}
      <div className="settings-body">
        {/* ── Settings sidebar navigation tree ────────────── */}
        <SettingsNavTree
          activeSection={activeSection}
          onNavigate={navigateToSection}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          mobileSidebarOpen={mobileSidebarOpen}
          onMobileClose={() => setMobileSidebarOpen(false)}
        />

        {/* ── Main content ──────────────────────────────── */}
        <main className="settings-content" ref={settingsKeyboardRef as React.Ref<HTMLElement>}>
          <div className="settings-content-header">
            {/* ── Section breadcrumb header ───────── */}
            {currentNavItem && (
              <header className="settings-section-header">
                <div className="settings-section-header-icon" aria-hidden="true">
                  {currentNavItem.icon}
                </div>
                <div className="settings-section-header-text">
                  {currentCategory && (
                    <button
                      type="button"
                      className="settings-section-header-category"
                      // Category expand is handled internally by SettingsNavTree
                      onClick={() => {}}
                      aria-label={l10n.getString(CATEGORY_I18N_KEYS_REF[currentCategory.label] ?? '')}
                    >
                      <Localized id={CATEGORY_I18N_KEYS_REF[currentCategory.label] ?? ''}>
                        {currentCategory.label}
                      </Localized>
                    </button>
                  )}
                  <h1 className="settings-section-header-title">
                    <Localized id={NAV_L10N_KEYS_REF[currentNavItem.key] ?? ''}>{currentNavItem.label}</Localized>
                  </h1>
                </div>
              </header>
            )}
          </div>
          <div className="settings-section-content" key={sectionKey}>
            <div key={activeSection}>
            {renderSection(activeSection)}
          </div>
          </div>
        </main>
      </div>

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="settings-footer">
        <span className="settings-footer-left">
          <button
            type="button"
            className="settings-footer-theme-toggle"
            onClick={toggleTheme}
            aria-label={
              theme === 'light'
                ? l10n.getString('settings-theme-toggle-dark-aria')
                : l10n.getString('settings-theme-toggle-light-aria')
            }
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
          <Localized id="settings-app-version" vars={{ version: appVersion }}>
            <span>OZ-POS Enterprise v{appVersion}</span>
          </Localized>
        </span>
        <span className="settings-footer-right">
          <span className="settings-footer-shortcut">
            <kbd>Ctrl</kbd>+<kbd>S</kbd>
            <Localized id="settings-btn-save"><span>Save</span></Localized>
          </span>
          <Localized id="settings-license-type-value">
            <span>Proprietary Commercial License</span>
          </Localized>
        </span>
      </footer>
    </div>
  );
}
