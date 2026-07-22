# 0.0.18 — Full-Stack Sprint: E2E, Cloud, Payments, Notifications, APIs & Polish

> **Goal:** 16 areas across 3 waves. **(1) GTM-critical:** Midtrans QRIS, cloud server, Docker. **(2) Notifications & Analytics:** low-stock alerts, WhatsApp, multi-store dashboard, PostgreSQL sync. **(3) Polish:** E2E, i18n, HAL, loyalty extraction, DTOs, config validation, API docs, release readiness.
>
> **Current state:** 32 / 32 items complete (100% 🎉) · Updated 2026-07-22

---

## 📋 Sprint Plan

| # | Area | Items | Status |
|---|------|-------|--------|
| 🟢 | E2E Test Expansion | 2 | 2/2 ✅ |
| 🔴 | Cloud Server Hardening | 2 | 2/2 ✅ |
| 🟠 | Midtrans QRIS Payment Gateway | 2 | 2/2 ✅ |
| 🟡 | Low Stock Alert System | 2 | 2/2 ✅ |
| 🔵 | API Documentation (OpenAPI) | 2 | 2/2 ✅ |
| 🟣 | PostgreSQL Sync Daemon | 2 | 2/2 ✅ |
| ⚪ | Docker & DevEx | 2 | 2/2 ✅ |
| 🟤 | i18n Completion | 2 | 2/2 ✅ |
| 🔷 | Customer Display HAL Driver | 2 | 2/2 ✅ |
| 🔶 | Release Readiness | 2 | 2/2 ✅ |
| 📱 | WhatsApp Notification Integration | 2 | 2/2 ✅ |
| 📊 | Multi-Store Centralized Dashboard | 2 | 2/2 ✅ |
| 🎯 | Loyalty Module Extraction | 2 | 2/2 ✅ |
| 🧱 | Shared DTO & Validation Crates | 2 | 2/2 ✅ |
| ⚙️ | Config Validation Layer | 2 | 2/2 ✅ |
| 🕸️ | Topology Persistence Wiring | 2 | 2/2 ✅ |
| **Total** | | **32** | **32/32 (100% 🎉)** |

## 📋 Additional Completed Work (merged into 0.0.18)

| Sprint | Items | Status |
|--------|-------|--------|
| 🟢 P150 — Fuzz Testing Infrastructure | 2 | 2/2 ✅ |
| 🔴 P151 — DB Corruption Recovery | 2 | 2/2 ✅ |
| 🟡 P152 — Rate Limiting Integration Tests | 2 | 2/2 ✅ |
| 🔵 P153 — Automated A11y Testing | 2 | 2/2 ✅ |
| 🟣 P154 — TypeScript API Client SDK | 2 | 2/2 ✅ |
| 🟢 P200 — A11y Bug Fixes | 2 | 2/2 ✅ |
| 🔴 P201 — Error Handling Polish | 2 | 2/2 ✅ |
| 🟡 P202 — Final Cleanup | 2 | 2/2 ✅ |
| 🟢 P210 — Warning Resolution | 2 | 2/2 ✅ |
| 🔴 P211 — API SDK Polish | 2 | 2/2 ✅ |
| 🟡 P212 — Security & Docs | 2 | 2/2 ✅ |
| 🟣 P213 — Codebase Polish | 2 | 2/2 ✅ |
| 🟢 P220 — Test Rescue (80 tests) | 2 | 2/2 ✅ |
| 🟢 P230 — Test Rescue (25 tests) | 4 | 4/4 ✅ |
| 🔴 P221/P231 — Lint & Clippy Fixes | 2 | 2/2 ✅ |
| 🟢 P240 — Gate Pipeline | 2 | 2/2 ✅ |
| 🟢 P250 — Remaining 8 Test Fixes | 2 | 2/2 ✅ |
| 🔴 P251 — Clippy Cleanup | 1 | 1/1 ✅ |
| 🟢 P260 — CHANGELOG Updates | 1 | 1/1 ✅ |
| 🟢 P262 — Scalar API Docs | 2 | 2/2 ✅ |
| 🟢 P270 — Sync Error Classification | 1 | 1/1 ✅ |
| 🔴 P271 — Pre-Sync Health Check | 2 | 2/2 ✅ |
| 🟢 P140 — License Server Hardening | 2 | 2/2 ✅ |
| 🔴 P141 — CRM Module Hardening | 2 | 2/2 ✅ |
| 🟡 P142 — KDS Edge Cases | 2 | 2/2 ✅ |
| 🔵 P143 — Reporting Analytics | 2 | 2/2 ✅ |
| 🟣 P144 — Security & Config Audit | 2 | 2/2 ✅ |
| 🟢 P130 — Performance Benchmarks | 2 | 2/2 ✅ |
| 🔴 P131 — Mobile Build Pipeline | 2 | 2/2 ✅ |
| 🟡 P132 — Plugin Ecosystem | 2 | 2/2 ✅ |
| 🔵 P133 — CI/CD & DevOps | 2 | 2/2 ✅ |
| 🟣 P134 — Bug Bash Round 2 | 2 | 2/2 ✅ |
| 🟢 P120 — Database Migration Rollback | 2 | 2/2 ✅ |
| 🔴 P121 — Lua Sandbox Audit | 2 | 2/2 ✅ |
| 🟡 P122 — Offline Sync Tests | 2 | 2/2 ✅ |
| 🔵 P123 — Payment Error Recovery | 2 | 2/2 ✅ |
| 🟣 P124 — Bug Bash & Polish | 2 | 2/2 ✅ |
| 🟡 P272 — Sync Status UI | 1 | 1/1 ✅ |

