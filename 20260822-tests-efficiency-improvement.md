# Test Efficiency Improvement — Plan & Journal (2026-08-22)

- **Document ID:** 20260822-tests-efficiency-improvement
- **Status:** Planning — baseline pending
- **Owner:** OZ-POS engineering (test-focused agent sessions)
- **Version locked at:** 0.0.29
- **Goal:** Reduce the wall-clock time of every test area in the repo **without reducing test quality** (same assertions, same coverage, same failure-detection power). One area at a time, measure → improve → re-measure → record, until no further gain is worth taking.

---

## 1. Why

A fast test suite is a force multiplier:

- Shorter CI feedback loop → fewer context switches → fewer bugs shipped.
- `check.sh` (the local pre-push gate) currently runs the full matrix: Rust unit/integration/doctests, UI lint/typecheck/tests, i18n, parity gates. Every second saved here is paid back on every commit.
- Slow suites invite skipping: developers stop running full suites locally, and coverage silently rots.

**Hard guardrail:** an improvement is only accepted if it keeps (or raises) test quality. Specifically, it must NOT:

- delete, weaken, or `#[ignore]` assertions;
- reduce the set of covered code paths;
- merge independent tests into one that can no longer pinpoint the failing case;
- replace deterministic assertions with flaky or timing-dependent ones;
- increase test flakiness (any area that becomes flaky is rolled back immediately).

---

## 2. Scope

| In scope | Out of scope |
|----------|--------------|
| Rust unit tests (per-crate `*_tests.rs` + inline `#[cfg(test)]`) | Production code behaviour changes (unless they exist solely to speed tests up) |
| Rust integration tests (`tests/` dirs) | CI runner/hardware provisioning |
| Rust doctests | Test **coverage** improvements (separate effort — see `docs/coverage/README.md`) |
| UI Vitest suite (389 files: `ui/src/__tests__/`) | Writing new tests (this doc is about *speed of existing tests*) |
| UI a11y suite (`npm run test:a11y`) | |
| E2E Playwright specs (`ui/e2e/`, 26 specs) | |
| Node script tests (`scripts/__tests__`, 4 files) | |
| Compile-time of test binaries (profile tweaks, `nextest`, feature gating) | |
| Gate runner costs in `scripts/check.sh` / `scripts/check-ui.mjs` | |

---

## 3. Measurement protocol (must read before timing anything)

Comparisons are only meaningful if the measurement is reproducible. Follow this protocol for **every** baseline and every re-measurement.

1. **Pin the machine.** Record `hostname`, OS, CPU, RAM, and whether the machine is otherwise idle. Never compare numbers from different machines.
2. **Pin the commit.** Record `git rev-parse --short HEAD` next to every number. Only compare numbers at the same commit (or with an explicit `(changed: …)` note).
3. **Three-phase measurement.** For each area, run its command 3 times and take the **median**:
   - **Cold** = first run after `git clean`-equivalent state / no build artifacts (measures full compile + run).
   - **Warm** = subsequent runs with artifacts present (measures incremental rebuild + run).
   - Record **both**. Most improvements target warm time; profile/codegen changes target cold time.
4. **Single command per area.** Each area has exactly one canonical command (see §4). Time the whole command with `Measure-Command` (PowerShell) or `time` (bash). Do not subtract setup.
5. **Same parallelism.** Record `nproc` and any env overrides (`VITEST_MAX_THREADS`, `--test-threads`, `--jobs`). A machine at a different load invalidates the comparison.
6. **Write the number into the results table (§5) and the area's log (§7) in the same session**, with the protocol stamp:
   `machine: <hostname> · cpu: <cores> · commit: <sha> · phase: cold|warm · run: <1|2|3>`

> Every area table in §7 contains a `measure()` block — copy the stamp from there when updating §5.

---

## 4. Area taxonomy

Each area = **one measurable command**. Commands with shared compile artifacts (Rust crates) are still measured separately; the median-of-3 protocol absorbs the shared-build effect as long as the phase is recorded.

