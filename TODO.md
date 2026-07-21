# 0.0.17 — Roadmap Gap Closure

> **Goal:** Close the remaining ROADMAP gaps across Analytics, Backend Hardening, CI/CD, Mobile Builds, and Fuzz Testing.
>
> **Current state:** 12 / 25 items complete · Updated 2026-07-21

---

## 📋 Sprint Plan

| Sprint | Items | Status |
|--------|-------|--------|
| 🔴 P61 — Analytics & Reports | 5 | 0/5 ⬜ |
| 🔵 P62 — Backend Hardening (cont.) | 5 | 0/5 ⬜ |
| 🟢 P63 — CI/CD & DevOps | 4 | 0/4 ⬜ |
| 🟡 P64 — Mobile Build Pipeline | 4 | 0/4 ⬜ |
| 🟣 P65 — Fuzz Testing | 4 | 0/4 ⬜ |
| ⚪ P66 — Performance Benchmarks | 3 | 0/3 ⬜ |
| **Total** | **25** | **0/25 ⬜** |

---

### 🔴 P61 — Analytics & Reports

> **Goal:** Wire up analytics export (BigQuery/Snowflake), scheduled email report delivery, and custom report builder UI.

- [x] **P61-1: Analytics export to cloud warehouse** ✅ — Created `crates/oz-core/src/export/cloud_destination.rs` with `ExportDestination` enum (BigQuery, Snowflake), config structs, `CloudExportConfig`, `CloudExporter` trait with async `export()` method using real HTTP APIs (BigQuery insertAll + OAuth2 JWT auth, Snowflake SQL REST + session auth). Added `Store::save_cloud_export_config()` and `get_cloud_export_config()` persistence in settings table. 7 unit tests pass.
- [x] **P61-2: Scheduled email report delivery (backend)** ✅ — Already fully implemented: `apps/cloud-server/src/email.rs` has `start_report_sender_loop()`, `generate_report_email()`, SMTP transport via `lettre`. `ReportScheduleConfig` exists with save/load in settings.
- [x] **P61-3: Scheduled email report UI** ✅ — Added schedule config panel in EmailReportSettings.tsx (Settings → Reports): enabled toggle, cadence selector, send time, timezone, lookback days, 7 report type checkboxes, recipient list with add/remove. Backend: get_report_schedule/save_report_schedule Tauri commands in email.rs, registered in lib.rs, API layer in email.ts.
- [x] **P61-4: Custom report builder (backend)** ✅ — Already fully implemented: `Store::build_custom_report()` with column whitelist, date range filtering, SQL injection protection. Exposed via Tauri command in `apps/desktop-client/src/commands/reports.rs`.
- [x] **P61-5: Custom report builder (UI)** ✅ — Already fully implemented: `CustomReportScreen.tsx` with dataset selector, column checkboxes, date range picker, live results table, CSV export, loading/error/empty states. Registered in App.tsx under route `custom-report` with manager role.

---

### 🔵 P62 — Backend Hardening (cont.)

> **Goal:** Harden remaining ~16 unwrap/expect calls in production code paths.

- [x] **P62-1: cart.rs expect() audit** ✅ — All 5 `.expect()` calls are in `#[cfg(test)]` test assertions only. Production code already clean. No changes needed.
- [x] **P62-2: stock_counts.rs + rate_limiter.rs hardening** ✅ — stock_counts.rs: all unwrap/expect calls are in `#[cfg(test)]` only (production code clean). rate_limiter.rs: replaced `.expect("rate limiter lock poisoned")` with `PoisonError::into_inner` recovery pattern.
- [x] **P62-3: Payment driver hardening** ✅ — Replaced 6 `.expect()` calls (3x `HeaderValue::from_str` + 3x `Client::builder().build()`) in stripe.rs, square.rs, qris.rs with `unwrap_or_else` + `tracing::error!` + degraded fallback. Added `tracing` workspace dependency.
- [x] **P62-4: Startup + sync hardening** ✅ — `startup/src/lib.rs`: replaced `.expect("pending sale reaper: open DB")` with `match` + `tracing::error!` + early return. `pg_transport.rs` production code already clean (all `.expect()` in `#[cfg(test)]` only).
- [x] **P62-5: auth.rs JWT encoding hardening** ✅ — `create_token()` now returns `Result<TokenResponse, Error>` instead of panicking. Route handler returns 500 on encoding failure.

