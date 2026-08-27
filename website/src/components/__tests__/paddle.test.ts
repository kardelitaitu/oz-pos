// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock runtime-config so paddle.ts's module-level `const API = licenseApiUrl()` resolves.
vi.mock('../../lib/runtime-config', () => ({
  licenseApiUrl: () => 'https://license.test',
}));

/**
 * Unit coverage for paddle.ts pure helpers. The checkout overlay (loadPaddle,
 * openPaddleCheckout) requires a full Paddle SDK mock and is covered by the
 * CheckoutButton integration tests. These pin the session / placeholder logic.
 */

// Dynamic import after mocks are hoisted so the module-level API resolves correctly.
let paddle: typeof import('../paddle');

beforeEach(async () => {
  vi.resetModules();
  paddle = await import('../paddle');
  sessionStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('isPlaceholderPriceId', () => {
  it('returns true for pri_placeholder_ prefixed ids', () => {
    expect(paddle.isPlaceholderPriceId('pri_placeholder_pro_monthly')).toBe(true);
    expect(paddle.isPlaceholderPriceId('pri_placeholder_')).toBe(true);
  });

  it('returns false for real Paddle price ids', () => {
    expect(paddle.isPlaceholderPriceId('pri_01h7x1234567890abcdef')).toBe(false);
  });

  it('returns false for undefined', () => {
    expect(paddle.isPlaceholderPriceId(undefined)).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(paddle.isPlaceholderPriceId('')).toBe(false);
  });
});

describe('hasSession', () => {
  it('returns false when sessionStorage is empty', () => {
    expect(paddle.hasSession()).toBe(false);
  });

  it('returns true when a session token is present', () => {
    sessionStorage.setItem('oz_session', 'tok_abc123');
    expect(paddle.hasSession()).toBe(true);
  });

  it('returns false after the session is cleared', () => {
    sessionStorage.setItem('oz_session', 'tok_abc123');
    paddle.clearSession();
    expect(paddle.hasSession()).toBe(false);
  });
});

describe('clearSession', () => {
  it('removes both oz_session and oz_email from sessionStorage', () => {
    sessionStorage.setItem('oz_session', 'tok_abc');
    sessionStorage.setItem('oz_email', 'user@test.com');
    paddle.clearSession();
    expect(sessionStorage.getItem('oz_session')).toBeNull();
    expect(sessionStorage.getItem('oz_email')).toBeNull();
  });

  it('does not throw when sessionStorage is empty', () => {
    expect(() => paddle.clearSession()).not.toThrow();
  });
});

describe('getSessionEmail', () => {
  it('returns the cached email from sessionStorage', async () => {
    sessionStorage.setItem('oz_email', 'cached@test.com');
    const email = await paddle.getSessionEmail();
    expect(email).toBe('cached@test.com');
  });

  it('fetches from /me when no cached email exists', async () => {
    sessionStorage.setItem('oz_session', 'tok_test');
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ tenant: { email: 'fetched@test.com' } }),
      }),
    );
    const email = await paddle.getSessionEmail();
    expect(email).toBe('fetched@test.com');
    expect(sessionStorage.getItem('oz_email')).toBe('fetched@test.com');
  });

  it('returns null when there is no session token', async () => {
    const email = await paddle.getSessionEmail();
    expect(email).toBeNull();
  });

  it('returns null when the /me endpoint fails', async () => {
    sessionStorage.setItem('oz_session', 'tok_test');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }));
    const email = await paddle.getSessionEmail();
    expect(email).toBeNull();
  });

  it('returns null when fetch throws (network error)', async () => {
    sessionStorage.setItem('oz_session', 'tok_test');
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network')));
    const email = await paddle.getSessionEmail();
    expect(email).toBeNull();
  });

  it('caches the email from /me into sessionStorage', async () => {
    sessionStorage.setItem('oz_session', 'tok_test');
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ tenant: { email: 'new@test.com' } }),
      }),
    );
    await paddle.getSessionEmail();
    expect(sessionStorage.getItem('oz_email')).toBe('new@test.com');
  });
});
