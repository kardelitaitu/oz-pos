# UI Wiring & Element Audit Plan — OZ-POS Desktop App

- **Audit ID:** 2026-07-23-ui-wiring-audit
- **Status:** Planning
- **Scope:** `ui/src/` React/TypeScript front-end used by `apps/desktop-client/` (Tauri Windows app)
- **Out of scope:** Rust/Tauri command layer, database, API contracts, styling-only issues
- **Goal:** Verify that buttons, inputs, forms, modals, and interactive elements are correctly wired to their handlers, states, and side-effects; identify inconsistencies, accessibility gaps, and missing test coverage before release.

---

## 1. Current baseline (quick scan)

| Metric | Count | Notes |
|--------|-------|-------|
| `<Button>` usages | ~163 | Central `ui/src/components/Button.tsx` component |
| Raw `<button>` usages | ~223 | Mix of legacy markup, shell UI, and test mocks |
| `onClick=` handlers | ~166 | Spread across features and components |
| `type="button"` explicit | ~181 | Many raw buttons already explicit |
| `type="submit"` | ~2 | Login / PIN forms paths |
| Button unit tests | 1 file | `ui/src/__tests__/Button.test.tsx` — 26 tests, comprehensive for component itself |

**Known hotspots already spotted:**

1. **Hybrid button ecosystem** — design-system `<Button>` coexists with many raw `<button>` elements carrying bespoke class names.
2. **Test mocks using raw `<button>`** — several test files (`DataManagementScreen.test.tsx`, `DesignSystem.test.tsx`, `CustomReportScreen.test.tsx`, etc.) mock the UI with raw `<button>` instead of the real `<Button>` component, masking wiring regressions.
3. **Workspace pin button** — `WorkspaceHome.tsx` uses a `<span role="button">` inside a `<button>` to avoid invalid nested `<button>` HTML. Functionally correct but semantically complex and keyboard-focusable only via JS.
4. **Raw buttons without `type="button"`** — many raw buttons inside forms or dynamic markup may accidentally submit forms if not explicitly typed.
5. **Mixed `loading` vs `state="processing"`** — `Button` supports both `loading` (deprecated) and `state`; audit should check consistency.

---

## 2. Audit categories

### 2.1 Button wiring consistency

Verify for every interactive button in the app:

| Check | Pass criteria |
|-------|---------------|
| **Component choice** | Prefer `<Button>` over raw `<button>` unless there is a documented reason (e.g., shell UI, icon-only, custom animations). |
| **Type attribute** | Every button inside or near a `<form>` has explicit `type="button"` or `type="submit"` as intended. |
| **Disabled state** | `disabled` is wired to loading/error states; destructive actions disable while in-flight. |
| **Loading state** | Async actions use `loading` / `state="processing"` to show spinner and prevent double-submission. |
| **Click handler** | `onClick` is defined and not an inline `() => undefined` or stale closure. |
| **Keyboard accessibility** | Buttons are reachable by keyboard; icon-only buttons have `aria-label`. |
| **Double-submit prevention** | Forms and modals disable submit while the mutation is pending. |

### 2.2 Form wiring

| Check | Pass criteria |
|-------|---------------|
| **Submit wiring** | `<form onSubmit={handleSubmit}>` is used instead of relying solely on `onClick`. |
| **Validation feedback** | Required fields disable submit until valid; inline errors appear on blur/submit. |
| **Reset / cancel** | Cancel buttons have `type="button"` and close/clean state without submitting. |
| **Enter key behavior** | Pressing Enter in a single-field form triggers submit as expected. |

### 2.3 Modal & dialog wiring

| Check | Pass criteria |
|-------|---------------|
| **ConfirmDialog** | Destructive actions use `ConfirmDialog`; native `confirm()` should be gone. |
| **Close wiring** | Close button, backdrop click, and Escape key all route through the same cancel/cleanup logic. |
| **Focus trap** | Focus remains inside the modal while open. |
| **Action disable** | Confirm button is disabled while the async action runs. |

### 2.4 Navigation & route wiring

| Check | Pass criteria |
|-------|---------------|
| **Nav items** | Shell navigation buttons route to the correct route and update active state. |
| **Workspace switching** | `WorkspaceHome` cards correctly call `setActiveWorkspace` and handle loading/error states. |
| **Back / cancel** | Back buttons restore the previous view or close the current overlay. |

### 2.5 Toast & error wiring

| Check | Pass criteria |
|-------|---------------|
| **Success feedback** | User-triggered mutations show a success toast. |
| **Error feedback** | Failed actions surface user-readable errors via `ErrorState`, toast, or inline message. |
| **Remaining `alert()`** | Remove or migrate the remaining native `alert()` calls (noted in UI state audit). |

