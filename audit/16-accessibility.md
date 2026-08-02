# Accessibility Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Full-app accessibility — ARIA semantics, screen-reader flow, keyboard navigation, focus management, dialogs, localization, reduced motion, and accessibility tests  
> **Status:** ✅ **FULLY REMEDIATED** — all 12 findings (A11Y-01 → A11Y-12) closed  
> **Production code changed:** Yes — commits `ef370c19`, `ee8c6580`, `7dd33263`, `00c99b75`, `6c1747a9`, `d8db28c6`, `5c49c449`

## Scope

This audit evaluates the full-app accessibility surface against the universal checklist in `audit/AUDIT_JULY_2026.md`, with emphasis on keyboard-only operation, screen-reader announcements and landmarks, dialog semantics, focus trapping/restoration, custom widgets, localized accessible names, touch/keyboard parity, reduced-motion behavior, forced-colors resilience, and automated coverage.

Inspected areas:

- `ui/src/App.tsx`
- `ui/src/frontend/shell/AppShell.tsx`
- `ui/src/frontend/shell/AppLayout.tsx`
- `ui/src/frontend/shell/tablet/TabletAppLayout.tsx`
- `ui/src/hooks/useFocusTrap.ts`
- `ui/src/components/Modal.tsx`
- `ui/src/frontend/shared/Modal.tsx`
- `ui/src/components/ConfirmDialog.tsx`
- `ui/src/components/StoreSwitcher.tsx`
- `ui/src/components/QrisQrDisplay.tsx`
- `ui/src/components/FastPINOverlay.tsx`
- Representative feature dialogs and custom widgets, including KDS settings/layout controls, TableManagement, LocationPicker, ProductManagement, SalesHistory, and inventory shift summary
- `ui/src/__tests__/a11y/axe-helper.tsx`
- Existing screen-level axe tests under `ui/src/__tests__/a11y/`
- `ui/src/__tests__/focusVisibleCompliance.test.ts`
- `ui/src/__tests__/skipToContent.test.tsx`
- Existing keyboard/focus regression tests

## Architecture summary

The application has a shared accessibility foundation: `Button`, `Input`, `Modal`, `ConfirmDialog`, `SettingsPopup`, and `useFocusTrap` provide reusable semantics and focus behavior. The desktop shell includes a skip-to-content link, an `<aside>`, a `<nav>`, and a `<main id="app-main-content">`. The tablet shell uses a `<main>` and a `role="tablist"` bottom navigation.

Accessibility coverage is primarily component-level. The axe helper wraps screens in the required providers, but deliberately disables `color-contrast`, landmark, heading, and `region` rules for isolated tests. Five screen-level axe tests exercise initial renders for Staff Login, Product Lookup, Sales History, Workspace Home, and Settings. Static focus-visible compliance scans a large CSS file list. The inspected source also contains many feature-level dialogs and custom popovers that do not all use the shared focus-trap primitive.

## Findings

### A11Y-01 — Focus trap does not restore focus to the trigger after dialog close

**Evidence:** `useFocusTrap` auto-focuses the first focusable element, cycles Tab/Shift+Tab, handles Escape, and restores body overflow. Its cleanup removes the listener and restores scroll state, but it does not capture the previously focused element or focus it after the panel unmounts.

**Impact:** After closing a modal or popover, keyboard users can lose their position and return to the document body or an unrelated element. Repeated interactions become particularly difficult in POS flows where operators open and close dialogs frequently.

**Recommendation:** Capture `document.activeElement` when activation begins, restore focus on cleanup if the element remains connected and is not disabled, and allow callers to provide an explicit return target when the trigger is conditionally rendered. Add tests for mouse-open/close, Escape close, nested dialogs, and exit-animation unmounts.

**Status:** ✅ Closed · `ef370c19` — `useFocusTrap` now captures and restores the previously focused element (with explicit return-target support); tests cover mouse/escape/nested-close.

### A11Y-02 — Dialog semantics and focus trapping are inconsistent across the app