### 4.1 Rust — unit tests (per crate)

Run from repo root: `cargo test -p <crate> [--features slow-tests]`. The `slow-tests` feature gate exists in `oz-core` and friends to keep the *default* suite fast while integration-heavy tests remain runnable on demand.

| Area | Command | Unit-test files |
|------|---------|-----------------|
| A01 | `cargo test -p foundation` | 1 |
| A02 | `cargo test -p oz-core` | 39 |
| A03 | `cargo test -p oz-security` | 7 |
| A04 | `cargo test -p oz-reporting` | 6 |
| A05 | `cargo test -p oz-plugin` | 6 |
| A06 | `cargo test -p oz-payment` | 5 |
| A07 | `cargo test -p oz-cli` | 5 |
| A08 | `cargo test -p oz-logging` | 5 |
| A09 | `cargo test -p oz-notification` | 4 |
| A10 | `cargo test -p oz-lua` | 3 |
| A11 | `cargo test -p oz-hal` | 3 |
| A12 | `cargo test -p oz-api` | 3 |
| A13 | `cargo test -p apps/cloud-server` | 14 |
| A14 | `cargo test -p apps/desktop-client` | 5 (+ 6 integration) |
| A15 | `cargo test -p apps/tablet-client` | 2 |
| A16 | `cargo test -p modules/…` (sales, inventory, crm, tax, settings, staff, reporting, terminal, currency, loyalty) | 2 each (~20 total) |
| A17 | `cargo test -p platform/…` (core, kernel, startup, sync) | 2 sync + others |

### 4.2 Rust — integration tests (top-level `tests/` dirs)

| Area | Command | Integration files |
|------|---------|-------------------|
| A18 | `cargo test -p oz-core --test '*'` | 23 |
| A19 | `cargo test -p oz-payment --test '*'` | 7 |
| A20 | `cargo test -p apps/desktop-client --test '*'` | 6 |
| A21 | `cargo test -p oz-hal --test '*'` | 2 |
| A22 | `cargo test -p platform/sync --test '*'` | 2 |
| A23 | `cargo test -p oz-cli --test '*'` | 1 |
| A24 | `cargo test -p apps/cloud-server --test '*'` | 1 |
| A25 | `cargo test -p modules/tax --test '*'` | 1 |

### 4.3 Rust — doctests & full workspace sweep

| Area | Command | Notes |
|------|---------|-------|
| A26 | `cargo test --doc --workspace` | Doctests; not covered by `nextest` |
| A27 | `cargo nextest run --workspace --all-features --exclude oz-pos-app --exclude oz-pos-tablet` | Full sweep as run by `check.sh` (preferred runner: nextest, per-test process isolation) |
| A28 | `cargo test --workspace --all-features -- --test-threads <n>` | Fallback sweep (used when nextest absent) |

### 4.4 UI — Vitest (jsdom)

Run from `ui/`: `npm run test` (full suite) or scoped runs. Global setup: `pool: threads`, `fileParallelism: true`, `testTimeout: 10s`.

| Area | Command | Files |
|------|---------|-------|
| A29 | `npm run test` | 389 (369 top-level + 10 `hooks/` + 7 `a11y/` + 2 `utils/` + 1 `test-utils/`) |
| A30 | `npm run test:a11y` | 7 |
| A31 | `npm run test:coverage` | full suite + v8 coverage |
| A32 | `vitest run src/__tests__/<group>/` (e.g. `features/`, `hooks/`) | per-group scoped runs for targeted iteration |

### 4.5 E2E — Playwright (Docker-provisioned)

Run from `ui/`: `npm run test:e2e -- <spec>` or the managed `npm run e2e` pipeline. Docker backend must be provisioned; these are the most expensive areas.

| Area | Command | Specs |
|------|---------|-------|
| A33 | `npm run test:e2e -- e2e/api.spec.ts` | api |
| A34 | `npm run test:e2e -- e2e/perf-smoke.spec.ts` | perf-smoke |
| A35 | `npm run test:e2e` (remaining 24 specs) | pos, kds, retail, sale, shift, settings, product, refund, auth, inventory, reporting, admin, tablet, etc. |

