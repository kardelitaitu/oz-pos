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

### A. Shared Test Server (platform-sync integration)

- [ ] **A1.** Refactor `platform/sync/tests/integration_test.rs` to spawn one `TestServer` via
  `tokio::sync::OnceCell` or `std::sync::LazyLock` instead of per-test `spawn_test_server()`.
- [ ] **A2.** Add a `reset_state()` helper that clears the test server's in-memory state between
  tests without restarting the server.
- [ ] **A3.** Verify all 19 integration tests pass with shared server (`cargo test -p platform-sync --test integration_test`).
- [ ] **A4.** Measure before/after wall-clock time and record improvement.

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

### D. DB Snapshot for Migration Tests

- [ ] **D1.** In `oz-core` and other crates that call `migrations::fresh_db()` in every test,
  create a `LazyLock<Connection>` snapshot seeded with all migrations so tests clone the
  snapshot instead of re-running the full migration chain.
- [ ] **D2.** Add a `fn fresh_from_snapshot() -> Connection` helper and migrate tests to use it.
- [ ] **D3.** Verify all tests still pass — snapshot must be read-only cloned (use
  `conn.backup()` or `ATTACH DATABASE` + copy).

### E. Cargo Dev Profile Tuning

- [ ] **E1.** Review `[profile.dev]` overrides in workspace `Cargo.toml`. Already has
  `opt-level = 3` for `argon2`, `aes-gcm`, `aead`, `zstd`.
- [ ] **E2.** Add `opt-level = 3` for `rusqlite` (heavily used in test DB operations).
- [ ] **E3.** Add `opt-level = 3` for `serde_json` (used in every sync transport test).
- [ ] **E4.** Consider `[profile.dev] split-debuginfo = "off"` on Windows to speed up
  linking after test compilation.

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

### G. Shared Mock Modules

- [ ] **G1.** Create `ui/src/__tests__/test-utils/mocks/tauri.ts` — extract all
  `vi.mock('@tauri-apps/api/core')` variations (used in 34+ test files).
- [ ] **G2.** Create `ui/src/__tests__/test-utils/mocks/components.ts` — extract common
  component mocks (`ZoomProvider`, `LocaleContext`, `ThemeToggle`, etc.).
- [ ] **G3.** Create `ui/src/__tests__/test-utils/mocks/api.ts` — extract API module mocks
  (`@/api/settings`, `@/api/staff`, `@/api/inventory`, etc.).
- [ ] **G4.** Migrate `RetailPosScreen.test.tsx` (34 mocks → 3 imports).
- [ ] **G5.** Migrate `PosScreen.test.tsx` (30 mocks → 3 imports).
- [ ] **G6.** Migrate remaining test files that use 6+ `vi.mock` calls.
- [ ] **G7.** Verify all tests pass after each migration step.

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
