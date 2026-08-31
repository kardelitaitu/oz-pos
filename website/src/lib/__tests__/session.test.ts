// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getSessionToken } from '../session';

/**
 * R1 httpOnly-cookie session helper tests: getSessionToken must prefer the
 * Worker's /__oz/session cookie endpoint and fall back to sessionStorage
 * when the endpoint is absent (no-Worker dev) or returns no token.
 */

describe('getSessionToken — R1 httpOnly cookie', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    sessionStorage.clear();
  });

  it('returns the token from /__oz/session when the cookie exists', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ token: 'cookie.token' }),
    }));

    expect(await getSessionToken()).toBe('cookie.token');
    expect(fetch).toHaveBeenCalledWith('/__oz/session');
  });

  it('falls back to sessionStorage when the endpoint returns no token', async () => {
    sessionStorage.setItem('oz_session', 'stored.token');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ error: 'no token field' }),
    }));

    expect(await getSessionToken()).toBe('stored.token');
  });

  it('falls back to sessionStorage when the endpoint 401s', async () => {
    sessionStorage.setItem('oz_session', 'stored.token');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ error: 'not signed in' }),
    }));

    expect(await getSessionToken()).toBe('stored.token');
  });

  it('falls back to sessionStorage when there is no Worker (fetch rejects)', async () => {
    sessionStorage.setItem('oz_session', 'stored.token');
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('no worker')));

    expect(await getSessionToken()).toBe('stored.token');
  });

  it('returns null when neither the cookie nor sessionStorage has a token', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ error: 'not signed in' }),
    }));

    expect(await getSessionToken()).toBeNull();
  });
});
