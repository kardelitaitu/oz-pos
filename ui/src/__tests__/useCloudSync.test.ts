// ── useCloudSync hook tests ───────────────────────────────────────
//
// Covers the hook in isolation (no RetailOptionsScreen wrapper):
//   1. Lazy localStorage hydration on mount
//   2. IPC token load on mount (get_setting)
//   3. persist() routing (localStorage + updateSyncSettings + set_setting)
//   4. syncNow — empty URL guard, success, backend error, exception, in-flight guard
//   5. Auto-sync setInterval — setup, fire, cleanup, clamp, in-flight no-double-fire
//
// The hook's external dependencies (`@tauri-apps/api/core` for `invoke`,
// and `@/api/offline` for the typed wrappers) are hoisted-via
// `vi.hoisted` so the mock factories can reference them.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { renderHookInAct } from '@/test-utils/renderInAct';
import {
  useCloudSync,
  type L10nLike,
  type ToastInput,
  type UseCloudSyncDeps,
} from '@/hooks/useCloudSync';

// ── Hoisted mock state ────────────────────────────────────────────
//
// `vi.mock` factories are hoisted to the top of the file, so any state
// they reference must be created via `vi.hoisted()` to be available
// when the factory runs.

const mocks = vi.hoisted(() => ({
  syncRun: vi.fn(),
  syncPull: vi.fn(),
  pendingSyncCount: vi.fn(),
  updateSyncSettings: vi.fn(),
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
}));

vi.mock('@/api/offline', () => ({
  syncRun: () => mocks.syncRun(),
  syncPull: () => mocks.syncPull(),
  pendingSyncCount: () => mocks.pendingSyncCount(),
  updateSyncSettings: (args: unknown) => mocks.updateSyncSettings(args),
}));

// ── Test helpers ──────────────────────────────────────────────────

function makeL10n(): L10nLike {
  // The hook's `t()` helper falls back to a default string when
  // `getString` returns `null`, so an always-null L10n exercises the
  // fallback path (the most common case for un-localised test envs).
  return { getString: () => null };
}

function makeDeps(overrides: Partial<UseCloudSyncDeps> = {}): UseCloudSyncDeps {
  return {
    addToast: vi.fn(),
    l10n: makeL10n(),
    ...overrides,
  };
}

/** Pull the captured toast calls into a typed array for assertions. */
function capturedToasts(addToast: ReturnType<typeof vi.fn>): ToastInput[] {
  // `vi.fn()` types `mock.calls` loosely as `any[][]`, but the hook
  // always calls `addToast(toast)` with a single arg, so narrowing via
  // a cast is safe and keeps the helper self-documenting.
  return (addToast.mock.calls as unknown as [ToastInput][]).map(([t]) => t);
}

beforeEach(() => {
  localStorage.clear();

  // Default mock returns — all async calls resolve successfully.
  // Individual tests override as needed.
  mocks.syncRun.mockResolvedValue({ synced: 1, failed: 0, error: null });
  mocks.syncPull.mockResolvedValue({
    productsPulled: 1,
    taxRatesPulled: 0,
    usersPulled: 0,
    error: null,
  });
  mocks.pendingSyncCount.mockResolvedValue(0);
  mocks.updateSyncSettings.mockResolvedValue(undefined);
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_setting') return null;
    if (cmd === 'set_setting') return null;
    return null;
  });
});

afterEach(() => {
  // Any test that called `vi.useFakeTimers()` must be torn down here
  // so the next test starts with real timers.
  vi.useRealTimers();
});

// ── Tests ─────────────────────────────────────────────────────────

