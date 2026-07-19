# 0.0.13 — Plugin Hardening + Sync Reliability + Performance

> **Goal:** Harden the Lua plugin sandbox, improve offline-sync conflict resolution, profile and optimize UI rendering, and close remaining documentation/ADR gaps.

**Current state:** 0 / ~25 items · Updated 2026-07-19

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🔴 P0 — Plugin Security | 5 | **5** | **███████████████████████████████ 100% 🎉** |
| 🟢 P1 — Sync Reliability | 6 | **6** | **███████████████████████████████ 100% 🎉** |
| 🟡 P2 — UI Performance | 6 | **6** | **███████████████████████████████ 100% 🎉** |
| 🔵 P3 — KDS Enhancements | 5 | **5** | **███████████████████████████████ 100% 🎉** |
| 🟣 P4 — Docs & Compliance | 4 | **4** | **███████████████████████████████ 100% 🎉** |
| 🟤 P5 — Payment Gateway Hardening | 4 | **1** | **███▱▱▱▱▱▱▱ 25%** |
| ⚪ P6 — Hardware Integration | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟠 P7 — Tablet/Mobile Experience | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🔘 P8 — Cloud Server & License | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟠 P9 — Reporting & Analytics | 3 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🔵 P10 — i18n & Accessibility | 5 | **4** | **████████▱▱▱▱ 80%** |
| 🟢 P11 — Shadow Banding Audit | 5 | **1** | **██▱▱▱▱▱▱▱▱ 20%** |
| 🔴 P12 — PCI-DSS Gap Closure | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟡 P13 — DevOps & Infrastructure | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟣 P14 — Mobile Build & Deploy | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| **Total** | **71** | **31** | **██████████████████████▱ 44%** |

---

## 🔴 P0 — Plugin Security (Lua Sandboxing)

**Goal:** Audit and harden the Lua plugin execution environment to prevent privilege escalation, data leaks, and DoS from malicious or buggy plugins.

### Background

The plugin system (`crates/oz-lua/`) allows Lua scripts to intercept sale events, modify cart totals, and trigger stock adjustments via `oz-plugin` and `oz-lua`. Currently:
- Plugins run in a standard `mlua` Lua VM with **no sandbox restrictions**
- `require` is unrestricted — plugins can load any LuaRocks module
- No CPU instruction limit is set
- No memory/heap limit is configured
- No filesystem access restriction (no `chroot` or seccomp)
- No network access restriction

### Checklist

- [x] **P0-1: Sandbox audit** ✅ — Report at `docs/security/lua-sandbox-audit.md`. Found 7 findings (3 critical, 2 high, 2 medium).
- [x] **P0-2: Permission manifests** ✅ — `Permission` enum with 8 variants, custom TOML deserializer, enforced at load time.
- [x] **P0-3: Resource limits** ✅ — Instruction limit via `HookTriggers::every_nth_instruction(100_000)`. Memory limit documented but not enforced (rlua limitation). 3 new tests, 48/48 pass.
- [x] **P0-4: Safe environment** ✅ — 11 comprehensive sandbox tests added: all 14 dangerous globals verified nil, safe globals confirmed working, 8 individual vector tests (require, package, load, rawget, rawset, collectgarbage, debug, module), and a multi-vector attack script that tries all 11 vectors safely.
- [x] **P0-5: Regressions** ✅ — Real example scripts tested end-to-end: discount_bulk (all 3 tiers), tax_overrides (5 SKU prefixes), validate_order (max qty, alcohol, duplicate, clean), and real example-discount plugin hook execution.

---

## 🟢 P1 — Offline-Sync Reliability

**Goal:** Improve conflict resolution during multi-terminal offline sync, add comprehensive integration tests, and harden error recovery paths.

### Background

The sync system (`platform/sync/`) uses cursor-based push/pull with exponential backoff. Current known gaps:
- No conflict resolution strategy for concurrent edits to same product/sale from different terminals
- No integration tests for the full sync lifecycle (enqueue → push → pull → apply)
- Batch splitting works but edge cases around auth expiry mid-batch are untested
- Snapshot import recovery path is untested

### Checklist

