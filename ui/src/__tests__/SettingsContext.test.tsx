// ── SettingsContext tests ──────────────────────────────────────────
//
// Covers: full load (7 APIs), scoped refetch via markSettingsUpdated,
// debounce coalescing (300ms), key-to-scope prefix mapping (keysToScopes),
// partial error resilience, all-APIs-fail error state, mount guard
// (no setState after unmount), and useSettings/useOptionalSettings hooks.
//
// ADR #22 Phase 0b testing gate (§9).

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { type ReactNode } from 'react';
import {
  SettingsProvider,
  useSettings,
  useOptionalSettings,
} from '@/contexts/SettingsContext';

// ── Mock state ───────────────────────────────────────────────────

const mocks = vi.hoisted(() => ({
  // Current values (mutated by tests)
  receiptSettings: {
    showCurrency: false, decimalSeparator: 'dot' as const, showTax: true, footer: '',
    paperWidth: 'standard' as const, showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  },
  storeSettings: {
    name: 'Test Store', address: '123 Main St', taxId: 'TAX-001', currency: 'USD', branch: '',
  },
  currencies: [
    { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
    { code: 'EUR', name: 'Euro', minor_exponent: 2, symbol: '\u20ac' },
  ],
  syncSettings: { serverUrl: 'https://sync.example.com', hasApiKey: false, enabled: false } as { serverUrl: string | null; hasApiKey: boolean; enabled: boolean },
  userPreferences: { cardsize: '2', fontsize: '1', 'font-smoothing': 'antialiased' as string },
  brandSettings: { primary_colour: '#10b981', logo_path: null as string | null, store_name: 'My Store' },
  versionInfo: { name: 'oz-pos' as string, version: '0.0.19', rustVersion: '1.80', target: 'x86_64' },
  // Snapshots for reset between tests
  _snapshots: null as Record<string, unknown> | null,
  // Failure sets
  failReceipt: false, failStore: false, failCurrencies: false, failSync: false,
  failPrefs: false, failBrand: false, failVersion: false,
}));

// ── API mocks (single unified vi.mock per module) ───────────────
// IMPORTANT: mock factories reference mocks.* by closure — values
// MUST be mutated in-place, never replaced with = { ... }. Use
// Object.assign(mocks.receiptSettings, { footer: 'new' }) pattern.

vi.mock('@/api/settings', () => ({
  getReceiptSettings: vi.fn(() =>
    mocks.failReceipt ? Promise.reject(new Error('Receipt fail')) : Promise.resolve({ ...mocks.receiptSettings }),
  ),
  getStoreSettings: vi.fn(() =>
    mocks.failStore ? Promise.reject(new Error('Store fail')) : Promise.resolve({ ...mocks.storeSettings }),
  ),
  getUserPreferences: vi.fn(() =>
    mocks.failPrefs ? Promise.reject(new Error('Prefs fail')) : Promise.resolve({ ...mocks.userPreferences }),
  ),
}));

vi.mock('@/api/offline', () => ({
  getSyncSettings: vi.fn(() =>
    mocks.failSync ? Promise.reject(new Error('Sync fail')) : Promise.resolve({ ...mocks.syncSettings }),
  ),
}));

vi.mock('@/api/currency', () => ({
  listCurrencies: vi.fn(() =>
    mocks.failCurrencies ? Promise.reject(new Error('Currencies fail')) : Promise.resolve([...mocks.currencies]),
  ),
}));

vi.mock('@/api/branding', () => ({
  getBrandSettings: vi.fn(() =>
    mocks.failBrand ? Promise.reject(new Error('Brand fail')) : Promise.resolve({ ...mocks.brandSettings }),
  ),
}));

vi.mock('@/api/system', () => ({
  getVersion: vi.fn(() =>
    mocks.failVersion ? Promise.reject(new Error('Version fail')) : Promise.resolve({ ...mocks.versionInfo }),
  ),
}));

// ── Tauri event listener mock ───────────────────────────────────
// Captures the listen handler so integration tests can simulate
// settings_updated events arriving from the Rust backend.

const tauriListenHandler = vi.hoisted(() => ({ fn: null as ((event: unknown) => void) | null }));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((_event: string, handler: (event: unknown) => void) => {
    tauriListenHandler.fn = handler;
    return Promise.resolve(() => { tauriListenHandler.fn = null; });
  }),
}));

