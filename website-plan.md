# OZ-POS Website Plan

> Static marketing site hosted on Cloudflare Pages — two locales, Paddle checkout,
> tenant-email auth via the license server (PocketBase datastore on Northflank).

---

## 1. Overview

| Aspect | Detail |
|--------|--------|
| **Purpose** | Marketing site + pricing page + license purchase flow |
| **Locales** | `en` (global/international) and `id` (Indonesia) |
| **Hosting** | Cloudflare Pages (free tier) — static files only |
| **Checkout** | Paddle.js overlay (handles payments, tax, VAT, invoicing) |
| **Auth** | Tenant-email OTP sessions issued by the **license server** — no new auth collection |
| **Database** | PocketBase (existing license-server datastore on Northflank) — **internal only**, the website never calls it directly |
| **Pricing** | Real tier enum `free` / `trial` / `pro` / `premium` / `enterprise`; placeholder prices per locale |
| **Repo location** | `website/` directory in the monorepo |

> **Grounding:** everything in this plan maps to code that already exists
> (`apps/license-server/`, `crates/oz-core/src/subscription.rs`,
> `apps/license-server/pb_schema.json`). Sections marked **(new server work)**
> are endpoints the license server does not have yet — they must be built,
> not assumed.

---

## 2. Tech Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Framework | **Astro** | Static-first, partial hydration, built-in i18n routing |
| Styling | **Tailwind CSS** | Fast to build, consistent design system |
| Checkout | **Paddle.js overlay** | No backend — Paddle handles tax, VAT, invoicing |
| Auth | **License server API (Go)** | Tenant identity already lives there; the Go router issues OTP sessions and already rate-limits |
| Hosting | **Cloudflare Pages** | Free, fast global CDN, auto deploys from Git |
| Analytics | Cloudflare Web Analytics | Free, privacy-friendly |

### Why auth goes through the license server, not PocketBase directly

The license server already runs PocketBase on Northflank with **four
collections**: `license_keys`, `tenants`, `subscriptions`, `tenant_machines`
(see `apps/license-server/pb_schema.json`). The customer's identity in this
system is the **`tenants` record** — `email` (unique), `phone`, and the
bcrypt-hashed `api_key` used by the POS client.

The plan's original `web_users` collection would create a **second, disjoint
auth system** for the same customer — the opposite of futureproof. Instead:

- the website account **is** the tenant;
- all browser traffic goes to the license server's Go router
  (`ratelimit.go`, CORS-controlled) — PocketBase is never exposed to the
  browser (its admin UI and `/api/collections/*` surface stay internal);
- authentication is **email OTP** — no password field exists on `tenants`
  today, so OTP avoids adding and resetting a new credential class.

```
Website (Cloudflare Pages, static)
    │
    ├── Paddle.js checkout → payment processed
    │
    └── License Server API (Northflank, Go + PocketBase)   ← all new endpoints
        ├── POST /api/v1/web/request-otp     (new)  email → 6-digit code
        ├── POST /api/v1/web/verify-otp      (new)  email + code → session + subscription
        ├── POST /api/v1/web/me              (new)  Bearer → current tenant + subscription
        ├── POST /api/v1/web/logout          (new)  invalidate session
        └── POST /api/v1/paddle/webhook      (new)  Paddle events → mint key + subscription
            └── PocketBase (internal): tenants · subscriptions · license_keys · tenant_machines
```

---

## 3. Site Structure

```
website/
├── astro.config.mjs
├── package.json
├── tailwind.config.ts
├── tsconfig.json
├── public/
│   ├── favicon.ico
│   ├── og-image.png
│   └── fonts/
├── src/
│   ├── layouts/
│   │   └── Base.astro
│   ├── components/
│   │   ├── Header.astro
│   │   ├── Footer.astro
│   │   ├── PricingCard.astro
│   │   ├── FeatureTable.astro
│   │   ├── Hero.astro
│   │   ├── CheckoutButton.tsx    # Interactive: Paddle.js overlay
│   │   ├── AuthForm.tsx          # Interactive: OTP request/verify (license server API)
│   │   └── LocaleSwitcher.tsx
│   ├── content/
│   │   └── pricing/
│   │       ├── en.ts
│   │       └── id.ts
│   ├── i18n/
│   │   ├── en.json
│   │   └── id.json
│   └── pages/
│       ├── index.astro
│       ├── [locale]/
│       │   ├── index.astro
│       │   ├── pricing.astro
│       │   ├── features.astro
│       │   ├── download.astro
│       │   ├── login.astro
│       │   ├── account.astro
│       │   └── legal/
│       │       ├── privacy.astro
│       │       └── terms.astro
│       └── 404.astro
└── wrangler.toml
```