---

### 🟡 P272 — Sync Status UI (deferred)

> **Goal:** Surface sync connection status in the shell header so users can see at a glance if the cloud server is reachable.

- [x] **P272-1: Sync status indicator** ✅ — Created `useSyncConnection` hook (polls `testSyncConnection` IPC every 60s, returns connected/disconnected/checking). Wired green/red/yellow dot into StatusBar left segment with pulse animation for checking state. Added 3 Fluent keys (en + id).
- [x] **P272-2: Status indicator tests** ✅ — 6 tests for hook (initial checking, connected, disconnected ok:false, disconnected throw, polling, cleanup) + 4 tests for StatusBar (connected dot, disconnected dot, checking dot, always visible).

**Files:** `ui/src/hooks/useSyncConnection.ts` (new), `ui/src/__tests__/useSyncConnection.test.ts` (new), `StatusBar.tsx` (updated), `StatusBar.css` (updated), `shared.ftl` (updated), `shared.id.ftl` (updated), `StatusBar.test.tsx` (updated).

---

### Wave 1 — Quick Wins

#### 🔴 1.1 Android APK Build Documentation

> **Status:** CI pipeline exists at `.github/workflows/android.yml`. Ready for finalization.

#### 🟢 1.2 i18n Gaps — Fixed ✅

Fixed 12 hardcoded English strings in `EmailReportSettings.tsx`:
- **7 report type labels**: `Daily Revenue`, `Weekly Revenue`, `Monthly Revenue`, `Top Products`, `Hourly Heatmap`, `Category Breakdown`, `Low Stock Alerts` → `l10n.getString()` with Fluent keys
- **3 aria-labels**: Recipient #{n}, Remove recipient #{n}, Add recipient → `l10n.getString('settings-schedule-recipient-*', { number })`
- **2 error fallbacks**: `'Failed to send test email'`, `'Failed to save schedule'` → `l10n.getString()`
- Added 12 Fluent keys to `settings.ftl` + 12 Indonesian translations to `settings.id.ftl`

#### ✅ 1.3 tokio-console + flamegraph — Already implemented

- `platform/startup/src/console.rs` has both real and no-op `init_console_subscriber()`
- Desktop, tablet clients call it; cloud-server has its own init
- `scripts/profile.ps1` + `scripts/profile.sh` exist with full parameter sets

### 🟢 Wave 2 — Custom Report Builder ✅

> **Status:** Fully implemented in this sprint. Drag-and-drop column picker with 6 datasets, backend query generation with SQL injection protection via column whitelist, search/filter, column reordering, CSV export, error handling.

| Task | Files | Scope |
|------|-------|-------|
| **Frontend** | `CustomReportScreen.tsx`, `CustomReportScreen.css` | Drag-and-drop column picker, 6 datasets (sales, inventory, customers, staff, tax_rates, shifts), search/filter, column reordering, results table, CSV export, error/empty states |
| **Backend** | `crates/oz-core/src/export/mod.rs` | `build_custom_report()` with whitelist-filtered SQL, `get_dataset_def()` for 6 datasets with date filter support, SQL injection protection |
| **Fluent i18n** | `reports.ftl`, `reports.id.ftl` | 11 keys (en + id), including column labels, dataset names, empty/error states |
| **Tests** | `CustomReportScreen.test.tsx`, `crates/oz-core/src/export/mod.rs` | 19 vitest tests (column toggle, dataset switch, search, drag-and-drop, error handling, Run Report disabled state) + 8 Rust tests (unknown dataset, column filtering, sales basic, inventory, empty columns, CSV export) |

### 🟡 Wave 2 — Remaining Feature Work

| Task | Scope | Estimated Effort |
|------|-------|-----------------|
| **Analytics Cloud Export** | BigQuery/Snowflake adapter, credential management, scheduled export | **Major** (2-3 days) |

### 📊 Final Gate State

| Gate | Status |
|------|--------|
| `cargo fmt` | ✅ Clean |
| `cargo clippy` | ✅ 0 errors |
| `npm run typecheck` | ✅ 0 errors |
| `npm run lint` | ✅ 0 errors |
| `vitest` | ✅ 2,936 passed, 0 failures |

> **Cumulative: 113 → 0 pre-existing vitest failures (100% reduction)**
