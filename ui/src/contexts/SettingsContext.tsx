/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  useMemo,
  type ReactNode,
} from 'react';
import {
  getReceiptSettings,
  getStoreSettings,
  getUserPreferences,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
} from '@/api/settings';
import {
  getSyncSettings,
  type SyncSettingsDto,
} from '@/api/offline';
import {
  listCurrencies,
  type CurrencyDto,
} from '@/api/currency';
import { getBrandSettings } from '@/api/branding';
import { getVersion, type VersionInfo } from '@/api/system';
import { useAuth } from './AuthContext';

// ── Types ────────────────────────────────────────────────────────────

/** Brand subset that SettingsContext tracks. */
export interface SettingsBrandState {
  colour: string;
  storeName: string;
}

/** User preference subset that SettingsContext tracks. */
export interface SettingsPreferencesState {
  cardSize: number;
  fontSize: number;
  fontSmoothing: string;
}

/** All settings state held by the context. */
export interface SettingsState {
  receipt: ReceiptSettingsDto;
  store: StoreSettingsDto;
  sync: SyncSettingsDto;
  brand: SettingsBrandState;
  preferences: SettingsPreferencesState;
  currencies: CurrencyDto[];
  appVersion: string;
}

/** Default state used before the initial fetch completes. */
const DEFAULT_SETTINGS: SettingsState = {
  receipt: {
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
  },
  store: { name: '', address: '', taxId: '', currency: 'IDR', branch: '' },
  sync: { serverUrl: null, hasApiKey: false, enabled: false },
  brand: { colour: '#10b981', storeName: '' },
  preferences: { cardSize: 0, fontSize: 0, fontSmoothing: 'antialiased' },
  currencies: [],
  appVersion: '',
};

/** Public API exposed by `useSettings()`. */
export interface SettingsContextValue {
  /** The current settings snapshot. */
  settings: SettingsState;
  /** True during initial fetch and during active refetch windows. */
  loading: boolean;
  /** Error message when ALL APIs fail; null when at least one succeeded. */
  error: string | null;
  /** True when the most recent load succeeded partially (some APIs failed). */
  hasPartialError: boolean;
  /** Force an immediate full reload (bypasses debounce). */
  refetch: () => Promise<void>;
  /** Keys from the most recent `settings_updated` event (debounced). */
  lastChangedKeys: string[];
  /**
   * Called by save handlers after settings are persisted to the backend.
   * Triggers a debounced scoped refetch so all consumers reflect the change.
   */
  markSettingsUpdated: (keys: string[]) => void;
}

// ── Context ──────────────────────────────────────────────────────────

const SettingsContext = createContext<SettingsContextValue | null>(null);

// ── Key-prefix → scope mapping ──────────────────────────────────────

type SettingsScope = 'receipt' | 'store' | 'sync' | 'brand' | 'preferences' | 'currencies' | 'version';

const SCOPE_PREFIXES: Array<{ prefix: string; scope: SettingsScope }> = [
  { prefix: 'receipt.', scope: 'receipt' },
  { prefix: 'store.', scope: 'store' },
  { prefix: 'currency.', scope: 'currencies' },
  { prefix: 'sync.', scope: 'sync' },
  { prefix: 'brand.', scope: 'brand' },
  { prefix: 'prefs.', scope: 'preferences' },
  { prefix: 'user.', scope: 'preferences' },
];

/** Map a list of changed keys to the unique set of affected scopes. */
function keysToScopes(keys: string[]): Set<SettingsScope> {
  const scopes = new Set<SettingsScope>();
  for (const key of keys) {
    let matched = false;
    for (const { prefix, scope } of SCOPE_PREFIXES) {
      if (key.startsWith(prefix)) {
        scopes.add(scope);
        matched = true;
        break;
      }
    }
    if (!matched) {
      // Unknown key → full refetch
      return new Set<SettingsScope>(['receipt', 'store', 'sync', 'brand', 'preferences', 'currencies', 'version']);
    }
  }
  return scopes;
}

