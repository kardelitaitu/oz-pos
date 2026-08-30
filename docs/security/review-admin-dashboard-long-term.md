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

**Status (2026-08-30 re-review): OPEN — was overclaimed as resolved.** `worker.ts` still ships `style-src 'self' 'unsafe-inline'` and `admin.js` still uses ~12 `style="..."` + 7 `cssText`. Note the real CSP dependency is narrower than the finding implies: `style-src` gates only `style=""` *attributes* and `<style>` elements — CSSOM writes (`el.style.cssText`, `el.style.x = …`) are **not** gated. So the fix is to move the ~14 `style="..."` occurrences (mostly SVG chart markup) to presentation attributes (`fill=`/`stroke=`) or classes, then tighten CSP to `style-src 'self'`. The `cssText`/`.style.x` uses can stay.

### L2 — No `alt` / `aria-label` on SVG Icons

The KPI icons and chart SVGs have no `aria-hidden="true"` (or it's on the SVG itself but not on the container). Minor for screen readers.

**Status (2026-08-30 re-review): ✅ Resolved (#75).** KPI icons carry `aria-label`s; decorative SVGs are `aria-hidden`.

### L3 — `theme.js` Loaded Synchronously in `<head>`

The theme script blocks rendering. For a 1KB file it's negligible, but it's a pattern to note.

**Status (2026-08-30 re-review): ✅ Won't fix — by design.** Synchronous `<head>` execution is the *correct* anti-FOUC pattern for a theme script (1.3 KB); `defer`/`async` would reintroduce the theme flash it prevents.

### L4 — Login Page Has No Theme Switcher

The admin login page (`login.html`) is always dark. The dashboard has a theme toggle, but the login page doesn't — creates a flash of dark on redirect.

**Status (2026-08-30 re-review): ✅ Resolved (#75).** Sun/moon toggle + `theme.js` on both admin and dashboard login pages.

---

## 6. Recommendations (Priority Order)

| # | Priority | Finding | Action | Status |
|---|----------|---------|--------|--------|
| 1 | **HIGH** | Unsafe `innerHTML` patterns (C1, C2) | Replace with `textContent` / `el()` for all API-sourced strings (defense-in-depth; not exploitable today due to server-generated hex keys + enum-constrained fields) | ✅ Resolved — `showTenantDetail` / `upgradePrompt` use `el()` + `textContent` (Phase 1) |
| 2 | **HIGH** | MOCK fallback masks failures (C4) | Show error banner when API fails; keep MOCK only as last-resort skeleton | ✅ Resolved — MOCK object removed; API errors render a retry/error state (Phase 1) |
| 3 | **HIGH** | Tenants list has no pagination (C3) | Add page controls + pass `?page=` / `?perPage=` to the API | ✅ Resolved — pagination controls + `?page=`/`?perPage=`/`?search=` (Phase 2) |
| 4 | **HIGH** | Monolithic admin.js (H1) | Split into testable modules (stats.js, tenants.js, charts.js) or move to a build step | ✅ Resolved — pure helpers extracted into `admin-utils.js` (charts, formatting, cards, API auth, i18n) with unit tests |
| 5 | **HIGH** | Zero tests (H2) | Add unit tests for chart rendering, helpers, and API mock fallback | ✅ Resolved — 76 unit tests in `src/__tests__/admin-utils.test.ts` (+14 worker auth-gate tests); both suites now execute in CI via the `website-tests` gate. The 2026-08-30 bug hunt added 52 of those tests and fixed 19 real bugs (see §8.1) |
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

_Status as of the re-verification pass (PRs #64–#89): items 1–13 and 15 are done; 14 remains open. Item 16 has two landed slices (#88: focus-visible, skip links, modal dialog + ESC; #89: upgrade-prompt dialog + login skip links) — the full keyboard/focus audit remains open._

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
16. ◐ Accessibility pass (ARIA, keyboard navigation, focus management) — #75 (KPI aria-labels), #88 (focus-visible, skip links, modal + ESC), #89 (upgrade-prompt dialog, login skip links) landed; full keyboard/focus audit remains open

---

## 8. Re-verification addendum (2026-08-30)

Independent re-review of every claim above against the code on `main`,
merge history (`git log -S` + merge-ancestry), and a local test run:

- **All 13 recommendation-table resolutions verified in code** — cookie
  scoping (`Domain=${hostname}`), `?token=` removal, chart guards,
  backend `?search=` (+ `regexp.QuoteMeta`), FX server cache (1 h TTL),
  pagination, MOCK removal, safe kv-grid, `STRINGS`/`t()` on all four
  pages, theme toggles on both logins.
- **True PR attribution** (an earlier summary table misfiled several):
  M1/M2/M4/M5/M6 and C1–C4 all landed in **#60** (Phase 1+2 hardening);
  H4 in **#68**; M3 in **#74**; H1/H2 in #69/#70/#72; H3 in #71/#73/#76/#77;
  L2/L4 in **#75**. #67 (license-suite speed) and #78 (timezone-fragile
  fixtures) resolve none of the M findings.
- **H2 enforcement gap closed**: the 24 admin-utils tests + 14 worker
  tests existed but **no CI workflow executed them** (`astro check` only
  type-checks). Now gated end-to-end: `website-tests` registered in
  `scripts/gates.json`, `npm test` step in `website.yml` (fail-fast,
  before the portal build), `website test` step in `scripts/check.sh`.
- **L1 corrected to OPEN**, **L3 marked won't-fix/by-design** — see §5.
- Test count corrected: **24**, not "25+".

### 8.1 Bug hunt (2026-08-30, TDD) — 21 bugs found & fixed

A focused hunt over `admin.js`/`admin-utils.js` against the Go server's
actual JSON shapes found six real bugs — none caught by the pre-existing
24 tests, which covered only escapeHtml/fmt/statusPill:

| # | Sev | Bug | Fix |
|---|-----|-----|-----|
| B1 | P0 | `tenants.forEach(t => …)` — callback param shadowed the i18n `t()`; `t('tenant.details')` threw on the first row → Tenants tab = header + empty tbody | row builder extracted to `tenantRow(tenant, onDetails)` |
| B2 | P0 | `showTenantDetail`: `const t = data.tenant` shadowed `t()` identically → detail modal **always** showed "Failed to load" | kv mapping extracted to `tenantDetailRows(data)` |
| B3 | P1 | churn bars read `d.count`, but `admin_stats.go` sends `monthBucket{Month, Churn}` with `count` at Go zero → chart permanently flat/NaN | `svgBarChart(id, data, {valueKey})`; churn passes `'churn'` |
| B4 | P1 | `svgDonut` single 100% slice → one arc with start==end → draws nothing (SVG spec) → empty ring beside a "100%" legend | full circle split into two 180° arcs |
| B5 | P2 | `svgChart` did `d.month.slice(5)` unguarded — M1 protected values but not labels; one month-less row killed the render | label guarded + escaped |
| B6 | P2 | `renderDashboard` dereferenced `m.revenueTrend.forEach` / `m.kpis.mrrUsd` **before** the chart guards ran → partial payload = blank dashboard | `normalizeStats(m)` guarantees shapes |

Commits `b238540b`, `ac7ed317`, `27af049f`, `c18a3e00`, `de489a16`
(prefix `(bugs)website:admin`). Suite 24→40 tests; full website suite
566/566. Known residuals logged in `JOURNAL.md` (lockout-countdown timer
race, `escHandler` listener leak, per-request `/__oz/session` fetch).

**Round 2** (same day) hunted those residuals and the login flow — six
more bugs, all fixed with tests (suite 40→57, full website 623/623):

| # | Sev | Bug | Fix |
|---|-----|-----|-----|
| B7 | P1 | `showLockoutCountdown` spawned a new `setInterval` per 429 without clearing the previous — the shorter stale timer re-enabled the button **early** during a longer lockout; the survivor zombie-rewrote the restored label | `startLockoutCountdown` keeps one tracked timer per button |
| B10 | P2 | `fetchFxRate` awaited an un-timed fetch — firewalled FX API hung the whole dashboard render | `fetchFxRate(fetchImpl, timeoutMs)` with `AbortSignal.timeout` |
| B11 | P2 | both modal builders leaked the document ESC handler on every non-ESC close (button/backdrop) — stale handlers kept reacting to later ESCs | `mountModal(root, box)` — one idempotent `close()` owns all paths |
| B12 | P1 | `api()` awaited two un-timed fetches per call (session + license API) — a hung connection froze every tab forever, no error state | `fetchWithTimeout` on both calls (default 15s) |
| B13 | P2 | `exchangeForCode` navigated to `/?code=undefined` on a code-less 200 — silent login loop | `exchangeUrlFrom(body)` validates + surfaces `login.exchangeFailed` |
| B14 | P3 | `setAuthMode` overwrote the lockout countdown label on tab switch — disabled button labelled "Send Verification Code" | label writes skipped while `isLockoutActive(btn)` |

Round-2 commits: `5dfe72d9`, `1670a282`, `96f6d3f9`, `2b19570c`,
`cafcca11`. Remaining residuals (stale-response race in `renderTenants`,
raw-enum `statusPill` text, no URL state for tab/search/page) logged in
`JOURNAL.md`.

**Round 3** (same day) closed those residuals and hunted the modal
action paths — four more bugs, one candidate dropped as unreachable
(suite 57→70, full website 636/636):

| # | Sev | Bug | Fix |
|---|-----|-----|-----|
| B15 | P2 | `renderTenants` let a slow page-2 response overwrite page 3 (last-arrival-wins) — rows and pagination header disagreed | `createSeqGuard()`: superseded responses discarded on success + error paths |
| B16 | P3 | server status enum leaked raw into pills and the detail modal (`grace_period`) | `statusLabel()` maps the enum via STRINGS; unknown → raw fallback |
| B18 | P2 | OTP resend cooldown went invisible after a tab switch (timer ran on, element hidden) — user clicked into a 429 blind | `startCountdown`/`stopCountdown`/`countdownActive` on the node; `setAuthMode` re-shows while active |
| B19 | P1 | tenant modal actions had no double-click guard — Renew POSTed +365 days per click, double-click = +730 | `busyWrap` single-flight wrapper on all four buttons |

B17 (detail-modal fetch race) was investigated and **dropped**: the
loading overlay blocks all interaction, so two detail fetches cannot
overlap. Round-3 commits: `bb73e268`, `d3017b63`, `fc184c30`. The B19
helper was independently written by the concurrent agent session and
adopted (attribution in the commit message).

**Round 4** (same day) ran the adversarial pass over the hunt's own
fixes — three more, including one self-inflicted regression
(suite 70→76, full website 642/642):

| # | Sev | Bug | Fix |
|---|-----|-----|-----|
| B20 | P1 | B10/B12 called `AbortSignal.timeout()` unconditionally — Chrome/WebView 103+/Safari 16+ only; on older WebViews EVERY `api()` call threw TypeError, making the dashboard permanently broken on browsers that worked before the timeout fixes | `timeoutSignal()` availability guard; un-timed fetch beats broken fetch |
| B21 | P2 | tab click during an in-flight login flipped `currentMode` — the response handler wrote the wrong mode's label and could start the OTP cooldown on the password tab | `setAuthMode` extracted with `isSubmitting()` veto; refused flips leave DOM untouched |
| B22 | P2 | `login-btn` is `type=submit`: Enter in any input triggers implicit form submission, which **ignores the disabled state** — the 429 lockout countdown was bypassable by pressing Enter | `handleLogin` vetoes while `isLockoutActive(btn)`; restore label mirrors the mode's real state |

Round-4 commits: `70d5d869`, `aa808a93`, `d183645e`. Lesson recorded:
timeout/abort primitives are themselves compatibility surfaces — a
hardening fix can regress more than the bug it cures.

**Round 5** (same day) completed full end-to-end coverage of the admin
area — every SPA file plus the worker's admin auth gate — and found two
bugs in the one-time-code exchange (worker suite 17→19, full website
644/644):

| # | Sev | Bug | Fix |
|---|-----|-----|-----|
| B24 | P1 | exchange-code failure on `admin.ozpos.my.id` 302'd to the MARKETING host's `/admin/login` — which has no `/api/v1/` proxy (gated to `DASHBOARD_HOSTS`), so login.js's relative POSTs 404: a dead login form, user stranded | redirect to the clean URL on the same host; the no-session gate serves login locally and the destination survives re-login |
| B24b | P1 | the exchange SUCCESS 302 used `url.pathname` raw — `/?code=x` at path `//evil.com` produced `Location: //evil.com/`: a protocol-relative **open redirect** on the admin host | path forced single-slash (`/^[/\\]+/`) before reuse |

Round-5 commit: `d3085d8e`. The pre-existing worker test asserted the
marketing bounce — it pinned the bug and was corrected with a note.
B23 (innerHTML+= breaking SVG viewBox) was investigated and dropped:
the HTML parser's SVG attribute adjustment fixes camelCase attrs.
**Coverage is now complete**: admin.js, login.js, admin-utils.js,
theme.js, index.html, login.html, and the worker admin gate have each
been read end-to-end during the hunt.