---

## 4. Locale Strategy

### Routing (path-based only — no subdomains)

| URL | Locale | Content |
|-----|--------|---------|
| `oz-pos.com/` | Auto-detect → redirect | Detects country or browser lang |
| `oz-pos.com/en/` | English | Global pricing (USD) |
| `oz-pos.com/id/` | Bahasa Indonesia | Indonesia pricing (IDR) |
| `oz-pos.com/en/pricing` | English pricing | USD price cards |
| `oz-pos.com/id/pricing` | Indonesian pricing | IDR price cards |

### Locale Detection

| Signal | Method | Priority |
|--------|--------|----------|
| URL | Explicit `/en/` or `/id/` | Highest |
| Cloudflare header | `CF-IPCountry: ID` | High |
| Browser language | `navigator.language.startsWith('id')` | Low |
| Manual toggle | User clicks toggle in header | Override |

---

## 5. Registration & Auth (Tenant Email OTP)

### Why no self-signup

A web account is only useful once a customer has a license. Tenant records
are **created by the Paddle webhook at first purchase** (email comes from
Paddle). Trial users who never buy run the offline 90-day trial (`trial`
tier) and have no web account — nothing to manage.

### Identity = the `tenants` collection (exists today)

| Field | Type | Notes |
|-------|------|-------|
| `id` | text (auto) | PocketBase record id |
| `email` | email, unique index | The login identifier |
| `phone` | email-adjacent, **required today** | ⚠️ webhook must supply it (Paddle custom field) or the schema must relax it |
| `api_key` | text (bcrypt hash) | POS client credential — not a web password |
| `api_key_lookup` | text (SHA-256, indexed) | O(1) tenant resolution for the POS API |
| `status` | select: active / suspended / revoked | Gate web sessions on this |

### New license-server endpoints (new server work)