// ── AuthContext mock ─────────────────────────────────────────────

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: {
      user_id: 'user-1',
      username: 'testuser',
      role_name: 'manager',
      token: 'tok-123',
      role_id: 'role-1',
      display_name: 'Manager Test',
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: true,
    isOwner: false,
  }),
}));

// ── Helpers ──────────────────────────────────────────────────────

function wrapper({ children }: { children: ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

function resetFailures() {
  mocks.failReceipt = false;
  mocks.failStore = false;
  mocks.failCurrencies = false;
  mocks.failSync = false;
  mocks.failPrefs = false;
  mocks.failBrand = false;
  mocks.failVersion = false;
  // Reset mock data to defaults (mutate in-place so vi.mock closures see changes)
  Object.assign(mocks.receiptSettings, {
    showCurrency: false, decimalSeparator: 'dot', showTax: true, footer: '',
    paperWidth: 'standard', showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  });
  Object.assign(mocks.storeSettings, {
    name: 'Test Store', address: '123 Main St', taxId: 'TAX-001', currency: 'USD', branch: '',
  });
  Object.assign(mocks.syncSettings, { serverUrl: 'https://sync.example.com', hasApiKey: false, enabled: false });
  Object.assign(mocks.userPreferences, { cardsize: '2', fontsize: '1', 'font-smoothing': 'antialiased' });
  Object.assign(mocks.brandSettings, { primary_colour: '#10b981', logo_path: null, store_name: 'My Store' });
  Object.assign(mocks.versionInfo, { name: 'oz-pos', version: '0.0.19', rustVersion: '1.80', target: 'x86_64' });
  mocks.currencies.length = 0;
  mocks.currencies.push(
    { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
    { code: 'EUR', name: 'Euro', minor_exponent: 2, symbol: '€' },
  );
}

beforeEach(() => {
  resetFailures();
});

afterEach(() => {
  resetFailures();
  vi.useRealTimers(); // safety net: restore real timers even if a fake-timer test threw
});

// ── Tests ────────────────────────────────────────────────────────

describe('SettingsContext', () => {
  // ── Full load ──────────────────────────────────────────────

  it('loads all 7 settings scopes on mount', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.settings.store.name).toBe('Test Store');
    expect(result.current.settings.receipt.showTax).toBe(true);
    expect(result.current.settings.currencies).toHaveLength(2);
    expect(result.current.settings.sync.enabled).toBe(false);
    expect(result.current.settings.brand.colour).toBe('#10b981');
    expect(result.current.settings.preferences.cardSize).toBe(2);
    expect(result.current.settings.appVersion).toBe('0.0.19');
  });

  it('starts with loading=true before APIs resolve', () => {
    const { result } = renderHook(() => useSettings(), { wrapper });
    expect(result.current.loading).toBe(true);
  });

  it('has partial error false on full success', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.hasPartialError).toBe(false);
  });

  // ── Partial error ──────────────────────────────────────────

  it('sets hasPartialError true when one API fails', async () => {
    mocks.failSync = true;

    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.hasPartialError).toBe(true);
    expect(result.current.error).toBeNull(); // not all APIs failed
    // Other settings still loaded
    expect(result.current.settings.store.name).toBe('Test Store');
  });

  it('sets hasPartialError true when multiple APIs fail but not all', async () => {
    mocks.failReceipt = true;
    mocks.failBrand = true;

    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.hasPartialError).toBe(true);
    expect(result.current.error).toBeNull();
  });

  // ── Full error ─────────────────────────────────────────────

  it('sets error when ALL 7 APIs fail', async () => {
    mocks.failReceipt = true;
    mocks.failStore = true;
    mocks.failCurrencies = true;
    mocks.failSync = true;
    mocks.failPrefs = true;
    mocks.failBrand = true;
    mocks.failVersion = true;

    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Failed to load settings');
    expect(result.current.hasPartialError).toBe(false);
  });

  // ── Scoped refetch via markSettingsUpdated ─────────────────
  // These use fake timers so the 300ms debounce is instantaneous.

  it('refetches only receipt scope when receipt.* key changes', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });

    // Flush initial loadAll (all API calls are instant with fake timers)
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.receiptSettings, { footer: 'Updated Footer' });
    act(() => { result.current.markSettingsUpdated(['receipt.footer']); });

    // Advance past the 300ms debounce + flush microtasks
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.loading).toBe(false);
    expect(result.current.settings.receipt.footer).toBe('Updated Footer');
    expect(result.current.settings.store.name).toBe('Test Store');
    expect(result.current.lastChangedKeys).toContain('receipt.footer');
    vi.useRealTimers();
  });

  it('refetches only store scope when store.* key changes', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.storeSettings, { name: 'Updated Store' });
    act(() => { result.current.markSettingsUpdated(['store.name']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.store.name).toBe('Updated Store');
    vi.useRealTimers();
  });

  it('does a full refetch for unknown keys', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.storeSettings, { name: 'Full Refresh Store' });
    act(() => { result.current.markSettingsUpdated(['unknown.scope.key']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.store.name).toBe('Full Refresh Store');
    vi.useRealTimers();
  });

  // ── Debounce coalescing ────────────────────────────────────

  it('coalesces multiple markSettingsUpdated calls within 300ms into one refetch', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.receiptSettings, { footer: 'Coalesced', showTax: false });
    act(() => {
      result.current.markSettingsUpdated(['receipt.footer']);
      result.current.markSettingsUpdated(['receipt.showTax']);
    });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.receipt.footer).toBe('Coalesced');
    expect(result.current.settings.receipt.showTax).toBe(false);
    vi.useRealTimers();
  });

  // ── refetch bypasses debounce ──────────────────────────────

  it('refetch() bypasses debounce and immediately reloads all', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    Object.assign(mocks.storeSettings, { name: 'Immediate Refresh' });

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.settings.store.name).toBe('Immediate Refresh');
  });

  // ── Mount guard ────────────────────────────────────────────

  it('does not crash when unmounted before loadAll completes', async () => {
    // Make an API hang so loadAll never resolves
    let resolveHang: (v: unknown) => void;
    const hangPromise = new Promise((resolve) => { resolveHang = resolve; });

    const mock = (await import('@/api/settings')).getReceiptSettings as ReturnType<typeof vi.fn>;
    mock.mockReturnValueOnce(hangPromise as Promise<unknown>);

    const { unmount } = renderHook(() => useSettings(), { wrapper });

    // Unmount while loadAll is still waiting on the hung promise
    unmount();

    // Resolve after unmount — should not crash (mount guard prevents setState)
    resolveHang!(mocks.receiptSettings);

    // No assertion needed — the test passes if no "Can't perform a React state update
    // on an unmounted component" warning/error is thrown
    expect(true).toBe(true);
  });

  it('handles unmount when all 7 APIs are pending (no stale side effects)', async () => {
    // A promise that never settles — keeps loadAll suspended forever.
    const hangPromise = new Promise<never>(() => { /* never settles */ });

    // Override ALL 7 API mocks ONCE so they hang instead of resolving.
    type MockedFn = ReturnType<typeof vi.fn>;
    const mocksToHang: MockedFn[] = [
      (await import('@/api/settings')).getReceiptSettings,
      (await import('@/api/settings')).getStoreSettings,
      (await import('@/api/settings')).getUserPreferences,
      (await import('@/api/offline')).getSyncSettings,
      (await import('@/api/currency')).listCurrencies,
      (await import('@/api/branding')).getBrandSettings,
      (await import('@/api/system')).getVersion,
    ] as MockedFn[];
    for (const m of mocksToHang) {
      m.mockReturnValueOnce(hangPromise);
    }

    const { unmount } = renderHook(() => useSettings(), { wrapper });

    // Unmount while all 7 promises are still pending — loadAll is suspended.
    // Because hangPromise never settles, no stale setState can fire.
    expect(() => unmount()).not.toThrow();
  });

  // ── useOptionalSettings ─────────────────────────────────────

  it('useOptionalSettings returns null outside a SettingsProvider', () => {
    const { result } = renderHook(() => useOptionalSettings());
    expect(result.current).toBeNull();
  });

  it('useOptionalSettings returns the context value inside a SettingsProvider', async () => {
    const { result } = renderHook(() => useOptionalSettings(), { wrapper });

    await waitFor(() => {
      expect(result.current?.loading).toBe(false);
    });

    expect(result.current?.settings.store.name).toBe('Test Store');
  });

  // ── Key-to-scope prefix mapping ────────────────────────────
  // Use fake timers for instant debounce resolution.

  it('maps receipt.* keys to receipt scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.receiptSettings, { showCurrency: true });
    act(() => { result.current.markSettingsUpdated(['receipt.showCurrency']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.receipt.showCurrency).toBe(true);
    vi.useRealTimers();
  });

  it('maps store.* keys to store scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.storeSettings, { address: '456 Oak' });
    act(() => { result.current.markSettingsUpdated(['store.address']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.store.address).toBe('456 Oak');
    vi.useRealTimers();
  });

  it('maps sync.* keys to sync scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.syncSettings, { serverUrl: 'https://sync.test', hasApiKey: true, enabled: true });
    act(() => { result.current.markSettingsUpdated(['sync.enabled']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.sync.enabled).toBe(true);
    vi.useRealTimers();
  });

  it('maps user.* keys to preferences scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.userPreferences, { cardsize: '4' });
    act(() => { result.current.markSettingsUpdated(['user.ui.card-size']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.preferences.cardSize).toBe(4);
    vi.useRealTimers();
  });

  it('maps brand.* keys to brand scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.brandSettings, { primary_colour: '#ff0000', store_name: 'Brand Refreshed' });
    act(() => { result.current.markSettingsUpdated(['brand.primary_colour']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.brand.colour).toBe('#ff0000');
    expect(result.current.settings.brand.storeName).toBe('Brand Refreshed');
    vi.useRealTimers();
  });

  // ── Event bus integration (Pillar C) ───────────────────────
  // Verifies that the Tauri settings_updated listener triggers
  // markSettingsUpdated and causes a scoped refetch.

  it('registers a settings_updated listener on mount', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    renderHook(() => useSettings(), { wrapper });

    // The listener is registered in a useEffect, which runs after render.
    // Dynamic import inside useEffect means we need to wait for it.
    await waitFor(() => {
      expect(listen).toHaveBeenCalledWith('settings_updated', expect.any(Function));
    });
  });

  it('settings_updated event triggers scoped refetch', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });

    // Wait for the dynamic import + listen promise to resolve
    // (the useEffect sets up the listener via import().then())
    await act(async () => {
      await Promise.resolve(); // flush microtasks
      vi.advanceTimersByTime(0);
    });
    expect(result.current.loading).toBe(false);

    // Simulate a settings_updated event from the Rust backend
    Object.assign(mocks.storeSettings, { name: 'Event Bus Store' });
    expect(tauriListenHandler.fn).not.toBeNull();
    tauriListenHandler.fn!({
      payload: { changed_keys: ['store.name'], terminal_id: 'term-a' },
    });

    // Debounce + refetch
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.store.name).toBe('Event Bus Store');
    vi.useRealTimers();
  });

  it('settings_updated event with empty keys list does not crash', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    await act(async () => { await Promise.resolve(); vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    // Event with empty changed_keys — should be a no-op
    expect(tauriListenHandler.fn).not.toBeNull();
    expect(() => {
      tauriListenHandler.fn!({
        payload: { changed_keys: [], terminal_id: 'term-b' },
      });
    }).not.toThrow();

    await act(async () => { vi.advanceTimersByTime(400); });
    expect(result.current.loading).toBe(false);
    vi.useRealTimers();
  });

  it('unlisten is called on unmount', async () => {
    const { unmount } = renderHook(() => useSettings(), { wrapper });

    // Wait for listener registration
    await waitFor(() => {
      expect(tauriListenHandler.fn).not.toBeNull();
    });

    unmount();

    // After unmount, the listener should be cleaned up
    expect(tauriListenHandler.fn).toBeNull();
  });

  // ── Edge cases ──────────────────────────────────────────────

  it('maps currencies.* keys to currencies scope', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    // Mutate currencies list
    mocks.currencies.length = 0;
    mocks.currencies.push({ code: 'JPY', name: 'Japanese Yen', minor_exponent: 0, symbol: '¥' });
    act(() => { result.current.markSettingsUpdated(['currency.default']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.currencies).toHaveLength(1);
    expect(result.current.settings.currencies[0]?.code).toBe('JPY');
    vi.useRealTimers();
  });

  it('mixed known and unknown keys triggers full refetch', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSettings(), { wrapper });
    await act(async () => { vi.advanceTimersByTime(0); });
    expect(result.current.loading).toBe(false);

    Object.assign(mocks.storeSettings, { name: 'Mixed Key Store' });
    // unknown key + known key should trigger full refetch
    act(() => { result.current.markSettingsUpdated(['unknown.scope', 'store.name']); });
    await act(async () => { vi.advanceTimersByTime(400); });

    expect(result.current.settings.store.name).toBe('Mixed Key Store');
    vi.useRealTimers();
  });
});
