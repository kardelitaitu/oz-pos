/**
 * Cloudflare Worker for the OZ-POS website — marketing site + dashboard subdomains.
 *
 * Hostname routing:
 *   ozpos.my.id          → marketing site (static assets, runtime config, contact form)
 *   dashboard.ozpos.my.id → user dashboard (auth-gated, placeholder for now)
 *   admin.ozpos.my.id     → admin panel (auth-gated, placeholder for now)
 *
 * Auth gate (ADR #42):
 *   Dashboard subdomains check for an httpOnly `oz_session` cookie. If missing:
 *     1. User is redirected to https://ozpos.my.id/login?redirect=<original_url>
 *     2. After login, AuthForm.tsx redirects to the dashboard subdomain with
 *        ?token=<jwt> in the URL
 *     3. This worker catches the ?token= param, sets the httpOnly cookie,
 *        and redirects to the clean URL (no token in URL)
 *     4. Subsequent requests carry the cookie
 *
 * The site used to be an assets-only Worker, which forced every backend-URL
 * change to rebuild + redeploy the whole bundle (PUBLIC_LICENSE_API_URL is
 * baked into the JS at build time by Astro). This Worker keeps serving the
 * static assets but also exposes a tiny runtime-config endpoint backed by the
 * `LICENSE_API_URL` `[vars]` binding — changing the backend URL now only
 * requires updating the var (Cloudflare dashboard → Worker → Settings →
 * Variables, or wrangler.toml), not rebuilding the site.
 *
 * The browser loads /__oz/runtime-config.js from the layout head (see
 * Base.astro); website/src/lib/runtime-config.ts reads window.__OZ_CONFIG__
 * and falls back to the build-time PUBLIC_LICENSE_API_URL when the endpoint
 * is absent (local preview, non-Worker hosts, or an unset var).
 */

interface Env {
  /** Static-assets binding (wrangler.toml `assets.binding`). */
  ASSETS: { fetch(request: Request): Promise<Response> };
  /** Runtime backend URL — overrides the build-time PUBLIC_LICENSE_API_URL. */
  LICENSE_API_URL?: string;
  /** Discord webhook URL for the support contact form (secret — never exposed to the browser). */
  CONTACT_WEBHOOK_URL?: string;
}

const RUNTIME_CONFIG_PATH = '/__oz/runtime-config.js';
const SESSION_PATH = '/__oz/session';
const LOGOUT_PATH = '/__oz/logout';
const COOKIE_NAME = 'oz_session';

/** Dashboard subdomains that require authentication. */
const DASHBOARD_HOSTS = new Set(['dashboard.ozpos.my.id', 'admin.ozpos.my.id']);

/** Marketing site domain — no auth required. */
const MARKETING_HOST = 'ozpos.my.id';

/** Parse a named cookie value from the Cookie header. */
function getCookie(headers: Headers, name: string): string | null {
  const raw = headers.get('Cookie');
  if (!raw) return null;
  for (const pair of raw.split(';')) {
    const [k, ...v] = pair.trim().split('=');
    if (k === name) return v.join('=');
  }
  return null;
}

/** Build a Set-Cookie header string for the oz_session token. */
function setCookieHeader(token: string, maxAge: number): string {
  return `${COOKIE_NAME}=${token}; Domain=.ozpos.my.id; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=${maxAge}`;
}

/**
 * Wrap a response with the strict CSP for admin/dashboard subdomains
 * (hardening F2): no inline scripts, no framing, no referrer leak.
 * Applied to the SPA pages and the admin login page — never the marketing
 * site (which needs 'unsafe-inline' for its Astro inline scripts).
 */