All web endpoints run inside the existing Go router (rate-limited, CORS
allow-listed) and read/write PocketBase server-side. None are raw
`/api/collections/*` calls from the browser.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/web/request-otp` | POST | `{email}` → looks up tenant, sends 6-digit code via SMTP. **Always returns 200** (no account enumeration). |
| `/api/v1/web/verify-otp` | POST | `{email, code}` → issues a short-lived session token + subscription summary |
| `/api/v1/web/me` | GET | `Authorization: Bearer <token>` → tenant profile + subscription + license status |
| `/api/v1/web/logout` | POST | Invalidates the session token |

### Why OTP, not password

- `tenants` has no password field; OTP means no new credential to store,
  hash, or reset.
- The license server already sends email (verification/receipts) — SMTP is
  already an env-var concern on Northflank.
- OTP is the natural upgrade path to magic-link / WebAuthn later.

### Session token storage (decision)

| Approach | Storage | Security | Status |
|----------|---------|----------|--------|
| **sessionStorage** (v1, shipped) | `oz_session` in `sessionStorage`, set by `/verify-otp` | Scoped to the tab, cleared on tab close (better than localStorage), survives reloads — good UX for an account page | v1 |
| httpOnly cookie | Set by license server with `SameSite=None; Secure` + `X-CSRF-Token` header | Best, but cross-site cookie handling + CSRF needed | Hardening follow-up |
| localStorage | Browser storage | Vulnerable to XSS persistence | **Never** |

v1 stores the short-lived session token (default 24h TTL, server-side expiry)
in `sessionStorage` — scoped to the tab and cleared on close, unlike
localStorage. The account page reads it to call `/me`; any 401 clears it and
returns the user to the signed-out state. Hardening path stays the httpOnly
cookie + CSRF header.

### Email delivery (license server SMTP env vars on Northflank)

| Email Type | When | Content |
|------------|------|---------|
| OTP Code | Login request | 6-digit code |
| License receipt | Paddle webhook | Key string + tier + expiry |
| Subscription events | Created / cancelled / payment failed | Status notices |

---

## 6. Pricing (Placeholder — mapped to the real tier enum)

> **Placeholder prices — the user will modify them.** Tier names below are
> the **actual** `tier_key` values the schema and the client understand
> (`free`, `trial`, `pro`, `premium`, `enterprise` in
> `apps/license-server/pb_schema.json` and `crates/oz-core/src/subscription.rs`).

### Global (USD)

| Tier | tier_key | Price | Type |
|------|----------|-------|------|
| Free | `trial` | $0 | 90-day trial |
| Pro | `pro` | $19/mo | Monthly |
| Premium | `premium` | $49/mo | Monthly |
| Enterprise | `enterprise` | Custom | Contact sales |

### Indonesia (IDR)

| Tier | tier_key | Price | Type |
|------|----------|-------|------|
| Free | `trial` | Rp 0 | 90-day trial |
| Pro | `pro` | Rp 299.000/mo | Monthly |
| Premium | `premium` | Rp 749.000/mo | Monthly |
| Enterprise | `enterprise` | Custom | Contact sales |

> **"1-Time / perpetual" is NOT in the tier enum today.** Adding it means a
> new `tier_key` value (e.g. `lifetime`) in the schema **and** client-side
> quota/licensing changes (`crates/oz-core/src/subscription.rs`). Excluded
> from v1 — decide separately.

### Feature Comparison (placeholders)

| Feature | Free (`trial`) | Pro | Premium | Enterprise |
|---------|----------------|-----|---------|------------|
| Duration | 90 days | Monthly | Monthly | Custom |
| Stores | 1 | 1 | Unlimited | Unlimited |
| Registers | 1 | 2 | Unlimited | Unlimited |
| Warehouses | 1 | 1 | Unlimited | Unlimited |
| QRIS Payment | ✗ | ✓ | ✓ | ✓ |
| Cloud Sync | ✗ | ✓ | ✓ | ✓ |
| Lua Scripting | ✗ | ✗ | ✓ | ✓ |
| Priority Support | ✗ | ✗ | ✓ | ✓ |

---

## 7. Paddle Integration

### Setup Steps (One-Time)

1. Create Paddle account at [paddle.com](https://paddle.com)
2. Add products in Paddle dashboard (6 products: Pro/Premium/Enterprise × 2 locales)
3. Create a webhook pointing at the **license server** (`https://license.oz-pos.com/api/v1/paddle/webhook`)
4. Get credentials: `PADDLE_VENDOR_ID`, `PADDLE_CLIENT_TOKEN` (site) and
   `PADDLE_WEBHOOK_SECRET` (server, new env var)

### Checkout Flow

```
User clicks "Buy" → Paddle.js overlay → payment + VAT handled by Paddle
→ Paddle webhook → license server → license key generated + emailed
→ customer activates in the POS with the key
```

### Paddle Product Mapping (6 products)

| Product ID | Locale | tier_key | Price |
|------------|--------|----------|-------|
| `PADDLE_PRO_GLOBAL` | Global | `pro` | $19/mo |
| `PADDLE_PREMIUM_GLOBAL` | Global | `premium` | $49/mo |
| `PADDLE_ENT_GLOBAL` | Global | `enterprise` | Custom |
| `PADDLE_PRO_ID` | Indonesia | `pro` | Rp 299.000/mo |
| `PADDLE_PREMIUM_ID` | Indonesia | `premium` | Rp 749.000/mo |
| `PADDLE_ENT_ID` | Indonesia | `enterprise` | Custom |

### Webhook: License Server (not a Worker)

Paddle webhooks go **directly to the license server** on Northflank. The
license server already owns the RSA signing key and PocketBase — a
Cloudflare Worker would only add a hop. (This supersedes any earlier
"Worker" mention.)

