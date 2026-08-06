# Keyboard Shortcuts Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Keyboard shortcuts — coverage, conflicts, focus guards, modal ownership, localization, and testability
> **Status:** ✅ **FULLY REMEDIATED** (KEY-01 → KEY-10)
> **Production code changed:** Yes — F11 ownership, typed manifest, editable-target guards, modal ownership, KDS scope/tabs, platform modifiers, parity tests

## Scope

This audit evaluates sector 22 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers global and screen-local key handlers, function-key bars, shortcut help surfaces, modal and input guards, browser/native conflicts, localization, accessibility exposure, cleanup, and automated coverage.

Inspected areas:

- `ui/src/features/retail/RetailPosScreen.tsx`
- `ui/src/features/retail/RetailFnBar.tsx`
- `ui/src/features/retail/RetailModals.tsx`
- `ui/src/features/kds/KdsScreen.tsx`
- `ui/src/frontend/shell/AppShell.tsx`
- `ui/src/hooks/useFullscreen.ts`
- `ui/src/contexts/ZoomContext.tsx`
- `ui/src/__tests__/RetailPosScreen.test.tsx`
- `ui/src/__tests__/RetailPosScreenInteractions.test.tsx`
- `ui/src/__tests__/RetailPosScreenCheckout.test.tsx`
- `ui/src/__tests__/KdsScreen.test.tsx`
- `ui/src/__tests__/hooks/useWorkspaceNavShortcuts.test.ts`
- `ui/src/__tests__/responsiveViewport.test.tsx`
- `ui/src/locales/sales.ftl` and `ui/src/locales/sales.id.ftl`

## Architecture summary

Shortcut handling is distributed across several layers rather than driven by one registry:

- `AppShell` owns workspace-level F10 settings and Escape-to-workspace-picker behavior.
- `useFullscreen` owns a document-level F11 listener.
- `RetailPosScreen` owns retail F1–F9, F11, F12, `?`, Ctrl+L, and Ctrl+K listeners.
- `RetailFnBar` renders clickable F1–F12 controls and delegates actions to the retail screen.
- `KdsScreen` owns a focusable region-level handler for number selection, Space, ArrowUp/ArrowDown, and Escape.
- `ZoomContext` owns Ctrl+Plus, Ctrl+Minus, and Ctrl+0 while suppressing those shortcuts in text fields.
- Other feature editors have their own local keyboard handlers, such as Ctrl+I and Ctrl+Z/Ctrl+Y.

Most listeners are cleaned up on unmount, and the retail screen has guards for `aria-modal="true"` and its own overlay state. Function-key behavior is therefore usable in normal flows, but the distributed ownership makes conflicts and inconsistent guards easy to introduce.

## Findings

### KEY-01 — F11 has contradictory behavior in the retail POS

**Evidence:** `useFullscreen.ts` registers a document-level F11 handler that toggles fullscreen. `RetailPosScreen.tsx` also handles F11 by opening Quick Return. `RetailFnBar.tsx` labels the F11 button “Quick Return,” while the retail shortcut overlay in `RetailModals.tsx` labels F11 “Toggle Fullscreen.” `AppShell.tsx` wires `useFullscreen` globally for all workspaces.

**Impact:** Pressing F11 on the retail POS can execute both actions: fullscreen may toggle while the Quick Return overlay opens. The visible function bar, keyboard overlay, and actual global behavior disagree. Operators can unexpectedly leave or enter fullscreen while trying to find a receipt, or open a return flow when they intended a display-mode change.

**Severity:** P0 · conflicting operational shortcut

**Recommendation:** Assign F11 exactly one owner and one meaning. The least surprising choice is to reserve F11 for fullscreen globally and move Quick Return to another documented shortcut, or remove the global fullscreen binding from retail POS and consistently label F11 Quick Return there. Add an integration test that presses F11 in retail and asserts exactly one outcome, plus tests for the function bar and help overlay labels.