### 4.6 Node script tests & gates

| Area | Command | Files |
|------|---------|-------|
| A36 | `npm run test:scripts` | 4 (`pipefail`, `run-e2e`, `verify-architecture-boundaries`, `verify-ci-docs-drift`) |
| A37 | `bash scripts/check.sh` | full local pre-push gate (aggregate) |
| A38 | `node ../scripts/check-ui.mjs` (`npm run check:all` from `ui/`) | UI gate aggregate |

---

## 5. Results — comparison table (THE summary)

> Updated after every accepted improvement. Baseline = first protocol measurement (cold/warm, median-of-3). Blank = not yet measured.

| Area | Baseline (s) | 1st improvement | 2nd improvement | 3rd improvement | Techniques used |
|------|-------------|-----------------|-----------------|-----------------|-----------------|
| A01 foundation | 1.3 (cold 6.9) | 1.29 (cold 4.25) | | | `codegen-units=256` test-profile override (cold −38%, warm flat) |
| A02 oz-core | | | | | |
| A03 oz-security | | | | | |
| A04 oz-reporting | | | | | |
| A05 oz-plugin | | | | | |
| A06 oz-payment | | | | | |
| A07 oz-cli | | | | | |
| A08 oz-logging | | | | | |
| A09 oz-notification | | | | | |
| A10 oz-lua | | | | | |
| A11 oz-hal | | | | | |
| A12 oz-api | | | | | |
| A13 cloud-server | | | | | |
| A14 desktop-client | | | | | |
| A15 tablet-client | | | | | |
| A16 modules | | | | | |
| A17 platform | | | | | |
| A18 oz-core integration | | | | | |
| A19 oz-payment integration | | | | | |
| A20 desktop-client integration | | | | | |
| A21 oz-hal integration | | | | | |
| A22 platform/sync integration | | | | | |
| A23 oz-cli integration | | | | | |
| A24 cloud-server integration | | | | | |
| A25 tax integration | | | | | |
| A26 doctests | | | | | |
| A27 nextest workspace sweep | | | | | |
| A28 cargo fallback sweep | | | | | |
| A29 vitest full suite | | | | | |
| A30 a11y suite | | | | | |
| A31 vitest coverage | | | | | |
| A32 vitest per-group | | | | | |
| A33 e2e api | | | | | |
| A34 e2e perf-smoke | | | | | |
| A35 e2e remaining | | | | | |
| A36 script tests | | | | | |
| A37 check.sh aggregate | | | | | |
| A38 check:all aggregate | | | | | |

**Totals (A27 + A29 + A33–A36 as the canonical CI sweep):** baseline ___ s → current ___ s → **Δ ___ s (−__%)**

---

## 6. Improvement loop (how we run this campaign)

1. **Pick an area** — start with the biggest contributor to the CI sweep (§5 totals), then move down. No area is off-limits except those owned by another in-flight agent (see §8).
2. **Baseline** — measure per §3 (median of 3, cold + warm), stamp it, fill the first column of §5.
3. **Hypothesize** — pick ONE technique from the playbook (§6.1) or a new idea; state the expected mechanism and target before changing anything.
4. **Implement** — smallest change that tests the hypothesis. Keep the change isolated so it can be reverted cleanly.
5. **Re-measure** — same protocol. If warm/cold improved and the area's tests still pass **with identical assertions**, record it as the next improvement column in §5 and log the diff in §7.
6. **Repeat** — go back to step 3 for the same area. Stop when two consecutive attempts produce <5% gain or a regression risk, and mark the area **plateaued** in §7.
7. **Quality gate** — before accepting ANY improvement, run the area's full suite + the two neighbouring areas to confirm no cross-area breakage. Flaky = roll back.
8. **Commit** — one commit per accepted improvement (`test:perf(area): …`), message stating measured before/after. Local commit only; no push without explicit instruction.

