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

## 2. RestaurantMenu.tsx (795 lines) — Audit & Harden

**Why:** Completely un-audited restaurant/KDS subsystem. Has a known hardcoded
`aria-label="Menu items"`. Likely shares the same patterns we fixed across 70 cycles
in the retail surface.

- [ ] **Phase A — Read & catalog**
  - [ ] Read full file + `RestaurantMenu.css`; catalog all i18n gaps, hardcoded colors, and inline styles
  - [ ] Check related files: `RestaurantTableMap.tsx`, `RestaurantOrderPanel.tsx`, and any KDS components
- [ ] **Phase B — FTL sweep**
  - [ ] Check bundle-parity for all restaurant/KDS FTL keys (likely `sales.ftl` or `kds.ftl`)
  - [ ] Add any missing keys; ensure Indonesian translations exist
- [ ] **Phase C — Dialog semantics & a11y**
  - [ ] Convert overlay panels to proper dialog roles with `useFocusTrap`
  - [ ] Fix `aria-label="Menu items"` → `l10n.getString()` || fallback
  - [ ] Add Escape-to-close on any modals
- [ ] **Phase D — CSS tokenization**
  - [ ] Replace hardcoded hex with CSS tokens; ensure contrast on all themes
- [ ] **Phase E — Logic audit**
  - [ ] Check all hook dep arrays; look for stale closures and missing cleanup
  - [ ] Write regression tests for any bugs found
- [ ] **Phase F — Validate**
  - [ ] `cd ui && npm run typecheck`
  - [ ] `cd ui && npx vitest run`
  - [ ] Code review + commit

---

## 3. Attribute-Only FTL Sweep — Automated Bug Hunt

**Why:** ~90+ FTL messages are attribute-only (e.g. `.aria-label = ...` with no
message value). When called via `l10n.getString()`, they silently return `undefined`.
We already fixed 3 instances of this class of bug (cycles 64–66). A scripted sweep
will catch the rest.

- [ ] **Step 1 — Extract attribute-only keys**
  - [ ] Run the awk one-liner across all `ui/src/locales/*.ftl` to build a list of attribute-only message IDs
- [ ] **Step 2 — Cross-reference with codebase usage**
  - [ ] `grep -rn "l10n.getString" ui/src/ --include="*.tsx"` to find all usages
  - [ ] For each match, check: is the key attribute-only? If yes, is there a fallback (`||` or `null,` fallback)?
  - [ ] Flag every `l10n.getString(ATTR_ONLY_KEY)` with no fallback as a **BUG**
- [ ] **Step 3 — Fix each bug**
  - [ ] Option A (preferred): Convert the FTL message from attribute-only to simple key=value
  - [ ] Option B: Add a `|| 'fallback'` in the code (only if the attribute-only format is needed for `<Localized attrs={...}>`)
  - [ ] Update both `en` and `id` bundles
- [ ] **Step 4 — Add a CI guard (optional but recommended)**
  - [ ] Write a script `scripts/check-attribute-only-ftl.sh` that fails CI if any attribute-only key is used via `l10n.getString()` without a fallback
  - [ ] Wire into `.github/workflows/` or the pre-commit hook
- [ ] **Step 5 — Validate**
  - [ ] `cd ui && npm run typecheck`
  - [ ] `cd ui && npx vitest run`
  - [ ] Bundle-parity on all FTL bundles
  - [ ] Code review + commit