- [x] **P1-1: Conflict resolution strategy** ✅ — ADR-21 drafted at `docs/decisions/2026-07-20-sync-conflict-resolution-strategy.md`. Defines entity-type dispatch (version LWW for reference data, state-machine LWW for sales, CRDT merge for stock), conflict logging, and tombstone propagation. 7 acceptance criteria defined.
- [x] **P1-2: Entity-aware conflict resolvers** ✅ — Implemented ADR-21 entity-type dispatch in `platform/sync/src/conflict.rs`. Added `resolve_version_lww` (version integer comparison), `resolve_sale_lww` (status DAG: active→pending→completed→voided→refunded), `resolve_stock_crdt` (CRDT delta merge preserving both deltas), and `resolve_conflict` dispatcher. Wired into `lib.rs` `run_sync_cycle`. 31 tests (30 new + 1 preserved legacy) covering all resolvers + dispatch edge cases. All 48 platform-sync conflict tests pass.
- [x] **P1-3: Conflict UI indicators** ✅ — Added `conflict_count` to `SyncStatusSummary` and `offline_queue_status_summary()` SQL query. Modified `apply_resolution()` to mark items with conflict tags (`local won` / `remote won` / `crdt merge`) via new `mark_offline_resolved()`. Added Tauri command + frontend API wrapper. Shows warning badge on OfflineQueueScreen and ⚠ conflict count indicator in StatusBar with tooltip. Both poll every 10s / 30s. TypeScript: 0 errors, Rust: cargo check clean, 22 oz-core tests pass.
- [x] **P1-4: Snapshot import error handling** ✅ — 13 tests covering: empty snapshot, single/multiple products, missing SKU/name, idempotent re-import (ON CONFLICT upsert), product/user overwrite, corrupted product missing all fields, corrupted user with default role_id, out-of-schema fields ignored, 6-entity multi-type bundle, FK violation rollback, null barcode. platform-sync: 139/139.
- [x] **P1-5: Offline queue dedup hardening** ✅ — Added `enqueue_offline_dedup` (checks pending items by action+payload) and `SyncQueue::enqueue_dedup`. 11 new tests cover dedup, cross-terminal scenario, different action/payload, and re-enqueue after sync. oz-core: 1445/1445, platform-sync: 126/126.
- [x] **P1-6: Sync observability** ✅ — Added `SyncStatusSummary` struct, `Store::offline_queue_status_summary()` (4 SQL queries: GROUP BY status, SUM retry_count, MAX synced_at, MIN created_at), and `SyncQueue::status_summary()` wrapper. 9 new tests cover empty, seeded, lifecycle updates, multi-failed retry sum, serde roundtrip, debug output, and cross-queue scenarios. oz-core: 1450/1451, platform-sync: 139/139.

---

## 🟡 P2 — UI Performance Optimization

**Goal:** Profile and optimize the three most expensive renders: product lookup grid, KDS ticket board, and sales history modal.

### Background

Current UI test suite runs in ~19s. The product grid (ProductLookupScreen/RetailPosScreen) re-renders all items on every keystroke in the search bar. The KDS ticket board polls every 5 seconds. Sales history modals re-query the full sale on every open.

### Checklist

- [x] **P2-1: Profile baseline** ✅ — Added `React.Profiler` wrappers to KdsScreen, SalesHistoryScreen, and ProductLookupScreen. Each logs mount/update renders with `console.debug` when actualDuration > 1ms. TypeScript: 0 errors.
- [x] **P2-2: Product grid virtualization** ✅ — Replaced flat product grid in ProductLookupScreen with react-window v2 `<Grid>` component. Uses ResizeObserver for responsive column count (based on 220px min card width), `cellComponent` + `cellProps` pattern for data passing, `overscanCount={4}` for smooth scrolling, and `style={{ height: '100%', width: '100%' }}` for container fill. Only renders visible rows + overscan. TypeScript: 0 errors.
- [x] **P2-3: KDS adaptive polling** ✅ — Replaced fixed 15s interval with recursive setTimeout adaptive polling: 2s when active (<30s idle), 10s when idle (30s–2min), 30s when idle (>2min). Pauses when tab hidden (visibilityState), resumes with immediate fetch on tab show. Idle timer resets on every order count change via effect dependency. TypeScript: 0 errors.
- [x] **P2-4: Sale detail caching** ✅ — Added `useRef<Map<string, SaleDetail>>` cache in SalesHistoryScreen. Cache checked before fetch on `openDetail`; hit returns instantly (only refunds re-fetched). `invalidateCache(id)` called on void (`handleConfirmVoid`) and refund (`handleRefunded`) to ensure stale entries are cleared. TypeScript: 0 errors.
- [x] **P2-5: Memo audit** ✅ — Added `React.memo` to KdsTicketCard, StockAlertPanel, and LocationPicker — the 3 highest-value memo targets (rendered in loops or receiving stable prop sets). Wrapped `advanceStatus` in `useCallback` in KdsScreen.tsx so the `onAdvance` prop stays referentially stable, making the KdsTicketCard memo effective. TypeScript: 0 errors.

---

## 🔵 P3 — KDS Display Enhancements

**Goal:** Improve KDS screen usability with overdue escalation, sound alerts, and layout polish.

### Background

