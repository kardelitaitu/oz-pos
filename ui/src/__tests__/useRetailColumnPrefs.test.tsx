/**
 * Unit tests for `useRetailColumnPrefs` — per-user retail grid column
 * visibility, persisted to localStorage with server merge (ADR #36 D4).
 *
 * Same persistence pattern as `useKdsPreferences` (restore instantly from
 * localStorage, merge with the server copy, write-through on change), so the
 * mock shape mirrors that suite: AuthContext/WorkspaceContext provide the
 * user identity, `@/api/settings` is the server, and localStorage is the
 * local cache. `parseColumns` / `readLocalPrefs` must filter out-of-domain
 * column ids and fall back to defaults on corrupt data.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import type { ReactNode } from 'react';
import { useRetailColumnPrefs } from '@/features/retail/hooks/useRetailColumnPrefs';
import { RETAIL_COLUMNS, RETAIL_COLUMN_DEFAULTS } from '@/features/retail/hooks/useRetailColumnPrefs';

// ── Mocks ────────────────────────────────────────────────────────────

let mockUserId = 'user-1';
let mockSessionToken = 'token-1';
let mockServerPrefs: Record<string, string> | null = null;
let mockServerError = false;

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: mockUserId ? { user_id: mockUserId } : null,
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: mockSessionToken,
  }),
}));

vi.mock('@/api/settings', () => ({
  getUserPreferencesScoped: vi.fn(async () => {
    if (mockServerError) throw new Error('server unavailable');
    return mockServerPrefs ?? {};
  }),
  setUserPreferencesScoped: vi.fn(async () => {}),
}));

// ── Wrapper ──────────────────────────────────────────────────────────

function Wrapper({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

// ── Helpers ──────────────────────────────────────────────────────────

const storeKey = (userId: string) => `oz-retail-cols-${userId}`;

// ── Tests ────────────────────────────────────────────────────────────

describe('useRetailColumnPrefs', () => {
  beforeEach(() => {
    localStorage.clear();
    mockUserId = 'user-1';
    mockSessionToken = 'token-1';
    mockServerPrefs = null;
    mockServerError = false;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('returns defaults when no localStorage and no server data', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
    expect(result.current.prefs.hideInactive).toBe(false);
  });

  it('exposes only legal column ids in the defaults', () => {
    expect([...RETAIL_COLUMN_DEFAULTS].every((c) => RETAIL_COLUMNS.includes(c))).toBe(true);
  });

  it('toggleColumn adds a hidden column', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    act(() => {
      result.current.toggleColumn('barcode');
    });
    expect(result.current.prefs.visibleColumns).toContain('barcode');
  });

  it('toggleColumn removes a visible column', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    act(() => {
      result.current.toggleColumn('sku');
    });
    expect(result.current.prefs.visibleColumns).not.toContain('sku');
  });

  it('toggleColumn preserves the other columns', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    act(() => {
      result.current.toggleColumn('stock');
    });
    expect(result.current.prefs.visibleColumns).toContain('name');
    expect(result.current.prefs.visibleColumns).toContain('price');
  });

  it('setHideInactive flips the hide-inactive toggle', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.hideInactive).toBe(false);
    act(() => {
      result.current.setHideInactive(true);
    });
    expect(result.current.prefs.hideInactive).toBe(true);
    act(() => {
      result.current.setHideInactive(false);
    });
    expect(result.current.prefs.hideInactive).toBe(false);
  });

  it('persists column changes to localStorage', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    act(() => {
      result.current.toggleColumn('barcode');
    });
    const stored = localStorage.getItem(storeKey('user-1'));
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!) as { visibleColumns: string[] };
    expect(parsed.visibleColumns).toContain('barcode');
  });

  it('persists hide-inactive to localStorage', () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    act(() => {
      result.current.setHideInactive(true);
    });
    const parsed = JSON.parse(localStorage.getItem(storeKey('user-1'))!) as { hideInactive: boolean };
    expect(parsed.hideInactive).toBe(true);
  });

  it('restores columns from localStorage on init', () => {
    localStorage.setItem(storeKey('user-1'), JSON.stringify({
      visibleColumns: ['sku', 'price', 'notes'],
      hideInactive: true,
    }));
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual(['sku', 'price', 'notes']);
    expect(result.current.prefs.hideInactive).toBe(true);
  });

  it('drops out-of-domain column ids from localStorage', () => {
    localStorage.setItem(storeKey('user-1'), JSON.stringify({
      visibleColumns: ['sku', 'not-a-column', 'price'],
      hideInactive: false,
    }));
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual(['sku', 'price']);
  });

  it('falls back to defaults when localStorage has no valid columns', () => {
    localStorage.setItem(storeKey('user-1'), JSON.stringify({
      visibleColumns: ['bogus', 'also-bogus'],
      hideInactive: false,
    }));
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
  });

  it('falls back to defaults when localStorage is corrupt JSON', () => {
    localStorage.setItem(storeKey('user-1'), '{not valid json');
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
    expect(result.current.prefs.hideInactive).toBe(false);
  });

  it('falls back to defaults when localStorage is not an array', () => {
    localStorage.setItem(storeKey('user-1'), JSON.stringify({ visibleColumns: 'sku,price' }));
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
  });

  it('returns loading=true initially and false after server fetch', async () => {
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.loading).toBe(true);
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.loading).toBe(false);
  });

  it('merges server prefs after the fetch resolves', async () => {
    mockServerPrefs = {
      retail_visible_columns: JSON.stringify(['sku', 'price']),
      retail_hide_inactive: 'true',
    };
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.prefs.visibleColumns).toEqual(['sku', 'price']);
    expect(result.current.prefs.hideInactive).toBe(true);
    expect(result.current.loading).toBe(false);
  });

  it('caches the server merge into localStorage', async () => {
    mockServerPrefs = { retail_visible_columns: JSON.stringify(['name']) };
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    void result;
    const parsed = JSON.parse(localStorage.getItem(storeKey('user-1'))!) as { visibleColumns: string[] };
    expect(parsed.visibleColumns).toEqual(['name']);
  });

  it('parses a malformed server columns string back to defaults', async () => {
    mockServerPrefs = { retail_visible_columns: '{broken', retail_hide_inactive: 'false' };
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
  });

  it('keeps localStorage prefs when the server is unavailable', async () => {
    localStorage.setItem(storeKey('user-1'), JSON.stringify({
      visibleColumns: ['sku', 'barcode'],
      hideInactive: true,
    }));
    mockServerError = true;
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.prefs.visibleColumns).toEqual(['sku', 'barcode']);
    expect(result.current.prefs.hideInactive).toBe(true);
  });

  it('returns defaults immediately when there is no signed-in user', async () => {
    mockUserId = '';
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
    // No session → the load effect resolves without a server call.
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.loading).toBe(false);
    expect(result.current.prefs.visibleColumns).toEqual([...RETAIL_COLUMN_DEFAULTS]);
  });

  it('skips the server write when there is no signed-in user', async () => {
    mockUserId = '';
    const { result } = renderHook(() => useRetailColumnPrefs(), { wrapper: Wrapper });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    // Local state still toggles (the local-first UX), but the server call is
    // guarded on a session — persist() early-returns.
    act(() => {
      result.current.toggleColumn('barcode');
      result.current.setHideInactive(true);
    });
    expect(result.current.prefs.visibleColumns).toContain('barcode');
    expect(result.current.prefs.hideInactive).toBe(true);
    const settingsMock = (await import('@/api/settings')) as { setUserPreferencesScoped: ReturnType<typeof vi.fn> };
    expect(settingsMock.setUserPreferencesScoped).not.toHaveBeenCalled();
  });
});