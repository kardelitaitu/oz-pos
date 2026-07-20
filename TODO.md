# 0.0.15 — Polish, Physical Device Testing & Remaining Gaps

> **Goal:** Close the remaining unchecked ROADMAP items, resolve code TODOs, wire up email report delivery, complete Thai i18n translations, and validate on physical devices.

**Current state:** 0 / 20 items complete (0%) · Updated 2026-07-20

---

## 🟢 P54 — Thai i18n Completion

> The scaffolding was created in P22 (24 `.th.ftl` bundles with `[TH] … [/TH]` markers), now fill in real Thai translations.

- [ ] **P54-1: Translate core bundles** — Translate `shared.th.ftl`, `auth.th.ftl`, `pos.th.ftl` (highest-visibility, ~200 keys total). Use the `[TH] … [/TH]` markers as translation slots.
- [ ] **P54-2: Translate feature bundles** — Translate `inventory.th.ftl`, `sales.th.ftl`, `reporting.th.ftl`, `settings.th.ftl`, `staff.th.ftl` (~300 keys).
- [ ] **P54-3: Translate remaining bundles** — Translate `kds.th.ftl`, `crm.th.ftl`, `tax.th.ftl`, `tables.th.ftl`, `terminals.th.ftl`, `stock-transfers.th.ftl`, and all other domain bundles (~200 keys).
- [ ] **P54-4: Thai locale QA** — Run `scripts/lint-i18n.sh`, verify bundle parity, spot-check 10 screens in Thai locale. Fix truncation/overflow on short labels.

## 🟡 P55 — Email Report Delivery

> P15-5 created `ReportScheduleConfig` (persisted in settings table). Now wire up an SMTP backend to actually send scheduled reports.

- [ ] **P55-1: SMTP configuration** — Add `smtp_host`, `smtp_port`, `smtp_username`, `smtp_password`, `smtp_from` settings. Create `SmtpConfig` struct with validation.
- [ ] **P55-2: Email report generator** — Create `ReportEmailBuilder` that takes an `AnalyticsBundle`, renders HTML + plain-text versions with summary tables + charts (ASCII for plain-text).
- [ ] **P55-3: Scheduled send loop** — Background task in cloud-server that polls `report_schedule` settings, checks `send_at_time`, runs `export_analytics_bundle()`, and sends via SMTP.
- [ ] **P55-4: Send test email UI** — Add "Send Test Report" button to settings screen. Validates SMTP config and sends a sample report to the configured recipients.

## 🔵 P56 — Code TODO Resolution

> Resolve the 5 remaining TODO/FIXME comments in production code.