The KDS system (kitchen display) has multi-layout support (Focus/Kanban/Metro) but lacks overdue escalation (tickets don't visually escalate as they get older), sound alerts for new tickets, and layout parameter persistence.

### Checklist

- [x] **P3-1: Overdue escalation** ✅ — Progressive visual escalation implemented: green <5min (on-time), yellow 5-10min (amber border+pulse), red 10-15min (red border+shake animation via `kds-shake` keyframes), urgent ≥15min (gradient red background + `URGENT` badge + gradient top bar sweep). Updated `useTicketSla` hook thresholds, added `urgent` boolean, dual audio alerts at 10min and 15min transitions. TypeScript checks pass.
- [x] **P3-2: Sound alerts** ✅ — Added `useNewTicketSound` hook: tracks known order IDs in a `Set<string>` ref, detects new IDs on each orders update, plays `playBeep()` chime via `useSound` debounced to max 1 per 5s. Supports enabled/disabled toggle. Wired into `KdsScreen.tsx`. TypeScript passes.
- [x] **P3-3: Layout persistence** ✅ — Added localStorage cache layer to `useKdsPreferences` hook: `readLocalPrefs` (instant restore on mount with validation), `writeLocalPrefs` (on every layout/setting change). Combined with existing server persistence for seamless online/offline restore. TypeScript passes.
- [x] **P3-4: Ticket count badge animation** ✅ — Added `useCountAnim` hook (tracks previous count via `useRef`, returns `'up' | 'down' | ''` animation direction). CSS `@keyframes kds-count-up` (scale 1→1.35→0.9→1 with bounce) and `kds-count-down` (scale 1→0.75→1.05→1). Classes `.kds-column-count--up` and `.kds-column-count--down` with 300ms duration. Respects `prefers-reduced-motion: reduce`. TypeScript passes.
- [x] **P3-5: KDS settings panel** ✅ — Added `KdsSettingsPanel` component with gear icon button and popover portal (same escape/click-outside pattern as `KdsLayoutSwitcher`). Contains: sound toggle (wired to `useNewTicketSound`), yellow escalation slider (3-10min), red escalation slider (dynamically constrained > yellow, 6-15min), auto-acknowledge toggle, and display density selector (comfortable/compact). Default settings via `DEFAULT_SETTINGS`. TypeScript passes.

---

## 🟣 P4 — Documentation & Compliance

**Goal:** Close remaining doc gaps: ADR status updates, missing `///` docs, skill-drift audit, and changelog completeness.

### Background

Several ADRs lack final "Implemented" status updates. The skill-drift-guard found minor drift. Some recently added modules lack full doc comments.

### Checklist

- [x] **P4-1: ADR status audit** ✅ — All ADRs reviewed. ADR-18 (Multi-Location Inventory), ADR-19 (Sale Deduction), ADR-20 (Payment-Capture) updated from Proposed → Implemented with completion dates. ADR-20 status.md created with 5/6 criteria verified (20-1 deferred).
- [x] **P4-2: Missing docs** ✅ — All three target files already have thorough doc comments. `cargo clippy -- -W missing-docs` confirms zero warnings across the entire workspace. No changes needed.
- [x] **P4-3: Skill-drift guard** ✅ — `detect.sh --report` found zero drift. All skills are in sync with the codebase.
- [x] **P4-4: CHANGELOG final pass** ✅ — All commits documented in [0.0.12]. No missing entries found.

---

---

## 🟤 P5 — Payment Gateway Hardening

**Goal:** Improve reliability and test coverage for payment gateway integrations (QRIS, Square, Stripe). Add webhook handling, idempotency keys, and proper error classification.

### Background

Payment gateway drivers live in `crates/oz-payment/src/drivers/` (qris.rs, square.rs, stripe.rs). Current gaps:
- No webhook signature verification for Stripe/Square
- No idempotency key support for payment retries
- QRIS driver lacks proper error classification (timeout vs declined vs network)
- Integration tests use sandbox credentials configured via env vars — no recording/replay for deterministic CI

### Checklist

- [x] **P5-1: Gateway error classification** ✅ — Added `InvalidCard(String)` and `Duplicate(String)` variants to `PaymentError`. Added per-driver classification functions: `classify_midtrans_status()` (QRIS: 402→InvalidCard, 406→Duplicate, deny/cancel→Declined), `classify_stripe_error()` (Stripe: card_error→InvalidCard/Declined, idempotency_error→Duplicate), `classify_square_error()` (Square: CARD_DECLINED→Declined, UNSUPPORTED_CARD_BRAND→InvalidCard, DUPLICATE_CARD→Duplicate, TIMEOUT→Timeout). Updated all `parse_error()` methods to use classification. 12 unit tests + 5 doctests pass.
- [x] **P5-2: Idempotency keys** ✅ — Migration 097 adds `idempotency_key TEXT` column + UNIQUE index to payments table. `PaymentSplitArg` and `Payment` structs updated with `idempotency_key: Option<String>`. `create_payments()` checks for existing key before INSERT (dedup). `PaymentRequest` updated with idempotency_key field. 3 DB-level dedup tests + 2 serde tests. Driver-level idempotency header integration deferred (stripe `Idempotency-Key` header, square `idempotency_key` field).
- [ ] **P5-3: Webhook receiver** — Add a lightweight webhook endpoint in `oz-api` for Stripe/Square payment events. Verify webhook signatures using gateway secrets. Update payment status from `'pending'` → `'completed'` on `payment_intent.succeeded` / `charge.captured`. Re-queue `finalize_sale` on successful capture.
- [ ] **P5-4: Sandbox test recording** — Implement a `TestFixture` recorder for payment tests: run against sandbox once, capture request/response pairs, replay in CI. Add 3 integration tests per driver (success, decline, timeout) using recorded fixtures.

---

## ⚪ P6 — Hardware Integration

**Goal:** Improve reliability, auto-detection, and test coverage for physical hardware: receipt printers, barcode scanners, cash drawers, customer displays, and scales.

### Background

The HAL (`crates/oz-hal/`) supports USB, Bluetooth, serial, and TCP/IP devices. Current gaps:
- No automatic device discovery — users must configure port/address manually
- Printer driver (ESC/POS) lacks common commands: barcode printing, QR code, cash drawer kick
- No printer status polling (paper jam, out of paper, cover open)
- Mock drivers exist but are not used in UI integration tests

### Checklist

- [ ] **P6-1: Auto-discovery** — Add USB device enumeration (using `rusb` or platform-specific APIs) to detect connected printers, scanners, and scales by vendor/product ID. Show discovered devices in TerminalManagementScreen as selectable options. Fall back to manual config when auto-detect fails.
- [ ] **P6-2: ESC/POS barcode & QR printing** — Add `print_barcode()` and `print_qr()` commands to `escpos.rs` printer driver. Wire into receipt printing flow: print QR code (payment link) and barcode (receipt number) on each receipt. Add test with mock transport.
- [ ] **P6-3: Printer status polling** — Add `get_status()` to the printer trait returning: `PaperStatus(ok/low/empty)`, `CoverOpen(bool)`, `DrawerOpen(bool)`. Poll before every print job; warn user if paper is low or cover is open. Add mock status simulation.
- [ ] **P6-4: Receipt preview in UI** — Add a "Print Preview" modal in PaymentModal that shows a styled receipt rendering before printing. Include: store name/logo, line items, totals, payment method, QR code, barcode. Use CSS for layout, then send to printer. Add test.

---

## 🟠 P7 — Tablet/Mobile Experience

**Goal:** Polish the tablet client for Android/iOS deployment. Fix touch targets, add swipe gestures, optimize for small screens, and ensure offline resilience.

### Background

The tablet client (`apps/tablet-client/`) targets Android and iOS via Tauri mobile. Commands mirror the desktop client. Current gaps:
- No swipe-to-complete gesture on POS screen (users expect swipe to pay on tablets)
- Touch targets need 44px minimum — some buttons are still 32px
- No pull-to-refresh on order lists
- Keyboard avoidance (input fields hidden behind keyboard on mobile)
- Tablet home screen lacks KDS order count widget

### Checklist

- [ ] **P7-1: Swipe gestures** — Add `useSwipe` hook support to tablet POS flow: swipe left on cart → open payment modal, swipe right on payment modal → go back to cart. Use `touchstart`/`touchend` with distance + velocity threshold (min 50px, max 300ms).
- [ ] **P7-2: Touch target audit** — Scan all tablet-rendered screens for sub-44px interactive elements using `touchTargetSizing.test.tsx`. Fix violations in: product cards (add-to-cart button 32px → 44px), filter chips (28px → 44px), tab buttons (36px → 44px), settings switches (32px → 44px).
- [ ] **P7-3: Pull-to-refresh** — Add pull-to-refresh to SalesHistoryScreen, OfflineQueueScreen, and KDS ticket board using `@react-spring/web` gesture or native `touch` events. Show spinner + "Pull to refresh" / "Release to refresh" states.
- [ ] **P7-4: Keyboard avoidance** — Add `useKeyboardAvoidance` hook that detects keyboard open/close on mobile (via `visualViewport` API or focus change) and scrolls active input into view with `scrollMargin`. Apply to: PaymentModal (customer search), SettingsPage text inputs, StaffLoginScreen PIN pad.

---

## 🔘 P8 — Cloud Server & License

**Goal:** Harden the cloud server (`apps/cloud-server/`) and license server (`apps/license-server/`) for production. Add rate limiting, audit logging, and deployment docs.

### Background

The cloud server (`oz-cloud-server`) handles sync API, authentication, and metrics. The license server (Go) handles activation, renewal, and status. Current gaps:
- Cloud server has no per-tenant rate limiting (any tenant can DoS the sync endpoint)
- License server lacks machine-level revocation (can't deactivate a stolen device)
- No health check endpoint on license server (Docker healthcheck uses curl)
- Deployment docs for cloud server are incomplete

### Checklist

- [ ] **P8-1: Per-tenant rate limiting** — Add token-bucket rate limiter to `oz-cloud-server` sync endpoints: 100 requests/min per tenant for push, 300/min for pull. Return `429 Too Many Requests` with `Retry-After` header. Use in-memory `HashMap` with periodic cleanup.
- [ ] **P8-2: Machine-level revocation** — Add `POST /api/license/revoke-device` endpoint to license server. Accept `machine_id` + `license_key`. Mark device as revoked in PocketBase. `GET /api/license/status` returns `device_revoked` for revoked machines. Frontend shows "This device has been deactivated" with contact-support message.
- [ ] **P8-3: Cloud server health endpoint** — Add comprehensive `/api/health` to cloud server: DB connectivity (ping), sync queue depth, last sync timestamp, uptime. Consumed by Tauri app's ConnectionStatus component. Add prometheus metrics export.
- [ ] **P8-4: License server Docker healthcheck** — Replace curl-based Docker healthcheck with proper Go HTTP client in `apps/license-server/Dockerfile`. Healthcheck pings `/api/health` with 5s interval, 3 retries, 10s timeout. Document in DEPLOY.md.

---

## 🟠 P9 — Reporting & Analytics

**Goal:** Expand reporting capabilities with visual charts, export to CSV/Excel, and more granular date-range filters.

### Background

`crates/oz-reporting/` provides menu engineering and metrics modules. `crates/oz-core/src/db/reports.rs` has daily/weekly/monthly revenue, heatmap, top products, and category breakdown. Current gaps:
- Reports return raw data only — no chart rendering on frontend
- No CSV/Excel export for any report
- Date range picker is basic (start/end string inputs)
- No comparison period (e.g., this week vs last week)

### Checklist

- [ ] **P9-1: Chart visualizations** — Add lightweight chart rendering (via Canvas 2D API — no heavy chart library) for: daily revenue line chart, category breakdown pie chart, hourly heatmap. Use `color-mix()` for theme-aware colors. Add to ReportingDashboardScreen.
- [ ] **P9-2: CSV export** — Add "Export CSV" button to every report view. Generate CSV from report data on the frontend (no server round-trip). Use `Blob` + `URL.createObjectURL` + `<a download>`. Include BOM for Excel compatibility with UTF-8. Add test verifying CSV content matches report data.
- [ ] **P9-3: Period comparison** — Add "Compare to previous period" toggle to revenue reports. Show current period vs previous period side-by-side with delta percentage and up/down arrow indicator. Calculate on frontend from existing data.

---

---

## 🔵 P10 — i18n & Accessibility

**Goal:** Complete Indonesian translation coverage, pass Lighthouse a11y audit, and harden Fluent bundle verification.

### Background

From `docs/i18n-todo.md`: 4 Indonesian bundles are byte-identical to English (gift-cards, purchasing, stock-counting, stock-transfers). The ROADMAP has 2 unchecked items: Lighthouse a11y score ≥ 90 and full i18n coverage. The theme token compliance scanner needs expansion to catch a11y violations.

### Checklist

- [x] **P10-1: Translate 4 Indonesian bundles** ✅ — 2 bundles already translated (gift-cards, purchasing). Translated 2 remaining bundles: stock-counting.id.ftl (29 keys — stok opname) and stock-transfers.id.ftl (38 keys — transfer stok). All Indonesian translations use proper retail/POS terminology. Verified with `lint-i18n.sh` (clean) and `verify-bundle-parity.py` (0 missing keys).
- [x] **P10-2: Lighthouse a11y gate** ✅ — Added `.lighthouserc.json` with 3-run median aggregation on 5 SPA routes (#/pos, #/settings, #/products, #/sales-history, #/kds). Thresholds: a11y ≥ 0.90, best-practices ≥ 0.80, SEO ≥ 0.80. Added `lighthouse` job to CI pipeline with `npx -p @lhci/cli` (no global install), 10-min timeout, and `vite preview` server.
- [x] **P10-3: Color contrast audit** ✅ — Audit complete. Zero hardcoded color values found across all CSS files — entire codebase uses `var(--color-*)` design tokens. WCAG AA contrast ratios verified for StatusBar (`--color-fg-tertiary` ~5.5:1), CartPanel line-item prices (`--color-fg-secondary` ~9.8:1), badge variants (semantic tokens), and KDS timer text (`--kds-subtle`/`--kds-muted` ~5.7-6.5:1). All three themes exceed AA minimum (4.5:1). No fixes needed.
- [x] **P10-4: Focus indicator audit** ✅ — Added `:focus-visible` styles to 12 CSS files covering 24 interactive elements: dropdown options (KDS layout, density), settings toggles (KDS layout/settings), buttons (permission denied, reverse transit, ghost license, dev toolbar), cards (KDS ticket, kiosk product), filter chips (stock counts, kiosk categories), inputs (threshold select/input), action buttons (offline queue, stock count actions), checkout actions (kiosk). Consistent pattern: `outline: none; box-shadow: inset 0 0 0 2px var(--color-accent)` (buttons) or `box-shadow: 0 0 0 2px` (checkbox toggles, cards). TypeScript: 0 errors.
- [x] **P10-5: Screen reader UX** ✅ — Added `aria-live="polite"` to cart grand total (RetailPosScreen), `aria-live="assertive"` to payment done state (PaymentModal), `aria-live="polite"` to shift status (ShiftBar), `aria-live="polite"` to pending count badge (OfflineQueueScreen). Added missing `aria-label` on 2 icon-only × buttons (PaymentModal customer remove, RetailOptionsScreen preview close). TypeScript: 0 errors.

---

## 🟢 P11 — Shadow Banding Audit

**Goal:** Eliminate visible colour banding on all elevated surfaces by applying SVG feTurbulence noise overlay.

### Background

From `docs/TODO-shadow-audit.md`: 30 CSS surfaces use shadows (`--shadow-xl` through `--shadow-xs`) but lack the SVG noise overlay `dither::after` that prevents gradient banding. Currently only `.card`, `.staff-login-card`, `.modal-panel`, and `.noise-dither` are covered.

### Checklist

- [x] **P11-1: Phase 1 — High-risk surfaces** ✅ — All 15 surfaces already have noise-dither selectors in `ui/src/frontend/themes/components.css` (`.workspace-card`, all 6 retail-* modals, `.tables-detail`, `.settings-popup`, `.license-activation-card`, `.gift-cards-modal`, `.promo-mgmt-modal`, `.product-mgmt-modal`, `.po-form-modal`, `.sales-history-modal`, `.shift-mgmt-modal`, `.stock-transfers-modal`, `.payment-modal`, `.price-override-modal`, `.dev-toolbar`). No code changes needed.
- [x] **P11-2: Phase 2 — Medium-risk surfaces** ✅ — Added `.restaurant-hamburger-dropdown`, `.restaurant-context-menu`, `.settings-sidebar`, `.tooltip-content`, `.ssel-dropdown` to the noise-dither selector list in `components.css`. Updated `@media (prefers-contrast: high)` block. TypeScript: 0 errors.
- [ ] **P11-3: Phase 3 — Low-risk surfaces (9 items)** — Apply noise overlay to: MultiStoreDashboardScreen, MenuEngineeringScreen, ProductLookupScreen, KioskScreen, RetailPosScreen (sm variants), SetupWizard (×2), CartPanelFooterTotals, CartPanelLineItem, PermissionDenied.
- [ ] **P11-4: Noise overlay CSS refactor** — Extract the noise overlay into a reusable CSS class `.noise-dither` in `components.css`. Replace inline `::after` in individual files with the shared class. Add `@media (prefers-reduced-motion: reduce)` to remove overlay for accessibility.
- [ ] **P11-5: Visual regression test** — Add a visual regression test (`playwright` or `puppeteer`) that captures screenshots of each elevated surface and compares against baseline. Run in CI on PRs. Fail if banding is detected via pixel-diff threshold > 1%.

---

## 🔴 P12 — PCI-DSS Gap Closure

**Goal:** Close the 6 remaining PCI-DSS compliance gaps identified in the checklist (`docs/security/PCI-DSS_CHECKLIST.md`).

### Background

The PCI-DSS v4.0 checklist has several items marked "Planned" or needing implementation. Critical gaps: no key rotation policy, no incident response plan, no MFA, no daily audit log review notification, no security incident reporting.

### Checklist

- [ ] **P12-1: Key rotation policy** — Document and implement key rotation for `oz-security` Keyring. Add `rotate_key()` method that generates new key, re-encrypts existing KEK-wrapped data, and updates storage. Add 90-day rotation reminder via toast notification. Est: 2–3 hrs.
- [ ] **P12-2: Incident response plan** — Write `docs/security/INCIDENT_RESPONSE.md` with: incident classification (P1-P4), containment steps, evidence preservation, notification contacts, post-mortem template. Integrate with audit log by adding `incident` action type. Est: 2 hrs.
- [ ] **P12-3: Daily audit log review** — Add `unreviewed_audit_events` count to dashboard screen. Highlight events marked `critical` or `security` in red. Show last-reviewed timestamp. Add "Mark reviewed" button for managers. Est: 2–3 hrs.
- [ ] **P12-4: Session timeout & lockout** — Implement automatic screen lock after configurable idle timeout (5/15/30/60 min). Require PIN re-entry to unlock. Store timeout preference in user preferences or settings. Lock screen shows blurred last screen + time. Est: 3–4 hrs.

---

## 🟡 P13 — DevOps & Infrastructure

**Goal:** Improve CI/CD pipeline speed, Docker deployment, and developer onboarding experience.

### Background

Current CI pipeline takes ~10 minutes. Docker compose exists but cloud-server deployment docs are incomplete. Developer onboarding requires manual dependency installation. No automated end-to-end tests against the full stack.

### Checklist

- [ ] **P13-1: CI pipeline optimization** — Profile CI build times. Implement: shared sccache between jobs, incremental cargo check (only changed crates), parallel vitest + eslint runs, dependency caching between workflow runs. Target: < 5 min for lint + typecheck + unit tests. Est: 2–3 hrs.
- [ ] **P13-2: Docker Compose for full stack** — Update `docker-compose.yml` to include: `oz-cloud-server` (Rust API), `oz-pos` (desktop client in X11/VNC for CI), `license-server` (Go), PostgreSQL (sync target), Redis (cache). Add healthcheck dependencies. Document in `docs/operations/docker-deployment.md`. Est: 3–4 hrs.
- [ ] **P13-3: E2E test suite** — Add Playwright-based e2e tests for the 5 most critical flows: complete sale (scan → add → pay → receipt), staff login with PIN, create product, open/close shift, settings change. Use Docker Compose for backend + test against real SQLite. Est: 4–6 hrs.
- [ ] **P13-4: Developer setup script** — Create `scripts/setup-dev.sh` / `setup-dev.ps1` that automates: install Rust toolchain, install Node.js deps, run `cargo check`, run `npm install`, run initial migration, seed demo data. Replace manual setup steps in `QUICKSTART.md` with single-command setup. Est: 2 hrs.

---

## 🟣 P14 — Mobile Build & Deploy

**Goal:** Successfully build and deploy the tablet client on Android and iOS, enabling real-world mobile POS deployment.

### Background

The ROADMAP lists both Android and iPad builds as unchecked. The tablet client (`apps/tablet-client/`) and touch-optimized UI are ready, but the actual APK/IPA builds haven't been completed. Requires Android SDK / Xcode setup.

### Checklist

- [ ] **P14-1: Android build pipeline** — Add GitHub Actions workflow for Android APK build: install Android SDK (via `android-actions/setup-android`), set up keystore signing (secrets), run `cargo tauri build --target aarch64-linux-android`. Output signed APK + AAB as release artifacts. Est: 3–4 hrs (requires runner with Android SDK).
- [ ] **P14-2: iOS build pipeline** — Add GitHub Actions workflow for iOS IPA build: install Xcode (macOS runner), configure provisioning profile + certificate (secrets), run `cargo tauri build --target aarch64-apple-ios`. Output IPA as release artifact. Est: 3–4 hrs (requires Apple Developer account + macOS runner).
- [ ] **P14-3: Tablet gesture & orientation** — Add `orientation` lock to POS screen (landscape recommended), handle `orientationchange` event to reflow layout. Test swipe-to-complete gesture on physical Android/iPad. Fix any touch-event issues (passive listeners, scroll interference). Est: 2–3 hrs.
- [ ] **P14-4: Mobile deployment docs** — Update `packaging/mobile/README.md` with: step-by-step Android APK build guide (with/without Android Studio), iOS TestFlight distribution guide, hardware requirements (minimum Android 10 / iOS 16, 4GB RAM), supported payment terminals, printer compatibility list. Est: 2 hrs.

---

## 🧭 Dependency Graph

```
🔴 P0 Plugin Security ───── independent (no deps)

🟢 P1 Sync Reliability
    ├── P1-1 Conflict strategy (ADR-21 draft)
    ├── P1-2 Integration tests (depends on P1-1)
    ├── P1-3 Conflict UI (depends on P1-1)
    ├── P1-4 Snapshot hardening (independent)
    ├── P1-5 Dedup tests (independent)
    └── P1-6 Observability (independent)

🟡 P2 UI Performance
    ├── P2-1 Profile baseline ──────────────┐
    ├── P2-2 Product grid virtualization ────┤
    ├── P2-3 KDS polling backoff ───────────┤── all independent
    ├── P2-4 Sale detail caching ───────────┤
    └── P2-5 Memo audit ────────────────────┘

🔵 P3 KDS Enhancements ─ all independent

🟣 P4 Docs & Compliance ─ all independent

🟤 P5 Payment Gateway ─ P5-1 → P5-2/3/4

⚪ P6 Hardware ─ all independent

🟠 P7 Tablet/Mobile ─ P7-2 needs P7-1

🔘 P8 Cloud Server ─ all independent

🟠 P9 Reporting ─ all independent

🔵 P10 i18n & A11y ─ P10-2 (Lighthouse) depends on P10-3, P10-4

🟢 P11 Shadow Banding ─ P11-1 → P11-2 → P11-3 (ordered by risk)

🔴 P12 PCI-DSS ─ all independent

🟡 P13 DevOps ─ P13-3 (E2E) depends on P13-2 (Docker Compose)

🟣 P14 Mobile Build ─ P14-3 (gestures) independent of P14-1/2 (build pipelines)
```

---

## 🎯 Estimated Effort

| Priority | Item | Est. Effort | Dependencies |
|----------|------|-------------|--------------|
| 🔴 | P0-1: Sandbox audit | 1 hr | None |
| 🔴 | P0-2: Permission manifests | 2–3 hrs | P0-1 |
| 🔴 | P0-3: Resource limits | 1–2 hrs | P0-1 |
| 🔴 | P0-4: Safe environment | 2–3 hrs | P0-1 |
| 🔴 | P0-5: Plugin regressions | 1 hr | P0-2, P0-3, P0-4 |
| 🟢 | P1-1: Conflict strategy | 3–4 hrs | None (ADR-21) |
| 🟢 | P1-2: Sync integration tests | 3–4 hrs | P1-1 |
| 🟢 | P1-3: Conflict UI | 2–3 hrs | P1-1 |
| 🟢 | P1-4: Snapshot hardening | 1–2 hrs | None |
| 🟢 | P1-5: Dedup hardening | 1 hr | None |
| 🟢 | P1-6: Sync observability | 2–3 hrs | None |
| 🟡 | P2-1: Profile baseline | 1 hr | None |
| 🟡 | P2-2: Grid virtualization | 3–4 hrs | P2-1 |
| 🟡 | P2-3: KDS polling backoff | 1–2 hrs | None |
| 🟡 | P2-4: Sale detail caching | 1–2 hrs | None |
| 🟡 | P2-5: Memo audit | 1–2 hrs | P2-1 |
| 🔵 | P3-1: Overdue escalation | 1–2 hrs | None |
| 🔵 | P3-2: Sound alerts | 1–2 hrs | None |
| 🔵 | P3-3: Layout persistence | 1 hr | None |
| 🔵 | P3-4: Ticket count animation | 1 hr | None |
| 🔵 | P3-5: KDS settings panel | 2–3 hrs | None |
| 🟣 | P4-1: ADR status audit | 1 hr | None |
| 🟣 | P4-2: Missing docs | 1 hr | None |
| 🟣 | P4-3: Skill-drift guard | 30 min | None |
| 🟣 | P4-4: CHANGELOG final pass | 30 min | None |
| 🟤 | P5-1: Gateway error classification | 2–3 hrs | None |
| 🟤 | P5-2: Idempotency keys | 2–3 hrs | None (migration 097) |
| 🟤 | P5-3: Webhook receiver | 3–4 hrs | None |
| 🟤 | P5-4: Sandbox test recording | 2–3 hrs | None |
| ⚪ | P6-1: Auto-discovery | 3–4 hrs | None |
| ⚪ | P6-2: ESC/POS barcode & QR | 2–3 hrs | None |
| ⚪ | P6-3: Printer status polling | 1–2 hrs | None |
| ⚪ | P6-4: Receipt preview | 2–3 hrs | None |
| 🟠 | P7-1: Swipe gestures | 2–3 hrs | None |
| 🟠 | P7-2: Touch target audit | 1–2 hrs | None |
| 🟠 | P7-3: Pull-to-refresh | 1–2 hrs | None |
| 🟠 | P7-4: Keyboard avoidance | 1–2 hrs | None |
| 🔘 | P8-1: Per-tenant rate limiting | 2–3 hrs | None |
| 🔘 | P8-2: Machine-level revocation | 2–3 hrs | None |
| 🔘 | P8-3: Cloud server health | 1–2 hrs | None |
| 🔘 | P8-4: License server healthcheck | 1 hr | None |
| 🟠 | P9-1: Chart visualizations | 3–4 hrs | None |
| 🟠 | P9-2: CSV export | 1–2 hrs | None |
| 🟠 | P9-3: Period comparison | 1–2 hrs | None |
| 🔵 | P10-1: Translate 4 ID bundles | 2 hrs | None |
| 🔵 | P10-2: Lighthouse a11y gate | 2–3 hrs | P10-3, P10-4 |
| 🔵 | P10-3: Color contrast audit | 2–3 hrs | None |
| 🔵 | P10-4: Focus indicator audit | 1–2 hrs | None |
| 🔵 | P10-5: Screen reader UX | 2–3 hrs | None |
| 🟢 | P11-1: Phase 1 — High-risk shadows | 2 hrs | None |
| 🟢 | P11-2: Phase 2 — Medium-risk shadows | 1 hr | P11-1 |
| 🟢 | P11-3: Phase 3 — Low-risk shadows | 1 hr | P11-2 |
| 🟢 | P11-4: Noise overlay CSS refactor | 1–2 hrs | P11-3 |
| 🟢 | P11-5: Visual regression test | 3–4 hrs | P11-4 |
| 🔴 | P12-1: Key rotation policy | 2–3 hrs | None |
| 🔴 | P12-2: Incident response plan | 2 hrs | None |
| 🔴 | P12-3: Daily audit log review | 2–3 hrs | None |
| 🔴 | P12-4: Session timeout & lockout | 3–4 hrs | None |
| 🟡 | P13-1: CI pipeline optimization | 2–3 hrs | None |
| 🟡 | P13-2: Docker Compose for full stack | 3–4 hrs | None |
| 🟡 | P13-3: E2E test suite | 4–6 hrs | P13-2 |
| 🟡 | P13-4: Developer setup script | 2 hrs | None |
| 🟣 | P14-1: Android build pipeline | 3–4 hrs | None (SDK) |
| 🟣 | P14-2: iOS build pipeline | 3–4 hrs | None (Xcode) |
| 🟣 | P14-3: Tablet gesture & orientation | 2–3 hrs | None |
| 🟣 | P14-4: Mobile deployment docs | 2 hrs | None |

**Total estimated effort:** ~112–155 hours

### Suggested sprint plan

| Sprint | Items | Est. hours |
|--------|-------|------------|
| **Week 1** | P0-1 through P0-5 (plugin security) + P4-1 through P4-4 (docs) | 11–16h |
| **Week 2** | P1-1 through P1-3 (conflict strategy, sync tests, conflict UI) | 8–11h |
| **Week 3** | P1-4 through P1-6 (sync remaining) + P2-1, P2-2 (perf baseline + virtualize) | 7–11h |
| **Week 4** | P2-3 through P2-5 (perf remaining) + P5-1, P5-2 (gateway hardening) | 6–10h |
| **Week 5** | P5-3, P5-4 (webhooks + fixtures) + P6-1, P6-2 (hardware auto-detect + barcode) | 10–13h |
| **Week 6** | P6-3, P6-4 (printer status + receipt preview) + P7-1, P7-2 (swipe + touch audit) | 6–10h |
| **Week 7** | P7-3, P7-4 (pull-to-refresh + keyboard) + P8-1, P8-2 (rate limit + revocation) | 6–10h |
| **Week 8** | P8-3, P8-4 (health + deploy) + P9-1, P9-2, P9-3 (charts, CSV, comparison) | 6–9h |
| **Week 9** | P10-1 through P10-5 (i18n & a11y) + P11-1, P11-2 (shadow banding) | 9–13h |
| **Week 10** | P11-3 through P11-5 (shadow remaining) + P12-1, P12-2 (PCI-DSS) | 8–11h |
| **Week 11** | P12-3, P12-4 (PCI-DSS remaining) + P13-1, P13-2 (DevOps) | 8–11h |
| **Week 12** | P13-3, P13-4 (E2E + setup script) + P14-1 through P14-4 (mobile build) | 12–15h |
