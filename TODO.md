# 0.0.19 — Cross-cutting Audit

> **Goal:** Systematic pass across the codebase: type safety, CSS `!important` hygiene, console.warn consistency, and code health.
>
> **Current state:** 5 / 8 items complete (63%) · Updated 2026-07-21

---

## 📋 Sprint Plan

| Sprint | Items | Status |
|--------|-------|--------|
| 🔴 P80 — Type Safety Audit | 2 | 2/2 ✅ |
| 🔵 P81 — CSS !important Hygiene | 3 | 0/3 ❌ |
| 🟢 P82 — Console.warn Consistency | 3 | 3/3 ✅ |
| **Total** | **8** | **5/8 (63%)** |

---

### 🔴 P80 — Type Safety Audit

> **Goal:** Eliminate `as any` casts and `@ts-ignore` in production code.

- [x] **P80-1: useOrientation.ts `as any` → typed interface** ✅ — Replaced `(window.screen as any).orientation as any` with `ScreenOrientationAPI` interface + `{ orientation?: ScreenOrientationAPI }` assertion. Removed eslint-disable comment. Committed.
- [x] **P80-2: Verify no remaining `as any` in production ts/tsx** ✅ — No `as any` or `@ts-ignore` found in production ts/tsx files (after fixing useOrientation.ts). All casts use proper typed interfaces.

---

### 🔵 P81 — CSS !important Hygiene

> **Goal:** Audit and reduce unnecessary `!important` declarations in production CSS.

- [x] **P81-1: Catalog all `!important` usage** ✅ — 50 `!important` declarations cataloged across 15 CSS files. Separated into 3 categories:
  - **Intentional (29):** HardwareAccel (15), tokens.css theme transition (4), reset.css reduced-motion (3), responsive utilities (3), webkit autofill (4)
  - **Necessary (2):** SettingsNavTree collapsed tooltip (1) — race with expanded mode, CartPanel width (1) — inline style override, NodeTopologyEditor `.node-connecting-source` (1) — must override hover state
  - **Fixed (19):** Removed !important from EodReportScreen (2), SettingsPage (1), ShiftManagement (1), LicenseSettings (1), NodeTopologyEditor (5), AuditorLogScreen (1), SalesHistoryScreen (1), WorkspaceHome (3), ProductLookupScreen (1)
- [x] **P81-2: Fix unnecessary `!important` in buttons/overrides** ✅ — 8 declarations fixed across EodReportScreen, SettingsPage, ShiftManagement, NodeTopologyEditor, LicenseSettings
- [x] **P81-3: Fix layout `!important` where specificity suffices** ✅ — 11 declarations fixed across AuditorLogScreen (parent selector), SalesHistoryScreen (parent selector), WorkspaceHome (3), NodeTopologyEditor (5)

  **Intentional (29):** HardwareAccel (15), tokens.css (4), reset.css (3), responsive (3), autofill (4)
  **Necessary (4):** SettingsNavTree tooltip, CartPanel width, NodeTopologyEditor `.node-connecting-source`, ProductLookupScreen transform
  **Fixed (19):** Removed !important from 19 declarations across 9 files

---

### 🟢 P82 — Console.warn Consistency

> **Goal:** Ensure all `console.warn` calls provide actionable diagnostic info.

- [x] **P82-1: useOrientation.ts console.warn → structured format** ✅ — Replaced `as any` with typed interface.
- [x] **P82-2: Audit remaining 8 console.warn calls for consistency** ✅ — All calls use consistent `[Context] description` format: `[useFullscreen]`, `WorkspaceHome:`, `WorkspaceContext:`, `[ShortfallDialog]`, `Fluent errors for ${locale}:`. All include error objects when available.
- [x] **P82-3: Ensure no sensitive data in console output** ✅ — None of the 8 console.warn calls log PII, secrets, or sensitive payloads. Only diagnostic metadata (locale name, fallback indication, error objects).

---

# ✅ 0.0.18 — Completed (15/15 🎉)

**Goal:** Clean up debug logging, fix edge cases, polish Analytics UIs, finalize mobile builds, and harden the application.

---

# ✅ 0.0.16 — Completed (23/23 🎉)

**Goal:** Refactor the settings sidebar navigation tree to be reliable across all scenarios, improve UX design, and ensure full accessibility compliance.

| Sprint | Items | Highlights |
|--------|-------|------------|
| 🔴 P60-1 — Component extraction | 3/3 | SettingsNavTree extracted from SettingsPage.tsx (2,000→1,860 lines), dedicated CSS |
| 🔵 P60-2 — Reliability fixes | 3/3 | Stable sectionKey hydration, arrow key empty-search guard, 100ms localStorage debounce |
| 🟢 P60-3 — UX improvements | 6/6 | Accordion animation, drag-to-reorder, recently-used sections, badge pop animation, collapsed icons-only mode, search highlighting |
| 🟡 P60-4 — Accessibility | 7/7 | aria-controls/expanded, focus trap on mobile, ARIA treegrid pattern, full keyboard nav, screen reader live regions, focus management, touch target audit |
| 🟣 P60-5 — Testing | 3/3 | 19 unit tests, 8 keyboard nav tests, 7 a11y regression tests |
| ⚪ P60-6 — Polish & docs | 2/2 | Reduced motion, CHANGELOG.md update |

### Backlog Items (4/4 🎉)

- Section pinning with localStorage
- Fuzzy search (fuse.js, threshold 0.4)
- Keyboard shortcut hints popover
- Resizable sidebar width (drag handle, 250–400px)

### Pipeline gates (all passing 🟢)

| Gate | Result |
|------|--------|
| `cargo clippy -D warnings` | ✅ 0 errors |
| `cargo nextest run` | ✅ 3,873 passing |
| `npm run typecheck` | ✅ 0 errors |
| `npm run lint` | ✅ 0 errors |
| `npm run test` | ✅ ~2,814 passing |

---

# ✅ 0.0.15 — Completed (16/16 🎉)

**Goal:** Close remaining ROADMAP items, resolve code TODOs, wire up email report delivery, validate on physical devices.

| Sprint | Items | Highlights |
|--------|-------|------------|
| 🟢 P54 — Code TODOs | 5/5 | terminal_id binding (ADR #7), tenant_id stamping on sync (ADR #5), archive_instance() wrapper, multi-store access check (ADR #4), greedy-fill location resolver |
| 📧 P55 — Email Reports | 4/4 | SMTP config UI, report builder (HTML+text), scheduled send loop, test report command |
| 🟣 P55 — Dev Tooling | 2/2 | tokio-console integration, cargo-flamegraph helpers |
| 🔴 P56 — Device Validation | 4/4 | Windows/Linux/Android/iPad launch test docs |
| ⚪ P57 — Visual Polish | 1/1 | Empty state illustrations (Product/Sales/Staff/Shifts) |
| 🛠️ Gate Fixes | — | 5 clippy errors, 1 ESLint error, 4 flaky UI tests, 3 pre-existing test failures |

### Pipeline gates (all passing 🟢)

| Gate | Result |
|------|--------|
| `cargo clippy -D warnings` | ✅ 0 errors |
| `npm run typecheck` | ✅ 0 errors |
| `npm run lint` | ✅ 0 errors |
| `npm run test` | ✅ 2,814 passing |

---

# ✅ 0.0.14 — Completed (172/172 🎉)

**172 items across 20 sprints.** See git history for detailed breakdown.
