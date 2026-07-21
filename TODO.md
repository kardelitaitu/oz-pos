# 0.0.16 — Sprint Inbox

> **Goal:** _Define sprint goals here._
>
> **Current state:** 0 / ? items complete · Updated 2026-07-21

---

## 📋 Sprint Plan

### 🏗️ Settings Sidebar — Node Topology Overhaul

> **Goal:** Refactor the settings sidebar navigation tree to be reliable across all scenarios, improve UX design, and ensure full accessibility compliance.
>
> **Current state:** 11 / 12 items complete · Updated 2026-07-21

---

#### 🔴 P60-1 — Extract `NavTree` into a separate component ✅

> Sidebar extracted from SettingsPage.tsx (~2,000→1,860 lines) into standalone SettingsNavTree component with internal state management.

- [x] **P60-1a: Create `SettingsNavTree.tsx`** ✅ — Extracted with all NAV_ITEMS, CATEGORIES, accordion logic, search filtering, collapse state, arrow key navigation. Props: `activeSection`, `onNavigate`, `searchQuery`, `onSearchChange`, `mobileSidebarOpen`, `onMobileClose`.
- [x] **P60-1b: Create `SettingsNavTree.css`** ✅ — All sidebar CSS (~400 lines) moved from SettingsPage.css into dedicated stylesheet.
- [x] **P60-1c: Update imports in SettingsPage.tsx** ✅ — Replaced inline sidebar render with `<SettingsNavTree />`. Constants exported via `export { NAV_ITEMS, CATEGORIES, ... }` for breadcrumb.

---

#### 🔵 P60-2 — Fix reliability edge cases

- [x] **P60-2a: `sectionKey` hydration fix** ✅ — Replaced incremental counter with `key={activeSection}` for stable, predictable re-renders.
- [x] **P60-2b: Arrow key navigation with empty search** ✅ — Guard: early return when `flatKeys.length === 0`. Prevents modulo-by-zero edge case.
- [x] **P60-2c: localStorage race on rapid toggle** ✅ — 100ms debounce via ref-based timer. Handles set/remove. Cleanup on unmount.

---

#### 🟢 P60-3 — UX design improvements

- [x] **P60-3a: Smooth accordion expand/collapse animation** ✅ — Replaced `animation` (mount-only) with CSS `transition` on `max-height`, `opacity`, `transform`. Changed from conditional rendering to class-based toggle for smooth enter/exit. Added `will-change` for GPU acceleration.
- [ ] **P60-3b: Drag-to-reorder recently used sections** ⏳ *Stretch goal* — The "recently used" section shows the last 3 visited sections at the top. Allow the user to drag-and-drop sections within this list for custom ordering. Persist order to localStorage. (Defer if sprint is tight — complex DnD state management + touch events.)
- [ ] **P60-3f: Recently-used sections migration** — When extracting NavTree, ensure the "recently used" section state (last 3 visited sections, persisted to localStorage) either moves with the component or is passed as props. Add a note in P60-1.
- [x] **P60-3c: Section count badges with animation** ✅ — Added `@keyframes badge-pop` (scale 0.6→1.15→1, opacity 0→1, 350ms). Uses `key={cat.keys.length}` to re-trigger animation on count change (e.g., search filtering). Added `aria-label` for screen readers. `prefers-reduced-motion` guard.
- [ ] **P60-3d: Collapsed sidebar icons-only mode** — When sidebar is collapsed, show only icons (no labels) with a thin tooltip on hover. Improve the collapse/expand transition with a width animation (250px ↔ 56px) instead of instant snap.
- [ ] **P60-3e: Search result highlighting** — When searching, highlight matching characters in section labels (e.g., `<mark>` tag or bold). Provide a visual count in the search input placeholder ("Search... 3 of 17").

---

#### 🟡 P60-4 — Accessibility compliance