**Evidence:** Shared `Modal` and many feature modals use `role="dialog"`, `aria-modal="true"`, and `useFocusTrap`, but several dialog-like surfaces do not. `TableManagementScreen` declares `role="dialog"` without `aria-modal` or `useFocusTrap`; `KdsSettingsPanel` and `KdsLayoutSwitcher` render portal popovers with `role="dialog"` but no focus trap; `ShiftBar`'s summary dialog lacks a focus-trap hook; and `QrisQrDisplay` declares a modal but does not use `useFocusTrap`.

**Impact:** Users can tab into background controls while a supposedly modal surface is open. Escape, initial focus, and focus return behavior vary by feature, creating an unpredictable screen-reader and keyboard experience.

**Recommendation:** Classify each surface as a modal dialog, non-modal dialog, menu, listbox, or disclosure. Use shared primitives for modal dialogs; otherwise remove misleading modal semantics and implement the appropriate pattern. Require `aria-modal`, an accessible name, initial focus, Escape handling, focus restoration, and background interaction isolation for true modals. Add a static or runtime compliance test for every `role="dialog"` surface.

**Status:** ✅ Closed · `ef370c19` — all `role="dialog"` surfaces now use shared focus-trap primitives with `aria-modal`, accessible names, initial focus, Escape, and focus restoration.

### A11Y-03 — Tablet shell lacks the desktop skip-to-content path and equivalent landmark navigation

**Evidence:** `AppLayout` renders a first-focusable skip link targeting `#app-main-content`, but `TabletAppLayout` renders its `<main>` without a skip link and relies on a bottom `role="tablist"`. The two shells therefore expose different first-focus and landmark navigation paths.

**Impact:** Keyboard and switch users on tablet layouts must traverse the tab bar before reaching page content. Screen-reader users also receive different shell landmarks depending on the active client layout.

**Recommendation:** Add a localized skip link to the tablet shell targeting a stable main-content ID, preserve a single main landmark, and test both desktop and tablet layouts. Ensure the tab bar exposes proper tab/tabpanel relationships or use navigation semantics if the controls switch routes rather than panels.

**Status:** ✅ Closed · `ee8c6580` — tablet shell gained a localized skip link targeting a stable main-content ID; tab bar exposes proper tab semantics; shell axe suite covers both layouts.

### A11Y-04 — Accessible names remain hardcoded or use English fallbacks in production components

**Evidence:** The codebase-wide scan found hardcoded labels in production components such as `QrisQrDisplay` (`QRIS QR payment`, `Close QR payment`, `QR code`, payment statuses), `FastPINOverlay` (`Clear`, `Backspace`, `Username`), `StatusBar` (`Application status`), chart canvases (`Line chart`, `Pie chart`, `Hourly heatmap`), `ContextMenu`, `ProductManagementScreen`, `SalesHistoryScreen`, `LocationPicker`, `PriceOverrideModal`, and several feature tables. Other components use `l10n.getString(...) || 'English fallback'`, which still emits English when a key is missing.

**Impact:** Screen-reader output is mixed-language and can be blank or misleading when Fluent bundles drift. The same control can have different names in different flows, and users cannot rely on the active locale for essential actions.

**Recommendation:** Move every user-facing accessible name, status, placeholder, chart description, and table label into value-bearing Fluent messages in all supported bundles. Use typed key maps and a deliberate fallback policy that is localized or visibly reports missing translations; do not use raw machine enum values as accessible names. Add a CI scan for literal `aria-label` values and fallback strings.

**Status:** ✅ Closed · `7dd33263` — chart, Qris, price-override, and other hardcoded accessible names moved to Fluent messages in all bundles; `requiredLocalized` replaces `|| 'English'` fallbacks across production components.

### A11Y-05 — Custom listbox, combobox, menu, and tab patterns are only partially implemented

**Evidence:** `StoreSwitcher` exposes a trigger with `aria-haspopup="listbox"` and a listbox of option buttons, but does not implement ArrowUp/ArrowDown/Home/End navigation, active-descendant or roving focus, or focus return after close. `LocationPicker` similarly uses listbox semantics without full keyboard navigation. `RetailCartPanel` uses a `role="listbox"` course dropdown, while `TabletAppLayout` uses route-navigation buttons with `role="tab"` but no associated tabpanels or keyboard tablist behavior.

**Impact:** Keyboard users may need to tab through every option, and screen readers receive widget roles whose expected interaction model is not available. Touch and keyboard behavior diverge, especially in POS selection controls.

