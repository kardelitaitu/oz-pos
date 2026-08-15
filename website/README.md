# OZ-POS Website

Astro marketing site (en + id locales) with docs, pricing, and the license
dashboard. Static build — deployed to Cloudflare Workers static assets.

## Local development

```bash
npm install        # first time
npm run dev        # http://localhost:4321
npm run check      # astro check + i18n audit gate (precheck runs scripts/audit-i18n.mjs)
npm run build      # i18n audit gate + build to dist/
```

`npm run dev` and `npm run build` will print a Vite warning about unset
`PUBLIC_*` vars — expected. The site degrades gracefully:

- `PUBLIC_LICENSE_API_URL` unset → login/account pages render a
  "not configured" state instead of failing.
- `PUBLIC_PADDLE_CLIENT_TOKEN` unset → checkout buttons fall back to a
  contact link (the whole checkout path is dead-code-eliminated at build).
- `PUBLIC_CONTACT_ENDPOINT` unset → contact form falls back to mailto.

## Environment variables (build-time, Astro `PUBLIC_*` only)

See `.env.example` for the full list with comments.

| Variable | Purpose |
|----------|---------|
| `PUBLIC_LICENSE_API_URL` | License server web API (OTP auth + license status) |
| `PUBLIC_PADDLE_CLIENT_TOKEN` | Paddle.js v2 client token (empty = mailto fallback) |
| `PUBLIC_PADDLE_ENVIRONMENT` | Paddle SDK env: `sandbox` or `production` (default `production`) |
| `PUBLIC_CONTACT_ENDPOINT` | Contact-form target on the license server (empty = mailto) |

## Deploy (Cloudflare Workers static assets)

CI deploys on every push to `main` (`.github/workflows/website.yml`), or
manually:

```bash
cd website
PUBLIC_LICENSE_API_URL=https://license.oz-pos.com \
PUBLIC_PADDLE_CLIENT_TOKEN=<token> \
PUBLIC_PADDLE_ENVIRONMENT=sandbox \
npm run build
npx wrangler deploy   # reads wrangler.toml; needs CLOUDFLARE_API_TOKEN + CLOUDFLARE_ACCOUNT_ID
```

`public/_headers` (CSP) and `public/_redirects` (301s) are honored by
Workers static assets exactly as on Pages. Cloudflare Pages (Git
integration) also works — same build command/output; `wrangler.toml` is
then ignored and env vars go in Pages → Settings → Builds.