- [ ] **P56-1: terminal_id binding (ADR #7)** — In `WorkspaceContext.tsx`, resolve `terminal_id` from device binding via a Tauri command (`get_device_id()` returning MAC/hostname). Remove the hardcoded empty string.
- [ ] **P56-2: tenant_id on tax_rates/users sync** — In `cloud-server/src/sync_api.rs`, stamp `tenant_id` from JWT claims on POST handlers for `tax_rates` and `users` (same pattern as existing handlers).
- [ ] **P56-3: archive_instance() wrapper (ADR #5)** — Add a public `Store::archive_instance()` method to `crates/oz-core/src/db/workspaces.rs` for proper encapsulation (replaces inline SQL in test).
- [ ] **P56-4: user_store_access check (ADR #4 Phase 2)** — In `list_active_instances()`, add user_store_access row filtering for non-owner roles in multi-store mode.
- [ ] **P56-5: greedy-fill location resolver (ADR-19)** — Implement greedy-fill algorithm in `location_resolver.rs` to use the `qty` parameter for distributing stock deduction across locations.

## 🟣 P57 — Developer Tooling

> Add tokio-console integration and flamegraph helpers for performance debugging.

- [ ] **P57-1: tokio-console integration** — Add `console-subscriber` to cloud-server. Document `tokio-console` launch command in `docs/benchmarks/`. Add `#[tokio::test]` console smoke test.
- [ ] **P57-2: cargo-flamegraph helpers** — Create `scripts/profile.sh` / `scripts/profile.ps1` that wraps `cargo flamegraph` with sane defaults (PID, frequency, output path). Document in benchmark docs.

## 🔴 P58 — Physical Device Validation

> Verify the app actually runs on target hardware — not just CI builds.

- [ ] **P58-1: Windows desktop launch test** — Build `oz-pos-app` release binary. Launch on Windows 10/11. Verify: login → workspace picker → POS → payment → receipt. Log any runtime errors.
- [ ] **P58-2: Linux desktop launch test** — Build `oz-pos-app` release binary on Ubuntu 22.04+. Launch, verify core POS flow. Log any webkit2gtk or library issues.
- [ ] **P58-3: Android APK install test** — Build signed APK via `android.yml`. Install on Android 10+ physical device. Verify: touch targets, barcode scan, KDS ticket board, payment flow.
- [ ] **P58-4: iPad install test** — Build signed IPA via `ios.yml`. Install on iPadOS 16+ via TestFlight. Verify: tablet layout, split-view, swipe gestures, receipt printing.

## ⚪ P59 — Visual Polish & Edge Cases

> Small UX improvements that make a big difference.

- [ ] **P59-1: Empty state illustrations** — Add SVG illustrations to all empty states (no products, no sales, no staff, no shifts). Currently using text-only messages.

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🟢 P54 — Thai i18n | 4 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| 🟡 P55 — Email Reports | 4 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| 🔵 P56 — Code TODOs | 5 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| 🟣 P57 — Dev Tooling | 2 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| 🔴 P58 — Device Validation | 4 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| ⚪ P59 — Visual Polish | 1 | 0 | ░░░░░░░░░░░░░░░░ 0% |
| **Total** | **20** | **0** | **0%** |

---

<br>

# 📋 0.0.14 — Completion Summary (100% 🎉)

> **172 items across 20 sprints — all done.** Committed on `0.0.13` branch, delivered 2026-07-20.

| Sprint | Items | Highlights |
|--------|-------|------------|
| P15-P20 — Ecosystem & Polish | 20 | Analytics export, i18n migration, scheduled reports, loyalty engine + UI, promotions, plugin docs, theming, Android/iOS CI builds, AI demand forecasting + CRDT sync research ADRs |
| P21-P26 — ROADMAP & Features | 12 | ROADMAP audited (40+ checked off), Thai scaffolding (24 bundles), product bundles domain + UI, custom report builder engine + UI, cloud warehouse analytics research + CSV export |
| P26-P29 — Test Performance | 19 | nextest replaces cargo test in CI, 3x speedup, 5-way shard, `.config/nextest.toml`, UI vitest 4-way shard, `test-ui-changed.sh`, e2e 3-way shard, coverage gate, skill-drift gate |
| P30-P31 — Production Hardening | 8 | Dependency audit + CI gate, gitleaks secrets scan, input validation hardening, rate limiting audit, criterion benchmarks, flamegraph profiling, CI SLOs, bundle size audit |
| P32-P33 — Error & Observability | 8 | unwrap/expect audit, user-facing error codes, retry backoff, graceful degradation, structured tracing, health dashboard, 9 Prometheus metrics, 5 alert thresholds + runbook |
| P34-P35 — Backup & Recovery | 6 | backup-db.sh + restore-db.sh, integration test, release checklist + release.sh, CI release job |
| P36-P37 — Code Quality & Docs | 6 | Dead code audit, cargo doc coverage, TODO audit, admin guide + user guide + API reference docs |
| P38-P39 — UI State + Integration | 6 | Loading/empty/error state audit, backup/restore integration test, sync failure recovery tests, payment failure handling test |
| P40-P41 — CI/CD & Security | 7 | sccache + rust-cache, dependency caching audit, CI pipeline dashboard, pre-merge gates, SAST clippy gate, Trivy container scanning, license audit |
| P42-P43 — DB & DevEx | 7 | WAL mode audit, index audit, vacuum/integrity, connection pool audit, pre-commit hook hardening, dev setup scripts, 48 scripts audit |
| P44-P45 — Migration & Cleanup | 6 | WAL mode in migrations, customers + inventory indexes, cargo doc warning fixes, unused imports, error message consistency |
| P46-P47 — Build & Coverage | 5 | check.ps1 local run, release build smoke test, UI production build, test count audit (7,623), untested error paths |
| P48 — Final Cleanup | 4 | Doc warnings below 50, full check.ps1 pipeline, clean git tree, 6,640/6,640 tests pass |
| P49 — Doc Warning Reduction | 6 | Auto-fix 17, empty code blocks, HAL + module links, batch 2 links, webhooks URL fix. 98 → 11 (-89%) |
| P50 — Zero Doc Warnings | 4 | Remaining 11 fixed across 9 files. Private-item links fixed. Doc coverage audit. |
| P51 — Benchmark Reports | 3 | Criterion bench targets, `docs/benchmarks/baseline-2026-07-20.md`, regression tracking dashboard |
| P52 — CI Nightly Builds | 3 | `.github/workflows/nightly.yml` (10 jobs, 3 AM UTC), README badge, report artifact |
| P53 — Fuzz Testing | 3 | `fuzz/` workspace, 3 targets (SKU, money, Cart/Sale), CI fuzz job (push-only, non-blocking) |
| E2E Coverage | 34+43 | 38/38 routes covered, ~77 tests, 17 spec files, `navigateTo` DRY-refactored |

### Final pipeline gates (all passing 🟢)

| Gate | Result |
|------|--------|
| `cargo fmt` | ✅ Clean |
| `cargo clippy` | ✅ 0 warnings |
| `cargo nextest` | ✅ 3,821 passed |
| `cargo test --doc` | ✅ 43 passed, 5 ignored |
| `cargo doc` | ✅ 11 warnings (-89%) |
| `npm run typecheck` | ✅ 0 errors |
| `npm run lint` | ✅ 0 errors |
| `npm run test` | ✅ 2,814 passed |
| `lint-i18n.sh` | ✅ Clean |

<br>

---

<details>
<summary>📦 0.0.14 — Detailed Sprint Archives (click to expand)</summary>

All 172 items across 20 sprints are 100% complete. Detailed per-sprint breakdowns preserved in git history (`git log --oneline 0.0.13`).

Key files created in 0.0.14:
- `fuzz/` — cargo-fuzz workspace (3 targets)
- `.github/workflows/nightly.yml` — 10-job nightly CI
- `docs/benchmarks/baseline-2026-07-20.md` — Criterion benchmark baselines
- `docs/benchmarks/regression-tracking.md` — Historical tracking dashboard
- `docs/admin-guide.md`, `docs/user-guide.md`, `docs/api-reference.md` — User-facing docs
- `docs/ci-pipeline.md` — CI job catalog
- `docs/observability/logging-2026-07-20.md` — Structured logging docs
- `docs/operations/runbook.md` — Incident response procedures
- `docs/security/audit-2026-07-20.md` — Dependency + secrets audit
- `docs/security/sast-2026-07-20.md` — Static analysis results
- `docs/security/license-audit-2026-07-20.md` — License compliance
- `docs/decisions/2026-07-20-ai-demand-forecasting-research.md`
- `docs/decisions/2026-07-20-cloud-warehouse-analytics-research.md`
- `docs/decisions/2026-07-20-crdt-sync-research.md`
- `docs/decisions/2026-07-20-voice-controlled-checkout-research.md`

</details>
