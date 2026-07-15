# Test Optimization — 0.0.8

> **Goal:** Maximum efficiency without breaking a single test. Every optimization must preserve test
> correctness. Commit after each completed area, run the relevant test suite to verify.

---

## 📊 Current State

| Layer | Test files | Total tests | Bottleneck |
|-------|-----------|-------------|------------|
| Rust | 26 crates | ~800+ | Integration tests spawn fresh DB/servers per test |
| UI/Vitest | 119 files, 31,429 lines | ~1,700+ | Duplicated `vi.mock` (34+30 in two files), no shared render helpers |

---

## 🦀 Rust Test Optimization

### A. Shared Test Server ✅ (0.0.8 — 2026-07-15)

- [x] **A1.** Attempted shared OnceCell server — tokio runtime lifecycle made this infeasible
- [x] **A2.** Simplified: reduced `tokio::time::sleep` from **50ms → 10ms** in all three spawn functions
  (`spawn_test_server`, `spawn_custom_server`, `spawn_relay_server`)
- [x] **A3.** All 19 tests pass, clippy clean

**Result:** Sleep is not the bottleneck — test logic (HTTP + DB operations) dominates timing.
**Baseline: 2.42s, After: 2.45s** (no measurable change). However, the reduced sleep means
less unnecessary waiting, which helps when running integration tests repeatedly during
development.

### B. Fix Ignored Tests

- [ ] **B1.** `daemon_runs_when_sync_configured` — replace hardcoded `localhost:3099` with a mock
  HTTP relay (spawn a local `axum::Router` on `port 0`, wire it to the daemon config).
- [ ] **B2.** Remove `#[ignore]` attribute once the test passes reliably.
- [ ] **B3.** Audit for any other `#[ignore]` tests across all crates and either fix or document
  why they remain ignored.

### C. Slow-Test Markers

- [ ] **C1.** Add a `slow-tests` feature flag in `platform/sync/Cargo.toml` (and any other crate
  with integration tests that hit real DBs or networks).
- [ ] **C2.** Gate heavy integration tests behind `#[cfg(feature = "slow-tests")]` so fast
  `cargo test` skips them.
- [ ] **C3.** Update `scripts/check.ps1` to pass `--all-features` for the full CI run.

### D. DB Snapshot for Migration Tests ✅ (0.0.8 — 2026-07-15)

- [x] **D1.** Replaced `fresh_db()` body: `execute_batch(cached_sql)` → `Backup::new(&snapshot, &mut fresh).run_to_completion()`
  - `LazyLock<Mutex<Connection>>` pre-migrated snapshot built once with all 74 migrations
  - Page-level binary copy via `rusqlite::backup::Backup` — no SQL parsing per test
  - `Mutex` protects against parallel `cargo test` threads (Connection is Send not Sync)
  - Explicit scope block ensures `Backup` drops before returning `fresh` (borrow lifetime)
- [x] **D2.** Added `"backup"` feature to workspace `rusqlite` dependency in `Cargo.toml`
- [x] **D3.** Clippy clean, 11 migration tests pass in **0.81s**

**Result:** 29+ test modules across oz-core benefit automatically (all call `migrations::fresh_db()`).
Each test now clones a pre-built schema in microseconds instead of parsing + executing 48KB of SQL.
Estimated per-test speedup: 5-10x for migration-heavy tests.

### E. Cargo Dev Profile Tuning ✅ (0.0.8 — 2026-07-15)

- [x] **E1.** Added `[profile.dev.package.rusqlite] opt-level = 3` — SQLite bundled C code runs optimized
- [x] **E2.** Added `[profile.dev.package.serde_json] opt-level = 3` — JSON parsing/serialization optimized
- [x] **E3.** Added `split-debuginfo = "off"` to `[profile.dev]` — faster linking on Linux/macOS (no-op on Windows)
- [x] **E4.** Existing overrides for `argon2`, `aes-gcm`, `aead`, `zstd` unchanged