### 2.6 Test coverage for wiring

| Check | Pass criteria |
|-------|---------------|
| **Handler invocation** | Tests fire clicks and assert the correct handler is called with expected args. |
| **Disabled behavior** | Tests verify submit button is disabled when invalid/loading. |
| **Loading state** | Tests verify spinner/aria-busy appears during async operations. |
| **Mock fidelity** | Test mocks use the real `<Button>` component where possible rather than a fake `<button>`. |

---

## 3. Methodology

### 3.1 Automated inventory

Run mechanical searches to produce a baseline spreadsheet:

```bash
cd ui

# Count Button vs raw button
npx grep-tool "<Button" src --ext tsx
npx grep-tool "<button" src --ext tsx

# Find raw buttons missing explicit type inside forms
npx grep-tool "<button" src --ext tsx -A 2 -B 1 | grep -v 'type="button"\|type="submit"'

# Find onClick handlers on non-button elements
npx grep-tool "onClick=" src --ext tsx | grep -v "<Button\|<button"

# Find loading vs state="processing" usage
npx grep-tool "loading" src --ext tsx
npx grep-tool 'state="processing"' src --ext tsx
```

> *Use the existing `code_searcher` results as the seed; transform them into a tracked artifact in this folder.*

### 3.2 Manual review matrix

For each major feature under `ui/src/features/`, fill out a matrix row:

| Feature | Primary buttons | Forms | Modals | A11y issues | Wiring bugs | Tests cover wiring? |
|---------|-----------------|-------|--------|-------------|-------------|---------------------|
| sales/CartScreen | ... | ... | ... | ... | ... | ... |
| sales/PaymentModal | ... | ... | ... | ... | ... | ... |
| inventory/StockCountForm | ... | ... | ... | ... | ... | ... |
| ... | ... | ... | ... | ... | ... | ... |

### 3.3 Test pass

Run the UI test suite and capture wiring-related failures:

```bash
cd ui
npm run test -- --run
```

Capture any test failures related to:
- Button clicks not firing
- Form submission not happening
- Modal confirm/cancel not behaving
- Loading states not reflecting

### 3.4 Accessibility spot-check

Run the existing a11y tests and manual keyboard walkthrough:

```bash
cd ui
npm run test -- --run a11y
```

Focus on:
- Buttons reachable by `Tab`
- Icon-only buttons have accessible names
- Disabled buttons are communicated via `aria-disabled` or `disabled`

---

## 4. Deliverables

1. **Baseline inventory CSV** — every button/element found, its file, line, type, and handler.
2. **Findings report** — categorized as Critical / High / Medium / Low with file/line references.
3. **Prioritized fix backlog** — issues grouped by feature and impact.
4. **Test coverage recommendations** — list of screens that need wiring tests.
5. **(Optional) Fix PRs** — only if the team decides to move from "plan" to "execute".

---

## 5. Suggested phases

| Phase | Duration | Activities |
|-------|----------|------------|
| **Phase 1 — Inventory** | 1 session | Run automated searches, produce baseline CSV, categorize Button vs raw button. |
| **Phase 2 — Critical wiring check** | 1–2 sessions | Payment flow, login/PIN, modals with destructive actions, workspace switching. |
| **Phase 3 — Feature-by-feature matrix** | 2–3 sessions | Fill the manual review matrix; identify missing `type` attrs, double-submit risks, stale handlers. |
| **Phase 4 — Test coverage gap analysis** | 1 session | Map each finding to existing/missing tests; recommend new tests. |
| **Phase 5 — Report & backlog** | 1 session | Write findings report, produce fix backlog, schedule PRs. |

---

## 6. Definition of done

- Every interactive button in scope has been classified and its wiring verified against the checklist.
- All payment, login, and destructive-action flows have explicit test coverage or documented gaps.
- No raw `<button>` remains inside a `<form>` without an explicit `type`.
- `ConfirmDialog` is used for all destructive confirmations; native `confirm()` / `alert()` are removed.
- Report is approved and converted into actionable GitHub/Jira issues.

---

## 7. Related documents

- `docs/ui-state-audit-2026-07-20.md` — loading, empty, and error state coverage
- `docs/specs/_active/2026-07-12-desktop-app-audit.md` — desktop app security/integrity audit
- `ui/src/components/Button.tsx` — design-system button
- `ui/src/__tests__/Button.test.tsx` — button component tests

---

*End of plan.*