/** DEBOUNCE_MS window for coalescing rapid settings_updated events. */
const DEBOUNCE_MS = 300;

// ── Provider ─────────────────────────────────────────────────────────

interface SettingsProviderProps {
  children: ReactNode;
}

/**
 * Provides a single source of truth for all settings state.
 *
 * Fetches all settings on mount. Supports scoped refetch via
 * `markSettingsUpdated()` — called by save handlers after persisting
 * changes. The refetch is debounced (300ms) so rapid updates
 * (e.g. multiple toggles) trigger a single backend round-trip.
 *
 * When Phase 0e delivers the async event-bus bridge, the context's
 * internal listener will subscribe to `settings_updated` events
 * from the Rust backend for true real-time cross-terminal reactivity.
 */
export function SettingsProvider({ children }: SettingsProviderProps) {
  const [settings, setSettings] = useState<SettingsState>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasPartialError, setHasPartialError] = useState(false);
  const [lastChangedKeys, setLastChangedKeys] = useState<string[]>([]);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingKeysRef = useRef<Set<string>>(new Set());
  const mountedRef = useRef(true);

  // Read userId for preferences API
  const { session } = useAuth();
  const userId = session?.user_id ?? 'default';

  // ── Full load (all APIs) ────────────────────────────────────

  const loadAll = useCallback(async () => {
    setLoading(true);
    setError(null);

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

    let hasAnyFailure = false;
    try {
      if (rR.status === 'fulfilled') {
        setSettings((prev) => ({ ...prev, receipt: rR.value }));
      } else {
        hasAnyFailure = true;
      }
      if (sR.status === 'fulfilled') {
        setSettings((prev) => ({ ...prev, store: sR.value }));
      } else {
        hasAnyFailure = true;
      }
      if (cR.status === 'fulfilled') {
        setSettings((prev) => ({ ...prev, currencies: cR.value }));
      } else {
        hasAnyFailure = true;
      }
      if (syncR.status === 'fulfilled') {
        setSettings((prev) => ({ ...prev, sync: syncR.value }));
      } else {
        hasAnyFailure = true;
      }
      if (prefsR.status === 'fulfilled') {
        const p = prefsR.value;
        const cardSize = p['cardsize'] !== undefined
          ? Math.min(4, Math.max(0, parseInt(p['cardsize'], 10) || 0))
          : 0;
        const fontSize = p['fontsize'] !== undefined
          ? Math.min(4, Math.max(0, parseInt(p['fontsize'], 10) || 0))
          : 0;
        const fontSmoothing = p['font-smoothing'] ?? 'antialiased';
        setSettings((prev) => ({
          ...prev,
          preferences: { cardSize, fontSize, fontSmoothing },
        }));
      } else {
        hasAnyFailure = true;
      }
      if (brandR.status === 'fulfilled') {
        setSettings((prev) => ({
          ...prev,
          brand: {
            colour: brandR.value.primary_colour,
            storeName: brandR.value.store_name,
          },
        }));
      } else {
        hasAnyFailure = true;
      }
      if (verR.status === 'fulfilled') {
        setSettings((prev) => ({ ...prev, appVersion: verR.value.version }));
      } else {
        hasAnyFailure = true;
      }

      if (results.every((r) => r.status === 'rejected')) {
        setError('Failed to load settings');
        setHasPartialError(false);
      } else {
        setHasPartialError(hasAnyFailure);
      }
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [userId]);

  // ── Scoped refetch (key-prefix based) ───────────────────────

  const loadScoped = useCallback(async (keys: string[]) => {
    const scopes = keysToScopes(keys);

    // If full refetch requested, delegate to loadAll
    if (scopes.size >= 6) {
      await loadAll();
      return;
    }

    setLoading(true);
    const tasks: Array<Promise<unknown>> = [];

    if (scopes.has('receipt')) {
      tasks.push(
        getReceiptSettings().then((v) =>
          setSettings((prev) => ({ ...prev, receipt: v })),
        ),
      );
    }
    if (scopes.has('store')) {
      tasks.push(
        getStoreSettings().then((v) =>
          setSettings((prev) => ({ ...prev, store: v })),
        ),
      );
    }
    if (scopes.has('currencies')) {
      tasks.push(
        listCurrencies().then((v) =>
          setSettings((prev) => ({ ...prev, currencies: v })),
        ),
      );
    }
    if (scopes.has('sync')) {
      tasks.push(
        getSyncSettings().then((v) =>
          setSettings((prev) => ({ ...prev, sync: v })),
        ),
      );
    }
    if (scopes.has('preferences')) {
      tasks.push(
        getUserPreferences(userId).then((p) => {
          const cardSize = p['cardsize'] !== undefined
            ? Math.min(4, Math.max(0, parseInt(p['cardsize'], 10) || 0))
            : 0;
          const fontSize = p['fontsize'] !== undefined
            ? Math.min(4, Math.max(0, parseInt(p['fontsize'], 10) || 0))
            : 0;
          const fontSmoothing = p['font-smoothing'] ?? 'antialiased';
          setSettings((prev) => ({
            ...prev,
            preferences: { cardSize, fontSize, fontSmoothing },
          }));
        }),
      );
    }
    if (scopes.has('brand')) {
      tasks.push(
        getBrandSettings().then((v) =>
          setSettings((prev) => ({
            ...prev,
            brand: { colour: v.primary_colour, storeName: v.store_name },
          })),
        ),
      );
    }
    if (scopes.has('version')) {
      tasks.push(
        getVersion().then((v: VersionInfo) =>
          setSettings((prev) => ({ ...prev, appVersion: v.version })),
        ),
      );
    }

    await Promise.allSettled(tasks);
    if (mountedRef.current) setLoading(false);
  }, [userId, loadAll]);

  // ── Debounced update handler ────────────────────────────────

  const markSettingsUpdated = useCallback(
    (keys: string[]) => {
      // Accumulate all keys received within the debounce window
      for (const key of keys) {
        pendingKeysRef.current.add(key);
      }
      setLastChangedKeys(keys);

      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
      debounceRef.current = setTimeout(() => {
        if (!mountedRef.current) return;
        const allKeys = [...pendingKeysRef.current];
        pendingKeysRef.current.clear();
        loadScoped(allKeys);
      }, DEBOUNCE_MS);
    },
    [loadScoped],
  );

  // Wrapped refetch to bypass debounce
  const refetch = useCallback(async () => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    pendingKeysRef.current.clear();
    await loadAll();
  }, [loadAll]);

  // ── Initial load ────────────────────────────────────────────

  useEffect(() => {
    mountedRef.current = true;
    loadAll();
    return () => {
      mountedRef.current = false;
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [loadAll]);

  // ── Memoized value ──────────────────────────────────────────

  const value = useMemo<SettingsContextValue>(
    () => ({
      settings,
      loading,
      error,
      hasPartialError,
      refetch,
      lastChangedKeys,
      markSettingsUpdated,
    }),
    [settings, loading, error, hasPartialError, refetch, lastChangedKeys, markSettingsUpdated],
  );

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
}

// ── Hook ─────────────────────────────────────────────────────────────

/**
 * Access the shared settings state and mutation helpers.
 * Must be called within a `<SettingsProvider>`.
 */
export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx) {
    throw new Error('useSettings must be used within a <SettingsProvider>');
  }
  return ctx;
}

/**
 * Access settings state safely outside of a SettingsProvider.
 * Returns `null` when no provider wraps the calling tree.
 */
export function useOptionalSettings(): SettingsContextValue | null {
  return useContext(SettingsContext);
}
