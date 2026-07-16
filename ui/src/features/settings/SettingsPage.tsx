import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
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
import { Skeleton } from '@/components/Skeleton';
import { LanguageSelector } from '@/i18n/LanguageSelector';
import SettingsSelect from './SettingsSelect';
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
import { useContextMenu, ContextMenu } from '@/frontend/shared';
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
        <circle cx="12" cy="12" r="10" />
        <polyline points="12 6 12 12 16 14" />
      </svg>
    ),
  },
  {
    key: 'tax',
    label: 'Tax Rates',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <line x1="4" y1="6" x2="20" y2="6" />
        <line x1="4" y1="12" x2="20" y2="12" />
        <line x1="4" y1="18" x2="20" y2="18" />
        <line x1="8" y1="6" x2="8" y2="18" />
      </svg>
    ),
  },
  {
    key: 'license',
    label: 'License',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
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

const CATEGORY_I18N_KEYS: Record<string, string> = {
  Business: 'settings-category-business',
  Operations: 'settings-category-operations',
  System: 'settings-category-system',
  Management: 'settings-category-management',
};

const CATEGORIES: SettingsCategory[] = [
  { label: 'Business', keys: ['general', 'appearance'] },
  { label: 'Operations', keys: ['receipt', 'sync'] },
  { label: 'System', keys: ['about', 'license', 'features', 'data'] },
  { label: 'Management', keys: ['staff', 'terminals', 'stores', 'audit', 'offline', 'shifts', 'tax', 'exchange', 'promotions'] },
];

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

const NAV_L10N_KEYS: Record<string, string> = {
  general: 'settings-nav-general',
  appearance: 'settings-nav-appearance',
  receipt: 'settings-nav-receipt',
  sync: 'settings-nav-sync',
  about: 'settings-nav-about',
  features: 'settings-nav-features',
  data: 'settings-nav-data',
  staff: 'settings-nav-staff',
  terminals: 'settings-nav-terminals',
  stores: 'settings-nav-stores',
  audit: 'settings-nav-audit',
  offline: 'settings-nav-offline',
  shifts: 'settings-nav-shifts',
  tax: 'settings-nav-tax',
  license: 'settings-nav-license',
  exchange: 'settings-nav-exchange',
  promotions: 'settings-nav-promotions',
};

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

  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [defaultCurrency, setDefaultCurrencyState] = useState<string>('USD');

  const [sync, setSync] = useState<SyncSettingsDto>({
    serverUrl: null,
    hasApiKey: false,
    enabled: false,
  });
  const [syncServerUrl, setSyncServerUrl] = useState('');
  const [syncApiKey, setSyncApiKey] = useState('');
  const [syncApiKeyVisible, setSyncApiKeyVisible] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncAttemptResult | null>(null);

  const { session } = useAuth();
  const userId = session?.user_id ?? 'default';

  const [displayCardSize, setDisplayCardSize] = useState(0);
  const [displayFontSize, setDisplayFontSize] = useState(0);
  const [displayFontSmoothing, setDisplayFontSmoothing] = useState('antialiased');
  const [brandColour, setBrandColour] = useState('#10b981');
  const [brandStoreName, setBrandStoreName] = useState('');

  const cm = useContextMenu();

  const cmInput = useMemo(() => ({
    autoComplete: 'off' as const,
    autoCorrect: 'off' as const,
    spellCheck: false as const,
    'data-gramm': 'false' as const,
    onContextMenu: (e: React.MouseEvent<HTMLInputElement>) => cm.open(e, e.currentTarget),
  }), [cm]);

  // ── Sidebar navigation state ─────────────────────────────────
  const [activeSection, setActiveSection] = useState('general');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() =>
    localStorage.getItem('settings-sidebar-collapsed') === 'true',
  );
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

  /** Navigate to a section and record it as recently used. */
  const navigateToSection = useCallback((key: string) => {
    setActiveSection(key);
    setMobileSidebarOpen(false);
    setSectionKey((k) => k + 1);

    // Auto-expand the category containing this section
    const cat = CATEGORIES.find((c) => c.keys.includes(key));
    if (cat?.label) setExpandedCategory(cat.label);
  }, []);

  // ── Unsaved changes tracking ────────────────────────────────
  const [isDirty, setIsDirty] = useState(false);
  const markDirty = useCallback(() => { setIsDirty(true); }, []);

  // Warn before closing the tab / window when there are unsaved changes.
  useEffect(() => {
    function handleBeforeUnload(e: BeforeUnloadEvent) {
      if (isDirty) {
        e.preventDefault();
        // Modern browsers ignore custom messages; they show a generic prompt.
      }
    }
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [isDirty]);

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
    setSyncResult(null);
    setSyncApiKey('');
    setSyncApiKeyVisible(false);
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
      getDefaultCurrency(),
      getSyncSettings(),
      getUserPreferences(userId),
      getBrandSettings(),
      getVersion(),
    ]);
    const [rR, sR, cR, dcR, syncR, prefsR, brandR, verR] = results;

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
      if (dcR.status === 'fulfilled') setDefaultCurrencyState(dcR.value ?? 'USD');
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
      initialSnapshotRef.current = {
        receipt: rR.status === 'fulfilled' ? rR.value : (snapReceipt ?? receipt),
        store: sR.status === 'fulfilled' ? sR.value : store,
        defaultCurrency: dcR.status === 'fulfilled' ? (dcR.value ?? 'USD') : defaultCurrency,
        sync: syncR.status === 'fulfilled' ? syncR.value : sync,
        syncServerUrl: syncR.status === 'fulfilled' ? (syncR.value.serverUrl ?? '') : syncServerUrl,
        displayCardSize: snapCardSize ?? displayCardSize,
        displayFontSize: snapFontSize ?? displayFontSize,
        displayFontSmoothing: snapFontSmoothing ?? displayFontSmoothing,
        brandColour: snapBrandColour ?? brandColour,
        brandStoreName: snapStoreName ?? brandStoreName,
      };
    } finally {
      setLoading(false);
    }
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
      setDefaultCurrency({ code: defaultCurrency }),
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
      if (syncApiKey) setSyncApiKey('');
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

  // ── Sidebar search filtering ───────────────────────────────

  const q = searchQuery.toLowerCase().trim();
  const filteredCategories = !q
    ? CATEGORIES
    : CATEGORIES
        .map((cat) => ({
          ...cat,
          keys: cat.keys.filter((key) => {
            const item = NAV_ITEMS.find((n) => n.key === key);
            return item && (
              item.label.toLowerCase().includes(q) ||
              cat.label.toLowerCase().includes(q)
            );
          }),
        }))
        .filter((cat) => cat.keys.length > 0);

  // ── Keyboard shortcuts ────────────────────────────────────

  useEffect(() => {
    // Use filtered categories so arrow nav respects the search query
    const flatKeys = filteredCategories.flatMap((c) => c.keys);

    function handleKeyDown(e: KeyboardEvent) {
      // Ctrl+S / Cmd+S → save (guarded by saving flag)
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (!saving) handleSave();
        return;
      }

      // Escape → close mobile sidebar
      if (e.key === 'Escape' && mobileSidebarOpen) {
        e.preventDefault();
        setMobileSidebarOpen(false);
        return;
      }

      // Arrow keys navigate sections (skip when focused on inputs)
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;

      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const idx = flatKeys.indexOf(activeSection);
        if (idx === -1) return;
        const next = e.key === 'ArrowDown'
          ? (idx + 1) % flatKeys.length
          : (idx - 1 + flatKeys.length) % flatKeys.length;
        const nextKey = flatKeys[next];
        if (!nextKey) return;
        navigateToSection(nextKey);
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
    // Re-attach on searchQuery changes so arrow nav always uses the
    // current filtered list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeSection, mobileSidebarOpen, saving, searchQuery]);

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
                <label htmlFor="receipt-show-currency" className="settings-label">
                  <Localized id="settings-toggle-show-currency">
                    <span>Show currency symbol on amounts</span>
                  </Localized>
                </label>
                <span className="settings-field-input-wrap">
                  <label className="settings-toggle">
                    <span className="settings-toggle-switch">
                      <Localized id="settings-toggle-show-currency-aria" attrs={{ 'aria-label': true }}>
                        <input
                          id="receipt-show-currency"
                          type="checkbox"
                          checked={receipt.showCurrency}
                          onChange={(e) => { setReceipt({ ...receipt, showCurrency: e.target.checked }); markDirty(); }}
                          aria-label="Show currency symbol on amounts"
                        />
                      </Localized>
                      <span className="settings-toggle-slider" />
                    </span>
                  </label>
                </span>
              </div>

              {/* Decimal separator */}
              <div className="settings-field settings-field--horizontal">
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
                <label htmlFor="receipt-show-tax" className="settings-label">
                  <Localized id="settings-toggle-show-tax">
                    <span>Show tax line on receipts</span>
                  </Localized>
                </label>
                <span className="settings-field-input-wrap">
                  <label className="settings-toggle">
                    <span className="settings-toggle-switch">
                      <Localized id="settings-toggle-show-tax-aria" attrs={{ 'aria-label': true }}>
                        <input
                          id="receipt-show-tax"
                          type="checkbox"
                          checked={receipt.showTax}
                          onChange={(e) => { setReceipt({ ...receipt, showTax: e.target.checked }); markDirty(); }}
                          aria-label="Show tax line on receipts"
                        />
                      </Localized>
                      <span className="settings-toggle-slider" />
                    </span>
                  </label>
                </span>
              </div>

              {/* Paper width */}
              <div className="settings-field settings-field--horizontal">
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
                <label htmlFor="receipt-show-table-number" className="settings-label">
                  <Localized id="settings-toggle-show-table-number">
                    <span>Show table number on cart and receipts</span>
                  </Localized>
                </label>
                <span className="settings-field-input-wrap">
                  <label className="settings-toggle">
                    <span className="settings-toggle-switch">
                      <Localized id="settings-toggle-show-table-number-aria" attrs={{ 'aria-label': true }}>
                        <input
                          id="receipt-show-table-number"
                          type="checkbox"
                          checked={receipt.showTableNumber}
                          onChange={(e) => { setReceipt({ ...receipt, showTableNumber: e.target.checked }); markDirty(); }}
                          aria-label="Show table number on cart and receipts"
                        />
                      </Localized>
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
                      onChange={(e) => { setSyncServerUrl(e.target.value); markDirty(); }}
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
                  </div>
                </span>
              </div>

              <div className="settings-field settings-field--horizontal">
                <label htmlFor="sync-enabled" className="settings-label">
                  <Localized id="settings-sync-enabled">
                    <span>Enable Cloud Sync</span>
                  </Localized>
                </label>
                <span className="settings-field-input-wrap">
                  <label className="settings-toggle">
                    <span className="settings-toggle-switch">
                      <Localized id="settings-sync-enabled-aria" attrs={{ 'aria-label': true }}>
                        <input
                          id="sync-enabled"
                          type="checkbox"
                          checked={sync.enabled}
                          onChange={(e) => { setSync({ ...sync, enabled: e.target.checked }); markDirty(); }}
                          aria-label="Enable Cloud Sync"
                        />
                      </Localized>
                      <span className="settings-toggle-slider" />
                    </span>
                  </label>
                </span>
              </div>

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
                          setSyncResult({ synced: 0, failed: 0, error: l10n.getString('settings-sync-error') });
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
          <>
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
                    <span className="settings-license-value">&copy; 2024-2026 OZ-POS Contributors. All Rights Reserved.</span>
                  </Localized>
                </div>
                <div className="settings-license-row settings-license-row--last">
                  <Localized id="settings-commercial-contact"><span className="settings-license-label">Commercial Contact</span></Localized>
                  <span className="settings-license-value settings-license-value--mono">adikaradwiatmaja@gmail.com</span>
                </div>
              </div>
            </Card>

            <Card
              shadow="sm"
              header={<Localized id="settings-updates-heading"><h2 className="settings-section-title">Updates</h2></Localized>}
            >
              <div className="settings-form">
                <div className="settings-license-row">
                  <Localized id="settings-current-version"><span className="settings-license-label">Current Version</span></Localized>
                  <span className="settings-license-value">{appVersion}</span>
                </div>

                <div className="settings-actions" style={{ marginTop: '12px' }}>
                  {(updateState === 'idle' || updateState === 'up-to-date' || updateState === 'error') && (
                    <Button
                      variant="secondary"
                      onClick={handleCheckUpdates}
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

                  {(updateState === 'checking') && (
                    <Button
                      variant="secondary"
                      loading
                      disabled
                    >
                      <Localized id="settings-checking-for-updates">
                        <span>Checking…</span>
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
                    <Button
                      variant="primary"
                      loading
                      disabled
                    >
                      <Localized id="settings-installing-update">
                        <span>Installing…</span>
                      </Localized>
                    </Button>
                  )}
                </div>

                {updateState === 'up-to-date' && (
                  <p className="settings-hint" style={{ marginTop: '8px' }}>
                    <Localized id="settings-up-to-date">
                      <span>✓ You're up to date</span>
                    </Localized>
                  </p>
                )}

                {updateState === 'available' && (
                  <p className="settings-hint" style={{ marginTop: '8px' }}>
                    <Localized id="settings-update-available" vars={{ version: updateVersion }}>
                      <span>{updateVersion} is available</span>
                    </Localized>
                  </p>
                )}

                {updateState === 'error' && (
                  <p className="settings-hint settings-hint--error" style={{ marginTop: '8px' }}>
                    <Localized id="settings-update-check-error">
                      <span>Update check failed</span>
                    </Localized>
                  </p>
                )}
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

  const currentNavItem = NAV_ITEMS.find((n) => n.key === activeSection);
  const currentCategory = CATEGORIES.find((c) => c.keys.includes(activeSection));

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
        {/* ── Mobile backdrop ─────────────────────── */}
        <div
          className={`settings-sidebar-backdrop${mobileSidebarOpen ? ' visible' : ''}`}
          onClick={() => setMobileSidebarOpen(false)}
          aria-hidden="true"
        />

        {/* ── Sidebar ────────────────────────────────── */}
        <aside
          className={`settings-sidebar${sidebarCollapsed ? ' collapsed' : ''}${mobileSidebarOpen ? ' mobile-open' : ''}`}
          aria-label={l10n.getString('settings-sidebar-nav-aria')}
        >
          <div className="settings-sidebar-header">
            <button
              type="button"
              className="settings-sidebar-collapse-all"
              onClick={() => setExpandedCategory(null)}
              aria-label={l10n.getString('settings-sidebar-collapse-all-aria')}
              title={l10n.getString('settings-sidebar-collapse-all-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="14" height="14">
                <polyline points="6 15 12 9 18 15" />
              </svg>
            </button>
            <button
              type="button"
              className="settings-sidebar-toggle"
              onClick={() => setSidebarCollapsed((p) => !p)}
              aria-label={
                sidebarCollapsed
                  ? l10n.getString('settings-sidebar-expand-aria')
                  : l10n.getString('settings-sidebar-collapse-aria')
              }
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
            {q && filteredCategories.length === 0 ? (
              <div className="settings-sidebar-empty-search">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="1.75rem" height="1.75rem" aria-hidden="true">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                  <line x1="8" y1="11" x2="14" y2="11" />
                </svg>
                <Localized id="settings-sidebar-no-results">
                  <span className="settings-sidebar-empty-title">No matching sections</span>
                </Localized>
                <button
                  type="button"
                  className="settings-sidebar-empty-clear"
                  onClick={() => setSearchQuery('')}
                >
                  <Localized id="settings-sidebar-clear-results">Clear search</Localized>
                </button>
              </div>
            ) : (
              filteredCategories.map((cat) => {
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
                    <span className="settings-sidebar-section-label-wrap">
                      <span className="settings-sidebar-section-label">
                        <Localized id={CATEGORY_I18N_KEYS[cat.label] ?? ''}>{cat.label}</Localized>
                      </span>
                      {!sidebarCollapsed && (
                        <span className="settings-sidebar-count" title={`${cat.keys.length} items`}>
                          {cat.keys.length}
                        </span>
                      )}
                    </span>
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
                  {(isExpanded || sidebarCollapsed) && (
                    <div className="settings-sidebar-section-items">
                      {cat.keys.map((key) => {
                        const item = NAV_ITEMS.find((n) => n.key === key)!;
                        return (
                          <Tooltip key={key} content={l10n.getString(NAV_L10N_KEYS[item.key] ?? '')} showDelay={800}>
                            <button
                              type="button"
                              className={`settings-nav-item${activeSection === key ? ' settings-nav-item--active' : ''}`}
                              onClick={() => navigateToSection(key)}
                              aria-current={activeSection === key ? 'page' : undefined}
                              aria-label={l10n.getString(NAV_L10N_KEYS[item.key] ?? '')}
                            >
                              <span className="settings-nav-icon">{item.icon}</span>
                              <span className="settings-nav-label">
                                <Localized id={NAV_L10N_KEYS[item.key] ?? ''}>{item.label}</Localized>
                              </span>
                            </button>
                          </Tooltip>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })
            )}
          </nav>


        </aside>

        {/* ── Main content ──────────────────────────────── */}
        <main className="settings-content">
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
                      onClick={() => setExpandedCategory(currentCategory.label)}
                      aria-label={l10n.getString(CATEGORY_I18N_KEYS[currentCategory.label] ?? '')}
                    >
                      <Localized id={CATEGORY_I18N_KEYS[currentCategory.label] ?? ''}>
                        {currentCategory.label}
                      </Localized>
                    </button>
                  )}
                  <h1 className="settings-section-header-title">
                    <Localized id={NAV_L10N_KEYS[currentNavItem.key] ?? ''}>{currentNavItem.label}</Localized>
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
