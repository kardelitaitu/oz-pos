# Admin Dashboard Review — Long-Term Sustainability Report

**Date**: 2026-08-29  
**Scope**: `website/public/admin/` (SPA) + `website/worker.ts` (auth gate) + `apps/license-server/admin_*.go` (backend)  
**Focus**: Maintainability, security, scalability, and resilience for the long run.

---

## 1. Architecture Overview

```
Browser → admin.ozpos.my.id
  ├── No cookie → Worker serves /admin/login (login.html + login.js + login.css)
  └── Cookie → Worker serves /admin/     (index.html + admin.js + admin.css + theme.js)
        ├── Dashboard tab   → /api/v1/admin/stats  (real) → MOCK fallback
        ├── Tenants tab     → /api/v1/admin/tenants (paginated)
        └── Health tab      → /api/v1/admin/health
```

**Stack**: Vanilla JS (no framework), inline SVG charts, CSS custom properties for theming, strict CSP via worker injection.

---

## 2. Critical Findings

### C1 — Stored XSS in Tenant Detail Modal

**File**: `admin.js:373-380`  
**Risk**: The `showTenantDetail` function builds the modal content via `kv.innerHTML = '<span>...' + (t.status || '—') + '...</span>'`. The `license.key` field (`lic.key`) is a user-controlled string — a tenant's license key is a store-level identifier. If a tenant (or an attacker who can register a tenant) sets a license key containing `<script>` or HTML event attributes, the admin viewing their details will execute that code.

**Severity**: **HIGH** — stored XSS in the admin panel with full access to the `/__oz/session` endpoint (JWT theft).

**Fix**: Use `textContent` / `el()` for all user-controlled data in the modal. The `kv` div currently uses `innerHTML` with concatenated strings. Replace with `el('span', ...)` + `textContent`.

### C2 — Stored XSS in Upgrade Prompt

**File**: `admin.js:402`  
**Risk**: `box.innerHTML += '<p>... Current tier: ' + (data.subscription.tierKey || 'none') + '...</p>'`. The `tierKey` from the API is interpolated into HTML. Currently constrained to a select enum (plus/pro/premium/enterprise), but a future API change or injection during transit could insert arbitrary HTML.

**Severity**: **HIGH** — same propagation path as C1.

**Fix**: Use `textContent` or `el()` for the tier key display.

### C3 — No Pagination in Frontend for Tenants List

**File**: `admin.js:340-359`  
**Risk**: The admin SPA fetches `GET /api/v1/admin/tenants` without any `?page=` or `?perPage=` params. The backend defaults to page 1, 25 per page, and returns the first 25 only. **The frontend never requests the next page** — so the admin sees only the first 25 tenants. With 1,000+ tenants, only 25 are visible, and there's no way to load more or search.

**Severity**: **HIGH** — functional gap that will block operators as the business grows.

**Fix**: Add pagination controls (page numbers, prev/next) to `renderTenants`, pass `?page=` and `?perPage=` to the API, and display total count from the response.

### C4 — MOCK Data Fallback Masks Real Failures

**File**: `admin.js:148-151`  
**Risk**: When the `/api/v1/admin/stats` API fails (network error, server down, 500), `renderDashboard` catches the exception and silently falls back to the `MOCK` object. The admin sees fabricated numbers (1,247 users, 386 subscribers, etc.) with a "MOCK DATA" badge. **An operator could act on fake data**, especially if they don't notice the badge.

**Severity**: **HIGH** — operational risk. Real data disappears behind plausible-looking fake data.

**Fix**: Show an error banner ("Stats API unavailable — showing sample data") in addition to the MOCK badge, or disable the mock fallback and show an empty state with a retry button. The current behavior is too quiet.

---

## 3. High Findings

### H1 — Monolithic 450-line Vanilla JS File

**File**: `admin.js` (448 lines)  
**Risk**: All logic, data, chart rendering, i18n strings, event handling, and routing live in a single file. No module separation, no test coverage, no build step. Adding a feature means growing the monolith. The `MOCK` data object (lines 5-47) alone is 43 lines of hardcoded test data shipped to production.

**Severity**: **HIGH** — maintainability debt. Future developers will hesitate to touch it.

### H2 — Zero Unit Tests

**Risk**: `admin.js`, `login.js`, `theme.js`, `admin.css`, `login.css` have zero tests. The only test coverage for the admin dashboard is the worker's 14 integration tests (`worker.test.ts`), which test the HTTP routing and auth gate — not the JS rendering or user-facing behavior.

**Severity**: **HIGH** — every change risks regression.

### H3 — Hardcoded English Strings (No i18n)

**File**: `admin.js` — every label, tooltip, error message, button text, and table header is hardcoded English. The rest of the OZ-POS ecosystem uses `@fluent/react` and `.ftl` files for i18n. The admin dashboard has no locale support.

**Severity**: **HIGH** — inconsistent with the rest of the platform. If the admin needs to be localized in the future, every string must be extracted.

### H4 — Session Cookie Shared Across All Subdomains

**File**: `worker.ts:47` — `Domain=.ozpos.my.id`  
**Risk**: The `oz_session` cookie is sent to ALL `*.ozpos.my.id` subdomains, including the marketing site. While the cookie is HttpOnly (not readable by JS on the marketing site), the marketing site's CSP is less strict (`script-src 'unsafe-inline'`). If the marketing site is compromised, the attacker can make authenticated requests to the license API via the shared cookie.

**Severity**: **HIGH** — cross-subdomain session exposure.

---

## 4. Medium Findings

### M1 — No Loading / Error States for Charts

**File**: `admin.js:143-269`  
**Risk**: The `renderDashboard` function shows a skeleton while loading, but if the API partially fails (e.g., stats succeeds but chart data is missing), the charts render with `NaN` values or empty SVG. The `svgChart` function uses `Math.max(...data.map(...))` — if `data` is empty, `Math.max()` returns `-Infinity`, producing broken SVG.

