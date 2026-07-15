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
import { Skeleton } from '@/components/Skeleton';
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
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [sectionKey, setSectionKey] = useState(0);
  const [searchQuery, setSearchQuery] = useState('');

  // ── Recently visited sections (max 3, persisted to localStorage) ──
  const [recentSections, setRecentSections] = useState<string[]>(() => {
    try {
      const stored = localStorage.getItem('settings-recent-sections');
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.every((s: unknown) => typeof s === 'string')) {
          return parsed.slice(0, 3);
        }
      }
    } catch { /* malformed JSON — start fresh */ }
    return [];
  });

  /** Navigate to a section and record it as recently used. */
  const navigateToSection = useCallback((key: string) => {
    setActiveSection(key);
    setMobileSidebarOpen(false);
    setSectionKey((k) => k + 1);
    setRecentSections((prev) => {
      // Move `key` to front, deduplicate, keep max 3
      const next = [key, ...prev.filter((k) => k !== key)].slice(0, 3);
      localStorage.setItem('settings-recent-sections', JSON.stringify(next));
      return next;
    });
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

    try {
      if (rR.status === 'fulfilled') { setReceipt(rR.value); setDecimalSep(rR.value.decimalSeparator); }
      if (sR.status === 'fulfilled') setStore(sR.value);
      if (cR.status === 'fulfilled') setCurrencies(cR.value);
      if (dcR.status === 'fulfilled') setDefaultCurrencyState(dcR.value ?? 'USD');
      if (syncR.status === 'fulfilled') { setSync(syncR.value); setSyncServerUrl(syncR.value.serverUrl ?? ''); }
      if (prefsR.status === 'fulfilled') {
        const p = prefsR.value;
        const cs = p['cardsize'];
        if (cs !== undefined) setDisplayCardSize(Math.min(4, Math.max(0, parseInt(cs, 10) || 0)));
        const fs = p['fontsize'];
        if (fs !== undefined) setDisplayFontSize(Math.min(4, Math.max(0, parseInt(fs, 10) || 0)));
        if (p['font-smoothing'] !== undefined) setDisplayFontSmoothing(p['font-smoothing']);
      }
      if (brandR.status === 'fulfilled') {
        setBrandColour(brandR.value.primary_colour);
        setBrandStoreName(brandR.value.store_name);
        const palette = deriveAccentPalette(brandR.value.primary_colour);
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
    } finally {
      setLoading(false);
    }
  }, [userId, l10n, addToast]);

  useEffect(() => { load(); }, [load]);

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
                <label className="settings-field" htmlFor="settings-field-store-name">
                  {l10n.getString('settings-field-store-name')}
                  <Localized id="settings-store-name-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="settings-input" autoComplete="off"
                      type="text"
                      id="settings-field-store-name"
                      placeholder="OZ-POS Store"
                      value={store.name}
                      onChange={(e) => { setStore({ ...store, name: e.target.value }); markDirty(); }}
                    />
                  </Localized>
                </label>

                <label className="settings-field" htmlFor="settings-field-address">
                  {l10n.getString('settings-field-address')}
                  <Localized id="settings-address-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="settings-input" autoComplete="off"
                      type="text"
                      id="settings-field-address"
                      placeholder="123 Main Street"
                      value={store.address}
                      onChange={(e) => { setStore({ ...store, address: e.target.value }); markDirty(); }}
                    />
                  </Localized>
                </label>

                <label className="settings-field" htmlFor="settings-field-tax-id">
                  {l10n.getString('settings-field-tax-id')}
                  <Localized id="settings-tax-id-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="settings-input" autoComplete="off"
                      type="text"
                      id="settings-field-tax-id"
                      placeholder="12-3456789"
                      value={store.taxId}
                      onChange={(e) => { setStore({ ...store, taxId: e.target.value }); markDirty(); }}
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
                    onChange={(e) => { setDefaultCurrencyState(e.target.value); markDirty(); }}
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
                </div>

                <div className="settings-field">
                  <Localized id="settings-field-font-smoothing">
                    <span className="settings-label">Font Smoothing</span>
                  </Localized>
                  <select
                    className="settings-select"
                    id="settings-field-font-smoothing"
                    value={displayFontSmoothing}
                    onChange={(e) => { setDisplayFontSmoothing(e.target.value); markDirty(); }}
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
              <label className="settings-toggle" htmlFor="settings-toggle-show-currency">
                <Localized id="settings-toggle-show-currency-aria" attrs={{ 'aria-label': true }}>
                  <input
                    type="checkbox"
                    id="settings-toggle-show-currency"
                    checked={receipt.showCurrency}
                    onChange={(e) => { setReceipt({ ...receipt, showCurrency: e.target.checked }); markDirty(); }}
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
                    markDirty();
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
                    onChange={(e) => { setReceipt({ ...receipt, showTax: e.target.checked }); markDirty(); }}
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
                  onChange={(e) => { setReceipt({ ...receipt, paperWidth: e.target.value }); markDirty(); }}
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
                    className="settings-input" autoComplete="off"
                    type="text"
                    id="settings-field-receipt-footer"
                    placeholder="Thank you for shopping!"
                    value={receipt.footer}
                    onChange={(e) => { setReceipt({ ...receipt, footer: e.target.value }); markDirty(); }}
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
                    onChange={(e) => { setReceipt({ ...receipt, showTableNumber: e.target.checked }); markDirty(); }}
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
                    className="settings-input" autoComplete="off"
                    type="url"
                    id="settings-field-server-url"
                    placeholder="https://api.example.com"
                    value={syncServerUrl}
                    onChange={(e) => { setSyncServerUrl(e.target.value); markDirty(); }}
                  />
                </Localized>
              </label>

              <label className="settings-field" htmlFor="settings-field-api-key">
                {l10n.getString('settings-sync-api-key')}
                <Localized id={sync.hasApiKey ? 'settings-api-key-masked' : 'settings-api-key-placeholder'} attrs={{ placeholder: true }}>
                  <input
                    className="settings-input" autoComplete="off"
                    type="password"
                    id="settings-field-api-key"
                    placeholder={sync.hasApiKey ? '••••••••' : 'Enter API key'}
                    value={syncApiKey}
                    onChange={(e) => { setSyncApiKey(e.target.value); markDirty(); }}
                  />
                </Localized>
              </label>

              <label className="settings-toggle" htmlFor="settings-toggle-sync-enabled">
                <Localized id="settings-sync-enabled-aria" attrs={{ 'aria-label': true }}>
                  <input
                    type="checkbox"
                    id="settings-toggle-sync-enabled"
                    checked={sync.enabled}
                    onChange={(e) => { setSync({ ...sync, enabled: e.target.checked }); markDirty(); }}
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

  // ── Resolve current nav item + category for breadcrumb ─────

  const currentNavItem = NAV_ITEMS.find((n) => n.key === activeSection);
  const currentCategory = CATEGORIES.find((c) => c.keys.includes(activeSection));

  // ── Main render ──────────────────────────────────────────────

  return (
    <div className="settings-page">
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
        <div className="settings-topbar-right">
          <span className="settings-topbar-clock" aria-label={`${today}, ${clock}`}>
            {today} {clock}
          </span>
          <div className="settings-save-bar">
            {isDirty && !saving && !saved && <span className="settings-save-dot" aria-hidden="true" />}
            <Localized id="settings-btn-save-aria" attrs={{ 'aria-label': true }} vars={{ state: saved ? 'saved' : 'save' }}>
              <Button
                variant="primary"
                loading={saving}
                onClick={handleSave}
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

          {/* ── Sidebar search ──────────────── */}
          {!sidebarCollapsed && (
            <div className="settings-sidebar-search">
              <svg className="settings-sidebar-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                className="settings-sidebar-search-input"
                type="text"
                placeholder={l10n.getString('settings-sidebar-search-placeholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                aria-label={l10n.getString('settings-sidebar-search-aria')}
              />
              {searchQuery && (
                <button
                  type="button"
                  className="settings-sidebar-search-clear"
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
          )}

          <nav className="settings-sidebar-nav">
            {/* ── Recently used (only when not searching) ── */}
            {!q && recentSections.length > 0 && !sidebarCollapsed && (
              <div className="settings-recent-section">
                <span className="settings-recent-section-label">
                  <Localized id="settings-sidebar-recent">Recent</Localized>
                </span>
                {recentSections.map((key) => {
                  const item = NAV_ITEMS.find((n) => n.key === key);
                  if (!item) return null;
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

            {q && filteredCategories.length === 0 ? (
              <div className="settings-sidebar-empty-search">
                <Localized id="settings-sidebar-no-results">No matching sections</Localized>
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
                    <span className="settings-sidebar-section-label">
                      <Localized id={CATEGORY_I18N_KEYS[cat.label] ?? ''}>{cat.label}</Localized>
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
          <div className="settings-section-content" key={sectionKey}>
            {renderSection(activeSection)}
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
          <Localized id="settings-license-type-value">
            <span>Proprietary Commercial License</span>
          </Localized>
        </span>
      </footer>
    </div>
  );
}
