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

  it('redirects dashboard.ozpos.my.id to login when no cookie', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(302);
    expect(res.headers.get('Location')).toMatch(/^https:\/\/ozpos\.my\.id\/login\?redirect=/);
  });

  it('redirects admin.ozpos.my.id to login when no cookie', async () => {
    const req = new Request('https://admin.ozpos.my.id/');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(302);
    expect(res.headers.get('Location')).toMatch(/^https:\/\/ozpos\.my\.id\/login\?redirect=/);
  });

  it('sets cookie from ?token= param and redirects to clean URL', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/dashboard?token=my.jwt.token');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(302);
    expect(res.headers.get('Location')).toBe('/dashboard');
    const setCookie = res.headers.get('Set-Cookie');
    expect(setCookie).toContain('oz_session=my.jwt.token');
    expect(setCookie).toContain('HttpOnly');
    expect(setCookie).toContain('Secure');
    expect(setCookie).toContain('Domain=.ozpos.my.id');
  });

  it('serves placeholder dashboard page when cookie is present', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/', {
      headers: { Cookie: 'oz_session=valid.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    expect(res.headers.get('Content-Type')).toContain('text/html');
    expect(res.headers.get('Cache-Control')).toBe('no-store');
    expect(await res.text()).toContain('OZ-POS Dashboard');
  });

  it('serves placeholder admin page when cookie is present', async () => {
    const req = new Request('https://admin.ozpos.my.id/', {
      headers: { Cookie: 'oz_session=valid.jwt.token' },
    });
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(200);
    const body = await res.text();
    expect(body).toContain('OZ-POS Admin');
    expect(body).toContain('Admin Dashboard');
  });

  it('removes token param from URL after setting cookie', async () => {
    const req = new Request('https://dashboard.ozpos.my.id/settings?token=abc.def&theme=dark');
    const res = await worker.fetch(req, mockEnv);

    expect(res.status).toBe(302);
    // The remaining query params should be preserved
    expect(res.headers.get('Location')).toBe('/settings?theme=dark');
  });
});