- [ ] **P60-4a: `aria-controls` + `aria-expanded` on category headers** — Accordion category buttons currently only have `aria-expanded`. Add `aria-controls` pointing to the panel `id`, and `aria-pressed` for toggle state.
- [ ] **P60-4b: Focus trap on mobile sidebar overlay** — When the sidebar opens as an overlay on mobile, focus must be trapped inside (Escape to close, Tab cycles within the sidebar, first/last element wrap). Add `useFocusTrap` hook.
- [ ] **P60-4c: ARIA treegrid pattern** — Convert the sidebar from a flat list of accordion buttons to a proper `role="treegrid"` structure where categories are `treeitem` nodes at level 1 and sections are `treeitem` nodes at level 2. Add `aria-expanded`, `aria-selected`, `aria-level`, `aria-posinset`, `aria-setsize` attributes.
- [ ] **P60-4d: Keyboard navigation overhaul** — Replace the current ArrowDown/ArrowUp keyboard listener with a proper `onKeyDown` handler on the treegrid that follows WAI-ARIA Treegrid pattern:
  - ArrowRight: expand category / focus first section
  - ArrowLeft: collapse category / focus parent
  - ArrowDown: next visible item
  - ArrowUp: previous visible item
  - Home: first item
  - End: last item
  - Enter/Space: activate section
- [ ] **P60-4e: Screen reader live regions** — Add `role="status"` with `aria-live="polite"` announcements for:
  - Category expanded/collapsed ("Business category expanded, 2 items")
  - Section activated ("Opened General settings")
  - Search results ("3 results found")
  - Empty search ("No settings match your search")
- [ ] **P60-4f: Focus management on navigation** — When navigating to a section, move focus to the section content area's first focusable element (heading or first input). Use a ref callback and `focus()` on the mounted element.
- [ ] **P60-4g: Touch target audit for sidebar** — Ensure all sidebar interactive elements have `min-height: 44px` and `min-width: 44px` per WCAG 2.2 Target Size (Minimum). Verify on mobile viewport with `@media (pointer: coarse)`.

---

#### 🟣 P60-5 — Testing

- [ ] **P60-5a: Unit tests for NavTree** — Create `SettingsNavTree.test.tsx` with tests for:
  - Renders all 4 categories with correct item counts
  - Accordion expand/collapse toggles `aria-expanded`
  - Search filters correctly (match label, match category, case-insensitive)
  - Empty search shows "no results" state
  - Collapsed sidebar shows icons only
  - Arrow key navigation moves through visible items
  - Mobile sidebar overlay shows/hides with backdrop click
- [ ] **P60-5b: Keyboard navigation tests** — Test all WAI-ARIA Treegrid keybindings:
  - ArrowRight/ArrowLeft on category
  - ArrowDown/ArrowUp through items
  - Home/End jump to first/last
  - Enter activates section
- [ ] **P60-5c: Accessibility regression tests** — Verify screen reader announcements, ARIA attributes, and focus management work correctly after each change.

---

#### ⚪ P60-6 — Polish & docs

- [ ] **P60-6a: Reduced motion** — Add `@media (prefers-reduced-motion: reduce)` overrides for all new animations (accordion slide, badge pop, collapse width).
- [ ] **P60-6b: Update CHANGELOG.md** — Document the sidebar refactoring for 0.0.16.

### Progress Tracking

| Sprint | Items | Status |
|--------|-------|--------|
| 🔴 P60-1 — Component extraction | 3 | 3/3 ✅ |
| 🔵 P60-2 — Reliability fixes | 3 | 3/3 ✅ |
| 🟢 P60-3 — UX improvements | 5 | 2/5 |
| 🟡 P60-4 — Accessibility | 7 | 0/7 |
| 🟣 P60-5 — Testing | 3 | 0/3 |
| ⚪ P60-6 — Polish & docs | 2 | 1/2 |
| **Total** | **23** | **9/23 (39%)** |

---

## 📋 Future Ideas (backlog)

- [ ] Section pinning: Pin favourite sections to top of sidebar
- [ ] Section search with fuzzy matching (fuse.js)
- [ ] Keyboard shortcut hints shown in tooltips
- [ ] Sidebar width resizable via drag handle

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