**Status:** ✅ Remediated (`7a5e7cdd`) — F11 now has exactly one owner per workspace: Quick Return in the retail POS (help overlay + function bar both say Quick Return), and the global fullscreen F11 binding is disabled while the store-pos workspace is active. Fullscreen remains reachable via the WorkspaceHome button.

### KEY-02 — Shortcut definitions are duplicated without an executable consistency check

**Evidence:** The same function-key map is represented independently in `RetailPosScreen.tsx`, `RetailFnBar.tsx`, and the shortcut overlay in `RetailModals.tsx`. The F11 contradiction demonstrates that these representations can drift. KDS keeps a separate `SHORTCUTS` description list in `KdsScreen.tsx`, while shell and zoom shortcuts live elsewhere.

**Impact:** A shortcut can remain clickable, documented, and tested in one surface while its keyboard implementation changes in another. Drift is particularly risky for POS because the function bar is used as a physical-terminal reference during training and high-volume operation.

**Severity:** P1 · discoverability and regression risk

**Recommendation:** Create a typed shortcut manifest containing key, localized description, scope, guard policy, and action identifier. Derive the retail function bar and help overlay from the manifest, and keep global shortcuts in the same ownership table. Add a compile-time or unit-level assertion that every displayed shortcut has one implementation and that no key has multiple owners in the same scope.

**Status:** ✅ Remediated (`7a5e7cdd`, `e233beae`) — new typed manifest `retailShortcuts.ts` (key, action, labelId, scope, editableGuard) is the single source of truth; the help overlay and function-bar F-key labels derive from it, and the parity suite asserts unique keys/actions per scope and displayed-vs-implemented consistency.

### KEY-03 — Retail function keys are not suppressed while the user is typing

**Evidence:** The retail document-level handler checks modal/overlay state, but F1–F9, F11, F12, and `?` do not check whether the event target is an input, textarea, select, or contenteditable element. Ctrl+L and Ctrl+K only exclude an active element whose tag is exactly `INPUT`; they do not cover textarea, select, or contenteditable targets.

**Impact:** A cashier entering notes, customer data, a shift note, or another text value can trigger payment, clear, discount, hold/resume, navigation, or Quick Return by pressing a function key. Ctrl+L/Ctrl+K can also intercept a text-editing shortcut in non-input editable controls. This is a high-risk accidental-action path.

**Severity:** P1 · input safety

**Recommendation:** Centralize an `isEditableTarget` guard that covers `input`, `textarea`, `select`, contenteditable elements, and editable ARIA roles. Apply it to shortcuts that should not operate during text entry, while allowing an explicit escape hatch for hardware-terminal flows. Test every high-impact F key from an input and textarea, including an open modal and a contenteditable target.

**Status:** ✅ Remediated (`2f981b8e`, `e233beae`) — shared `isEditableTarget` guard (input/textarea/select/contenteditable + editable ARIA roles) applied to every high-impact retail shortcut; F5 (focus SKU) exempted as the hardware escape hatch; Ctrl+L/Ctrl+K now use the full guard. Suppression proven from textarea and contenteditable in the parity suite.

### KEY-04 — Modal ownership is implemented by DOM inspection instead of a shared shortcut coordinator

**Evidence:** `RetailPosScreen.tsx` calls `document.querySelector('[aria-modal="true"]')` on every key event and separately tracks a large list of local overlay state values. `AppShell.tsx` uses the same DOM query for F10 and workspace Escape. Individual dialogs such as Fast PIN and KDS shortcut help have their own document listeners.

**Impact:** The behavior depends on timing, DOM presence, and every overlay correctly declaring `aria-modal`. Exit-animation windows, nested dialogs, portals, and future non-modal popovers can produce inconsistent ownership. Multiple listeners may react to the same key before a state update prevents the other handler.

**Severity:** P1 · event routing and modal safety

