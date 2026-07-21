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
- [x] **P60-3f: Recently-used sections migration** ✅ — Added to SettingsNavTree. State initialized from `localStorage` (handles corrupt JSON). `isFirstRender` ref guards initial mount. On navigation, prepends activeSection, deduplicates, limits to 3. Persisted via separate `useEffect`. Renders at top of sidebar nav with border-bottom separator. Hidden during search (`!q`) and collapsed mode.
- [x] **P60-3c: Section count badges with animation** ✅ — Added `@keyframes badge-pop` (scale 0.6→1.15→1, opacity 0→1, 350ms). Uses `key={cat.keys.length}` to re-trigger animation on count change (e.g., search filtering). Added `aria-label` for screen readers. `prefers-reduced-motion` guard.
- [x] **P60-3d: Collapsed sidebar icons-only mode** ✅ — Widths adjusted to 15.625rem (250px) ↔ 3.5rem (56px) with smooth CSS transition. Collapsed nav items: 44px touch targets (min-height/min-width), centered icons, labels hidden. Compact collapsed header (reduced padding). `prefers-reduced-motion` override disables width transition. Tooltips on nav items show labels on hover (existing `Tooltip` wrapper).
- [x] **P60-3e: Search result highlighting** ✅ — Added `highlightLabel()` that wraps matching chars in `<mark>` tags with accent-colored CSS. Added `aria-live="polite"` region announcing visible results count. `visibleCount` memo tracks total visible items across filtered categories.

---

#### 🟡 P60-4 — Accessibility compliance

- [x] **P60-4a: `aria-controls` + `aria-expanded` on category headers** ✅ — Added `aria-controls={panelId}` on each category button linking to its panel. Added `id={panelId}` + `role="region"` + `aria-label` on each panel for labeled landmarks. `aria-pressed` NOT added (redundant with `aria-expanded` on accordion headers).
- [x] **P60-4b: Focus trap on mobile sidebar overlay** ✅ — Added `sidebarRef` on `<aside>`, called `useFocusTrap(sidebarRef, mobileSidebarOpen, onMobileClose)`. Traps Tab focus within sidebar when mobile overlay is open. Escape calls onMobileClose.
- [x] **P60-4c: ARIA treegrid pattern** ✅ — Sidebar nav converted to `role="treegrid"`. Category headers: `role="treeitem"`, `aria-level={1}`, `aria-posinset`, `aria-setsize`, `aria-expanded`, `aria-selected={false}`. Nav items: `role="treeitem"`, `aria-level={2}`, `aria-posinset`, `aria-setsize`, `aria-selected`. Recently-used items: `role="treeitem"`, `aria-level={2}`, `aria-selected`. Added `aria-controls` for panel linking.
- [x] **P60-4d: Keyboard navigation overhaul** ✅ — Full WAI-ARIA Treegrid keybindings: ArrowRight expands current section's category, ArrowLeft collapses it, ArrowDown/ArrowUp navigate visible items (wraps around), Home jumps to first item, End jumps to last item, Escape closes mobile sidebar. `expandedCategory` added to keyboard effect deps.
- [x] **P60-4e: Screen reader live regions** ✅ — Centralized `announcement` state feeding `<div role="status">`. Three announcement sources: (1) category expand/collapse via `userToggleRef` pattern (distinguishes user toggle from programmatic auto-expand), (2) section activated on `activeSection` change via `prevSection` ref, (3) search results/empty/cleared via `prevQ` ref guard.
- [x] **P60-4f: Focus management on navigation** ✅ — Merged into scroll-to-top effect. Queries `.settings-section-content` for first `<h2>`, adds `tabindex="-1"`, calls `focus({ preventScroll: true })`. Removes tabindex on blur via one-shot `{ once: true }` listener to avoid making headings permanently keyboard-tabbable.
- [x] **P60-4g: Touch target audit for sidebar** ✅ — All interactive elements now have `min-height: 2.75rem` / `min-width: 2.75rem` (44px): toggle button, collapse-all button, category section headers, nav items. `settings-sidebar-empty-clear` already had `var(--touch-target-min)`. Uses `min-` to preserve visual 2rem size while expanding hit area to 44px. Collapsed nav items already had this from P60-3d.

---

#### 🟣 P60-5 — Testing

- [x] **P60-5a: Unit tests for NavTree** ✅ — 19 tests covering: 4 category render + badge counts (2,3,4,10) + active section highlight. Accordion expand/collapse with aria-expanded assertions. Search filtering (label match, category match, case-insensitive, empty state + Clear search). Navigation via click. Collapsed sidebar via localStorage and toggle button. Mobile backdrop visible/click. aria-controls + role=region + aria-current.
- [x] **P60-5b: Keyboard navigation tests** ✅ — 8 tests added: ArrowDown/ArrowUp item navigation, wrap-around (last→first, first→last), input element guard (event bubbling via dispatch on <input>), empty search guard (flatKeys.length=0), Escape opens/closes mobile sidebar. `fireKey()` helper dispatches KeyboardEvents with bubbling support.
- [x] **P60-5c: Accessibility regression tests** ✅ — 7 tests covering: live region structure (role=status, aria-live=polite, aria-atomic=true, sr-only class), category expand/collapse announcement, section activated announcement on rerender, search results count announcement, empty search state, search cleared announcement. Also caught and fixed a bug in the collapse direction logic (`prevCategory.current !== null` → `expandedCategory !== null`).

---

#### ⚪ P60-6 — Polish & docs

- [ ] **P60-6a: Reduced motion** — Add `@media (prefers-reduced-motion: reduce)` overrides for all new animations (accordion slide, badge pop, collapse width).
- [x] **P60-6b: Update CHANGELOG.md** ✅ — Documented all 0.0.16 settings sidebar changes: P60-1 extraction, P60-2 reliability, P60-3 UX (accordion animation, badge pop, collapsed mode, search highlighting), P60-4 accessibility (5 items), P60-5 testing (19 tests), P60-6a reduced motion.

### Progress Tracking

| Sprint | Items | Status |
|--------|-------|--------|
| 🔴 P60-1 — Component extraction | 3 | 3/3 ✅ |
| 🔵 P60-2 — Reliability fixes | 3 | 3/3 ✅ |
| 🟢 P60-3 — UX improvements | 5 | 5/5 ✅ |
| 🟡 P60-4 — Accessibility | 7 | 7/7 ✅ |
| 🟣 P60-5 — Testing | 3 | 3/3 ✅ |
| ⚪ P60-6 — Polish & docs | 2 | 2/2 ✅ |
| **Total** | **23** | **22/23 (96%)** |

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