```
Paddle webhook
    │
    ▼
https://license.oz-pos.com/api/v1/paddle/webhook        (new server work)
    │
    ├── Verify Paddle signature (v1 public key / v2 webhook secret)
    ├── Dedup by event_id (Paddle retries — replays must be no-ops)
    ├── Parse event (subscription.created / updated / cancelled, transaction.completed)
    ├── Map Paddle product_id → tier_key (table above)
    ├── Upsert tenant by email (Paddle supplies email; collect phone as custom field)
    ├── Mint a human-readable license key string (e.g. OZ-PRO-XXXX-XXXX)
    ├── Insert license_keys record + create/update subscriptions:
    │   signed_payload + signature = the EXISTING RSA signing (not the key string)
    └── Return 200 OK (non-200 → Paddle retries)
```

> **Two different "keys":** the human-readable license key the customer
> types into the POS (a `license_keys` record) and the RSA-signed
> subscription payload (`signed_payload` + `signature` on `subscriptions`).
> The webhook must generate the first and *sign* the second — the RSA key
> does not "generate" the license key string.

### Paddle Webhook Events

| Event | Action |
|-------|--------|
| `subscription.created` | Upsert tenant, mint key, create subscription (RSA-signed) |
| `subscription.updated` | Update tier/status on `subscriptions` |
| `subscription.cancelled` | Mark subscription canceled (grace period via `grace_until`) |
| `subscription.payment_failed` | Flag for follow-up |
| `transaction.completed` | One-time purchases (only when a `lifetime` tier ships) |

---

## 8. Pages Breakdown

### Homepage (`/[locale]/`)

| Section | Content |
|---------|---------|
| Hero | "The Offline-First POS Platform" + CTA |
| Features Grid | 6 key features with icons |
| Screenshots | Desktop + tablet screenshots |
| Pricing Preview | Link to pricing page |
| CTA Banner | "Start Your Free Trial" |

### Pricing Page (`/[locale]/pricing`)

| Element | Detail |
|---------|--------|
| Tier Cards | Trial / Pro / Premium / Enterprise (real tier enum) |
| Feature Comparison | Full matrix below cards |
| Buy Buttons | Paddle overlay checkout (product id per locale) |
| Trust Signals | "30-day money back" · "Cancel anytime" |

### Login Page (`/[locale]/login`)

| Element | Detail |
|---------|--------|
| Email field | Request OTP (`/api/v1/web/request-otp`) |
| Code field | Verify (`/api/v1/web/verify-otp`) |
| Note | No password to reset — OTP only |

### Account Page (`/[locale]/account`)

| Element | Detail |
|---------|--------|
| License Info | `license_keys` record: key, tier, status, expiry |
| Subscription | `subscriptions` record: tier, next billing, `grace_until` |
| Actions | Change plan (Paddle link), cancel (via Paddle) |

Data comes from `/api/v1/web/me` — the account page is read-only.

### Download Page (`/[locale]/download`)

| Element | Detail |
|---------|--------|
| Platform Cards | Windows / macOS / Linux |
| Version Info | Current version (0.0.25), release date |
| System Requirements | OS version, RAM, disk |

---

## 9. Design System

### Color Palette

| Token | Value | Usage |
|-------|-------|-------|
| `--color-primary` | `#1a1a2e` | Dark navy — primary backgrounds |
| `--color-accent` | `#e94560` | Red accent — CTAs, highlights |
| `--color-surface` | `#16213e` | Card backgrounds |
| `--color-text` | `#eee2dc` | Body text |
| `--color-muted` | `#8d99ae` | Secondary text |

### Responsive Breakpoints

| Breakpoint | Width | Layout |
|------------|-------|--------|
| Mobile | < 640px | Single column |
| Tablet | 640–1024px | 2-column grid |
| Desktop | > 1024px | Full layout |

---

## 10. Deployment

### Cloudflare Pages

| Setting | Value |
|---------|-------|
| Framework | Astro |
| Build command | `npm run build` |
| Output | `dist/` |
| Root dir | `website/` |

### Custom Domain (path-based locales — no subdomains)

| Domain | Locale |
|--------|--------|
| `oz-pos.com` | Root → auto-redirect |
| `oz-pos.com/en` | English (global) |
| `oz-pos.com/id` | Bahasa Indonesia |

### Environment Variables

**Cloudflare Pages (site):**

| Variable | Value | Purpose |
|----------|-------|---------|
| `LICENSE_API_URL` | `https://license.oz-pos.com` | Web auth + license API (replaces any direct PocketBase URL) |
| `PADDLE_VENDOR_ID` | `12345` | Paddle vendor ID |
| `PADDLE_CLIENT_TOKEN` | `xxxxx` | Paddle.js token |

