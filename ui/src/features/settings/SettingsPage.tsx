import { useEffect, useState, useCallback, useRef, useMemo, lazy, Suspense } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  setReceiptSettings,
  setStoreSettings,
  setUserPreferences,
  setSettingScoped,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
} from '@/api/settings';
import { setDecimalSep } from '@/utils/storage';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { SettingsProvider, useSettings } from '@/contexts/SettingsContext';
import { useCurrency } from '@/contexts/CurrencyContext';
import {
  type CurrencyDto,
} from '@/api/currency';
import {
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

import {
  setBrandPrimaryColour,
  setBrandStoreName as setBrandStoreNameApi,
} from '@/api/branding';
import { useBrand } from '@/contexts/BrandContext';
import { deriveAccentPalette, applyAccentPalette } from '@/utils/color';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import { useToast } from '@/frontend/shared/Toast';
import { useTheme } from '@/frontend/shell/ThemeProvider';
import { useKeyboardAvoidance } from '@/hooks/useKeyboardAvoidance';
import FeatureToggleScreen from './FeatureToggleScreen';
import DataManagementScreen from './DataManagementScreen';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import { MultiStoreDashboardScreen, TopologyScreen } from '@/features/stores';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';
import ShiftManagementScreen from '@/features/shifts/ShiftManagementScreen';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';
import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';
import PromotionManagementScreen from '@/features/promotions/PromotionManagementScreen';
import LicenseSettings from './LicenseSettings';
import EmailReportSettings from './EmailReportSettings';
const GeneralSection = lazy(() => import('./sections/GeneralSection'));
const AppearanceSection = lazy(() => import('./sections/AppearanceSection'));
const ReceiptSection = lazy(() => import('./sections/ReceiptSection'));
const SyncSection = lazy(() => import('./sections/SyncSection'));
const AboutSection = lazy(() => import('./sections/AboutSection'));
import { useContextMenu, ContextMenu } from '@/frontend/shared';
import SettingsNavTree, {
  NAV_ITEMS as NAV_ITEMS_REF,
  CATEGORIES as CATEGORIES_REF,
  CATEGORY_I18N_KEYS as CATEGORY_I18N_KEYS_REF,
  NAV_L10N_KEYS as NAV_L10N_KEYS_REF,
} from './SettingsNavTree';

// ── Lazy-loaded workspace settings cards (ADR #22 Phase 3) ──
const WorkspaceStorePosSettings = lazy(() =>
  import('./workspace-cards/WorkspaceStorePosSettings').then((m) => ({ default: m.WorkspaceStorePosSettings })),
);
const WorkspaceRestaurantPosSettings = lazy(() =>
  import('./workspace-cards/WorkspaceRestaurantPosSettings').then((m) => ({ default: m.WorkspaceRestaurantPosSettings })),
);
const WorkspaceKdsSettings = lazy(() =>
  import('./workspace-cards/WorkspaceKdsSettings').then((m) => ({ default: m.WorkspaceKdsSettings })),
);
const WorkspaceInventorySettings = lazy(() =>
  import('./workspace-cards/WorkspaceInventorySettings').then((m) => ({ default: m.WorkspaceInventorySettings })),
);
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

/** Settings hub — sidebar-driven navigation across general, appearance, features, data management, staff, terminals, multi-store, audit, offline queue, shifts, tax, currency, and promotions. */
export default function SettingsPage() {
  return (
    <SettingsProvider>
      <SettingsPageContent />
    </SettingsProvider>
  );
}

/** Inner component that consumes useSettings() — wrapped by SettingsProvider. */
function SettingsPageContent() {
  const settingsCtx = useSettings();
  const loadError = settingsCtx.error;
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
  const { sessionToken } = useWorkspace();
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

  // ── Initialize local draft state from SettingsContext ─────
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    // Once context finishes loading, copy values into local editable state.
    // Only do this on the initial load — subsequent context refetches
    // (e.g. from markSettingsUpdated) should NOT overwrite user edits.
    if (!settingsCtx.loading && !initialized) {
      const s = settingsCtx.settings;
      setReceipt(s.receipt);
      setStore(s.store);
      setCurrencies(s.currencies);
      setSync(s.sync);
      setSyncServerUrl(s.sync.serverUrl ?? '');
      setDisplayCardSize(s.preferences.cardSize);
      setDisplayFontSize(s.preferences.fontSize);
      setDisplayFontSmoothing(s.preferences.fontSmoothing);
      setBrandColour(s.brand.colour);
      setBrandStoreName(s.brand.storeName);
      setAppVersion(s.appVersion);
      setDecimalSep(s.receipt.decimalSeparator);

      const palette = deriveAccentPalette(s.brand.colour);
      applyAccentPalette(palette);

      // Set the initial snapshot for Revert-to-saved
      initialSnapshotRef.current = {
        receipt: s.receipt,
        store: s.store,
        defaultCurrency,
        sync: s.sync,
        syncServerUrl: s.sync.serverUrl ?? '',
        displayCardSize: s.preferences.cardSize,
        displayFontSize: s.preferences.fontSize,
        displayFontSmoothing: s.preferences.fontSmoothing,
        brandColour: s.brand.colour,
        brandStoreName: s.brand.storeName,
      };

      // Show toast for partial load failures (regression guard from Phase 0b)
      if (settingsCtx.hasPartialError) {
        addToast({ message: l10n.getString('settings-load-partial'), type: 'error' });
      }

      setInitialized(true);
    }
  }, [settingsCtx.loading, settingsCtx.settings, settingsCtx.hasPartialError, initialized, defaultCurrency, addToast, l10n]);

  // Derive loading/error state from context
  const loading = settingsCtx.loading && !initialized;

  // Scroll content to top and focus the first heading when navigating sections.
  useEffect(() => {
    const contentEl = document.querySelector<HTMLElement>('.settings-content');
    if (contentEl) contentEl.scrollTop = 0;

    // P60-4f: Move focus to the first heading in the newly rendered section
    // so screen readers announce the section title and keyboard users can
    // tab into the section content naturally.
    const sectionEl = document.querySelector<HTMLElement>('.settings-section-content');
    if (sectionEl) {
      const heading = sectionEl.querySelector<HTMLElement>('h2');
      if (heading) {
        heading.setAttribute('tabindex', '-1');
        heading.focus({ preventScroll: true });
        // Remove tabindex after blur so headings don't remain focusable via Tab
        heading.addEventListener('blur', function onBlur() {
          heading.removeAttribute('tabindex');
          heading.removeEventListener('blur', onBlur);
        }, { once: true });
      }
    }
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
          setSettingScoped(sessionToken, 'sync.auth_token', syncApiKey)
            .catch(() => { /* best-effort */ });
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

    // Notify SettingsContext so other components reflect the changes
    const changedKeys: string[] = [];
    if (results[0]?.status === 'fulfilled') changedKeys.push('receipt.footer', 'receipt.showCurrency', 'receipt.showTax', 'receipt.paperWidth', 'receipt.showTableNumber', 'receipt.decimalSeparator');
    if (results[1]?.status === 'fulfilled') changedKeys.push('store.name', 'store.address', 'store.taxId');
    if (results[2]?.status === 'fulfilled') changedKeys.push('currency.default');
    if (results[3]?.status === 'fulfilled') changedKeys.push('prefs.cardsize', 'prefs.fontsize', 'prefs.font-smoothing');
    if (results[4]?.status === 'fulfilled') changedKeys.push('sync.serverUrl', 'sync.apiKey', 'sync.enabled');
    if (results[5]?.status === 'fulfilled') changedKeys.push('brand.primary_colour');
    if (results[6]?.status === 'fulfilled') changedKeys.push('brand.store_name');
    if (changedKeys.length > 0) {
      settingsCtx.markSettingsUpdated(changedKeys);
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
          <Button variant="secondary" onClick={() => { setInitialized(false); settingsCtx.refetch(); }}>
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
          <GeneralSection
            store={store}
            setStore={setStore}
            markDirty={markDirty}
            cmInput={cmInput}
            fieldErrors={fieldErrors}
            validateField={validateField}
            clearFieldError={clearFieldError}
            currencies={currencies}
            defaultCurrency={defaultCurrency}
            setDefaultCurrencyState={setDefaultCurrencyState}
            l10n={l10n}
          />
        );

      case 'appearance':
        return (
          <AppearanceSection
            displayCardSize={displayCardSize}
            setDisplayCardSize={setDisplayCardSize}
            displayFontSize={displayFontSize}
            setDisplayFontSize={setDisplayFontSize}
            displayFontSmoothing={displayFontSmoothing}
            setDisplayFontSmoothing={setDisplayFontSmoothing}
            brandColour={brandColour}
            setBrandColour={setBrandColour}
            brandStoreName={brandStoreName}
            setBrandStoreName={setBrandStoreName}
            markDirty={markDirty}
            l10n={l10n}
          />
        );

      case 'receipt':
        return (
          <ReceiptSection
            receipt={receipt}
            setReceipt={setReceipt}
            setDecimalSep={setDecimalSep}
            markDirty={markDirty}
            l10n={l10n}
          />
        );

      case 'sync':
        return (
          <SyncSection
            sync={sync}
            setSync={setSync}
            syncServerUrl={syncServerUrl}
            setSyncServerUrl={setSyncServerUrl}
            syncApiKey={syncApiKey}
            setSyncApiKey={setSyncApiKey}
            syncApiKeyVisible={syncApiKeyVisible}
            setSyncApiKeyVisible={setSyncApiKeyVisible}
            syncing={syncing}
            setSyncing={setSyncing}
            pulling={pulling}
            setPulling={setPulling}
            syncResult={syncResult}
            setSyncResult={setSyncResult}
            pullResult={pullResult}
            setPullResult={setPullResult}
            pendingCount={pendingCount}
            testing={testing}
            setTesting={setTesting}
            pingResult={pingResult}
            setPingResult={setPingResult}
            requesting={requesting}
            setRequesting={setRequesting}
            tokenExpiresAt={tokenExpiresAt}
            setTokenExpiresAt={setTokenExpiresAt}
            cmInput={cmInput}
            markDirty={markDirty}
            refreshPendingCount={refreshPendingCount}
            testSyncConnection={testSyncConnection}
            syncRun={syncRun}
            syncPull={syncPull}
            requestSyncToken={requestSyncToken}
            l10n={l10n}
            addToast={addToast}
          />
        );

      case 'email':
        return <EmailReportSettings />;

      case 'about':
        return (
          <AboutSection
            appVersion={appVersion}
            updateState={updateState}
            updateVersion={updateVersion}
            handleCheckUpdates={handleCheckUpdates}
            handleInstallUpdate={handleInstallUpdate}
          />
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

      case 'topology':
        return <TopologyScreen />;

      case 'store-pos':
        return (
          <Suspense fallback={<Skeleton variant="block" width="100%" height="12rem" />}>
            <WorkspaceStorePosSettings variant="full-page" />
          </Suspense>
        );

      case 'restaurant-pos':
        return (
          <Suspense fallback={<Skeleton variant="block" width="100%" height="12rem" />}>
            <WorkspaceRestaurantPosSettings variant="full-page" />
          </Suspense>
        );

      case 'kds':
        return (
          <Suspense fallback={<Skeleton variant="block" width="100%" height="12rem" />}>
            <WorkspaceKdsSettings variant="full-page" />
          </Suspense>
        );

      case 'inventory':
        return (
          <Suspense fallback={<Skeleton variant="block" width="100%" height="12rem" />}>
            <WorkspaceInventorySettings variant="full-page" />
          </Suspense>
        );

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
                onClick={handleSave}
                loading={saving}
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
        <form id="settings-form" className="settings-content" onSubmit={(e) => { e.preventDefault(); handleSave(); }} ref={settingsKeyboardRef as unknown as React.Ref<HTMLFormElement>}>
          <button type="submit" hidden aria-hidden="true" tabIndex={-1}>Save</button>
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
          <div className={`settings-section-content${activeSection === 'topology' ? ' settings-section-content--full' : ''}`} key={activeSection}><div key={activeSection}>
              <Suspense fallback={<div className="section-loading">Loading...</div>}>
                {renderSection(activeSection)}
              </Suspense>
            </div>
          </div>
        </form>
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