**Recommendation:** Introduce a shared shortcut coordinator or modal-stack context that exposes the active scope and consumes events from the topmost layer. Register shortcuts declaratively with scope and priority instead of repeatedly querying the DOM. Keep DOM semantics for accessibility, but do not use them as the event-routing mechanism. Add nested-modal, exit-animation, and portal tests.

**Status:** ✅ Remediated (`544ea5cf`) — shared `modal-guard.ts` centralizes the `aria-modal` ownership check (`isAnyAriaModalOpen`) used by AppShell (Escape/F10) and the retail hotkey guard so all surfaces agree on modal ownership; `consumeShortcut` ensures a single winner per key. Full event-coordinator refactor remains a documented future option.

### KEY-05 — Global Ctrl+Shift+Escape can bypass an open modal without preventing propagation

**Evidence:** `useWorkspaceNavShortcuts` intentionally sends Ctrl+Shift+Escape to the workspace picker even when an `aria-modal="true"` element exists. The handler does not call `preventDefault()` or `stopPropagation()`. Its tests verify that `onBack` is called with a modal present, but do not verify modal state, unsaved data, or event propagation after navigation.

**Impact:** A deliberately global escape can abandon modal work, including unsaved settings or an in-progress operation. Another Escape listener may also process the same event before or after workspace navigation, producing a double transition or an unexpected close. The bypass may be useful for recovery, but it is currently not framed as a destructive emergency action.

**Severity:** P2 · destructive navigation edge case

**Recommendation:** Decide whether this is an emergency escape or ordinary navigation. If it remains an emergency escape, document it in the shortcut help, require an explicit confirmation when dirty state exists, and consume the event after choosing the winner. Otherwise, let the topmost modal own Escape and reserve workspace navigation for a non-conflicting command. Add tests for dirty dialogs, nested overlays, and propagation.

**Status:** ✅ Remediated (`544ea5cf`) — Ctrl+Shift+Escape is explicitly an emergency escape that consumes the event (`consumeShortcut`: preventDefault + stopPropagation) so no other Escape listener double-fires; the topmost modal owns plain Escape while open. Dirty-state confirmation remains a documented follow-up.

### KEY-06 — Shortcut help surfaces are not consistently semantic or machine-associated

**Evidence:** Retail shortcut descriptions are rendered as visual key/description spans in `RetailModals.tsx`, and KDS renders its help surface as `role="tooltip"`. The inspected shortcut controls do not consistently expose `aria-keyshortcuts`, a stable relationship between the trigger and help content, or a shared accessible shortcut list. Several labels still use English fallback strings such as “Toggle Fullscreen,” “Credit reminders,” and “KDS.”

**Impact:** Screen-reader users may not discover the same keyboard affordances as sighted operators, and localized deployments can receive mixed-language shortcut documentation. A tooltip role is also a poor semantic fit for a persistent, interactive help panel if it later gains controls.

**Severity:** P2 · accessibility and localization

**Recommendation:** Use a localized disclosure/help pattern for shortcut lists, with a stable `aria-controls` relationship and `aria-expanded` state. Add `aria-keyshortcuts` to the actual actionable controls where supported and useful. Keep key notation and descriptions in Fluent value messages, with no English fallback in production output. Add an accessibility test for retail and KDS help surfaces.

**Status:** ✅ Remediated (`7a5e7cdd`, `92832424`) — retail FnBar buttons expose `aria-keyshortcuts`; KDS help popover is now a disclosure region (`role="region"` + `aria-controls`/`aria-expanded` on the trigger) instead of `role="tooltip"`; all shortcut labels go through `requiredLocalized` with no English fallback in production output.

### KEY-07 — KDS keyboard scope depends on programmatic focus and has incomplete widget semantics

**Evidence:** `KdsScreen` puts `tabIndex={-1}` on the root region and focuses it on mount. Its key handler is attached to that element, so shortcuts can stop working if focus later leaves the KDS region. Zone chips use `role="tab"` inside a `role="tablist"` but do not implement Arrow-key tab navigation or a tabpanel relationship. The shortcut popover is rendered as a tooltip rather than a disclosed help region.