describe('useCloudSync', () => {
  // 1. Lazy localStorage hydration ────────────────────────────────
  describe('lazy localStorage hydration', () => {
    it('hydrates all non-secret config keys on mount', async () => {
      localStorage.setItem('retail-sync-enabled', 'true');
      localStorage.setItem('retail-sync-server', 'https://sync.example.com');
      localStorage.setItem('retail-sync-interval', '30');
      localStorage.setItem('retail-sync-last', '2024-01-01 12:00:00');

      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      // The lazy initializers run synchronously during the first render,
      // so these are observable before any effect has a chance to run.
      expect(result.current.enabled).toBe(true);
      expect(result.current.serverURL).toBe('https://sync.example.com');
      expect(result.current.autoMinutes).toBe(30);
      expect(result.current.lastAt).toBe('2024-01-01 12:00:00');
    });

    it('uses default values when localStorage is empty', async () => {
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      expect(result.current.enabled).toBe(false);
      expect(result.current.serverURL).toBe('');
      expect(result.current.autoMinutes).toBe(0);
      expect(result.current.lastAt).toBeNull();
    });

    it('treats an empty lastAt string as null (not an empty timestamp)', async () => {
      localStorage.setItem('retail-sync-last', '');
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));
      expect(result.current.lastAt).toBeNull();
    });
  });

  // 2. IPC token load on mount ───────────────────────────────────
  describe('IPC token load on mount', () => {
    it('loads the auth token from the secure IPC channel', async () => {
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'get_setting') return 'tok-from-ipc';
        return null;
      });

      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      // `renderHookInAct` flushes the mount effect's async IPC inside
      // the act() boundary, so by the time we can assert the hook
      // has already transitioned from the "still loading" state to
      // the "token loaded" state. The intermediate "before" assertion
      // (`tokenLoaded === false`) would only be observable if we
      // used a deferred mock that never resolves — which is a more
      // brittle test setup than just asserting the final state.

      expect(result.current.tokenLoaded).toBe(true);
      expect(result.current.token).toBe('tok-from-ipc');
      expect(mocks.invoke).toHaveBeenCalledWith('get_setting', {
        key: 'sync.auth_token',
      });
    });

    it('handles IPC failure gracefully and still marks tokenLoaded', async () => {
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'get_setting') throw new Error('IPC unavailable');
        return null;
      });

      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      expect(result.current.tokenLoaded).toBe(true);

      // Token stays empty but the UI is un-blocked (the Saved button
      // is enabled as soon as tokenLoaded flips true).
      expect(result.current.token).toBe('');
      expect(result.current.tokenLoaded).toBe(true);
    });
  });

  // 3. persist() routing ─────────────────────────────────────────
  describe('persist() routing', () => {
    it('writes non-secret config to localStorage and routes config to updateSyncSettings + set_setting', async () => {
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        result.current.setAutoMinutes(15);
        result.current.setToken('my-token');
      });

      await act(async () => {
        await result.current.persist('user-1');
      });

      // Non-secret config in localStorage (UI hydration on reload).
      expect(localStorage.getItem('retail-sync-server')).toBe('https://sync.example.com');
      expect(localStorage.getItem('retail-sync-enabled')).toBe('true');
      expect(localStorage.getItem('retail-sync-interval')).toBe('15');

      // Backend settings DB via updateSyncSettings (what sync_run reads).
      expect(mocks.updateSyncSettings).toHaveBeenCalledWith({
        serverUrl: 'https://sync.example.com',
        apiKey: 'my-token',
        enabled: true,
      });

      // Auth token via the secure set_setting channel
      // (backward-compat with the legacy settings DB key).
      expect(mocks.invoke).toHaveBeenCalledWith('set_setting', {
        key: 'sync.auth_token',
        value: 'my-token',
        user_id: 'user-1',
      });
    });

    it('sends null for empty serverURL and token so the backend clears them', async () => {
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      await act(async () => {
        await result.current.persist('user-1');
      });

      expect(mocks.updateSyncSettings).toHaveBeenCalledWith({
        serverUrl: null,
        apiKey: null,
        enabled: false,
      });
    });

    it('swallows localStorage and IPC errors so a partial save is still surfaced elsewhere', async () => {
      mocks.updateSyncSettings.mockRejectedValue(new Error('IPC failed'));
      mocks.invoke.mockRejectedValue(new Error('set_setting failed'));

      // Simulate a quota / private-mode failure on the localStorage
      // write. `vi.spyOn` scopes the override to this test only —
      // the mock is restored in `afterEach` below — so it never leaks
      // into sibling tests.
      const setItemSpy = vi
        .spyOn(Storage.prototype, 'setItem')
        .mockImplementation(() => {
          throw new Error('localStorage quota');
        });

      try {
        const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

        act(() => {
          result.current.setServerURL('https://sync.example.com');
        });

        // Must not throw despite all three channels failing — the
        // parent Save flow should still be able to surface success
        // (or partial success) to the user.
        await act(async () => {
          await result.current.persist('user-1');
        });
      } finally {
        setItemSpy.mockRestore();
      }
    });
  });

  // 4. syncNow — success / failure / exception / in-flight guard ─
  describe('syncNow', () => {
    it('shows an error toast when serverURL is empty and does not call syncRun', async () => {
      const addToast = vi.fn();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps({ addToast })));

      await act(async () => {
        await result.current.syncNow();
      });

      expect(mocks.syncRun).not.toHaveBeenCalled();
      expect(capturedToasts(addToast)).toContainEqual({
        message: 'Sync failed — check server URL and token',
        type: 'error',
      });
    });

    it('success path: flips status to online, persists lastAt, shows success toast, refreshes pending count', async () => {
      mocks.syncRun.mockResolvedValue({ synced: 3, failed: 0, error: null });
      const addToast = vi.fn();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps({ addToast })));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
      });

      await act(async () => {
        await result.current.syncNow();
      });

      expect(result.current.status).toBe('online');
      expect(result.current.syncing).toBe(false);
      expect(result.current.lastAt).not.toBeNull();
      expect(localStorage.getItem('retail-sync-last')).not.toBeNull();
      expect(capturedToasts(addToast)).toContainEqual({
        message: 'Sync completed successfully',
        type: 'success',
      });
      // The round-trip always refreshes the pending count from the
      // authoritative source (the offline queue).
      expect(mocks.pendingSyncCount).toHaveBeenCalled();
    });

    it('failure path: backend error message surfaces in toast and status flips to offline', async () => {
      mocks.syncRun.mockResolvedValue({ synced: 0, failed: 0, error: 'server unreachable' });
      const addToast = vi.fn();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps({ addToast })));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
      });

      await act(async () => {
        await result.current.syncNow();
      });

      expect(result.current.status).toBe('offline');
      expect(capturedToasts(addToast)).toContainEqual({
        message: 'server unreachable',
        type: 'error',
      });
    });

    it('exception path: status flips to offline and the fallback error toast surfaces', async () => {
      mocks.syncRun.mockRejectedValue(new Error('IPC timeout'));
      const addToast = vi.fn();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps({ addToast })));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
      });

      await act(async () => {
        await result.current.syncNow();
      });

      expect(result.current.status).toBe('offline');
      expect(capturedToasts(addToast)).toContainEqual({
        message: 'Sync failed — check server URL and token',
        type: 'error',
      });
    });

    it('in-flight guard: a second syncNow while the first is in flight is a no-op', async () => {
      // The first sync is held open by a never-resolving promise until
      // we explicitly resolve it at the end of the test.
      let resolveFirst!: (v: unknown) => void;
      mocks.syncRun.mockReturnValueOnce(
        new Promise((resolve) => {
          resolveFirst = resolve;
        }),
      );

      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
      });

      // Kick off the first sync (don't await — we want it in-flight).
      let firstPromise!: Promise<void>;
      act(() => {
        firstPromise = result.current.syncNow();
      });

      // Immediately try a second sync — the syncingRef guard must
      // short-circuit it so syncRun is not called twice.
      await act(async () => {
        await result.current.syncNow();
      });

      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      // Resolve the first sync and let it complete so the test
      // doesn't leak a pending promise.
      await act(async () => {
        resolveFirst({ synced: 1, failed: 0, error: null });
        await firstPromise;
      });
    });
  });

  // 5. Auto-sync setInterval — fire / cleanup / clamp / in-flight ─
  describe('auto-sync setInterval', () => {
    it('does not start the interval when enabled is false', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setAutoMinutes(15);
      });

      vi.advanceTimersByTime(900_000);
      expect(mocks.syncRun).not.toHaveBeenCalled();
    });

    it('does not start the interval when autoMinutes is 0', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        // autoMinutes left at the default 0
      });

      vi.advanceTimersByTime(60_000);
      expect(mocks.syncRun).not.toHaveBeenCalled();
    });

    it('fires syncRun at the configured interval when enabled and autoMinutes > 0', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        result.current.setAutoMinutes(15); // → 900_000ms
      });

      // `vi.advanceTimersByTimeAsync` flushes microtasks between
      // timer advances, which is required here: the interval callback
      // kicks off an async `syncNow` (its `finally` block releases
      // `syncingRef` only after the awaited round-trip resolves).
      // Without the async variant the second advance would still see
      // the in-flight guard set and the second syncRun would be
      // short-circuited.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(2);
    });

    it('clamps the interval to MIN_AUTO_INTERVAL_MS (60s) for small autoMinutes', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        // 0.5 min → 30_000ms raw, but Math.max(MIN_AUTO_INTERVAL_MS, …)
        // raises it to 60_000ms. Using a fractional value is the only
        // way to exercise the clamp path itself (autoMinutes=1 yields
        // exactly 60_000ms which is a no-op for the Math.max).
        result.current.setAutoMinutes(0.5);
      });

      // At 30s raw, the clamped 60s interval has not elapsed — no fire.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(30_000);
      });
      expect(mocks.syncRun).not.toHaveBeenCalled();

      // Crossing 60s total — the clamp raised the interval, so the
      // first fire happens here, not at 30s.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(30_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);
    });

    it('cleans up the interval when enabled flips to false (no further fires)', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        result.current.setAutoMinutes(15);
      });

      // Confirm the interval is firing.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      act(() => {
        result.current.setEnabled(false);
      });

      // No further fires after the cleanup effect runs.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_800_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);
    });

    it('cleans up the interval when autoMinutes drops to 0 (no further fires)', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        result.current.setAutoMinutes(15);
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      act(() => {
        result.current.setAutoMinutes(0);
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_800_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);
    });

    it('does not double-fire when the previous round-trip is still in flight', async () => {
      vi.useFakeTimers();
      let resolveFirst!: (v: unknown) => void;
      mocks.syncRun.mockReturnValueOnce(
        new Promise((resolve) => {
          resolveFirst = resolve;
        }),
      );

      const { result } = await renderHookInAct(() => useCloudSync(makeDeps()));

      act(() => {
        result.current.setServerURL('https://sync.example.com');
        result.current.setEnabled(true);
        result.current.setAutoMinutes(15);
      });

      // First fire at 900s — kicks off an in-flight sync. We use the
      // sync variant deliberately here: the test wants the round-trip
      // to *stay* pending so the in-flight guard has something to
      // protect against on the second advance. Wrapped in
      // `await act(async)` because the timer firing executes an
      // async setState chain; without the async act boundary the
      // resulting `setSyncing(true)` lands outside the act and
      // triggers a vitest warning.
      await act(async () => {
        vi.advanceTimersByTime(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      // Second fire while first is still in flight — the syncingRef
      // guard must short-circuit it.
      await act(async () => {
        vi.advanceTimersByTime(900_000);
      });
      expect(mocks.syncRun).toHaveBeenCalledTimes(1);

      // Resolve the first sync so the test doesn't leak a pending
      // promise into the next test. Must be `await act(async)` (not
      // just `act`) so the microtask that runs `syncNow`'s `finally`
      // block (`syncingRef.current = false; setSyncing(false);`) is
      // included in the act() boundary — otherwise vitest emits an
      // act() warning for the late setState.
      await act(async () => {
        resolveFirst({ synced: 1, failed: 0, error: null });
      });
    });
  });
});