---

### 🟢 P63 — CI/CD & DevOps

> **Goal:** Add nightly full-matrix CI, coverage gate, and CI pipeline polish.

- [x] **P63-1: Nightly full-matrix CI** ✅ — Created `.github/workflows/nightly.yml` with 9 jobs: Rust test (3 OS matrix), Rust doc generation, 4-way UI shards, 3-way E2E shards with Docker Compose, desktop release builds (Linux AppImage + Windows MSI + macOS DMG), tablet release (Android APK), and cargo bench. Scheduled at 3 AM UTC daily.
- [x] **P63-2: Coverage gate in CI** ✅ — Already fully implemented: `scripts/coverage.sh` and `scripts/coverage.ps1` exist (use `cargo-llvm-cov` for cross-platform coverage). `.tarpaulin.toml` exists for Linux-only quick runs. CI `coverage` job uses `cargo-llvm-cov --lcov` with artifact upload. All already wired.
- [x] **P63-3: Skill drift CI check** ✅ — Already fully implemented: `skill-drift-guard` skills (4 bats tests) run in CI via `skill-drift-tests` job in `ci.yml` with `detect.sh` baseline check + `run-tests.sh` full suite.
- [x] **P63-4: CI pipeline polish (partial)** ✅ — Fixed `save-always` deprecation in all 6 `Swatinem/rust-cache@v2` usages → `save-if: ${{ github.ref == 'refs/heads/main' }}`. Added `SCCACHE_GHA_ENABLED: "true"` to top-level env to fix 0% sccache hit rate (was using ephemeral local disk). Remaining: sccache stats to workflow summary, E2E timeout reduction.

---

### 🟡 P64 — Mobile Build Pipeline

> **Goal:** Finalize Android APK + iPad build and deployment pipeline.

- [ ] **P64-1: Verify Android APK build works end-to-end** — Run `cargo tauri android build --apk` on CI, fix any compilation issues. Verify signed APK is generated.
- [ ] **P64-2: Android keystore management** — Document keystore generation, add GitHub Actions secrets guide, verify signing works in CI.
- [ ] **P64-3: iOS/iPad build docs** — Create `docs/operations/ios-build-guide.md`: Xcode setup, TestFlight distribution, code signing, CI considerations.
- [ ] **P64-4: Mobile release checklist** — Create `docs/releases/mobile-checklist.md`: pre-release testing (APK + IPA), touch target validation, performance profiling.

---

### 🟣 P65 — Fuzz Testing

> **Goal:** Set up cargo-fuzz targets for critical parsing and arithmetic paths.

- [ ] **P65-1: Money/CoreCurrency fuzz target** — Fuzz `Currency::from_str()` with arbitrary byte strings. Panic on invalid input.
- [ ] **P65-2: SKU/Barcode fuzz target** — Fuzz `Sku::try_new()` and `Barcode::new()` with arbitrary strings. Verify no panics, only validation errors.
- [ ] **P65-3: Sale/completion fuzz target** — Fuzz `complete_sale_deduction()` with random sale structs (invalid amounts, missing fields, extreme quantities).
- [ ] **P65-4: JSON/Lua parsing fuzz target** — Fuzz `oz-lua` script loading with random byte strings. Verify sandbox containment and no panics.

---

### ⚪ P66 — Performance Benchmarks

> **Goal:** Track benchmark regressions with comparison reports and baseline snapshots.

- [ ] **P66-1: Benchmark baseline snapshot** — Run `cargo bench -p oz-core` against current HEAD, save results as `docs/benchmarks/baseline-2026-07-21.md`. Include hardware/OS context.
- [ ] **P66-2: Regression tracking doc** — Create `docs/benchmarks/regression-tracking.md` with instructions for `critcmp` comparison workflow.
- [ ] **P66-3: CI benchmark comparison** — Add `cargo bench` job to nightly CI that posts regression report as PR comment when run on push events.

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
