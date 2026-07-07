import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { useFeatures, FEATURES } from '@/hooks/useFeatures';

// ── Mocks ────────────────────────────────────────────────────────────

const mockGetEnabledFeatures = vi.fn();

vi.mock('@/api/settings', () => ({
  getEnabledFeatures: (...args: unknown[]) => mockGetEnabledFeatures(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────

const ALL_FEATURE_KEYS = Object.values(FEATURES);

function resolveFeatures(keys: string[]) {
  mockGetEnabledFeatures.mockResolvedValue({ features: keys });
}

function rejectFeatures(message: string) {
  mockGetEnabledFeatures.mockRejectedValue(new Error(message));
}

function pendingFeatures() {
  mockGetEnabledFeatures.mockImplementation(() => new Promise(() => {}));
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('useFeatures', () => {
  beforeEach(() => {
    mockGetEnabledFeatures.mockReset();
  });

  // ── Lifecycle / State ──────────────────────────────────────────

  it('starts with loading=true and loaded=false', () => {
    pendingFeatures();
    const { result } = renderHook(() => useFeatures());

    expect(result.current.loading).toBe(true);
    expect(result.current.loaded).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.enabled.size).toBe(0);
  });

  it('sets loading=false and loaded=true after successful load', async () => {
    resolveFeatures(['simple-retail', 'cash-payment']);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.loaded).toBe(true);
    expect(result.current.error).toBeNull();
  });

  // ── Successful load ────────────────────────────────────────────

  it('populates enabled set from API response', async () => {
    resolveFeatures(['simple-retail', 'inventory-tracking', 'tax-engine']);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.enabled.size).toBe(3);
    });

    expect(result.current.enabled.has('simple-retail')).toBe(true);
    expect(result.current.enabled.has('inventory-tracking')).toBe(true);
    expect(result.current.enabled.has('tax-engine')).toBe(true);
  });

  it('handles empty features array', async () => {
    resolveFeatures([]);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.enabled.size).toBe(0);
    expect(result.current.error).toBeNull();
  });

  // ── isEnabled ──────────────────────────────────────────────────

  it('isEnabled returns true for enabled features', async () => {
    resolveFeatures(['cash-payment', 'card-payment', 'staff-login']);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isEnabled('cash-payment')).toBe(true);
    expect(result.current.isEnabled('card-payment')).toBe(true);
    expect(result.current.isEnabled('staff-login')).toBe(true);
  });

  it('isEnabled returns false for non-enabled features', async () => {
    resolveFeatures(['cash-payment']);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isEnabled('nfc-reader')).toBe(false);
    expect(result.current.isEnabled('loyalty-program')).toBe(false);
    expect(result.current.isEnabled('barcode-scanning')).toBe(false);
  });

  it('isEnabled respects feature key casing', async () => {
    resolveFeatures(['Simple-Retail']); // different casing
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // isEnabled uses exact Set.has(), so casing matters
    expect(result.current.isEnabled('simple-retail')).toBe(false);
    expect(result.current.isEnabled('Simple-Retail')).toBe(true);
  });

  // ── filterRoutes ───────────────────────────────────────────────

  it('filterRoutes passes routes without feature requirements', async () => {
    resolveFeatures([]);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Routes that don't map to a feature in ROUTE_FEATURE always pass
    const filtered = result.current.filterRoutes(['products', 'settings', 'staff']);
    expect(filtered).toEqual(['products', 'settings', 'staff']);
  });

  it('filterRoutes filters out routes whose feature is disabled', async () => {
    resolveFeatures([]); // no features enabled
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // 'sales' requires SIMPLE_RETAIL, 'stock-transfers' requires STOCK_TRANSFERS
    const filtered = result.current.filterRoutes(['sales', 'stock-transfers', 'settings']);
    // Both feature-gated routes are filtered out; settings passes through
    expect(filtered).toEqual(['settings']);
  });

  it('filterRoutes includes routes whose feature is enabled', async () => {
    resolveFeatures(['simple-retail', 'stock-transfers']);
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const filtered = result.current.filterRoutes(['sales', 'stock-transfers', 'settings']);
    expect(filtered).toEqual(['sales', 'stock-transfers', 'settings']);
  });

  it('filterRoutes handles mixed scenario', async () => {
    resolveFeatures(['simple-retail']); // SIMPLE_RETAIL on, STOCK_TRANSFERS off
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const filtered = result.current.filterRoutes([
      'sales',          // requires SIMPLE_RETAIL → enabled
      'stock-transfers', // requires STOCK_TRANSFERS → disabled
      'products',       // no feature required → always pass
    ]);
    expect(filtered).toEqual(['sales', 'products']);
  });

  // ── Error fallback ─────────────────────────────────────────────

  it('sets error message on API failure', async () => {
    rejectFeatures('Tauri IPC not available');
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Tauri IPC not available');
  });

  it('falls back to enabling all features on error', async () => {
    rejectFeatures('Connection refused');
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // All features should be enabled as a fallback
    expect(result.current.enabled.size).toBe(ALL_FEATURE_KEYS.length);
    for (const key of ALL_FEATURE_KEYS) {
      expect(result.current.enabled.has(key)).toBe(true);
    }
    expect(result.current.isEnabled('loyalty-program')).toBe(true);
    expect(result.current.isEnabled('kds-order')).toBe(false); // this isn't a real feature key
  });

  it('uses default error message for non-Error rejections', async () => {
    // Reject with a string instead of Error
    mockGetEnabledFeatures.mockRejectedValue('unknown rejection');
    const { result } = renderHook(() => useFeatures());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Failed to load features');
  });

  // ── Cancellation ───────────────────────────────────────────────

  it('does not update state after unmount', async () => {
    let resolvePromise!: (value: { features: string[] }) => void;
    mockGetEnabledFeatures.mockImplementation(
      () => new Promise<{ features: string[] }>((resolve) => {
        resolvePromise = resolve;
      }),
    );

    const { result, unmount } = renderHook(() => useFeatures());

    // Should be loading
    expect(result.current.loading).toBe(true);

    // Unmount before promise resolves
    unmount();

    // Resolve after unmount — the cancelled flag prevents state updates
    // so no "Can't perform a React state update on an unmounted component"
    // warning is emitted.
    await act(async () => {
      resolvePromise({ features: ['simple-retail', 'cash-payment'] });
    });
  });
});
