# Tablet & Mobile Responsiveness Audit — July 2026

> **Audit date:** 2026-08-02
> **Sector:** Mobile / tablet responsiveness — `index.tablet.html`, touch UX, viewport, tablet shell
> **Status:** ✅ **FULLY REMEDIATED** (TAB-01 → TAB-06 — commits `42263ef9`, `ed6ec31f`, `7780c206`, `6aedc287`, `27a1e0e1`, `7a82227a`)
> **Production code changed:** Yes — tablet client build wiring, shell layout, entry boot, viewport meta, E2E + unit coverage

## Scope

This audit evaluates sector 20 against the universal checklist in `audit/AUDIT_JULY_2026.md`:
responsive layout, touch-target sizing, viewport handling, orientation, accessibility of
the tablet shell, and end-to-end coverage of the tablet entry point.

Inspected areas:

- `ui/index.tablet.html` — tablet HTML entry + viewport meta
- `ui/src/main.tablet.tsx` — tablet React entry + CSS imports
- `ui/vite.tablet.config.ts` — tablet Vite build (port 1422, `dist-tablet`)
- `ui/src/frontend/shell/tablet/TabletAppShell.tsx` — tablet shell routing
- `ui/src/frontend/shell/tablet/TabletAppLayout.tsx` — bottom tab bar layout
- `ui/src/frontend/shell/tablet/tablet.css` — tablet touch-optimised styles
- `ui/src/hooks/useOrientation.ts` — orientation lock hook
- `ui/src/frontend/shell/AppLayout.css` — desktop shell base styles (dependency check)
- `ui/src/frontend/themes/responsive.css` — responsive utility classes
- `apps/tablet-client/tauri.conf.json` — tablet Tauri client build wiring
- `apps/tablet-client/src/lib.rs` — tablet client entry
- `ui/package.json` — npm scripts
- `ui/e2e/playwright.config.ts` + `ui/e2e/tablet-viewport.spec.ts` — E2E coverage
- `ui/src/__tests__/TabletAppLayout.test.tsx`, `useOrientation.test.ts` — unit coverage
- `.github/workflows/android.yml` — tablet CI build

## Architecture summary

The repository has a complete tablet-optimised front-end: `index.tablet.html` +
`main.tablet.tsx` boot a `TabletAppShell` that renders a thumb-reachable bottom tab bar
(`TabletAppLayout`), 48px touch targets, larger typography, safe-area insets, and an
orientation lock (`useOrientation('landscape-primary')`). The tablet UI builds separately
via `vite.tablet.config.ts` (port 1422, output `dist-tablet`).

However, the tablet Tauri client (`apps/tablet-client/tauri.conf.json`) is not wired to
this build: it points at the **desktop** dev server (`devUrl: http://localhost:1420`) and
the **desktop** build output (`frontendDist: ../../ui/dist`), and no `dev:tablet` /
`build:tablet` npm scripts exist. The tablet shell is effectively unreachable in the
shipping client, and several latent defects would surface the moment it is wired up.

## Findings

### TAB-01 — The tablet Tauri client ships the desktop UI, not the tablet UI

**Evidence:** `apps/tablet-client/tauri.conf.json` declares:
`beforeDevCommand: "npm run dev --prefix ../../ui"`, `devUrl: "http://localhost:1420"`,
`beforeBuildCommand: ""`, and `frontendDist: "../../ui/dist"` — all pointing at the
desktop entry (`index.html`, port 1420, `dist`). The tablet build (`vite.tablet.config.ts`)
outputs to `dist-tablet` and serves port 1422, and `ui/package.json` has no
`dev:tablet`/`build:tablet` script (only `bundle:check:tablet`). `.github/workflows/android.yml`
builds the tablet UI with `npx vite build --config vite.tablet.config.ts`, but the client's
Tauri config then loads `ui/dist` (the desktop app) at runtime.

**Impact:** The tablet-optimised shell — bottom tab bar, 48px touch targets, orientation
lock — is dead code in the shipping binary. Devices boot the desktop layout instead, so
every tablet-specific UX and a11y improvement is invisible to real users.

**Severity:** P1 · product capability

**Status:** ✅ Remediated — commit `42263ef9`

### TAB-02 — The tablet entry never loads AppLayout.css, so the shell layout lacks its base rules

**Evidence:** `main.tablet.tsx` imports `reset.css`, `tokens.css`, `components.css`, and
`responsive.css` — but **not** `AppLayout.css`. `TabletAppLayout` renders `.app-layout`,
`.app-content`, and `.app-sidebar`, whose base rules (`display: flex; flex-direction:
column; height: 100dvh;`) live in `AppLayout.css`, which is imported only by the desktop
`AppLayout.tsx`. `tablet.css` overrides `.tablet-shell .app-layout` with
`flex-direction: column-reverse; height: 100dvh;` but never sets `display: flex` itself.
Without `AppLayout.css`, `.app-layout` is a plain block: the bottom tab bar and
`flex: 1` content area cannot lay out correctly.

