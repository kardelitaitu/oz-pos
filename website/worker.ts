/**
 * Cloudflare Worker for the OZ-POS marketing site (Worker + static assets).
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

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

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
    if (url.pathname === '/api/contact' && request.method === 'POST') {
      const corsHeaders = {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type',
      };
      // Handle preflight
      if (request.method === 'OPTIONS') {
        return new Response(null, { headers: corsHeaders });
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