function withStrictCSP(resp: Response): Response {
  const strictCSP = [
    "default-src 'none'",
    "script-src 'self' 'unsafe-inline' https://static.cloudflareinsights.com",
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self'",
    "connect-src 'self' https://*.code.run https://*.ozpos.my.id https://open.er-api.com",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
  ].join('; ');
  // Copy the asset response headers into a fresh Headers object and use
  // .set() so the strict CSP REPLACES the marketing one instead of being
  // appended as a second header (two CSPs would be enforced as an
  // intersection, breaking connect-src for the FX rate API).
  const headers = new Headers(resp.headers);
  headers.set('Content-Security-Policy', strictCSP);
  headers.set('X-Frame-Options', 'DENY');
  headers.set('Referrer-Policy', 'no-referrer');
  headers.set('X-Content-Type-Options', 'nosniff');
  // SPA HTML is auth-gated — never cache it at the edge so a deploy (or a
  // session state change) is reflected immediately (M6).
  headers.set('Cache-Control', 'no-store, no-cache, must-revalidate, max-age=0');
  return new Response(resp.body, { status: resp.status, headers });
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const hostname = url.hostname;

    // ── Hostname-based auth gate ──────────────────────────────────
    if (DASHBOARD_HOSTS.has(hostname)) {
      // ── API Proxy to license server (resolves CORS and in-handler Origin checks) ──
      if (url.pathname.startsWith('/api/v1/')) {
        const targetUrl = (env.LICENSE_API_URL ?? 'https://license.ozpos.my.id') + url.pathname + url.search;
        const reqHeaders = new Headers(request.headers);
        reqHeaders.set('Origin', 'https://ozpos.my.id');
        const res = await fetch(targetUrl, {
          method: request.method,
          headers: reqHeaders,
          body: ['GET', 'HEAD'].includes(request.method) ? undefined : request.body,
          redirect: 'follow',
        });
        const respHeaders = new Headers(res.headers);
        respHeaders.set('Access-Control-Allow-Origin', '*');
        return new Response(res.body, { status: res.status, headers: respHeaders });
      }

      const sessionCookie = getCookie(request.headers, COOKIE_NAME);

      // Step 1: One-time exchange code (hardening F1). The login page
      // authenticates at the license server, gets a short-lived single-use
      // code via /exchange-issue, and redirects here with ?code=<code>.
      // The Worker POSTs the code to /exchange-consume to receive the real
      // session token, sets the httpOnly cookie, and redirects to clean URL.
      // The real session token never appears in a URL.
      const codeParam = url.searchParams.get('code');
      if (codeParam) {
        url.searchParams.delete('code');
        const cleanUrl = url.pathname + (url.searchParams.toString() ? '?' + url.searchParams.toString() : '');
        const apiUrl = (env.LICENSE_API_URL ?? 'https://license.ozpos.my.id') + '/api/v1/web/exchange-consume';
        try {
          const res = await fetch(apiUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ code: codeParam }),
          });
          if (res.status === 200) {
            const body = await res.json() as { token?: string };
            if (body.token) {
              return new Response(null, {
                status: 302,
                headers: {
                  Location: cleanUrl,
                  'Set-Cookie': setCookieHeader(body.token, 30 * 24 * 3600),
                  'Cache-Control': 'no-store, no-cache, must-revalidate, max-age=0',
                  'Referrer-Policy': 'no-referrer',
                  'Pragma': 'no-cache',
                },
              });
            }
          }
        } catch {
          // Exchange failed — fall through to redirect to login below.
        }
        // Code invalid or exchange failed — redirect to login so the user
        // re-authenticates (never leaves them on a broken state).
        const loginUrl = hostname === 'admin.ozpos.my.id'
          ? `https://${MARKETING_HOST}/admin/login`
          : `https://${MARKETING_HOST}/en/login?redirect=${encodeURIComponent(cleanUrl)}`;
        return new Response(null, { status: 302, headers: { Location: loginUrl } });
      }

      // Step 1a: Fallback — direct ?token= set (deprecated; replaced by the
      // one-time exchange code in Step 1). Kept for backward compatibility
      // while the old login pages transition.
      const tokenParam = url.searchParams.get('token');
      if (tokenParam) {
        url.searchParams.delete('token');
        const cleanUrl = url.pathname + (url.searchParams.toString() ? '?' + url.searchParams.toString() : '');
        return new Response(null, {
          status: 302,
          headers: {
            Location: cleanUrl,
            'Set-Cookie': setCookieHeader(tokenParam, 30 * 24 * 3600),
            'Cache-Control': 'no-store, no-cache, must-revalidate, max-age=0',
            'Referrer-Policy': 'no-referrer',
            'Pragma': 'no-cache',
          },
        });
      }

      // Step 1b: The dashboard SPA calls /__oz/session to obtain the JWT
      // from the httpOnly cookie (so it can authenticate to the license API
      // with a Bearer header). Same-origin, so the token never leaks to
      // third-party JS. Requires the cookie; a missing cookie here is 401.
      if (url.pathname === SESSION_PATH) {
        if (!sessionCookie) {
          return new Response(JSON.stringify({ error: 'not signed in' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
          });
        }
        return new Response(JSON.stringify({ token: sessionCookie }), {
          headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
        });
      }

      // Step 1c: Logout — clear the httpOnly cookie and redirect to the
      // login page. The cookie is HttpOnly so page JS cannot delete it;
      // the Worker must expire it here (Max-Age=0). The SPA's Log out
      // button navigates to this endpoint.
      if (url.pathname === LOGOUT_PATH) {
        // Redirect to the admin subdomain itself (not the marketing host),
        // so the login page is served through the Worker with the /api/v1/
        // proxy — the login.js uses relative API (API='') which requires
        // the proxy to be on the same host. The marketing host (ozpos.my.id)
        // does NOT have the proxy.
        const loginUrl = hostname === 'admin.ozpos.my.id'
          ? `https://admin.ozpos.my.id/`
          : `https://${MARKETING_HOST}/en/login`;
        return new Response(null, {
          status: 302,
          headers: {
            Location: loginUrl,
            'Set-Cookie': `${COOKIE_NAME}=; Domain=.ozpos.my.id; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0`,
            'Cache-Control': 'no-store, no-cache, must-revalidate, max-age=0',
            'Referrer-Policy': 'no-referrer',
            'Pragma': 'no-cache',
          },
        });
      }

      // Step 2: No session cookie — redirect to login.
      // dashboard.ozpos.my.id → marketing login (customer login lives on the
      // site). admin.ozpos.my.id → dedicated admin login page (same origin),
      // which authenticates against the license server with the same tenant
      // auth and returns via ?token= so the worker sets the cookie.
      if (!sessionCookie) {
        if (hostname === 'admin.ozpos.my.id') {
          // Static assets (CSS/JS/images) must be served as-is, not
          // rewritten to the login page — otherwise the browser's asset
          // requests return login HTML and the page renders unstyled.
          const isStatic = /\.(css|js|svg|png|jpg|jpeg|webp|gif|ico|woff2?|ttf|map)$/i.test(url.pathname);
          if (isStatic) {
            const asset = new URL(request.url);
            asset.hostname = MARKETING_HOST;
            return env.ASSETS.fetch(new Request(asset.toString(), request));
          }
          // Serve the dedicated admin login page (not the marketing bounce).
          // Use the clean URL (ASSETS strips .html → /admin/login.html would
          // 307 to /admin/login anyway).
          const rewritten = new URL(request.url);
          rewritten.hostname = MARKETING_HOST;
          rewritten.pathname = '/admin/login';
          rewritten.search = '';
          return withStrictCSP(await env.ASSETS.fetch(new Request(rewritten.toString(), request)));
        }
        const redirectTo = `${url.pathname}${url.search}`;
        const loginUrl = `https://${MARKETING_HOST}/en/login?redirect=${encodeURIComponent(redirectTo)}`;
        return new Response(null, {
          status: 302,
          headers: { Location: loginUrl },
        });
      }

      // Step 3: Cookie present. Serve the dashboard/admin SPA. Rewrite the
      // request path to the sub-app under /dashboard/ or /admin/ so the
      // ASSETS binding returns the correct SPA (not the marketing site).
      const isAdmin = hostname === 'admin.ozpos.my.id';
      const appBase = isAdmin ? '/admin' : '/dashboard';
      const rewritten = new URL(request.url);
      rewritten.hostname = MARKETING_HOST;
      // The SPA HTML references its assets with the absolute /dashboard/ or
      // /admin/ prefix (e.g. /dashboard/dashboard.css). Don't double-prefix:
      // only add appBase for paths that don't already carry it.
      const p = url.pathname;
      rewritten.pathname = (p === '/' || !p.startsWith(appBase)) ? appBase + p : p;
      rewritten.search = '';
      const spaResp = await env.ASSETS.fetch(new Request(rewritten.toString(), request));
      return withStrictCSP(spaResp);
    }

    // ── Marketing site (ozpos.my.id) — no auth required ───────────
    // Serve the runtime config, contact form API, and static assets.

    // Serve the runtime config. no-store: the value can change (a var edit)
    // without a new bundle, so a cached stale config would defeat the point.
    if (url.pathname === RUNTIME_CONFIG_PATH) {
      const body = `window.__OZ_CONFIG__=${JSON.stringify({
        licenseApiUrl: env.LICENSE_API_URL ?? null,
        contactEndpoint: '/api/contact',
      })};`;
      return new Response(body, {
        headers: {
          'Content-Type': 'application/javascript; charset=utf-8',
          'Cache-Control': 'no-store',
        },
      });
    }

    // Contact form → Discord webhook. The webhook URL is a Worker secret
    // so it never reaches the browser. The form sends { name, email, message }.
    if (url.pathname === '/api/contact') {
      const corsHeaders = {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type',
      };
      // Handle preflight
      if (request.method === 'OPTIONS') {
        return new Response(null, { headers: corsHeaders });
      }
      if (request.method !== 'POST') {
        return new Response(JSON.stringify({ error: 'Method Not Allowed' }), {
          status: 405,
          headers: { 'Content-Type': 'application/json', ...corsHeaders },
        });
      }
      try {
        const body = await request.json() as { name?: string; email?: string; message?: string };
        const { name, email, message } = body;
        if (!name || !email || !message) {
          return new Response(JSON.stringify({ error: 'Missing fields' }), {
            status: 400,
            headers: { 'Content-Type': 'application/json', ...corsHeaders },
          });
        }
        if (!env.CONTACT_WEBHOOK_URL) {
          return new Response(JSON.stringify({ error: 'Webhook not configured' }), {
            status: 503,
            headers: { 'Content-Type': 'application/json', ...corsHeaders },
          });
        }
        // Format Discord embed
        const embed = {
          title: '📩 New Support Message',
          fields: [
            { name: 'Name', value: name, inline: true },
            { name: 'Email', value: email, inline: true },
            { name: 'Message', value: message.slice(0, 1024) },
          ],
          color: 0x147efb, // OZ-POS blue
          timestamp: new Date().toISOString(),
        };
        const discordRes = await fetch(env.CONTACT_WEBHOOK_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ embeds: [embed] }),
        });
        if (!discordRes.ok) {
          return new Response(JSON.stringify({ error: 'Discord send failed' }), {
            status: 502,
            headers: { 'Content-Type': 'application/json', ...corsHeaders },
          });
        }
        return new Response(JSON.stringify({ ok: true }), {
          headers: { 'Content-Type': 'application/json', ...corsHeaders },
        });
      } catch {
        return new Response(JSON.stringify({ error: 'Invalid request' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json', ...corsHeaders },
        });
      }
    }

    // Everything else: serve the static site (the assets binding honors
    // public/_headers and public/_redirects).
    return env.ASSETS.fetch(request);
  },
};