// ── gateway.ts error-propagation TDD test ─────────────────────────
//
// Bug: getGatewayStatus() catches ALL errors from the underlying
// loggedInvoke('get_setting', ...) calls and returns a synthetic
// fallback [{ name: 'Gateway', configured: false, online: false }].
// This masks real backend failures (DB errors, auth/session expiry,
// missing-settings-table) as "no gateways configured" — the caller
// cannot distinguish a backend outage from a genuine empty-keys state.
//
// The contract violation: success returns 3 named gateways (Stripe,
// Square, QRIS); failure returns 1 generic "Gateway" row. Callers
// iterating the array get a different-length list with no error signal.
//
// This test proves the error is swallowed (RED) then guards the fix
// that propagates the error instead (GREEN).

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import { getGatewayStatus } from '@/api/gateway';

describe('getGatewayStatus error propagation', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('returns 3 named gateways when all settings resolve', async () => {
    // All three get_setting calls return a non-empty key.
    mockInvoke.mockResolvedValue('sk_test_key');
    const result = await getGatewayStatus();
    expect(result).toHaveLength(3);
    expect(result.map((g) => g.name)).toEqual(['Stripe', 'Square', 'QRIS (Midtrans)']);
    expect(result.every((g) => g.configured && g.online)).toBe(true);
  });

  it('returns 3 gateways with configured=false when keys are null', async () => {
    // No keys configured — get_setting returns null for all.
    mockInvoke.mockResolvedValue(null);
    const result = await getGatewayStatus();
    expect(result).toHaveLength(3);
    expect(result.every((g) => !g.configured && !g.online)).toBe(true);
  });

  it('PROPAGATES the error when the backend fails (does not swallow)', async () => {
    // Simulate a backend failure: DB locked, session expired, etc.
    // The invoke rejects with a real error.
    mockInvoke.mockRejectedValue(new Error('database is locked'));

    // The bug: getGatewayStatus currently catches this error and
    // returns [{ name: 'Gateway', configured: false, online: false }].
    // The fix: propagate the error so callers can surface it
    // (error toast, retry UI) instead of silently showing "offline".
    await expect(getGatewayStatus()).rejects.toThrow('database is locked');
  });

  it('does not return a synthetic "Gateway" fallback on error', async () => {
    mockInvoke.mockRejectedValue(new Error('backend unreachable'));
    // The current bug returns a 1-element array with name 'Gateway'.
    // After the fix, the function throws, so no array is returned at all.
    let threw = false;
    let caught: unknown = null;
    try {
      await getGatewayStatus();
    } catch (e) {
      threw = true;
      caught = e;
    }
    expect(threw).toBe(true);
    expect(caught).toBeInstanceOf(Error);
  });
});