**Northflank (license server, new):**

| Variable | Value | Purpose |
|----------|-------|---------|
| `PADDLE_WEBHOOK_SECRET` | `xxxxx` | Verify webhook signatures (HMAC-SHA256 over `ts:rawBody`, 5-min replay window) |
| `PADDLE_PRICE_TIERS` | `pri_x:pro,pri_y:premium` | Price id → tier_key map (unmapped prices fail provisioning with 500 → Paddle retries) |
| `PADDLE_API_KEY` | optional | Server-side Paddle API key — fallback via `GET /customers/{id}` when `custom_data.email` is absent (the checkout now passes it, so this is rarely needed) |
| `OZ_SMTP_HOST` / `OZ_SMTP_PORT` / `OZ_SMTP_USER` / `OZ_SMTP_PASSWORD` / `OZ_SMTP_FROM` | relay creds | OTP + license-key receipt emails (`net/smtp`, port defaults 587) |
| `OZ_WEB_ALLOWED_ORIGINS` | `https://oz-pos.com,https://oz-pos.adikaradwiatmaja.workers.dev,http://localhost:4321` | Web API CORS allowlist |
| `OZ_WEB_SESSION_TTL` | `24h` | Web session lifetime (Go duration) |

**Shipped web endpoints** (all rate-limited per plan §11, CORS allow-listed):

| Endpoint | Purpose |
|----------|---------|
| `POST /api/v1/web/request-otp` | Email → 6-digit code (always 200, no enumeration) |
| `POST /api/v1/web/verify-otp` | Email + code → session token + summary |
| `GET /api/v1/web/me` | Bearer → tenant profile + license + subscription |
| `POST /api/v1/web/logout` | Invalidate session (idempotent) |

---

## 11. Security

### CORS (on the license server, not PocketBase)

The website only talks to `https://license.oz-pos.com`. The Go router sets
CORS for the allow-listed origins; **PocketBase stays internal** — its admin
UI and `/api/collections/*` surface are never reachable from the browser.

```json
// License server CORS allowlist — OZ_WEB_ALLOWED_ORIGINS env var
// (comma-separated). Default when unset:
{ "allowedOrigins": ["https://oz-pos.adikaradwiatmaja.workers.dev", "https://oz-pos.com", "http://localhost:4321"] }
```

### Content Security Policy

Cloudflare Pages `_headers` file to allow Paddle.js iframe:

```
/*
  Content-Security-Policy: default-src 'self'; script-src 'self' https://cdn.paddle.com; frame-src https://*.paddle.com; connect-src 'self' https://license.oz-pos.com https://*.paddle.com; style-src 'self' 'unsafe-inline'
```

### Token Storage (v1 decision)

| Approach | Storage | Security |
|----------|---------|----------|
| **sessionStorage** (v1, shipped) | `oz_session` in sessionStorage, server-side 24h TTL | Tab-scoped, cleared on tab close, survives reload |
| httpOnly cookie (follow-up) | License server cookie + CSRF header | Best, needs cross-site cookie + CSRF work |
| localStorage | Browser storage | Vulnerable to XSS — never |

### CSRF

Bearer tokens (v1) are inherently CSRF-safe. If/when cookie sessions land,
add `X-CSRF-Token` on state-changing requests.

### Rate Limiting (already in the license server)

`apps/license-server/ratelimit.go` already rate-limits the POS endpoints —
the web endpoints reuse it:

| Endpoint | Limit | Window |
|----------|-------|--------|
| `/api/v1/web/request-otp` | 3 per email | 15 min |
| `/api/v1/web/verify-otp` | 5 attempts | 15 min |
| `/api/v1/paddle/webhook` | HMAC-verified + event-id dedup | replay-safe |

### Account Page Auth Guard

```tsx
useEffect(() => {
  const token = sessionToken; // in-memory, from /verify-otp
  if (!token) {
    window.location.href = `/${locale}/login`;
    return;
  }
  fetch(`${LICENSE_API_URL}/api/v1/web/me`, {
    headers: { Authorization: `Bearer ${token}` }
  }).then(res => {
    if (!res.ok) window.location.href = `/${locale}/login`;
  });
}, []);
```