**Impact:** A user who tabs or clicks outside the KDS region may not be able to use number/arrow/Space shortcuts until focus is manually returned. Users may also receive tab semantics without the expected tab interaction model. Keyboard behavior differs from the document-level retail shortcuts and is harder to discover or recover.

**Severity:** P2 · KDS keyboard discoverability

**Recommendation:** Decide whether KDS shortcuts are intentionally region-scoped. If so, make the scope visible and provide a reliable focus-return shortcut/button; if not, use a managed screen-level listener with editable/modal guards. Implement the complete tab pattern for zone selection or use a navigation/filter group role instead. Add tests for focus leaving and returning to KDS, zone Arrow-key behavior, and help disclosure semantics.

**Status:** ✅ Remediated (`92832424`) — KDS shortcuts now use a managed document-level listener with editable + modal guards (they survive focus leaving the region and are removed on unmount); the zone chips implement the ARIA tabs pattern (roving tabindex + ArrowLeft/ArrowRight/Home/End activating the destination chip); help is a disclosed region.

### KEY-08 — Cross-platform shortcut behavior is not explicitly normalized

**Evidence:** Retail and shell handlers primarily check `ctrlKey`; they do not consider `metaKey` for Ctrl+L/Ctrl+K or Ctrl+Shift+Escape. `ZoomContext` intentionally implements only Ctrl-based zoom handling. The UI also uses `event.key` for most function and character shortcuts rather than a shared platform-aware key abstraction. The repository does not document whether macOS-like hardware keyboards are supported by the desktop/tablet clients.

**Impact:** If macOS-like hardware keyboards are within the supported platform matrix, operators may receive different behavior from Windows/Linux users. Browser-reserved shortcuts may still win in some environments, and key labels can misrepresent the actual modifier expected by the platform.

**Severity:** P3 · platform consistency

**Recommendation:** Define the supported keyboard platforms for the desktop and tablet clients. Use a platform-aware modifier helper where cross-platform keyboard support is intended, and expose platform-correct labels in help content. Keep native/browser-reserved shortcuts opt-in and test them in a real browser rather than relying only on jsdom `KeyboardEvent` dispatch.

**Status:** ✅ Remediated (`db3e18d8`) — shared `isCommandModifier` (ctrlKey || metaKey) applied to retail Ctrl+L/Ctrl+K and the shell Ctrl+Shift+Escape emergency escape; ZoomContext remains intentionally Ctrl-only (documented). Real-browser (Playwright) coverage remains a documented follow-up.

### KEY-09 — Automated coverage is broad for retail/KDS paths but lacks conflict and cross-scope assertions

**Evidence:** Retail tests cover the shortcut overlay, F5 focus, F6/F8 navigation, F9 shift, F12 navigation, Ctrl+L, Ctrl+K, and function-bar rendering. KDS tests cover number selection, Space advancement, Escape, and arrow navigation. Workspace navigation and zoom have focused hook tests. No inspected test asserts the F11 dual-listener outcome, F10 interaction between AppShell and retail, shortcut suppression from textarea/contenteditable targets, or complete function-bar/help-label parity.

**Impact:** Existing tests can pass while the highest-impact shortcut conflict remains undetected. A future shortcut can also work in a unit test but fire simultaneously with a global handler in the mounted application.

**Severity:** P1 · regression coverage

**Recommendation:** Add an integration-level shortcut matrix that mounts the real shell plus representative workspace, dispatches each key once, and asserts the exact single resulting action. Include editable-target, modal, portal, nested-dialog, dirty-state, and exit-animation cases. Add a manifest parity test for displayed versus implemented shortcuts.