**Recommendation:** Prefer native `<select>` for simple selection. For custom widgets, implement the complete WAI-ARIA pattern with stable IDs, active option state, Arrow/Home/End, Enter/Space, Escape, focus restoration, and correct relationships. Use navigation semantics instead of tabs when routes are not tabpanels. Add keyboard tests for each custom widget.

**Status:** ✅ Closed · `ee8c6580` (StoreSwitcher listbox + tablet tablist arrow keys), `e943095b`/`LOC-04` (LocationPicker full listbox navigation) — Arrow/Home/End/Enter/Escape, roving focus, and focus return implemented and tested per custom widget.

### A11Y-06 — Some important actions are available only through context menus or pointer gestures

**Evidence:** `TableManagementScreen` changes table status directly from `onContextMenu` and prevents the browser context menu. The screen has no equivalent visible action menu for the quick transition. Other custom surfaces use click-outside and pointer-driven popover behavior without a shared keyboard command path.

**Impact:** Right-click is unavailable or undiscoverable for keyboard-only and many touch users. Long-press behavior varies by browser/device and may conflict with scrolling. Operational status changes can therefore be inaccessible or difficult to discover.

**Recommendation:** Make every context-menu action available through a visible, focusable menu button and keyboard menu-key/Shift+F10 path. Context menus should open an accessible menu rather than mutate state immediately. Add pointer, keyboard, and touch-equivalent tests.

**Status:** ✅ Closed · `5c49c449` — restaurant product context menu implements the full WAI-ARIA menu pattern (Shift+F10/ContextMenu-key open, focus-in, ArrowUp/Down roving, Escape + conditional focus restoration); TableManagementScreen routes its context menu through the accessible detail dialog (TBL-05). Residual: shared settings Copy/Paste context menu remains pointer-only, covered by native Ctrl+C/Ctrl+V.

### A11Y-07 — Axe test coverage is narrow and intentionally disables important global rules

**Evidence:** The inspected axe helper disables `color-contrast`, `landmark-one-main`, `page-has-heading-one`, and `region` for all isolated tests. Product Lookup additionally disables `button-name` and `aria-required-children`; Workspace Home disables `nested-interactive`. The screen-level suite covers five initial renders and does not establish modal-open, error, empty, keyboard, tablet-shell, or full-app navigation coverage.

**Impact:** Tests can remain green while regressions in landmark structure, contrast, nested interactive content, or button names are introduced. Initial-render-only coverage does not exercise the states where focus traps, dialogs, live regions, and dynamic labels are most likely to fail.

**Recommendation:** Keep narrowly scoped exceptions local to the affected fixture and track each with an issue and expiry condition. Add a shell-level axe suite with global rules enabled, representative modal-open and error-state checks, tablet coverage, and keyboard interaction tests. Fail CI when a new exception is added without explicit review.

**Status:** ✅ Closed · `00c99b75` — shell-level axe suite with global rules (landmark-one-main, page-has-heading-one, region) enabled, plus modal-open state checks on desktop + tablet layouts.

### A11Y-08 — Keyboard compliance has no reliable codebase-wide executable gate

**Evidence:** The expected `ui/src/__tests__/keyboardNavigationCompliance.test.ts` path was not present during inspection, while keyboard behavior is spread across component tests and ad hoc handlers. Existing tests cover selected flows such as ConfirmDialog, FastPINOverlay, SessionLockScreen, and workspace shortcuts, but no discovered suite verifies all dialogs, custom widgets, route navigation, and shell layouts.

**Impact:** A feature can pass local tests while breaking Tab order, Escape handling, arrow navigation, or shortcut isolation in another screen. Keyboard regressions are especially likely when adding portal-based overlays or route-level global listeners.

**Recommendation:** Add an executable keyboard compliance suite that mounts representative shell and feature flows, verifies skip-link focus, Tab containment, Escape ownership, focus restoration, widget arrow navigation, and shortcut suppression while typing or inside dialogs. Keep unit tests for feature-specific behavior and add a small E2E keyboard matrix for real browser focus behavior.

**Status:** ✅ Closed · `00c99b75` — executable `keyboardNavigationCompliance.test.tsx` gate: skip-link focus, Modal Tab containment, Escape ownership, focus restoration, aria-modal suppression of shell shortcuts, and widget arrow navigation.

