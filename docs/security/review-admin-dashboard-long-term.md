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

## 2. Top Findings (highest-impact issues)

### C1 — Unsafe `innerHTML` in Tenant Detail Modal (defense-in-depth, currently LOW exploitability)

**File**: `admin.js:373-380`  
**Risk**: `showTenantDetail` builds the modal via `kv.innerHTML = '<span>...' + (t.status || '—') + '...</span>'`. **Assessment after verifying the data model**: license keys are server-generated hex (`hex.EncodeToString` in `api_key.go`/`helpers.go`), and `tierKey`/`status`/`provider` are enum/select fields constrained at the PocketBase schema level — so none of the fields interpolated into this HTML are currently free-text user input. The tenant **email** (the one genuinely user-controlled field) is rendered via `el('h3')` → `textContent`, which is safe.

**Severity**: **MEDIUM** (was initially assessed HIGH) — not directly exploitable today, but the `innerHTML`-with-string-concatenation pattern is fragile. Any future field that becomes free-text (e.g., a tenant `notes` column, an editable `display_name`) instantly becomes stored XSS in the admin panel with JWT-theft capability via `/__oz/session`.

**Fix**: Replace `innerHTML` concatenation with `el('span', ...)` + `textContent` in `showTenantDetail`. Cheap, removes the whole class of future bugs.

### C2 — Unsafe `innerHTML` in Upgrade Prompt (defense-in-depth)

**File**: `admin.js:402`  
**Risk**: `box.innerHTML += '<p>... Current tier: ' + (data.subscription.tierKey || 'none') + '...</p>'`. Same pattern as C1 — `tierKey` is a schema-constrained enum today, but the pattern is unsafe for future fields.

**Severity**: **MEDIUM** (was initially assessed HIGH) — same rationale as C1.

**Fix**: Use `el('p', ...)` + `textContent` for the tier display.

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

### M6 — SPA HTML is Edge-Cached (acceptable, but worth documenting)

**File**: `worker.ts:248` — the SPA response (`withStrictCSP(spaResp)`) copies the ASSETS headers including `Cache-Control: public, max-age=0, must-revalidate`. Live check confirms `CF-Cache-Status: HIT`. **Assessment**: this is acceptable — `max-age=0` + `must-revalidate` means the edge/browser revalidates every request, so stale SPA content isn't served long-term. The one subtlety is that a deploy changing the SPA's hash-less file paths can serve the *previous* version for one revalidation cycle. Not a bug, but the admin dashboard's HTML/JS should be considered immutable-per-deploy; a future improvement is fingerprinting asset filenames (e.g. `admin-<hash>.js`) to eliminate even that window.

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

