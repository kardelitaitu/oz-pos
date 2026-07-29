# Top 3 High-Impact Improvements — July 29, 2026 Audit

> From the July 29 full-codebase audit: 3324/3324 tests, zero type errors, zero clippy
> warnings, ~77 hardcoded aria-labels across un-audited features, ~90+ attribute-only
> FTL messages that may silently return `undefined`.

---

## ✅ ~~1.~~ SettingsPage.tsx (1081 lines) — COMPLETE (commit `533247bc`)

> **Done:** Full audit of the largest UI file. Surprisingly clean — 244 CSS tokens,
> zero hardcoded hex, correct hook deps, proper cleanup. Only 2 hardcoded strings
> found: `placeholder="Search"` + Suspense fallback `Loading...`. Both fixed with
> new FTL keys. 49/49 tests pass, typecheck clean.

---

## ✅ ~~2.~~ RestaurantMenu.tsx (795 lines) — COMPLETE (commit `b3307810`)

> **Done:** Full audit — 13 FTL keys added (11 missing + 2 for hardcoded labels),
> 2 hardcoded aria-labels fixed, 1 fallback added. CSS all tokens, hooks all clean,
> dialog semantics correct (role=menu/tablist with Escape handling). 11/11 tests pass.

---

## ✅ ~~3.~~ Attribute-Only FTL Sweep — COMPLETE (commit `104c4891`)

> **Done:** Cross-referenced 268 attribute-only messages against 1212 `l10n.getString()`
> calls. Found 75 keys silently returning `undefined` across 25 files.
>
> **Fix:** 72 safe keys converted to `key = value` via `scripts/convert-safe-attr-ftl.py`
> (125 conversions, 16 bundles). 3 keys also used via `<Localized>` received `||`
> fallbacks in code. 3324/3324 tests pass, bundle parity verified, typecheck clean.