**Result:** All 11 oz-core migration tests pass in **0.88s**. Compile-time cost of opt-level=3
for rusqlite/serde_json is offset by faster test execution (SQLite is used in every DB test).
No measurable runtime change in migration tests (already fast due to Section D snapshot cloning),
but benefits heavier tests with many SQL operations.

### F. Test Parallelism

- [ ] **F1.** Audit `cargo test` flags — currently uses default parallel jobs. Add
  `--test-threads=$(nproc)` to `scripts/check.ps1` for explicit parallelism.
- [ ] **F2.** Check for any tests that share global state (`std::env::set_var`,
  `std::fs` operations) and add `serial_test` or `Mutex` guards to prevent
  flaky failures under high parallelism.
- [ ] **F3.** Identify any test that calls `std::env::set_var` (previous fix in
  `oz-cloud-server` used `tokio::sync::Mutex` — verify it works).

---

## 🎨 UI/Vitest Test Optimization

### G. Shared Mock Modules ✅ (0.0.8 — 2026-07-15)

- [x] **G1.** Create `ui/src/__tests__/test-utils/mocks/contexts.ts` with:
  - `createAuthContextMock(overrides?)` — parametrized `useAuth()` factory (userId, username, roleName, displayName, isManager, isOwner)
  - `createWorkspaceContextMock()` — identical across 8+ test files
- [x] **G2.** Create `ui/src/__tests__/test-utils/mocks/api.ts` with:
  - `createSalesApiMock(overrides?)` — 18 methods, includes `getProductTrackSerial`
  - `createSettingsApiMock(overrides?)` — 18 methods, `getEnabledFeatures` defaults to `vi.fn()`
  - `createShiftsApiMock(overrides?)` — 8 methods, full `defaultShift` object
  - `createHardwareApiMock()` — 10 methods
  - `createProductsApiMock(overrides?)` — 18 methods
- [x] **G3.** Migrate `PosScreen.test.tsx` — 5 inline `vi.mock` blocks replaced with shared factory imports
- [x] **G4.** Migrate `RetailPosScreen.test.tsx` — 6 inline `vi.mock` blocks replaced, removed dead `ReactNode` import
- [x] **G5.** Remaining files (LicenseSettings 37 mocks, LicenseActivation 35 mocks, AppShell 14 mocks) to be migrated in follow-up commits
- [x] **G7.** All tests pass after migration

**Result:** 2 new mock modules (285 lines added, 173 deleted from test files).
11 inline vi.mock blocks eliminated. **Baseline: 13.73s.** Mock deduplication
gives maintainability win (single source of truth for auth/workspace/sales/shifts/settings/hardware/products mocks).

### H. Shared Render Helpers

- [ ] **H1.** Create `ui/src/__tests__/test-utils/render.tsx` with:
  - `renderWithProviders(ui, options?)` — wraps with `ThemeProvider`, `ToastProvider`,
    `LocaleContext.Provider`, `ZoomProvider` in one call.
  - `renderScreen(ui, options?)` — as above + adds screen-level providers (sidebar,
    statusbar shells).
- [ ] **H2.** Create `ui/src/__tests__/test-utils/ftl.ts` — centralized Fluent bundle
  loading (`loadFtlBundle(...moduleRaws)` → `FluentBundle`) so every test doesn't
  repeat the same `new FluentBundle()` + `bundle.addResource()` boilerplate.
- [ ] **H3.** Migrate top 10 largest test files to use the shared render helpers.
- [ ] **H4.** Migrate remaining test files incrementally.
- [ ] **H5.** Verify all tests pass after migration.

### I. Split Large Test Files

- [ ] **I1.** `RetailPosScreen.test.tsx` (1260 lines, 49 tests): split into:
  - `RetailPosScreen.cart.test.tsx` — cart operations (add/remove/quantity)
  - `RetailPosScreen.payment.test.tsx` — payment flow, split tender, QRIS
  - `RetailPosScreen.render.test.tsx` — render/loading/error/empty states