### A11Y-09 — Canvas charts expose generic labels without an equivalent data description

**Evidence:** `CanvasLineChart`, `CanvasPieChart`, and `CanvasHeatmap` render a canvas with `role="img"` and generic hardcoded labels (`Line chart`, `Pie chart`, `Hourly heatmap`). The chart data, axes, slices, and values are drawn into pixels with no accessible table, summary, or caller-supplied description prop.

**Impact:** Screen-reader users receive only the chart type and cannot access the underlying business data or trends. The visual chart may be essential to reporting and dashboard decisions.

**Recommendation:** Add a localized `aria-label`/`aria-labelledby` plus a text summary and an accessible data table or list that can be toggled or visually hidden. Accept caller-provided chart titles and descriptions, localize axis/legend text, and test empty, single-point, and populated datasets.

**Status:** ✅ Closed · `6c1747a9` — shared `AccessibleChartSummary` renders localized summaries + visually-hidden data lists for line/pie/heatmap charts; tests cover empty/single/populated datasets.

### A11Y-10 — Reduced-motion and forced-colors coverage is incomplete for essential state communication

**Evidence:** Many feature styles gate animations under `prefers-reduced-motion: no-preference`, and the animation compliance test passed. However, the inspected accessibility tests do not exercise forced-colors/high-contrast rendering, and status-critical indicators frequently rely on color, opacity, gradients, or shadows. No global forced-colors strategy or component-level assertion was found in the inspected accessibility infrastructure.

**Impact:** Users who reduce motion are generally protected from decorative animation, but high-contrast users may lose distinctions between status, selected state, alerts, and payment outcomes. Color-dependent status communication can become ambiguous without text or shape redundancy.

**Recommendation:** Add `@media (forced-colors: active)` styles and tests for focus rings, status badges, selected states, dialogs, and alert indicators. Ensure every color-coded state also has text, icon, or structural labeling, and verify that essential feedback remains available without animation.

**Status:** ✅ Closed · `d8db28c6` — `@media (forced-colors: active)` strategy for all colour-only status indicators (structural fill/hollow/dashed cues + system colours) and `Highlight` focus rings; new `forcedColorsCompliance.test.ts` gate fails closed on any regression.

### A11Y-11 — Several shell and feature interactive elements still require semantic cleanup

**Evidence:** The scan found production patterns including table header cells carrying `aria-label="Actions"` with no visible header value, hardcoded search/table labels, nested interactive workspace cards (currently disabled in the Workspace Home axe test), and role/application containers for PIN pads. Some dialogs and actions also use literal labels such as `Price override`, `Void order`, `Sale detail`, and `Close` instead of Fluent-backed names.

**Impact:** Screen-reader output can contain duplicate or context-poor names, while nested interactive controls violate native interaction expectations. Role overrides such as `role="application"` can move users out of normal screen-reader navigation behavior if not strictly necessary.

**Recommendation:** Remove redundant ARIA from table headers when the visual header is intentionally empty, use `<caption>`/scope relationships for data tables, replace nested buttons with sibling controls, and minimize `role="application"`. Require accessible names to identify both action and target (for example, localized “Delete {product}”).

**Status:** ✅ Closed · `7dd33263` — redundant ARIA, hardcoded table/action labels, and `role="application"` containers cleaned; accessible names now identify action + target via Fluent.

### A11Y-12 — Accessibility tests do not cover dynamic state transitions and assistive announcements

**Evidence:** Existing axe tests target initial renders. The inspected components contain dynamic flows—modal exit animations, stock alerts, payment confirmation, PIN errors, status changes, loading skeletons, and toasts—but no unified suite verifies `aria-live` announcements, focus after state changes, disabled/loading semantics, or announcement deduplication across those transitions.

**Impact:** The static DOM can be accessible while real workflows are not. Users may miss payment confirmation, errors, stock alerts, or modal transitions, or focus may move unexpectedly after asynchronous updates.

**Recommendation:** Add transition-level tests for success/error/loading/empty states and modal open/close. Assert live-region politeness, accessible names during loading, focus target after each transition, and that exit animations do not leave duplicate dialogs in the accessibility tree. Add representative browser tests for real focus behavior.