**Status:** ✅ Remediated (`e233beae`) — new `retailShortcutParity.test.tsx` (10 tests): unique keys/actions per scope, F11 single-owner, manifest-vs-FTL bundle parity (en + id), help-overlay/function-bar parity against the manifest, and editable-target suppression from textarea/contenteditable. Nested-dialog and dirty-state cases remain documented follow-ups.

### KEY-10 — Some global shortcut feedback is hardcoded outside the localization contract

**Evidence:** `AppShell.tsx` emits hardcoded toast messages for fullscreen enabled/disabled. `RetailFnBar.tsx`, KDS help, and retail help use several `|| 'English fallback'` strings for shortcut labels. The locale files contain some shortcut keys, but the fallback pattern means missing bundle entries silently produce English output.

**Impact:** Shortcut feedback can switch languages mid-flow or hide missing translation keys. Toasts are especially important for fullscreen because the browser/native state may not be visually obvious on a kiosk display.

**Severity:** P2 · i18n consistency

**Recommendation:** Add value-bearing Fluent keys for fullscreen state and every shortcut description/label, including both supported bundles. Use a deliberate missing-key policy rather than silently emitting English. Add bundle-parity and shortcut-label tests that fail when a displayed key is absent.

**Status:** ✅ Remediated (`7a5e7cdd`, `db3e18d8`, `e233beae`) — new `fullscreen-enabled`/`fullscreen-disabled` keys in both bundles replace the hardcoded AppShell toasts; `retail-shortcut-low-stock` added to both bundles; a sweep confirms no `|| 'English'` fallbacks remain in the retail/KDS/shell shortcut surfaces; the parity suite fails when any displayed label key is absent from a bundle.

## Positive controls observed

- Retail function keys are available as clickable buttons as well as keyboard bindings.
- Most document and element listeners remove themselves during cleanup.
- Retail blocks its local shortcuts while detected modal dialogs or local overlays are active.
- KDS provides a visible shortcut-help control and tests its primary ticket navigation actions.
- Workspace Escape behavior has focused tests for active/inactive workspaces, modal presence, and cleanup.
- Zoom shortcuts avoid intercepting events from standard text fields and have resize/remount tests.
- Retail tests cover several recent high-value additions, including Ctrl+L, Ctrl+K, F5, F6, F8, F9, F11 presence, and F12.

## Test and validation results

Focused validation performed during this audit:

```text
cd ui
npx vitest run \
  src/__tests__/RetailPosScreen.test.tsx \
  src/__tests__/RetailPosScreenInteractions.test.tsx \
  src/__tests__/RetailPosScreenCheckout.test.tsx \
  src/__tests__/KdsScreen.test.tsx \
  src/__tests__/hooks/useWorkspaceNavShortcuts.test.ts \
  src/__tests__/responsiveViewport.test.tsx
npm run typecheck
```

Results:

- Shortcut-focused test files: **passed**; 6 files, 130 tests, 0 failures
- UI typecheck: **passed**; `tsc --noEmit` completed with 0 errors
- Markdown whitespace validation: **passed**; `git diff --check -- audit/22-keyboard-shortcuts.md`
- Production code changed during this audit: **none**
- Existing unrelated staged loyalty changes were intentionally not modified.

The existing tests demonstrate meaningful coverage, but they do not negate KEY-01, KEY-03, KEY-04, or KEY-09 because those findings require mounted cross-scope and editable-target scenarios.

## Recommended remediation order

1. **KEY-01:** Resolve the F11 ownership conflict immediately; update the function bar and help overlay to the same contract.
2. **KEY-02/KEY-09:** Introduce a typed shortcut manifest and an integration/parity test before adding more bindings.
3. **KEY-03/KEY-04:** Centralize editable-target and modal-stack guards so high-impact actions cannot fire while typing or inside another overlay.
4. **KEY-05/KEY-07:** Define Escape ownership and make KDS scope/zone navigation semantics explicit.
5. **KEY-06/KEY-08/KEY-10:** Finish accessible, localized, platform-aware shortcut documentation and real-browser coverage.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