| # | Priority | Finding | Action | Status |
|---|----------|---------|--------|--------|
| 1 | **HIGH** | Unsafe `innerHTML` patterns (C1, C2) | Replace with `textContent` / `el()` for all API-sourced strings (defense-in-depth; not exploitable today due to server-generated hex keys + enum-constrained fields) | ✅ Resolved — `showTenantDetail` / `upgradePrompt` use `el()` + `textContent` (Phase 1) |
| 2 | **HIGH** | MOCK fallback masks failures (C4) | Show error banner when API fails; keep MOCK only as last-resort skeleton | ✅ Resolved — MOCK object removed; API errors render a retry/error state (Phase 1) |
| 3 | **HIGH** | Tenants list has no pagination (C3) | Add page controls + pass `?page=` / `?perPage=` to the API | ✅ Resolved — pagination controls + `?page=`/`?perPage=`/`?search=` (Phase 2) |
| 4 | **HIGH** | Monolithic admin.js (H1) | Split into testable modules (stats.js, tenants.js, charts.js) or move to a build step | ✅ Resolved — pure helpers extracted into `admin-utils.js` (charts, formatting, cards, API auth, i18n) with unit tests |
| 5 | **HIGH** | Zero tests (H2) | Add unit tests for chart rendering, helpers, and API mock fallback | ✅ Resolved — 25+ unit tests in `src/__tests__/admin-utils.test.ts` |
| 6 | **HIGH** | No i18n (H3) | Extract strings to an i18n structure; at minimum, add English `.ftl` keys for future localization | ✅ Resolved — `STRINGS` key-value table + `t()` helper; all admin/dashboard/login strings extracted |
| 7 | **HIGH** | Shared session cookie (H4) | Restrict `Domain` to individual subdomains or use a dedicated auth domain | ✅ Resolved — cookie scoped to `admin.ozpos.my.id` / `dashboard.ozpos.my.id` (not the parent domain) |
| 8 | **MEDIUM** | No loading/error states for charts (M1) | Guard `svgChart` against empty/NaN data; add per-chart error states | ✅ Resolved — `svgChart` / `svgDonut` guard empty/NaN/zero data |
| 9 | **MEDIUM** | Remove `?token=` fallback (M3) | Delete the deprecated path once exchange-code rollout is confirmed stable | ✅ Resolved — `?token=` fallback removed; exchange-code (`?code=`) is the only handoff |
| 10 | **MEDIUM** | Duplicate `devices2` icon (M2) | Remove dead code | ✅ Resolved — dead code removed |
| 11 | **MEDIUM** | No search on tenants (M5) | Add a search endpoint on the backend + search input in the frontend | ✅ Resolved — backend `?search=` + frontend search input (Phase 2) |
| 12 | **INFO** | SPA HTML edge-cached (M6) | Acceptable (`max-age=0, must-revalidate`); optional future: fingerprint asset filenames | ✅ Resolved — `Cache-Control: no-store` on SPA HTML; `?v=` cache-busting now works via the worker query-string fix |
| 13 | **LOW** | Login page always dark (L4) | Add theme.js to the login page or auto-detect OS preference | ✅ Resolved — sun/moon theme toggle on both admin and dashboard login pages |

---

## 7. Phase Plan

_Status as of the hardening pass (PRs #64–#86): items 1–13 and 15 are done; 14 and 16 remain open._

### Phase 1 (immediate — hours) ✅
1. ✅ Fix stored XSS (C1, C2) — replace `innerHTML` with `textContent` in `showTenantDetail` + `upgradePrompt`
2. ✅ Fix MOCK fallback (C4) — add an error banner; don't silently fall back
3. ✅ Remove dead code (M2)
4. ✅ Set `Cache-Control: no-store` on SPA HTML (M6)

### Phase 2 (short-term — days) ✅
5. ✅ Add pagination controls to the tenants list (C3)
6. ✅ Guard `svgChart` against empty data (M1)
7. ✅ Add loading/error states for all API calls
8. ✅ Add search to tenants list (M5)

### Phase 3 (medium-term — weeks) ✅
9. ✅ Split `admin.js` into modules (H1) — pure helpers extracted into `admin-utils.js` (charts, formatting, cards, API auth, i18n)
10. ✅ Add unit tests for chart rendering, helpers, and error states (H2)
11. ✅ Add i18n infrastructure (H3) — `STRINGS` key-value pattern + `t()` helper
12. ✅ Remove `?token=` fallback after confirming `?code=` works (M3)

### Phase 4 (long-term) ⏳
13. ✅ Restrict session cookie domain (H4) — cookie scoped to each dashboard subdomain
14. ☐ Move admin dashboard to a build step (Vite/Rollup) for TypeScript, module resolution, and tree-shaking — **open**: not required while the dashboard stays a plain-vanilla static SPA
15. ✅ Add theme.js to the login page (L4) — sun/moon toggle on both admin and dashboard login pages
16. ☐ Accessibility pass (ARIA, keyboard navigation, focus management) — **open**: KPI icons have aria-labels and SVGs use aria-hidden, but a full keyboard/focus audit remains