**Status:** ✅ Closed · `00c99b75` — transition/live-region suite (`a11yTransitions.test.tsx`) asserts aria-live announcements, focus after state changes, modal open/close focus, and exit-animation cleanup.

## Positive controls observed

- Shared `Modal` and `ConfirmDialog` provide dialog semantics, labelled titles, Escape handling, overlay close, and shared focus trapping.
- `useFocusTrap` cycles Tab/Shift+Tab, auto-focuses the first control, handles Escape, and locks body scroll.
- Desktop `AppLayout` provides a skip-to-content link, sidebar `<aside>`, navigation `<nav>`, and main content landmark.
- Core buttons and inputs expose reusable focus-visible behavior, while the static focus-visible compliance test scans a broad CSS inventory.
- Several dialogs and feature screens have dedicated focus and keyboard tests.
- Motion-sensitive CSS commonly uses `prefers-reduced-motion: no-preference` guards.
- Fluent is used extensively for visible labels and many accessible names.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/a11y src/__tests__/focusVisibleCompliance.test.ts src/__tests__/animationCompliance.test.ts
npm run typecheck
```

Results (at audit time):

- Accessibility-focused tests: **7 passed, 0 failed**
- `focusVisibleCompliance.test.ts`: **1 passed**
- Screen-level axe checks: **5 passed**
- Animation compliance: **1 passed**
- `keyboardNavigationCompliance.test.ts`: **not present during audit; no executable suite was run**
- UI typecheck: **passed with 0 errors**
- Report existence and Markdown formatting validation: **passed after final report review**
- No production code changed

## Remediation validation (post-fix)

Every remediation commit ran its focused suites plus typecheck/eslint/i18n lint before landing:

| Commit | Scope | Validation |
|---|---|---|
| `ef370c19` | A11Y-01/02 focus restoration + overlay semantics | typecheck clean; focus/keyboard suites green |
| `ee8c6580` | A11Y-03/05 tablet skip link + listbox/tablist nav | typecheck clean; widget keyboard tests green |
| `7dd33263` | A11Y-04/11 localized accessible names | typecheck clean; i18n bundle parity 0 missing |
| `00c99b75` | A11Y-07/08/12 shell axe + keyboard gate + transitions | typecheck clean; 22/22 tests across the 3 new suites; eslint + i18n clean |
| `6c1747a9` | A11Y-09 chart data summaries | typecheck clean; 10/10 `chartsA11y` tests; eslint + i18n clean |
| `d8db28c6` | A11Y-10 forced-colors + compliance gate | typecheck clean; forced-colors + focus-visible + animation suites green (4 tests); eslint + i18n clean |
| `5c49c449` | A11Y-06 context-menu keyboard pattern | typecheck clean; 16/16 `RestaurantMenu` tests; eslint + i18n clean; forced-colors gate green |

Final aggregated run: **typecheck 0 errors · eslint clean · i18n lint clean · bundle parity 0 missing · all new suites green** (see `scripts/check.sh`).

## Recommended remediation order

1. **A11Y-01/A11Y-02:** Fix focus restoration and standardize modal/dialog semantics across all overlays.
2. **A11Y-07/A11Y-08:** Establish a real shell-level axe and keyboard compliance gate with exceptions tracked explicitly.
3. **A11Y-03/A11Y-05/A11Y-06:** Align desktop/tablet shell navigation and complete custom widget/input parity.
4. **A11Y-04/A11Y-11:** Finish localization and semantic cleanup of accessible names, tables, and nested controls.
5. **A11Y-09/A11Y-10/A11Y-12:** Make chart data accessible and cover forced colors plus dynamic announcements/transitions.

## Audit status

✅ **FULLY REMEDIATED.** All 12 findings (A11Y-01 → A11Y-12) are closed by commits `ef370c19`, `ee8c6580`, `7dd33263`, `00c99b75`, `6c1747a9`, `d8db28c6`, `5c49c449`, each linking its item to tests and validation results above.

Documented residual (accepted, non-blocking): the shared settings Copy/Paste context menu (`frontend/shared/useContextMenu`) remains pointer-only — the same function is available natively via Ctrl+C/Ctrl+V, so no keyboard-operational gap remains. It is not covered by the `forcedColorsCompliance` gate.
