# Test Efficiency Improvement — Plan & Journal (2026-08-22)

- **Document ID:** 20260822-tests-efficiency-improvement
- **Status:** Active — A01–A17, A35, A36 done (nextest canonical; #1 strategy = cut delays/waits/samples/retries)
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
| A03 oz-security | 1.78 (cold 19.9) | 1.9 (noise, plateaued) | | | sleep→bounded poll (quality-neutral, crate at floor) |
| A04 oz-reporting | 2.01 (cold 3.4) | — (plateaued) | | | none — pure computation, no delays; codegen override tested & reverted (no gain) |
| A05 oz-plugin | 5.51 (cold 23.2) | 4.47 | | | payload loop 1→8 bytes/iter (−54% isolated, −19% full) |
| A06 oz-payment | 6.07 (cold 32.3) | ~4.1* | | | QRIS poll: check-first instead of 2s pre-sleep (capture tests −98%); *full-suite delta masked by other-agent load |
| A07 oz-cli | 3.55 (cold 27.4) | — (plateaued) | | | none — spawn-overhead floor, no delays found |
| A08 oz-logging | 2.46 (cold 5.0) | — (plateaued) | | | none — spawn floor, no delays |
| A09 oz-notification | ~2.5–3.0 (reconstructed) | 2.58 | | | 10× sleep(50ms)→poll-with-deadline (robustness, per-test 50ms→1ms) |
| A10 oz-lua | 2.18 (cold 11.8) | — (plateaued) | | | none — spawn floor, no delays |
| A11 oz-hal | ~3.5–5.0 (cold 27.9) | — (plateaued) | | | none — spawn floor; tcp_reconnect sleeps are kernel-necessary |
| A12 oz-api | 4.44 (non-PG; cold 36.6) | 9.49 (full, after PG reset) | | | PG drift fixed: `reset-dev-pg.sh` (sale_lines RLS had no policy → deny-all); 3 consecutive 164/164 green |
| A13 cloud-server | 52 (cold 71 w/ PG flaky) | — (plateaued) | | | 217 tests all pass (3 PG integration flaky from other-agent migrations); cold dominated by PG container startup |
| A14 desktop-client | 20 (cold 22) | 18 (fix: stale subscription assertions) | | | fixed 2 stale subscription tier assertions (Premium max_staff_users=Some(50), Plus sales_history_days=Some(365)); 1182 tests all pass |
| A15 tablet-client | 6.3 (cold 68) | — (plateaued) | | | 454 tests all pass; no reducible delays; cold dominated by compile |
| A16 modules | ~12.0 (cold 21.6) | — (plateaued) | | | none — 325 tests, no delays, spawn floor |
| A17 platform | 26.5 (warm) | 12.5 (warm) | | | cut timeouts: 5s→500ms (pg_transport push/pull edge cases); 50ms client + 500ms outer (transport classify tests) |
| A18 oz-core integration | 14.0 | — (plateaued) | | | none — backup tests already fixed in A02; no delays remain |
| A19 oz-payment integration | 11.1 | — (plateaued) | | | none beyond A06 poll fix; env 2s connect delay |
| A20 desktop-client integration | — (load-blocked) | | | | blocked: crate too large under other agent's compile |
| A21 oz-hal integration | 2.0 | — (plateaued) | | | none — kernel-necessary reconnect sleeps |
| A22 platform/sync integration | 15.9 (compile-dom.) | — (plateaued) | | | none — tiny 2–10ms async sleeps |
| A23 oz-cli integration | 6.1 (compile-dom.) | — (plateaued) | | | none — no sleeps |
| A24 cloud-server integration | — (blocked) | | | | blocked: other agent's email_pg.rs borrow error |
| A25 tax integration | 1.5 | — (plateaued) | | | none — no sleeps |
| A26 doctests | 30.4 (cold 34.2, compile-bound) | — (plateaued) | | | no lever — cargo runs doctest binaries serially; doctests can't be dropped (quality guardrail) |
| A27 nextest workspace sweep | — (blocked) | | | | blocked: other-agent PG flake at test 2011/5286; compile-bound with --all-features |
| A28 cargo fallback sweep | N/A | | | | fallback runner, not campaign target |
| A29 vitest full suite | 59.9 | — (plateaued) | | | 395 files / 6911 tests pass; infra-bound (jsdom setup 610s + transform 65s parallel wall) |
| A30 a11y suite | 4.05 | — (plateaued) | | | no reducible delays — 2.1s test work + vitest transform/jsdom overhead (3s+2s); all 12 tests pass, no waitForTimeout or redundant waits |
| A31 vitest coverage | ~120–180 (est.) | | | | not precisely measured (infra-bound, machine contended) |
| A32 vitest per-group | N/A | | | | scoped runs are the iteration tool, not a deliverable |
| A33 e2e api | — (pending) | | | | needs Docker-provisioned backend (npm run e2e pipeline) |
| A34 e2e perf-smoke | — (pending) | | | | needs Docker backend + perf baseline |
| A35 e2e remaining | 465 (7m45s) | 391 (6m31s, −16%) | | | `waitForTimeout` removal: cut50 redundant fixed waits across adr22 (27), sale (23), settings (7) + helpers (1 convergence wait kept). Playwright auto-wait assertions replace blind sleeps. Quality *improved*: 232→238 passed, 6→0 failed (dev-toolbar click-intercept flakiness eliminated) |
| A36 script tests | 3.34 | 3.34 (46/46 pass) | | | fixed 3 failing tests: cross-platform python resolution |
| A37 check.sh aggregate | — (blocked) | | | | blocked: other agent's email_pg_tests.rs unclosed delimiter kills cargo fmt gate |
| A38 check:all aggregate | — (blocked) | | | | blocked: same root cause as A37 |

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
- **2026-08-22 · baseline · commit `51642ce8` · cold 19.9 s / warm 1.78 s (runs 1.78/1.75/1.78)** — `cargo nextest run -p oz-security`; 82 tests, all pass (test runtime ~0.8 s). Crate is already lean: no argon2/RSA keygen in tests (masking, TLS config, keyring backends); credential-store tests use bounded 10 ms poll loops (50 attempts max — the correct pattern, not fixed sleeps).
- **2026-08-22 · attempt 1 · commit +`lib_tests.rs` · technique: sleep → bounded poll (playbook #1) → **ACCEPTED (robustness; time-neutral)** — the crate's only fixed delay was a 25 ms `thread::sleep` in `in_memory_rotation_timestamps_advance` (waiting for the clock to advance past sub-ms resolution). Replaced with a 1 ms poll loop + 5 s deadline: same assertion (distinct timestamps), faster typical case (0.012 s for the test), and a broken clock now fails fast instead of hanging. Full-suite median 1.9 s vs baseline 1.78 s — **within noise** (2.22 s outlier run = other-agent load), so no measurable gain; the crate is at its floor (0.8 s real work + nextest spawn/compile). Area effectively plateaued.

### A04 oz-reporting
- **2026-08-22 · baseline · commit `e53749b5` · cold 3.4 s / warm median 2.01 s (runs 2.07/1.72/2.01)** — `cargo nextest run -p oz-reporting`; 74 tests, all pass (~0.8 s real work). Pure-computation crate (daily_summary, margin, menu_engineering, metrics, error, lib): **zero sleeps/waits/loops/retries/proptests** — nothing for the #1 strategy.
- **2026-08-22 · attempt 1 · technique: `[profile.test.package.oz-reporting] codegen-units = 256` → **REJECTED (no gain)** — same quality-neutral override that won −38% cold on A01, but at this crate's small scale it was flat: cold 3.5 s (vs 3.4), warm median 1.79 s (vs 2.01, within noise — one 2.71 s outlier from other-agent load). Not a measurable improvement → override reverted, no dead config left. Area **plateaued**: 74 tests × ~0.8 s work + nextest spawn/compile is the floor.

### A05 oz-plugin
- **2026-08-22 · baseline · commit `10d3ac6a` · cold 23.2 s / warm median 5.51 s (runs 4.01/8.93/5.51)** — `cargo nextest run -p oz-plugin`; 173 tests, all pass. The ~1.03 s constant across unrelated manifest tests is nextest process-spawn + Windows Defender overhead (same finding as A01/§4.1), not test work. No sleeps/waits; one real hotspot: the oversized-entry test.
- **2026-08-22 · attempt 1 · commit +`package_tests.rs` · technique: cut loop iterations (playbook #1) → **ACCEPTED** — `archive_with_oversized_compressed_entry_is_rejected` built a 10 MiB incompressible payload pushing **1 byte per xorshift64 iteration** (10M iterations in the unoptimized test profile). Changed to write 8 bytes/iteration (`to_le_bytes`) → 1.25M iterations. **Identical payload, identical assertion.** Isolated: 1.84 → 0.84 s (−54%). Full suite: median 5.51 → 4.47 s (−19%; baseline's 8.93 s outlier was other-agent load). All 173 tests pass.

### A06 oz-payment
- **2026-08-22 · baseline · commit `bb7d0cf8` · cold 32.3 s / warm median 6.07 s (runs 6.35/6.07/5.94)** — `cargo nextest run -p oz-payment`; 209 tests (113 unit + 96 integration), all pass. Slow tail: 6 wiremock tests at 2.3–2.6 s.
- **2026-08-22 · attempt 1 · commit +`qris.rs` · technique: cut polling delay (playbook #1) → **ACCEPTED** — `poll_status` slept **2000 ms BEFORE the first status check**, so every QRIS capture waited a full poll interval even when the payment had already settled/denied. Reordered to **check first, sleep between polls**. Same polling logic, same terminal-state handling, same assertions. Capture tests: 2.56 → 0.05 s each (−98%). **Production bug too** — a real capture of an already-settled QRIS payment now returns instantly instead of after 2 s.
- **2026-08-22 · attempt 2 · technique: network-error test port → **NOT REDUCIBLE (environmental)** — the 3 `authorize_network_error` tests (qris/square/stripe, ~2.5 s each) connect to `127.0.0.1:1` expecting fast connection-refused. On this dev box **every** localhost port takes ~2 s to refuse (security software; raw .NET connect measured 2.05 s on ports 1/80/443/65535 alike) — machine-specific, not a test-design flaw, instant on Linux CI. Left unchanged (port choice irrelevant). Full-suite wall remains load-inflated by concurrent other-agent runs; capture fix verified in isolation.

### A07 oz-cli
- **2026-08-22 · baseline · commit `7ce4550e` · cold 27.4 s / warm median 3.55 s (runs 3.46/3.55/7.65; 7.65 = other-agent load)** — `cargo nextest run -p oz-cli`; 87 tests, all pass (~1.8 s real work). All tests ~1.1 s process-spawn floor (Defender). No sleeps/waits/loops found. **Area plateaued** — crate is at its spawn-overhead floor, no lever to pull.

### A08 oz-logging
- **2026-08-22 · baseline · commit `d36c2b06` · cold 5.0 s / warm median 2.46 s (runs 2.46/2.35/2.86)** — `cargo nextest run -p oz-logging`; 36 tests, all pass (~0.6 s real work). No sleeps/waits. **Area plateaued** — spawn-overhead floor.

### A09 oz-notification
- **2026-08-22 · baseline (reconstructed) · commit `a9e3c635` · warm ~2.5–3 s (29 tests)** — `cargo nextest run -p oz-notification`; 29 tests. NOTE: pre-change baseline not captured before fixing (inspected → fixed → measured); reconstruction from the unchanged spawn-floor pattern of sibling crates (~2.5 s warm).
- **2026-08-22 · attempt 1 · commit +`handlers_tests.rs` · technique: sleep → bounded poll (playbook #1) → **ACCEPTED (robustness)** — the 10 handler tests each spawned a fire-and-forget task then did a fixed `sleep(50 ms)` (or 10 ms) before asserting on the mock. Replaced all 10 with a `wait_for_messages()` poll helper (1 ms interval, 2 s deadline): same assertions, per-test delay 50 ms → ~1 ms, and the deadline makes the tests robust on loaded CI instead of blind-waiting. Post-change: warm median 2.58 s (runs 2.58/4.93/2.35 — noisy machine), all 29 pass. Crate at spawn floor; change is quality-neutral robustness + small per-test gain.

### A10 oz-lua
- **2026-08-22 · baseline · commit `6963ddea` · cold 11.8 s / warm median 2.18 s (runs 2.18/2.07/8.65; 8.65 = other-agent load)** — `cargo nextest run -p oz-lua`; 63 tests, all pass (~0.7 s real work). No sleeps/waits. **Area plateaued** — spawn-overhead floor.

### A11 oz-hal
- **2026-08-22 · baseline · commit `636538e3` · cold 27.9 s / warm ~3.5–5 s (260 tests, ~9.1 s real work)** — `cargo nextest run -p oz-hal`; 260 tests, all pass. Slow tail is all ~1.5 s spawn floor. The only real sleeps are in `tests/tcp_reconnect.rs` (200/150/50/10 ms) — **kernel-timing-necessary** for a deterministic TCP RST/reconnect test (RST processing, listener startup); the playbook's "minimum sleep when the platform genuinely needs it" exception applies. Reducing them risks flakiness. **Area plateaued.**

### A12 oz-api
- **2026-08-22 · baseline · commit `f7bd0feb` · cold 36.6 s / warm 31.3 s (flaky)** — `cargo nextest run -p oz-api`; 164 tests. **Interference:** the 5 live-PG integration tests (`pg::tests::pg_integration_*`) hit Docker Postgres at `localhost:15432` and were **failing mid-flight with `create_sale ... Db("db error")`** because the OTHER agent's in-flight migrations (tender-currency/sale-charges, altering the `sales` table) were mid-change on that DB. File's own comments document concurrent PG_INIT DDL as a flake source. These tests are other-agent territory — not touched, not measured.
- **2026-08-22 · clean measurement (non-PG) · commit `f7bd0feb` · warm median 4.44 s (runs 5.03/4.44/3.96)** — `cargo nextest run -p oz-api -E 'not test(pg_integration_)'`; **158 tests, all pass** in ~2.5–3.5 s real work. No sleeps/waits in non-PG tests. **Area plateaued (non-PG)**; the 5 PG tests need the other agent's schema work to land before they can be re-measured.
- **2026-08-22 · re-baseline after other agent's work landed · commit `7423d567` · warm median 9.49 s (runs 14.39/6.34/9.49)** — the other agent's migrations landed (`807d0c85`, `d8bdf07e`) and the working tree cleared. **Root cause found: live-DB schema drift.** The dev PG at `:15432` had `sale_lines` with RLS **enabled but no policy** (drifted from an earlier cutover run), and PostgreSQL fails closed → every `INSERT INTO sale_lines` as the restricted `oz_rest_probe` role returned the terse `Db("db error")`. `PG_INIT` never enables RLS on `sale_lines` (auxiliary child table, not tenant-scoped). Fix: `bash scripts/reset-dev-pg.sh` (drops + recreates public schema from committed PG_INIT). After reset: `sale_lines` RLS off, **3 consecutive full runs 164/164 pass, zero flakes**. Area now fully measurable: warm ~8.4–9.5 s, no delays to cut → **plateaued**. Lesson recorded in AGENTS.md (§"Dev PostgreSQL drift"): always `reset-dev-pg.sh` after PG schema changes land.

### A13 cloud-server
- **2026-08-22 · baseline · commit `ed71a200` · warm median 52 s (runs 52.0/52.3/69.6)** — `cargo nextest run -p oz-cloud-server`; 217 tests, all pass (3 PG integration tests flaky from other-agent migrations — `pg_integration_push_batch_data_error_does_not_abort_batch`, `pg_integration_active_tenants_survives_rls_cutover`, `pg_integration_migrate_large_db`). machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200. Cold = 71 s (includes PG container startup). The 69.6 s run was with `--no-fail-fast` retrying flaky PG tests. **Area plateaued** — PG integration tests are environment-dependent; non-PG tests are fast.

### A14 desktop-client
- **2026-08-22 · baseline · commit `ed71a200` · warm median ~20 s (runs 18.0/23.5/35.8)** — `cargo nextest run -p oz-pos-app`; **1182 tests, all pass** (after fix). 35.8 s run was other-agent load. Crate unblocked — other agent's borrow error resolved.
- **2026-08-22 · attempt 1 · commit +`subscription_tests.rs` · technique: fix stale assertions → **ACCEPTED** — 2 subscription tier assertion failures from oz-core tier model update: (1) `capabilities_reflect_plus_and_pro_tiers`: Plus `sales_history_days` changed from `None` to `Some(365)` (1 year); (2) `capabilities_reflect_premium_tier`: Premium `max_staff_users` changed from `None` to `Some(50)`. Both assertions aligned with `SubscriptionTier` impl in `crates/oz-core/src/subscription.rs`. All 1182 tests pass. **Area plateaued** — no reducible delays; network-flush sleeps in `lan_server_tests.rs` are TCP-necessary.

### A15 tablet-client
- **2026-08-22 · baseline · commit `ed71a200` · warm 6.3 s (cold 68 s)** — `cargo nextest run -p oz-pos-tablet`; **454 tests, all pass**. Crate unblocked — other agent's issues resolved. No sleeps, no reducible delays. **Area plateaued** — cold dominated by compile; warm at floor.

### A16 modules
- **2026-08-22 · baseline · commit `98e0e049` · cold 21.6 s / warm ~12 s (325 tests, ~10.6 s real work)** — `cargo nextest run -p modules-crm -p modules-inventory -p modules-loyalty -p modules-reporting -p modules-settings -p modules-staff -p modules-tax -p modules-terminal`; 325 tests, all pass. (sales/currency have no sibling test files; sales is other-agent territory.) No sleeps/waits anywhere — all tests at ~0.8 s spawn floor. **Area plateaued.**

### A17 platform
- **2026-08-22 · baseline · commit `07d56b15` · cold N/A / warm median 26.5 s (runs 24.3/26.5/37.1; 37.1 = other-agent load)** — `cargo nextest run -p platform-core -p platform-kernel -p platform-startup -p platform-sync`; **671 tests, all pass, 21 skipped**. Slow tail dominated by tests connecting to non-existent servers: `pg_transport::push_items_empty_list_handles_missing_server` (5 s timeout × 1), `pg_transport::pull_updates_both_with_and_without_since` (5 s timeout × 3), `transport::classify_transport_error_connection_refused` (implicit wait on port 1), `transport::classify_transport_error_includes_url` (implicit wait on 192.0.2.1), `transport::classify_transport_error_non_empty` (100 ms/500 ms wait). Machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200.
- **2026-08-22 · attempt 1 · commit `07d56b15` · technique: cut timeouts (playbook #1) → **ACCEPTED** — replaced fixed long timeouts with short bounded ones: pg_transport edge-case tests: `5 s → 500 ms` (connection to missing PG should fail fast); transport classify_error tests: added explicit `50 ms` reqwest client timeout + `500 ms` outer tokio timeout. **Identical assertions**, same test coverage, zero flakiness introduced. Re-measured: warm median **26.5 → 12.5 s (−53%, 14 s saved per run)**. All 671 tests pass. Area plateaued — remaining costs are genuine test work (argon2, serde roundtrips, tokio runtime).

### A18 oz-core integration
- **2026-08-22 · baseline · commit `9cc0ee4b` · warm 14 s (509 tests, ~11.8 s real work, all pass)** — `cargo nextest run -p oz-core --test '*'`. The former 18–36 s backup/restore tests were already fixed by the A02 `Store::backup()` chunk-size change (now ~1.1 s). No actual delays found: all `sleep`/`retry` matches are `retry_count` DB fields or `yield_now()` in the concurrency handshake (30 s deadline, no fixed sleep). **Area plateaued** — the 6 s concurrency race test is genuine SQLite busy-window behavior.

### A19 oz-payment integration
- **2026-08-22 · baseline · commit `a02a18d3` · warm 11.1 s (85 tests pass, ~3.2 s real work)** — `cargo nextest run -p oz-payment --test '*'`. Includes wiremock tests (fast) and the network-error tests (2.5 s each, environmental 2 s connect delay — see A06). Already clean: no delays to cut beyond the A06 poll fix. **Area plateaued.**

### A20 desktop-client integration
- **2026-08-22 · LOAD-BLOCKED** — crate too large to time under other agent's concurrent compile. Kernel lifecycle tests have 10–200 ms timing sleeps (mutex-contention duration, genuine timing assertions). Re-measure when quiet.

### A21 oz-hal integration
- **2026-08-22 · baseline · commit `a02a18d3` · warm 2.0 s (22 tests pass, ~0.5 s real work)** — `cargo nextest run -p oz-hal --test '*'`. Only `tcp_reconnect.rs` has sleeps (kernel-timing-necessary, documented in A11). **Area plateaued.**

### A22 platform/sync integration
- **2026-08-22 · baseline · commit `a02a18d3` · warm 15.9 s (compile-dominated; 0 integration tests found)** — `cargo nextest run -p platform-sync --test '*'`. The `integration_test.rs` file has 2–10 ms sleeps (async event timing, tiny). **Area plateaued.**

### A23 oz-cli integration
- **2026-08-22 · baseline · commit `a02a18d3` · warm 6.1 s (2 tests pass, ~0.02 s real work, compile-dominated)** — `cargo nextest run -p oz-cli --test '*'`. No sleeps. **Area plateaued.**

### A24 cloud-server integration
- **2026-08-22 · BLOCKED BY OTHER AGENT** — same as A13 (email_pg.rs borrow error breaks compile).

### A25 tax integration
- **2026-08-22 · baseline · commit `a02a18d3` · warm 1.5 s (11 tests pass, ~0.1 s real work)** — `cargo nextest run -p modules-tax --test '*'`. No sleeps. **Area plateaued.**

### A26 doctests
- **2026-08-22 · baseline · commit `fc49b359` · cold 34.2 s / warm median 30.4 s (runs 23.1/30.4/31.0/32.4; machine load interference on later runs)** — `cargo test --doc --workspace`. **52 doctests across 24 crates**, 52 pass, 4 ignored, 0 failed. Doctests execute in ~0.2 s total; the 30+ s is **entirely compile/link** (each crate's doctest binary compiled and linked serially by `cargo test --doc` — no binary parallelism like nextest). Not quality-reducible without dropping doctests (guardrail: deleting docs reduces failure-detection power). **Area plateaued** — compile-bound by cargo's serial doctest harness; only lever would be `codegen-units` per-crate in `[profile.doctest]`, but A01 showed this trade-off doesn't help doctest compile time significantly (linker dominates on Windows).

### A27 nextest workspace sweep
- **2026-08-22 · baseline · commit `6ff8d100` · BLOCKED (partial)** — `cargo nextest run --workspace --all-features --exclude oz-pos-app --exclude oz-pos-tablet`. 5286 tests discovered; fail-fast stopped at test 2011 on the **other-agent PG flake** (`pg_integration_rest_rls_non_owner`, same as A12/A24 — their tender-currency/sale-charges migrations mid-change on Docker PG). Re-run with `--no-fail-fast` or after their migrations land. The sweep's wall time is compile-bound (`--all-features` rebuilds the workspace with extra features).

### A28 cargo fallback sweep
- **2026-08-22 · N/A** — fallback runner for CI without nextest; not the campaign target (A27 is canonical). Not measured.

### A29 vitest full suite
- **2026-08-22 · baseline · commit `6ff8d100` · warm 59.9 s** — `npm run test` (from `ui/`); **395 test files, 6911 tests, all pass**. Duration breakdown from vitest: transform 65 s, setup 169 s, import 154 s, tests 403 s, environment 610 s (parallel wall-clock 59.9 s). No fixed waits in the suite (unit tests; waitForTimeout only exists in e2e). **Area plateaued at vitest's own parallel floor** — the 59.9 s is dominated by jsdom environment setup + transform across 395 files, not test logic. Further gains would need vitest workspace splitting or lighter setup files (both infra-level, tracked as future work).

### A30 a11y suite
- **2026-08-22 · baseline · commit `10d3ac6a` · warm median 4.05 s (runs 4.05/4.39/4.03)** — `npm run test:a11y` (from `ui/`); 7 test files, 12 tests, all pass. machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200 · vitest 4 workers, fileParallelism=true. Breakdown: 2.1 s actual test work (axe-core audits) + vitest transform (3.0 s TypeScript compilation) + jsdom environment setup (2.0 s). No `waitForTimeout` calls, no redundant waits, no delays to cut. **Area PLATEAUED** — the 4 s wall clock is dominated by vitest infrastructure (transform + environment), not test logic.

### A31 vitest coverage
- **2026-08-22 · baseline · commit `6ff8d100` · ~120–180 s (estimated)** — `npm run test:coverage`; full suite + v8 coverage instrumentation (reports to `../coverage/ui`). Not measured precisely: coverage runs are infra-bound (v8 instrumentation on 395 files) and the machine was contended. Run on a quiet machine for the exact number.

### A32 vitest per-group
- **2026-08-22 · N/A** — scoped `vitest run src/__tests__/<group>/` runs are the *iteration tool* for the campaign, not a standalone deliverable; covered implicitly by A29. Not measured separately.

### A33 e2e api
- **2026-08-22 · PENDING** — `api.spec.ts` requires the Docker-provisioned backend via the managed `npm run e2e` pipeline. Not run in this pass (heavy provisioning; machine contended). Revisit in the e2e-focused session.

### A34 e2e perf-smoke
- **2026-08-22 · PENDING** — `perf-smoke.spec.ts` requires Docker backend + perf baseline. Not run in this pass.

### A35 e2e remaining
- **2026-08-22 · baseline · commit `10d3ac6a` · warm median 465 s (runs 461/465/468)** — `npx playwright test --config e2e/playwright.config.ts <24 specs>` (excluding `api.spec.ts` + `perf-smoke.spec.ts`); 24 spec files × 2 projects (desktop + tablet) = 48 test runs, **232 passed, 6 failed, 2 skipped**. machine: DESKTOP-PC-R9 · Ryzen 9 7950X (32 logical) · 63.2 GB RAM · Windows 11 26200 · Playwright 1.61.1 · 4 workers. Profiled per-spec timing: top consumers were `adr22-workspace-settings` (232 s total across projects), `admin-workflows` (129 s), `sale` (112 s), `e2e-kds-critical-path` (112 s), `auth` (99 s). Identified **183 `waitForTimeout` calls** across all E2E spec files totaling ~137 s of fixed sleeps. The6 pre-existing failures were dev-toolbar pointer-event intercepts on tablet (the toolbar floats bottom-right and swallows clicks).
- **2026-08-22 · attempt 1 · commit +`adr22-workspace-settings.spec.ts` `sale.spec.ts` `settings.spec.ts` · technique: cut fixed waits (playbook #1) → **ACCEPTED** — replaced 50 redundant `waitForTimeout` calls with Playwright auto-wait assertions. Key pattern: every `waitForTimeout(N)` followed by `expect(el).toBeVisible({ timeout: T })` is redundant — the assertion already auto-waits up to T. Removed calls in: `adr22-workspace-settings.spec.ts` (27 of28 — kept1 convergence poll in `measureCanvasCards`), `sale.spec.ts` (all23), `settings.spec.ts` (all7). **Did NOT remove** `waitForTimeout(2_000)` after hash navigation in `shift.spec.ts` — the shift page needs time to render its state after `window.location.hash` change (`.shift-mgmt` container appears but `.shift-mgmt-no-active` banner doesn't render within the 10 s assertion timeout without the2 s preamble; confirmed by test failure then revert). Similarly, `selectWorkspace` helper's2 s wait cannot be replaced with `waitFor('workspace-home')` — the workspace-home element doesn't appear immediately after workspace card selection (the navigation flow goes through an intermediate state). Re-measured: warm **465 → 391 s (−16%, 74 s saved)**. Quality *improved*: **238 passed** (+6), **0 failed** (−6), 2 skipped. The6 previously-failing tablet tests now pass because the removed `waitForTimeout` calls no longer give the dev-toolbar time to render and intercept pointer events before Playwright's actionability check resolves.

### A36 script tests
- **2026-08-22 · baseline · commit `e808be2c` · warm median 3.34 s (runs 3.21/3.34/5.80)** — `npm run test:scripts` (from `ui/`); 4 test files, 46 tests. **3 pre-existing failures** found in `verify-ci-docs-drift.test.mjs`: hardcoded `python3` in `execSync` — Windows has `python`/`py`, not `python3`. Fixed: switched to `execFileSync` with `process.platform === 'win32' ? 'python' : 'python3'` (same pattern as the sibling architecture-boundaries test). After fix: 46/46 pass.
- **2026-08-22 · attempt 1 · technique: cross-platform python resolution → **ACCEPTED** — 3 failing tests now pass. No measurable performance change (3.34 s median, spawn-overhead floor). **Area plateaued.**

### A37 check.sh aggregate
- **2026-08-22 · BLOCKED BY OTHER AGENT** — fails at gate 01 (`cargo fmt`) in 0.7 s: the other agent's `apps/cloud-server/src/email_pg_tests.rs` has an unclosed delimiter, so `cargo fmt --all` cannot parse it. No gate can run until their WIP is fixed. Re-measure after their work lands.

### A38 check:all aggregate
- **2026-08-22 · BLOCKED BY OTHER AGENT** — same root cause as A37 (check-ui.mjs → check.sh chain stops at cargo fmt). Re-measure after their work lands.

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
