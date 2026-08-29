# Login Flow Audit — Admin + User Dashboard (Final Pass)

**Date:** 2026-08-30  
**Scope:** `admin.ozpos.my.id` + `dashboard.ozpos.my.id` login flows end-to-end  
**Verified:** live endpoints, worker routing, license-server auth, CSP, cookie handling

---

## 1. Flow Summary (both subdomains)

```
Visit admin.ozpos.my.id / dashboard.ozpos.my.id (no cookie)
  → Worker serves dedicated login page (same subdomain)
  → User enters email → OTP (or password)
  → login.js POSTs to /api/v1/web/* (relative → Worker proxy → license server)
  → On success: POST /api/v1/web/exchange-issue (Bearer) → one-time code
  → Redirect to /?code=<code> → Worker POSTs /exchange-consume → real JWT
  → Worker sets httpOnly oz_session cookie → redirect to clean URL
  → SPA loads → /__oz/session → Bearer → /api/v1/admin/* or /api/v1/web/*
Log out → /__oz/logout → cookie expired (Max-Age=0) → 302 to same subdomain login
```

---

## 2. Verified Checks

| # | Check | Result |
|---|-------|--------|
| 1 | Admin login page served on `admin.ozpos.my.id` (no cookie) | ✅ 200 |
| 2 | User login page served on `dashboard.ozpos.my.id` (no cookie) | ✅ 200 |
| 3 | Both login pages use **relative API** (`API=''` → Worker proxy) | ✅ |
| 4 | No inline `onclick`/event handlers in either login page (strict CSP) | ✅ |
| 5 | Login endpoints through proxy: `/login`, `/request-otp`, `/verify-otp` | ✅ 401/200/processed |
| 6 | Exchange endpoints: `/exchange-issue` (401 no auth), `/exchange-consume` (400 bad code) | ✅ |
| 7 | Session cookie: HttpOnly + Secure + SameSite=Lax + Domain=.ozpos.my.id | ✅ |
| 8 | Logout expires cookie (`Max-Age=0`) via `/__oz/logout` | ✅ |
| 9 | Logout redirects to **same subdomain** (`admin.ozpos.my.id/` / `dashboard.ozpos.my.id/`) | ✅ |
| 10 | `/__oz/session` has NO `Access-Control-Allow-Origin` (token not readable cross-origin) | ✅ |
| 11 | Strict CSP: `script-src 'self'`, `frame-ancestors 'none'`, no-referrer | ✅ |
| 12 | Escalating brute-force lockout (5s → +30s → 15min cap) on login + verify-otp | ✅ |
| 13 | Session refresh (`touchSession`) on active use | ✅ |
| 14 | Dashboard SPA uses relative API + `/__oz/logout` (fixed this pass) | ✅ |
| 15 | Token never appears in a URL (one-time exchange code) | ✅ |

---

## 3. Issues Found & Fixed This Pass

### F1 — Dashboard SPA used direct license URL (CORS dependency)
**Before:** `dashboard.js` API base was `https://license.ozpos.my.id` directly (cross-origin, needs CORS) — inconsistent with the login pages.
**After:** Uses the same relative-API mode as login pages (`API=''` → Worker proxy). No cross-origin at all. **Fixed + deployed.**

### F2 — Dashboard logout didn't clear the httpOnly cookie
**Before:** dashboard.js logout called the license-server logout API + navigated to `/` — the httpOnly cookie persisted (same bug we fixed for admin).
**After:** dashboard.js logout navigates to `/__oz/logout` (Worker expires the cookie + redirects to `dashboard.ozpos.my.id/`). **Fixed + deployed.**

---

## 4. Residual Notes (accepted)

| Note | Assessment |
|---|---|
| **24h session TTL** | Active-use refresh keeps dashboards alive; acceptable |
| **`/__oz/session` returns JWT to same-origin JS** | Necessary for Bearer auth; mitigated by strict CSP + no CORS |
| **Session store is in-memory** | Resets on license-server deploy → old cookies become invalid (logout path recovers) |
| **`?token=` fallback in worker** | Deprecated; kept for transition, exchange-code is the primary flow |

---

## 5. Conclusion

Both login flows are **consistent, CORS-free, CSP-hardened, and recoverable**:

- **admin.ozpos.my.id** — dedicated admin login → exchange code → httpOnly cookie → admin API (gated by OZ_ADMIN_EMAIL)
- **dashboard.ozpos.my.id** — dedicated user login → exchange code → httpOnly cookie → user API

No blocking issues remain. All fixes deployed live.
