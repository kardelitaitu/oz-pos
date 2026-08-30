import { describe, it, expect, vi, beforeEach } from 'vitest';
import worker from '../../worker';

describe('Cloudflare Worker — worker.ts', () => {
  const mockEnv = {
    ASSETS: {
      fetch: vi.fn(async () => new Response('static asset')),
    },
    LICENSE_API_URL: 'https://license.test.ozpos.my.id',
    CONTACT_WEBHOOK_URL: 'https://discord.com/api/webhooks/mock',
  };

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('serves runtime-config.js with no-store headers', async () => {
    const req = new Request('https://ozpos.my.id/__oz/runtime-config.js');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(res.headers.get('Content-Type')).toContain('application/javascript');
    expect(res.headers.get('Cache-Control')).toBe('no-store');

    const text = await res.text();
    expect(text).toContain('https://license.test.ozpos.my.id');
    expect(text).toContain('/api/contact');
  });

  it('handles CORS OPTIONS preflight for /api/contact', async () => {
    const req = new Request('https://ozpos.my.id/api/contact', { method: 'OPTIONS' });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(res.headers.get('Access-Control-Allow-Origin')).toBe('*');
    expect(res.headers.get('Access-Control-Allow-Methods')).toContain('POST, OPTIONS');
  });

  it('returns 405 Method Not Allowed for GET /api/contact', async () => {
    const req = new Request('https://ozpos.my.id/api/contact', { method: 'GET' });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(405);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe('Method Not Allowed');
  });

  it('returns 400 when required contact fields are missing', async () => {
    const req = new Request('https://ozpos.my.id/api/contact', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Alice' }),
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(400);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe('Missing fields');
  });

  it('forwards valid contact message to Discord webhook', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
    });

    const req = new Request('https://ozpos.my.id/api/contact', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: 'Bob',
        email: 'bob@example.com',
        message: 'Hello OZ-POS team!',
      }),
    });

    const res = await worker.fetch(req, mockEnv);
    expect(res.status).toBe(200);
    const body = (await res.json()) as { ok: boolean };
    expect(body.ok).toBe(true);
    expect(global.fetch).toHaveBeenCalledWith(
      'https://discord.com/api/webhooks/mock',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('delegates unmatched paths to static ASSETS', async () => {
    const req = new Request('https://ozpos.my.id/en/docs');
    const res = await worker.fetch(req, mockEnv);

    expect(mockEnv.ASSETS.fetch).toHaveBeenCalledWith(req);
    expect(res.status).toBe(200);
    expect(await res.text()).toBe('static asset');
  });

  // ── Auth gate (ADR #42) ─────────────────────────────────────────

  it('serves dedicated dashboard login page when no cookie', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(mockEnv.ASSETS.fetch).toHaveBeenCalled();
  });

  it('serves dedicated admin login page when no cookie', async () => {
    const req = new Request('https://admin.ozpos.my.id/');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(mockEnv.ASSETS.fetch).toHaveBeenCalled();
  });

  it('no longer sets a session cookie from a deprecated ?token= param (M3)', async () => {
    // The ?token= fallback was removed (Phase 3 item 12); the one-time
    // exchange-code flow (?code=) is the only token handoff. A ?token=
    // request without a session cookie must NOT mint a cookie — it falls
    // through to the login page.
    const req = new Request('https://dashboard.ozpos.my.id/dashboard?token=my.jwt.token');
    const res = await worker.fetch(req, mockEnv);

    expect(res.headers.get('Set-Cookie')).toBeNull();
    // No session cookie → serve the dedicated login page (not a redirect
    // that would carry the token in the URL).
    expect(res.status).toBe(200);
  });

  it('clears the httpOnly cookie on /__oz/logout and redirects to login', async () => {
    const req = new Request('https://admin.ozpos.my.id/__oz/logout', {
      headers: { Cookie: 'oz_session=stale.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(302);
    // Logout redirects to the admin subdomain itself (login page served via
    // the Worker proxy), not the marketing host.
    expect(res.headers.get('Location')).toBe('https://admin.ozpos.my.id/');
    const setCookie = res.headers.get('Set-Cookie');
    expect(setCookie).toContain('oz_session=;');
    expect(setCookie).toContain('Max-Age=0');
    expect(setCookie).toContain('HttpOnly');
  });

  it('serves placeholder dashboard page when cookie is present', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/', {
      headers: { Cookie: 'oz_session=valid.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    // The dashboard SPA is served from ASSETS (rewritten to /dashboard/ path)
    expect(mockEnv.ASSETS.fetch).toHaveBeenCalled();
  });

  it('serves placeholder admin page when cookie is present', async () => {
    const req = new Request('https://admin.ozpos.my.id/', {
      headers: { Cookie: 'oz_session=valid.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(mockEnv.ASSETS.fetch).toHaveBeenCalled();
  });

  it('returns 401 from /__oz/session when no cookie', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/__oz/session');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(401);
    const body = await res.json() as { error: string };
    expect(body.error).toBe('not signed in');
  });

  it('returns token from /__oz/session when cookie present', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/__oz/session', {
      headers: { Cookie: 'oz_session=my.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    const body = await res.json() as { token: string };
    expect(body.token).toBe('my.jwt.token');
  });
});