### M2 — Duplicate Icon Code

**File**: `admin.js:185,188` — `devices` and `devices2` hold identical SVG strings. The `devices2` key is never referenced (`devices` is used at line 197). Dead code shipped to production.

### M3 — `?token=` Fallback Still Active

**File**: `worker.ts:154-172` — the deprecated `?token=` flow is still handled. The exchange code flow (`?code=`) is the intended replacement. The fallback should be removed after verifying the rollout.

### M4 — Client-Side FX Rate Fetch (No Server Cache)

**File**: `admin.js:54-60`  
**Risk**: The FX rate is fetched from `open.er-api.com` directly from the browser. If the external API is throttled or down, the admin dashboard shows a stale rate with a "stale" chip. The server-side `/api/v1/admin/stats` already returns `fxRate` with a 1-hour cache, but when the real API fails (falling to MOCK), the client makes a duplicate fetch.

### M5 — No Search on Tenants List

**File**: `admin.js:340-359`  
**Risk**: With 25 tenants per page and no search/filter, finding a specific tenant requires manually paging through hundreds of records. The backend has no search endpoint.

### M6 — Missing `Cache-Control` Headers on SPA HTML

**File**: `worker.ts:227` — the SPA response (`withStrictCSP(spaResp)`) copies the ASSETS response headers, which may include `Cache-Control: public, max-age=0, must-revalidate`. The `withStrictCSP` function doesn't set an explicit `no-store` for the HTML page, so the edge CDN may serve stale SPA content on hash-free static file requests.

---

## 5. Low / Info Findings

### L1 — Inline Styles via JS (CSP Dependency on `unsafe-inline` for style-src)

The admin dashboard heavily uses `style.cssText`, `style="..."` in SVG strings, and `innerHTML` with inline styles. This forces `style-src 'self' 'unsafe-inline'` in the CSP. While acceptable for an admin tool, it prevents achieving a fully strict CSP.

### L2 — No `alt` / `aria-label` on SVG Icons

The KPI icons and chart SVGs have no `aria-hidden="true"` (or it's on the SVG itself but not on the container). Minor for screen readers.

### L3 — `theme.js` Loaded Synchronously in `<head>`

The theme script blocks rendering. For a 1KB file it's negligible, but it's a pattern to note.

### L4 — Login Page Has No Theme Switcher

The admin login page (`login.html`) is always dark. The dashboard has a theme toggle, but the login page doesn't — creates a flash of dark on redirect.

---

## 6. Recommendations (Priority Order)

| # | Priority | Finding | Action |
|---|----------|---------|--------|
| 1 | **CRITICAL** | Stored XSS in modal/upgrade prompt (C1, C2) | Replace `innerHTML` with `textContent` / `el()` for all API-sourced strings |
| 2 | **HIGH** | MOCK fallback masks failures (C4) | Show error banner when API fails; keep MOCK only as last-resort skeleton |
| 3 | **HIGH** | Tenants list has no pagination (C3) | Add page controls + pass `?page=` / `?perPage=` to the API |
| 4 | **HIGH** | Monolithic admin.js (H1) | Split into testable modules (stats.js, tenants.js, charts.js) or move to a build step |
| 5 | **HIGH** | Zero tests (H2) | Add unit tests for chart rendering, helpers, and API mock fallback |
| 6 | **HIGH** | No i18n (H3) | Extract strings to an i18n structure; at minimum, add English `.ftl` keys for future localization |
| 7 | **HIGH** | Shared session cookie (H4) | Restrict `Domain` to individual subdomains or use a dedicated auth domain |
| 8 | **MEDIUM** | No loading/error states for charts (M1) | Guard `svgChart` against empty/NaN data; add per-chart error states |
| 9 | **MEDIUM** | Remove `?token=` fallback (M3) | Delete the deprecated path once exchange-code rollout is confirmed stable |
| 10 | **MEDIUM** | Duplicate `devices2` icon (M2) | Remove dead code |
| 11 | **MEDIUM** | No search on tenants (M5) | Add a search endpoint on the backend + search input in the frontend |
| 12 | **MEDIUM** | SPA caching headers (M6) | Set `Cache-Control: no-store` on the SPA HTML response in `withStrictCSP` |
| 13 | **LOW** | Login page always dark (L4) | Add theme.js to the login page or auto-detect OS preference |

---

## 7. Phase Plan

### Phase 1 (immediate — hours)
1. Fix stored XSS (C1, C2) — replace `innerHTML` with `textContent` in `showTenantDetail` + `upgradePrompt`
2. Fix MOCK fallback (C4) — add an error banner; don't silently fall back
3. Remove dead code (M2)
4. Set `Cache-Control: no-store` on SPA HTML (M6)

### Phase 2 (short-term — days)
5. Add pagination controls to the tenants list (C3)
6. Guard `svgChart` against empty data (M1)
7. Add loading/error states for all API calls
8. Add search to tenants list (M5)

### Phase 3 (medium-term — weeks)
9. Split `admin.js` into modules (H1) — at minimum, separate chart rendering, tenant management, and API helper into distinct files
10. Add unit tests for chart rendering, helpers, and error states (H2)
11. Add i18n infrastructure (H3) — even if only English, use a key-value pattern
12. Remove `?token=` fallback after confirming `?code=` works (M3)

### Phase 4 (long-term)
13. Restrict session cookie domain (H4)
14. Move admin dashboard to a build step (Vite/Rollup) for TypeScript, module resolution, and tree-shaking
15. Add theme.js to the login page (L4)
16. Accessibility pass (ARIA, keyboard navigation, focus management)