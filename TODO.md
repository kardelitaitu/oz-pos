# Top 3 High-Impact Improvements — July 29, 2026 Audit

> From the July 29 full-codebase audit: 3324/3324 tests, zero type errors, zero clippy
> warnings, ~77 hardcoded aria-labels across un-audited features, ~90+ attribute-only
> FTL messages that may silently return `undefined`.

---

## 1. SettingsPage.tsx (1081 lines) — Audit & Harden

**Why:** Largest UI file in the entire codebase. Completely un-audited for dialog semantics,
hardcoded strings, CSS tokens, focus trapping, and stale closures.

- [ ] **Phase A — Read & catalog**
  - [ ] Read full file; list all `<Localized>` blocks, `l10n.getString()` calls, `useCallback`/`useEffect` hooks, inline styles, and hardcoded `aria-label`/`placeholder`
  - [ ] Read `SettingsPage.css`; count CSS-token vs hardcoded hex usage
- [ ] **Phase B — FTL sweep**
  - [ ] Verify every `<Localized id="...">` key exists in both `settings.ftl` + `settings.id.ftl` (run bundle-parity)
  - [ ] Verify every `l10n.getString(...)` has either a fallback `||` or its key is a simple-value (not attribute-only) message
  - [ ] Add any missing FTL keys
- [ ] **Phase C — Dialog semantics & a11y**
  - [ ] Convert any `<button>` overlay panels to `<div role="dialog" aria-modal="true">` + `useFocusTrap`
  - [ ] Add Escape-key-to-close on all modals and overlays
  - [ ] Replace all hardcoded `aria-label="..."` with `l10n.getString()` || fallback
- [ ] **Phase D — CSS tokenization**
  - [ ] Replace any hardcoded hex colors with existing `var(--color-*)` / `var(--bg-*)` tokens
  - [ ] Ensure 44px minimum touch targets on interactive elements
- [ ] **Phase E — Logic audit**
  - [ ] Check all `useCallback`/`useEffect` dep arrays against eslint `react-hooks/exhaustive-deps`
  - [ ] Look for missing null guards, setState-after-unmount, and missing error boundaries
  - [ ] Write regression tests for any bugs found
- [ ] **Phase F — Validate**
  - [ ] `cd ui && npm run typecheck`
  - [ ] `cd ui && npx vitest run` (full suite)
  - [ ] Bundle-parity check on settings FTL
  - [ ] Code review + commit

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