### Offline / License Server Unreachable

| Scenario | Behavior |
|----------|----------|
| Login page + server down | Show "Service temporarily unavailable" message |
| Account page + server down | Show cached license info if available, prompt retry |
| Paddle checkout + server down | Paddle still processes payment; webhook retries later (dedup makes replays safe) |

---

## 12. Webhook Responsibilities (Summary)

- **Verify** the Paddle signature — never trust unauthenticated calls.
- **Dedup** by `event_id` — Paddle retries on non-200; a replayed event must
  be a no-op, not a second license.
- **Map** `product_id → tier_key` via the §7 table.
- **Upsert tenant** by email (phone comes from a Paddle checkout custom
  field, or the schema relaxes `phone` to optional).
- **Mint** the license key string and insert a `license_keys` record.
- **Sign** the subscription payload with the existing RSA private key
  (`signed_payload` + `signature`).
- **Return 200** so Paddle stops retrying.

---

## 13. License / Updates Policy

The 1-Time/perpetual SKU is **not** part of the tier enum yet (see §6).
Until a `lifetime` tier ships, only time-based tiers exist:

| License Type | tier_key | Updates Included |
|-------------|----------|------------------|
| Free Trial | `trial` | All updates during the 90-day trial |
| Pro | `pro` | All updates while subscribed |
| Premium | `premium` | All updates while subscribed |
| Enterprise | `enterprise` | Contract terms |

When a 1-Time SKU is added: perpetual use of the version purchased; minor
updates (0.0.x) within the same major version are free; major versions
(1.0 → 2.0) require a new purchase.

---

## 14. Development Workflow

```bash
cd website/
npm install
npm run dev     # localhost:4321
npm run build   # outputs to dist/
```

License server side (new work) needs locally:

- the four `/api/v1/web/*` endpoints and `/api/v1/paddle/webhook` ✅ shipped
  (webhook: signature verification, event-id dedup, subscription events →
  tenant upsert + license-key mint + RSA-signed subscription, receipt email)
- `PADDLE_WEBHOOK_SECRET` + `PADDLE_PRICE_TIERS` (+ `PADDLE_API_KEY` for the
  email fallback) + SMTP env vars
- Paddle **sandbox** mode for checkout + webhook testing (event-id dedup,
  retry replay, signature verification all testable in sandbox)

---

## 15. Future Enhancements

| Enhancement | Priority |
|-------------|----------|
| `lifetime` (1-Time) tier — schema + client change | P1 |
| httpOnly cookie sessions + CSRF | P1 |
| Paddle customer portal (change plan / invoices self-serve) | P2 |
| OAuth login (Google, GitHub) | P2 |
| Blog (markdown-based, SEO) | P2 |
| Annual pricing toggle | P2 |
| Customer testimonials | P3 |
| Newsletter signup | P3 |
| Dark/Light mode toggle | P3 |

---

## 16. Estimated Timeline

Includes the new license-server work the previous plan silently assumed.

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| **1. Scaffold** | 1 day | Astro + Tailwind + i18n routing |
| **2. Pages** | 2-3 days | Homepage, pricing, features, download |
| **3. License-server web API** | 2 days | `/web/*` OTP endpoints, session, SMTP, CORS |
| **4. Paddle** | 2 days | Checkout integration + webhook (signature, dedup, key minting, RSA signing) |
| **5. Auth pages** | 1 day | Login/OTP + account page (reads `/me`) |
| **6. Polish** | 1-2 days | Responsive, SEO, analytics |
| **7. Deploy** | 0.5 day | Cloudflare Pages + custom domain + env vars |
| **Total** | **~10-12 days** | Full site live + purchase flow end-to-end |

---

## 17. Cost Estimate

| Item | Cost | Notes |
|------|------|-------|
| Cloudflare Pages | **Free** | 500 builds/mo, unlimited bandwidth |
| Cloudflare Workers | **Free** | Not needed for v1 (no Worker hops) |
| Paddle | **5% + $0.50/txn** | Handles payment processing |
| License server | existing | Northflank Hobby tier, already deployed |
| Domain | ~$10/year | Cloudflare Registrar |
| **Total** | **~$0-10/mo** | Before Paddle transaction fees |