### 6.1 Playbook of proven techniques (reference)

| Technique | Applies to | Mechanism | Quality risk |
|-----------|-----------|-----------|--------------|
| `nextest` instead of `cargo test` | A01–A28 | Per-test process isolation, parallel by default, faster re-runs (~4.5× claimed) | None |
| Feature-gate slow integration tests (`slow-tests`) | A02, A18 | Default suite skips heavy tests; run on demand | **High** if gate default is wrong — verify CI runs the gate |
| `--test-threads` / nextest `--test-threads` tuning | A01–A28 | More parallelism on many-core hosts | Watch for resource contention/flakiness |
| Test profile: `strip`, `debug=1`, `codegen-units` | A01–A28 | Smaller/faster test binaries | None |
| `profile.tdd` for tight loops | A01–A28 | Fastest possible dev compile | Dev-only, not CI |
| Split one heavy test file into several (Vitest parallelism) | A29 | `fileParallelism` balances across workers | None if tests stay independent |
| `vi.mock` / `vi.hoisted` instead of real subsystems | A29 | Cut real I/O per test | Medium — mock must keep contract |
| Shared lightweight fixtures vs per-test DB setup | A18–A25 | One setup, many tests | Medium — test pollution risk; use transactions/rollback |
| `pool: threads` + `maxConcurrency` tuning | A29 | Better CPU utilization | Watch for flaky shared-state tests |
| `testTimeout` tuning (don't raise blindly) | A29 | Catch runaway tests faster | None |
| Parallel Playwright `workers` + `fullyParallel` | A33–A35 | More specs concurrently | **High** — shared backend state; isolate per-spec |
| Reuse Docker image / warm cache in e2e | A33–A35 | Cut provisioning time | None |
| `test:scripts` — node's built-in runner already parallel | A36 | — | None |
| Measure-first: profile with `--profile-time` / vitest `--reporter=verbose` to find the fat files | all | Target effort where it pays | None |

---

## 7. Per-area journal

> One entry per attempt. Keep the stamp format: `**<area> · <date> · commit <sha> · cold/warm <before>→<after> · technique: <name>`.
> Mark `PLATEAUED` when step 6 of §6 stops yielding ≥5%.

### A01 foundation
- **2026-08-22 · baseline · commit `e0e401f0` · cold 6.9 s / warm 1.3 s / warm 1.0 s (median warm 1.3 s)** — `cargo test -p foundation`; 452 unit + 23 proptests + doctests, all pass. machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200 · runs 6.9/1.3/1.0 s → median 1.3 s. The 6.9 s cold run includes compile; warm runs are ~1 s.
- **2026-08-22 · attempt 1 · commit `e0e401f0` · technique: cost-breakdown analysis → **PLATEAUED** — split measurement: `--lib` (475 tests) runs in **0.3 s**; doctests alone take 1.2 s (0.44 s merged-compile + 0.55 s run). Test runtime is ~0.1 s = ~8% of the 1.3 s median; the rest is cargo test-binary compile/link overhead that `[profile.test]` (strip, debug=1, codegen-units=16) already minimizes. No quality-preserving lever remains at crate level — doctests cannot be dropped (quality guardrail) and proptests are cheap at runtime. Area floor reached; further gains belong to workspace-level A27/A28.
- **2026-08-22 · attempt 2 · commit `e0e401f0`+Cargo.toml · technique: `[profile.test.package.foundation] codegen-units = 256` → **ACCEPTED (cold)** — hypothesis: test profile's `codegen-units = 16` (binary-size tuning) slows codegen; A01's 0.1 s runtime makes a larger, faster-to-compile binary quality-neutral. Measurement (settled after rebuild): cold **6.9 → 4.25 s (−38%)**, warm **1.3 → 1.29 s (flat)**. A transient 1.86 s warm reading during the measurement window was machine noise from concurrent other-agent load (`opencode` ~124k CPU-s) and rustdoc harness relink (~0.43 s per invocation); re-measured 1.29/0.99/1.30 s → median 1.29 s after the noise cleared. Doctest rustdoc rebuild is cargo-internal and not quality-reducible. Area plateaued for warm; cold compile win accepted.

### A02 oz-core
- [ ] baseline pending

### A03 oz-security
- [ ] baseline pending

### A04 oz-reporting
- [ ] baseline pending

### A05 oz-plugin
- [ ] baseline pending

### A06 oz-payment
- [ ] baseline pending

### A07 oz-cli
- [ ] baseline pending

### A08 oz-logging
- [ ] baseline pending

### A09 oz-notification
- [ ] baseline pending

### A10 oz-lua
- [ ] baseline pending

### A11 oz-hal
- [ ] baseline pending

### A12 oz-api
- [ ] baseline pending

### A13 cloud-server
- [ ] baseline pending

### A14 desktop-client
- [ ] baseline pending

### A15 tablet-client
- [ ] baseline pending

### A16 modules
- [ ] baseline pending

### A17 platform
- [ ] baseline pending

### A18 oz-core integration
- [ ] baseline pending

### A19 oz-payment integration
- [ ] baseline pending

### A20 desktop-client integration
- [ ] baseline pending

### A21 oz-hal integration
- [ ] baseline pending

### A22 platform/sync integration
- [ ] baseline pending

### A23 oz-cli integration
- [ ] baseline pending

### A24 cloud-server integration
- [ ] baseline pending

### A25 tax integration
- [ ] baseline pending

### A26 doctests
- [ ] baseline pending

### A27 nextest workspace sweep
- [ ] baseline pending

### A28 cargo fallback sweep
- [ ] baseline pending

### A29 vitest full suite
- [ ] baseline pending

### A30 a11y suite
- [ ] baseline pending

### A31 vitest coverage
- [ ] baseline pending

### A32 vitest per-group
- [ ] baseline pending

### A33 e2e api
- [ ] baseline pending

### A34 e2e perf-smoke
- [ ] baseline pending

### A35 e2e remaining
- [ ] baseline pending

### A36 script tests
- [ ] baseline pending

### A37 check.sh aggregate
- [ ] baseline pending

### A38 check:all aggregate
- [ ] baseline pending

---

## 8. Coordination & boundaries

- **Do NOT touch** areas or files actively being processed by another agent (tender-currency/sale-charges, KDS, topology hooks tests, backup/restore integration). When the two efforts overlap, the other agent's in-flight files win — pick a different area.
- Commit **only own files**; never stage another agent's WIP.
- Version stays locked at **0.0.29** everywhere. No version bumps, ever, unless explicitly requested.
- `cargo clippy` / full `cargo test --workspace` are NOT routine iteration tools (AGENTS.md) — use `cargo check -p <crate>` and targeted test runs during iteration; run the full sweep only for the §5 baseline of A27/A28.

---

## 9. Definition of done (campaign end)

- Every area in §4 has at least one protocol-measured baseline in §5.
- Every area either has ≥1 accepted improvement with before/after numbers, or is explicitly marked `PLATEAUED` with the reason.
- The canonical CI sweep (A27 + A29 + A33–A36) shows a measured, committed reduction vs its baseline, with zero flaky-test regressions.
- All accepted changes are committed locally with `test:perf(area): …` messages citing the measured numbers.

---

## 10. Related documents

- `docs/coverage/README.md` — coverage reports & tooling (coverage is out of scope here)
- `scripts/check.sh` — the local pre-push gate this campaign optimizes
- `scripts/check-ui.mjs` — UI gate runner (`npm run check:all`)
- `ui/vite.config.ts` — Vitest pool/parallelism/timeout configuration
- `docs/decisions/2026-08-09-local-sync-isolated-e2e-harness.md` — e2e harness decisions
- AGENTS.md — test organisation rules (`*_tests.rs` siblings, no inline-heavy tests, nextest)

---

*End of document.*
