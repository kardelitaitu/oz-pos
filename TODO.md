# 0.0.18 — Production Polish & Gap Closure

> **Goal:** Clean up debug logging, fix edge cases, polish Analytics UIs, finalize mobile builds, and harden the application.
>
> **Current state:** 0 / 16 items complete (0%) · Updated 2026-07-21

---

## 📋 Sprint Plan

| Sprint | Items | Status |
|--------|-------|--------|
| 🔴 P70 — Debug Log Cleanup | 2 | 0/2 ❌ |
| 🔵 P71 — Code Quality & Dead Code | 4 | 0/4 ❌ |
| 🟢 P72 — Analytics UI Polish | 4 | 0/4 ❌ |
| 🟡 P73 — Mobile Build Pipeline | 3 | 0/3 ❌ |
| 🟣 P74 — Edge Cases & Error Handling | 3 | 0/3 ❌ |
| **Total** | **16** | **0/16 (0%)** |

---

### 🔴 P70 — Debug Log Cleanup

> **Goal:** Remove ~50 production console.log debug statements across payment, sale, and inventory flows.

- [ ] **P70-1: PaymentModal.tsx debug log cleanup** — 24 console.log/warn/error calls removed or downgraded to comments. Critical errors (finalize/void failure) kept for debugging, flow-tracing logs removed.
- [ ] **P70-2: Other screens cleanup** — Audit remaining console.log in inventory, auth, and license screens.

---

### 🔵 P71 — Code Quality & Dead Code

> **Goal:** Fix unused imports, dead state references, and CSS inconsistencies.

- [ ] **P71-1: Fix SessionLockScreen runtime error** ✅ — Removed `setPinAttempts(0)` which would throw ReferenceError at runtime (no such state exists). Committed in 55ff5cad.
- [ ] **P71-2: Remove unused `fireEvent` import** ✅ — Fixed in SettingsNavTree.test.tsx. Committed in 862c9924.
- [ ] **P71-3: Unused import audit** — Search for and remove unused imports across all .tsx/.ts files.
- [ ] **P71-4: CSS token violation audit** — Find hardcoded color/spacing values that should be design tokens.

---

### 🟢 P72 — Analytics UI Polish

> **Goal:** Verify BigQuery, Snowflake, Scheduled Report, Custom Report UIs work end-to-end.

- [ ] **P72-1: Verify BigQuery config form renders** — Check `BigQueryConfigForm.tsx` renders with correct labels, validation, and save flow.
- [ ] **P72-2: Verify Snowflake connection settings** — Check `SnowflakeConnectionSettings.tsx` renders correctly.
- [ ] **P72-3: Verify email PDF delivery config** — Check `EmailPdfConfiguration.tsx` + scheduled report list in settings.
- [ ] **P72-4: Verify Custom Report Builder** — Check `CustomReportScreen.tsx` dataset selector, column filtering, CSV export.

---

### 🟡 P73 — Mobile Build Pipeline

> **Goal:** Finalize Android APK + iPad build CI and deployment docs.

- [ ] **P73-1: Android CI verification** — Verify `.github/workflows/android.yml` builds successfully (fix save-already done).
- [ ] **P73-2: iOS CI docs** — Add release build workflow trigger + signing setup docs.
- [ ] **P73-3: Mobile release script** — Create `scripts/build-mobile.sh` for one-tap APK + IPA builds.

---

### 🟣 P74 — Edge Cases & Error Handling

> **Goal:** Fix common runtime edge cases: forms, error boundaries, loading skeletons, empty states.

- [ ] **P74-1: PaymentModal edge cases** — Fix empty tendered input crash, zero-amount edge case, split bill edge case.
- [ ] **P74-2: Form edge cases** — Audit Settings forms for unsaved-changes warning, validation edge cases.
- [ ] **P74-3: Error boundary review** — Ensure ErrorBoundary covers all route-level screens with fallback UI.

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