**Impact:** The moment TAB-01 is fixed, the tablet shell renders broken — content clipped,
tab bar mis-positioned. The tablet layout is not self-contained.

**Severity:** P1 · correctness (blocked on TAB-01 fix)

**Status:** ✅ Remediated — commit `ed6ec31f`

### TAB-03 — Hardcoded English fallback in the tablet shell

**Evidence:** `TabletAppLayout.tsx:87` uses
`{l10n.getString('a11y-skip-to-content') ?? 'Skip to main content'}` — the exact
`getString(...) || fallback` pattern the requiredLocalized sweep (TAX-09) eliminated
elsewhere. `requiredLocalized` exists in `ui/src/frontend/shared/requiredLocalized.ts`.

**Impact:** A missing/misspelled key silently renders English instead of surfacing as a
dev-time i18n defect; inconsistent with the codebase-wide localization contract.

**Severity:** P2 · i18n consistency

**Status:** ✅ Remediated — commit `7780c206`

### TAB-04 — No E2E coverage of the actual tablet shell

**Evidence:** `e2e/tablet-viewport.spec.ts` runs under the `tablet` Playwright project
(1024×1366), but the shared `webServer` starts `npm run dev` (the desktop server on port
1420 serving `index.html`). The spec never visits `index.tablet.html`, so it exercises the
desktop shell at a tablet viewport — not the bottom tab bar, tablet.css, or orientation
lock. There is also no `dev:tablet` script, so running the tablet UI standalone requires
manually invoking `vite --config vite.tablet.config.ts`.

**Impact:** The tablet-specific UI has zero automated end-to-end coverage; a regression in
the tab bar, touch targets, or tablet routing could ship undetected.

**Severity:** P2 · quality assurance

**Status:** ✅ Remediated — commit `6aedc287` (also fixed the `main.tablet.tsx` boot blocker: `ThemeProvider` requires `BrandProvider`, which the entry never provided — the E2E caught it)

### TAB-05 — `user-scalable=no` / `maximum-scale=1.0` on the tablet viewport blocks zoom

**Evidence:** `index.tablet.html` viewport meta:
`width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover`.
The desktop `index.html` correctly omits `maximum-scale`/`user-scalable`. Disabling pinch
zoom violates WCAG 1.4.4 (Resize Text) and 1.4.10 (Reflow) — a real accessibility defect
on the tablet entry.

**Impact:** Users who need enlarged text cannot zoom the tablet UI.

**Severity:** P2 · accessibility

**Status:** ✅ Remediated — commit `27a1e0e1`

### TAB-06 — Tablet shell routing has no unit tests

**Evidence:** `TabletAppShell.tsx` handles setup-status bootstrapping, auth gating, workspace
routing (restaurant-pos/store-pos/kds fullscreen vs sidebar), and permission-fallback
navigation — but only `TabletAppLayout.test.tsx` (the presentational tab bar) and
`useOrientation.test.ts` have tests. The routing logic that decides what renders when is
untested.

**Impact:** Route regressions (e.g. workspace switch landing on the wrong tab, denied page
fallback) can pass CI undetected.

**Severity:** P3 · test coverage

**Status:** ✅ Remediated — commit `7a82227a`

## Positive controls observed

- A complete tablet shell exists: bottom tab bar, 48px tap targets, larger typography,
  safe-area insets, `user-select` guard with text inputs re-enabled.
- `useOrientation` correctly degrades when the ScreenOrientation API is unsupported and
  unlocks on unmount.
- `TabletAppLayout` implements the WAI-ARIA tablist keyboard pattern (roving tabindex,
  arrow/Home/End) and a skip-to-content link as the first focusable element.
- `touchTargetSizing.test.tsx` already scans `tablet.css` (in `CSS_FILES`) — the tablet
  stylesheet is under the 44px touch-target gate.
- The Playwright config defines a dedicated `tablet` project at 1024×1366.

## Recommended remediation order

1. **TAB-01/TAB-02:** Wire the tablet client to the tablet build (`dist-tablet`, port 1422)
   and make the shell layout self-contained so the tablet UI is actually shippable.
2. **TAB-03:** Use `requiredLocalized` for the skip link.
3. **TAB-05:** Remove `user-scalable=no`/`maximum-scale=1.0` from the tablet viewport.
4. **TAB-04/TAB-06:** Add a tablet-shell E2E that loads `index.tablet.html` and unit tests
   for `TabletAppShell` routing.
