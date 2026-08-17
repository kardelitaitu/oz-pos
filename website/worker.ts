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
      })};`;
      return new Response(body, {
        headers: {
          'Content-Type': 'application/javascript; charset=utf-8',
          'Cache-Control': 'no-store',
        },
      });
    }

    // Everything else: serve the static site (the assets binding honors
    // public/_headers and public/_redirects).
    return env.ASSETS.fetch(request);
  },
};
