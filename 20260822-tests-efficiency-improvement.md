# Test Efficiency Improvement — Plan & Journal (2026-08-22)

- **Document ID:** 20260822-tests-efficiency-improvement
- **Status:** Active — A01/A02 done (nextest canonical; #1 strategy = cut delays/waits/samples/retries)
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

### 4.1 Rust — unit + integration tests (per crate, nextest)

Run from repo root: `cargo nextest run -p <crate>`. **nextest is the canonical runner for ALL Rust areas** (per 2026-08-22 decision): it is what `check.sh` / CI use, runs each test in its own process, and parallelizes across cores — and unlike `cargo test` it runs unit **and** integration binaries together, in parallel. Doctests are NOT covered by nextest — they live in A26. Scoping: `--lib` (unit only), `--test <name>` (one integration binary), `-E 'binary(<name>)'` (filter expression). A `slow-tests` feature gate exists in `platform/sync` (not oz-core) to keep the default suite fast while heavy tests remain runnable on demand.

> **Windows caveat (measured 2026-08-22):** nextest spawns one process per test. On Windows, Defender scans each spawned exe (~1.5 s per process on this machine), so for **runtime-trivial crates** (foundation: 475 tests, 0.04 s actual work) nextest is *slower* than cargo test: **8.65 s vs 1.3 s**. nextest wins when test work is real and parallelizable (oz-core: 98.9 → 31.7 s). Rule of thumb: if a crate's `cargo test` warm time is < ~5 s AND dominated by process overhead, keep `cargo test` for that crate and record both numbers; otherwise use nextest.

| Area | Command | Unit-test files |
|------|---------|-----------------|
| A01 | `cargo nextest run -p foundation` | 1 |
| A02 | `cargo nextest run -p oz-core` | 82 (+ 23 integration) |
| A03 | `cargo nextest run -p oz-security` | 7 |
| A04 | `cargo nextest run -p oz-reporting` | 6 |
| A05 | `cargo nextest run -p oz-plugin` | 6 |
| A06 | `cargo nextest run -p oz-payment` | 5 |
| A07 | `cargo nextest run -p oz-cli` | 5 |
| A08 | `cargo nextest run -p oz-logging` | 5 |
| A09 | `cargo nextest run -p oz-notification` | 4 |
| A10 | `cargo nextest run -p oz-lua` | 3 |
| A11 | `cargo nextest run -p oz-hal` | 3 |
| A12 | `cargo nextest run -p oz-api` | 3 |
| A13 | `cargo nextest run -p apps/cloud-server` | 14 |
| A14 | `cargo nextest run -p apps/desktop-client` | 5 (+ 6 integration) |
| A15 | `cargo nextest run -p apps/tablet-client` | 2 |
| A16 | `cargo nextest run -p modules/…` (sales, inventory, crm, tax, settings, staff, reporting, terminal, currency, loyalty) | 2 each (~20 total) |
| A17 | `cargo nextest run -p platform/…` (core, kernel, startup, sync) | 2 sync + others |

> **A01/A02 baselines were measured before the nextest decision** (cargo test). A01 is 1.3 s warm either way; A02's canonical measurement is now nextest (see §5/§7). When re-baselining an old area, re-measure with nextest and keep both stamps.

### 4.2 Rust — integration tests (top-level `tests/` dirs)

nextest runs these **inside the A01–A17 crate commands** (same process pool, parallel). The rows below exist for measuring an integration-only baseline or isolating a slow binary.

| Area | Command | Integration files |
|------|---------|-------------------|
| A18 | `cargo nextest run -p oz-core --test '*'` | 23 |
| A19 | `cargo nextest run -p oz-payment --test '*'` | 7 |
| A20 | `cargo nextest run -p apps/desktop-client --test '*'` | 6 |
| A21 | `cargo nextest run -p oz-hal --test '*'` | 2 |
| A22 | `cargo nextest run -p platform/sync --test '*'` | 2 |
| A23 | `cargo nextest run -p oz-cli --test '*'` | 1 |
| A24 | `cargo nextest run -p apps/cloud-server --test '*'` | 1 |
| A25 | `cargo nextest run -p modules/tax --test '*'` | 1 |

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
| A01 foundation | 1.3 (cold 6.9) | 1.29 (cold 4.25) | | | `codegen-units=256` test-profile override (cold −38%, warm flat); cargo test kept (nextest 8.65 s — Windows spawn overhead, see §4.1) |
| A02 oz-core | 58.5 (cold 58.2) | 31.7 | | | nextest runner (cold −41%); backup chunk 5→512 pgs (warm −46%) |
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
3. **Hypothesize** — the **#1 strategy is to reduce delay, waiting, sample counts, retries, and repeats** (fixed `sleep`s, polling intervals, proptest case counts, loop iteration counts, retry loops). These are pure waste when they exceed what the assertion actually needs: cut them first, because they are usually quality-neutral (the same assertion runs, just fewer redundant times). Only when no delay/wait/repeat remains should you reach for runner/parallelism techniques (§6.1). State the expected mechanism and target before changing anything.
4. **Implement** — smallest change that tests the hypothesis. Keep the change isolated so it can be reverted cleanly.
5. **Re-measure** — same protocol. If warm/cold improved and the area's tests still pass **with identical assertions**, record it as the next improvement column in §5 and log the diff in §7.
6. **Repeat** — go back to step 3 for the same area. Stop when two consecutive attempts produce <5% gain or a regression risk, and mark the area **plateaued** in §7.
7. **Quality gate** — before accepting ANY improvement, run the area's full suite + the two neighbouring areas to confirm no cross-area breakage. Flaky = roll back.
8. **Commit** — one commit per accepted improvement (`test:perf(area): …`), message stating measured before/after. Local commit only; no push without explicit instruction.

### 6.1 Playbook of proven techniques (reference)

> **Priority order:** attack rows in this order. (1) Cut delay/waits/samples/retries/repeats — the #1 strategy. (2) Runner & parallelism. (3) Compile-time. (4) Fixtures/mocks. Only move down a level once the level above is exhausted on the current area.

| # | Technique | Applies to | Mechanism | Quality risk |
|---|-----------|-----------|-----------|--------------|
| 1 | **Cut fixed delays & sleeps** (`thread::sleep`, `sleep_ms`, `tokio::time::sleep`) | all Rust | Replace a `sleep` that merely waits for a state change with a condition poll / channel / `std::sync::Condvar` handshake; keep the minimum sleep only when the platform genuinely needs it (e.g. SQLite busy windows) | Low — but a poll needs a bounded deadline to stay deterministic |
| 1 | **Cut polling intervals & wait deadlines** (`wait_for_flag`, retry backoff `sleep`) | all Rust | Poll at 1–10 ms with a short deadline instead of 100–250 ms; shorten handshake timeouts to just above the real platform bound | Low — must keep the deadline ≥ worst-case platform time or tests become flaky |
| 1 | **Cut sample / case counts** (proptest `#![proptest_config]`, `Strategy::prop_map`, iteration counts) | foundation, oz-core | Fewer proptest cases (e.g. 256 → 64) still exercise the same property; 1000-iteration loops can become 100 while keeping the boundary cases explicit | Medium — verify the reduced count still hits the interesting cases (boundaries, overflow); keep edge-case assertions explicit |
| 1 | **Cut retry & repeat loops** (`retries`, `for _ in 0..N` re-runs) | all | A deterministic test needs no retry; convert "try N times then assert" into a single attempt + precise assertion, or a bounded poll | Low — only safe when the assertion is deterministic; never remove a retry that guards real flakiness |
| 1 | **Shrink fixture setup work** (seeding N rows where M suffice) | A18–A25 | 100-row seeds that only need 3 rows; `INSERT` loops replaced by batch inserts | Low — keep enough data to exercise indexes/joins |
| 2 | `nextest` instead of `cargo test` | A01–A28 | Per-test process isolation, parallel by default, faster re-runs (~4.5× claimed) | None |
| 2 | `--test-threads` / nextest `--test-threads` tuning | A01–A28 | More parallelism on many-core hosts | Watch for resource contention/flakiness |
| 3 | Test profile: `strip`, `debug=1`, `codegen-units` | A01–A28 | Smaller/faster test binaries | None |
| 3 | `profile.tdd` for tight loops | A01–A28 | Fastest possible dev compile | Dev-only, not CI |
| 4 | Split one heavy test file into several (Vitest parallelism) | A29 | `fileParallelism` balances across workers | None if tests stay independent |
| 4 | `vi.mock` / `vi.hoisted` instead of real subsystems | A29 | Cut real I/O per test | Medium — mock must keep contract |
| 4 | Shared lightweight fixtures vs per-test DB setup | A18–A25 | One setup, many tests | Medium — test pollution risk; use transactions/rollback |
| 2 | `pool: threads` + `maxConcurrency` tuning | A29 | Better CPU utilization | Watch for flaky shared-state tests |
| 4 | `testTimeout` tuning (don't raise blindly) | A29 | Catch runaway tests faster | None |
| 2 | Parallel Playwright `workers` + `fullyParallel` | A33–A35 | More specs concurrently | **High** — shared backend state; isolate per-spec |
| 4 | Reuse Docker image / warm cache in e2e | A33–A35 | Cut provisioning time | None |
| 4 | `test:scripts` — node's built-in runner already parallel | A36 | — | None |
| 1 | Feature-gate slow tests (`slow-tests`, in `platform/sync`) | A17, A22 | Default suite skips heavy tests; run on demand | **High** if gate default is wrong — verify CI runs the gate |
| 1 | Measure-first: profile with `--profile-time` / vitest `--reporter=verbose` to find the fat files | all | Target effort where it pays | None |

---

## 7. Per-area journal

> One entry per attempt. Keep the stamp format: `**<area> · <date> · commit <sha> · cold/warm <before>→<after> · technique: <name>`.
> Mark `PLATEAUED` when step 6 of §6 stops yielding ≥5%.

### A01 foundation
- **2026-08-22 · baseline · commit `e0e401f0` · cold 6.9 s / warm 1.3 s / warm 1.0 s (median warm 1.3 s)** — `cargo test -p foundation`; 452 unit + 23 proptests + doctests, all pass. machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200 · runs 6.9/1.3/1.0 s → median 1.3 s. The 6.9 s cold run includes compile; warm runs are ~1 s.
- **2026-08-22 · attempt 1 · commit `e0e401f0` · technique: cost-breakdown analysis → **PLATEAUED** — split measurement: `--lib` (475 tests) runs in **0.3 s**; doctests alone take 1.2 s (0.44 s merged-compile + 0.55 s run). Test runtime is ~0.1 s = ~8% of the 1.3 s median; the rest is cargo test-binary compile/link overhead that `[profile.test]` (strip, debug=1, codegen-units=16) already minimizes. No quality-preserving lever remains at crate level — doctests cannot be dropped (quality guardrail) and proptests are cheap at runtime. Area floor reached; further gains belong to workspace-level A27/A28.
- **2026-08-22 · attempt 2 · commit `e0e401f0`+Cargo.toml · technique: `[profile.test.package.foundation] codegen-units = 256` → **ACCEPTED (cold)** — hypothesis: test profile's `codegen-units = 16` (binary-size tuning) slows codegen; A01's 0.1 s runtime makes a larger, faster-to-compile binary quality-neutral. Measurement (settled after rebuild): cold **6.9 → 4.25 s (−38%)**, warm **1.3 → 1.29 s (flat)**. A transient 1.86 s warm reading during the measurement window was machine noise from concurrent other-agent load (`opencode` ~124k CPU-s) and rustdoc harness relink (~0.43 s per invocation); re-measured 1.29/0.99/1.30 s → median 1.29 s after the noise cleared. Doctest rustdoc rebuild is cargo-internal and not quality-reducible. Area plateaued for warm; cold compile win accepted.
- **2026-08-22 · nextest re-measurement · commit `f75badc9` · warm 8.65 s (runs 7.84/8.65/9.57)** — `cargo nextest run -p foundation`; 452 tests, all pass. **nextest is SLOWER here** (1.3 s cargo test → 8.65 s nextest): per-test process spawn + Windows Defender scan of each spawned exe (~1.5 s/test) dwarfs the 0.04 s of real test work. Kept cargo test as A01's canonical runner; this is the documented Windows caveat in §4.1.

### A02 oz-core
- **2026-08-22 · baseline (cargo test) · commit `e0e401f0` · cold 98.9 s** — `cargo test -p oz-core`; **3 pre-existing failures** found: `test_plus_quota_limits`, `test_pro_quota_limits` (`subscription_tests.rs`) and `enforce_store_quota_premium_allows_nine` (`store_profiles_tests.rs`) — stale assertions from pricing commit `668f8078` (Plus=1yr / Pro=5yr history, Premium=5 stores) that the code already implements. Repaired all 3 (assertions aligned with implementation + sibling `test_free_history_limit`; stale "up to 10 stores" doc comment in `subscription.rs` updated to 5). After repair: lib 2016 passed, full `cargo test` green.
- **2026-08-22 · runner switch · cargo test → cargo nextest · cold 58.2 s / warm 58.5 s (2525 tests, all pass)** — per user directive, A02 canonical runner is now **nextest** (also the check.sh / CI runner; per-test process isolation + full parallelism). −41% cold vs cargo test. Doctests move to A26.
- **2026-08-22 · attempt 1 · `Store::backup()` chunk size · warm 58.5 → 31.7 s (−46%)** — root cause: `run_to_completion(5, 250 ms)` copied 5 pages/chunk with a 250 ms pause; a ~1.4 MB fresh DB (~355 pages) incurred ~71 sleeps ≈ 18 s per backup test. Changed to `run_to_completion(512, 10 ms)` — one chunk, ~2 MB granularity, still yields to concurrent writers (online-backup contract preserved). **Production bug too** (`desktop-client data.rs:154`, `sync.rs:647`, `oz-cli commands.rs:190` — a real 1.4 MB backup took ~18 s). Backup tests: 18 s → 0.07 s. All 2525 tests pass.
- **2026-08-22 · plateau check** — new slow tail: 6.13 s `same_store_racing_writers_serialize_exactly_one_wins` (genuine SQLite busy-race handshake, documented ~5 s platform behavior — not a fixed sleep), 4.27 s `verify_tampered_payload_fails` (argon2 KDF, deliberately slow — security-sensitive, do not touch), 2.6 s sync tests. Remaining costs are inherent; further gains belong to A18 (integration) / A27 (workspace sweep).

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