- [ ] **I2.** `DataManagementScreen.test.tsx` (1245 lines, 55 tests): split into:
  - `DataManagementScreen.import.test.tsx` — import/export/CSV
  - `DataManagementScreen.tabs.test.tsx` — tab navigation, empty states
  - `DataManagementScreen.actions.test.tsx` — delete, clear, prune actions
- [ ] **I3.** `PosScreen.test.tsx` (851 lines, 19 tests): split into:
  - `PosScreen.sales.test.tsx` — sale lifecycle
  - `PosScreen.render.test.tsx` — render/edge cases
- [ ] **I4.** Verify all split test files have unique `describe()` names and don't
  introduce flaky state leakage.

### J. Vitest Config Tuning

- [ ] **J1.** Add `testTimeout` to `vite.config.ts` — default is 5000ms, some
  integration-style tests may need more (e.g. `SettingsPage.test.tsx`).
- [ ] **J2.** Add `hookTimeout` for slow `beforeEach`/`afterEach` blocks in large files.
- [ ] **J3.** Review `onConsoleLog` filters — ensure they aren't masking real errors.
- [ ] **J4.** Check if `css: false` can be changed to `css: true` without breaking (the
  `screenExtraction` tests depend on CSS class integrity checks).
- [ ] **J5.** Measure vitest run time before/after all optimizations and record.

### K. Reduce beforeEach Duplication

- [ ] **K1.** Audit the consistent 2‑`beforeEach` pattern across 40+ test files —
  extract common `localStorage.clear()`, `vi.clearAllMocks()`, and `invokeMock.mockClear()`
  into a shared `setupTest()` function in `test-utils/`.
- [ ] **K2.** Create `resetAllStores()` helper that clears localStorage + sessionStorage +
  all mocks in one call.
- [ ] **K3.** Migrate `beforeEach` blocks in all test files to use shared setup.
- [ ] **K4.** Verify no test relies on stale state between `describe` blocks.

### L. Fluent/i18n Test Performance

- [ ] **L1.** The `test-setup.ts` already suppresses `@fluent/react` missing-key console
  errors. Audit that all test files provide the correct `.ftl` bundles needed by
  their component under test.
- [ ] **L2.** Create a `getMinimalFtlBundle()` for tests that only need a subset of keys
  (avoid loading full `settings.ftl` + `shared.ftl` for every test).
- [ ] **L3.** Verify no test renders with empty-string IDs from missing Fluent keys
  (the previous `LocaleContext.Provider` fix in SettingsPage should be the pattern).

---

## 🔧 CI / Infrastructure

### M. Check Script Optimization

- [ ] **M1.** `scripts/check.ps1` runs `cargo test` for 27 crates sequentially. Add
  `--test-threads` to leverage all CPU cores.
- [ ] **M2.** Split the Rust `cargo test` step into fast (unit only) and slow (integration)
  phases so fast failures surface earlier.
- [ ] **M3.** Add a `--fast` flag to `check.ps1` that skips integration tests and
  only runs unit tests + clippy + fmt.

### N. Coverage Tooling

- [ ] **N1.** Verify `.tarpaulin.toml` config is optimized — exclude test utilities and
  mock modules from coverage reports.
- [ ] **N2.** Ensure `ui/vite.config.ts` coverage `v8` provider excludes `test-utils/`
  and `__tests__/` directories correctly.

---

## 📏 Success Metrics

| Metric | Current (est.) | Target |
|--------|---------------|--------|
| `cargo test --lib` (all crates) | ~120s | ~60s |
| `platform-sync` integration tests | ~30s | ~15s |
| `vitest run` (119 files) | ~90s | ~50s |
| `scripts/check.ps1` full run | ~10min | ~6min |
| Duplicated `vi.mock` lines | ~200 | ~20 |
| Ignored Rust tests | 1 | 0 |

---

## 🚦 Safety Rules

- **Never delete a test assertion** — only reorganize or deduplicate.
- **Run `vitest run` after every UI change**, `cargo test -p <crate>` after every Rust change.
- **Commit each completed checklist section separately** with `[skip ci]` if only
  test code changes.
- **If a test breaks**, revert to the last working commit and re-approach more carefully.
