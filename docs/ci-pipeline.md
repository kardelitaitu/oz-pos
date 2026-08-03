# CI Pipeline Dashboard — OZ-POS

<!-- Audit stamp: 2026-08-03 · AUDIT-27 remediation · status: REWRITTEN — matrix and gate policy reconciled with current workflows (ci.yml, e2e-pr.yml, nightly.yml, release.yml, security.yml, docs.yml) and local runners (check.sh, check-ui.mjs) -->

> Last updated: 2026-08-03

## Workflow inventory

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | PR + push to `main` | Required PR/push gate: Rust fmt/clippy/panic-inventory/tests, UI lint/typecheck/tests, Lighthouse, Docker build+scan+smoke, coverage (advisory), dependency audit, fuzz (advisory), skill drift, flaky-quarantine registry, CI-docs-drift, PR security baseline, E2E (3-shard) |
| `e2e-pr.yml` | PR (`ui/e2e/**` + E2E infra only) | Fast, changed-spec E2E complement — main CI already runs full E2E on every PR. `run-e2e.mjs` exits 2 (`SKIPPED-NO-SPEC`) when no spec changed; the workflow treats it as a neutral skip with a notice, never a false pass |
| `nightly.yml` | Daily 03:00 UTC + manual | Full matrix: cross-platform Rust tests, docs, UI shards, E2E shards, release builds, benchmarks, flaky detection |
| `release.yml` | Tag push `v*` | Build + blocking Trivy scan + publish all artifacts |
| `security.yml` | Weekly Monday + manual | Full-tree cargo audit, cargo deny, Trivy scans |
| `docs.yml` | Push to `main` (docs paths) + PR (docs/workflow paths) | cargo doc → GitHub Pages, preceded by the required `ci-docs-drift` gate so a stale job matrix can't be published |
| `android.yml` / `ios.yml` | Push to `main` | Mobile build pipelines |

## Job Matrix (ci.yml)

| Job | Trigger | Runtime | Cache | Shards | Blocks |
|-----|---------|---------|-------|--------|--------|
| `rust-fmt` | PR + push | ~30s | none | — | ✅ Required |
| `rust-panic-inventory` | PR + push | ~10s | none | — | ✅ Required |
| `rust-clippy` | PR + push | ~3min | rust-cache + sccache | — | ✅ Required |
| `rust-test-fast` | PR only | ~2min each | rust-cache + sccache | 5-way | ✅ Required |
| `rust-test-apps` | PR + push | ~3min | rust-cache + sccache | — | ✅ Required (AUDIT-27 CI-01) |
| `sync-slow-tests` | Push only | ~3min | rust-cache + sccache | — | ✅ Required |
| `rust-test-full` | Push only | ~5min | rust-cache + sccache | 2 OS | ✅ Required |
| `ui-lint` | PR + push | ~40s | npm cache | — | ✅ Required |
| `ui-typecheck` | PR + push | ~30s | npm cache | — | ✅ Required |
| `ui-test` | PR + push | ~2min each | npm + vitest cache (per-shard key) | 4-way | ✅ Required |
| `lighthouse` | PR + push | ~2min | npm cache | — | ⚠️ Advisory (≥ 90) |
| `docker` | PR + push | ~3min | Docker layer cache | — | ✅ Required (build + blocking Trivy + smoke) |
| `coverage` | PR + push | ~5min | rust-cache | — | ⚠️ Advisory |
| `audit` | PR + push | ~30s | — | — | ⚠️ Advisory on PR, ✅ Required on push (AUDIT-27 CI-03) |
| `security-pr` | PR only | ~40s | — | — | ✅ Required when manifests changed — fail-closed if base SHA can't be resolved (AUDIT-27 CI-10) |
| `fuzz` | Push only | ~10min | rust-cache | — | ⚠️ Advisory (crash artifacts uploaded) |
| `skill-drift-tests` | PR + push | ~20s | — | — | ✅ Required |
| `flaky-quarantine` | PR + push | ~10s | — | — | ✅ Required (AUDIT-27 CI-09) |
| `windows-config` | PR + push | ~10s | — | — | ✅ Required (AUDIT-28 — NSIS installMode + asInvoker manifests) |
| `ci-docs-drift` | PR + push | ~10s | — | — | ✅ Required (AUDIT-27 CI-08 — verifies this table stays true) |
| `e2e-docker-image` | Push only | ~4min | Docker GHCR | — | Push path |
| `e2e` | PR + push | ~6min each | npm + rust-cache + Docker GHCR | 3-way | ✅ Required |

## Gate manifest — single source of truth (AUDIT-27 CI-08)

All gate **names** and **status** live in `scripts/gates.json`. It is the one place a gate is added, renamed, or re-leveled:

- **Shared gates** (declared by BOTH `check.sh` and `check:all`): UI lint, UI typecheck, UI unit tests, i18n lint, FTL dedupe.
- **check.sh-only** (repo gate): Rust fmt/clippy/tests, migration, skill-drift, panic-inventory, a11y (advisory), feature registry, plugin-guide parity, windows config drift (NSIS installMode + asInvoker), CI docs drift.
- **check:all-only** (UI gate): bundle budget, E2E, perf smoke.
- **CI-only / nightly** gates carry the enforcing `workflow` + `job` and a `status` of `required` | `advisory` | `required-on-push`.

`scripts/verify-ci-docs-drift.py` (wired into `ci.yml`, `nightly.yml`, `docs.yml`, and `check.sh`) derives everything from this manifest and **fails closed** when:

1. a job referenced in the tables below no longer exists in `.github/workflows/*.yml`, or a documented workflow file is missing;
2. a manifest gate is not declared by the runners it lists (`check.sh` / `check:all`);
3. a gate's enforcing job contradicts its status — `required` jobs must NOT set `continue-on-error: true`; `advisory` jobs must (at job or step level); `required-on-push` jobs must gate it on a `${{ ... }}` condition (e.g. the `audit` gate).

`check-ui.mjs` self-audits against the same manifest and fails `check:all` if a manifest `check:all` gate is not declared.

## Required vs advisory policy (AUDIT-27 CI-03)

Checks are split into **required** (block merges) and **advisory** (informational, must not be confused with pass):

- **Required:** format, clippy, panic-inventory, all Rust tests (incl. app crates), UI lint/typecheck/tests, Docker build + blocking Trivy scan, E2E, skill drift, flaky-quarantine registry.
- **Advisory on PR, required on push:** dependency audit (`cargo audit`, `npm audit --audit-level=high`) — findings recorded on PR, blocking on `main` pushes.
- **Reviewed baseline:** `.cargo/audit.toml` documents the single accepted advisory (RUSTSEC-2023-0071, `rsa` medium, no fix available — private key used only for operator-side signing; re-audit on release). Every ignore entry there carries owner/review-date/justification. Adding entries requires review.
- **Advisory:** Lighthouse a11y, coverage, fuzz (crash artifacts uploaded), UI A11y regression suite (`continue-on-error`).

## Caching Strategy

### Rust (cargo)
- **rust-cache** (`Swatinem/rust-cache@v2`): Caches `target/` keyed by `Cargo.lock`. `save-always: true` persists cache even on job failure.
- **sccache** (`mozilla/sccache-action@v0.0.10`): Compiler cache shared across jobs.

### Node.js (npm)
- **npm cache** (`actions/setup-node@v4` with `cache: 'npm'`): Caches `~/.npm` keyed by `package-lock.json`.
- **vitest cache** (`actions/cache@v4`): Persists `node_modules/.cache/vitest`. **Per-shard save key** (`vitest-cache-${{ runner.os }}-${{ matrix.shard }}-...`) so concurrent shard writes don't contend (AUDIT-27 CI-05).

### Docker
- **BuildKit inline cache** (`type=gha` backend).
- **GHCR pre-built images** (`e2e-docker-image` job) on push to main; E2E jobs pull before building.

### E2E
- Playwright browsers via `npx playwright install chromium --with-deps` (desktop-only project runs in ci.yml + nightly).
- The PR E2E workflow (`e2e-pr.yml`) runs the FULL project matrix (desktop + tablet) via `npm run e2e`, so it installs `chromium webkit --with-deps` — the `tablet` project is iPad Pro 11 emulation (WebKit engine).
- Docker layer cache pulled from GHCR before local build.

## Pre-Merge Validation Gates

Enforced via GitHub branch protection (`Settings → Branches → main → Require status checks`). The CI config alone does not auto-enforce these.

| Gate | Job | Blocks Merge |
|------|-----|-------------|
| Format | `rust-fmt` | ✅ Required |
| Panic policy | `rust-panic-inventory` | ✅ Required |
| Lint (Rust) | `rust-clippy` | ✅ Required |
| Lint (UI) | `ui-lint` | ✅ Required |
| TypeCheck | `ui-typecheck` | ✅ Required |
| Unit Tests (Rust) | `rust-test-fast` (5 shards) + `rust-test-apps` | ✅ Required |
| Unit Tests (UI) | `ui-test` (4 shards) | ✅ Required |
| E2E Tests | `e2e` (3 shards) | ✅ Required |
| Docker Build + Scan | `docker` (blocking Trivy CRITICAL/HIGH) | ✅ Required |
| Security (manifests changed) | `security-pr` | ✅ Required |
| Flaky quarantine registry | `flaky-quarantine` | ✅ Required |
| Windows config drift | `windows-config` | ✅ Required |
| Skill Drift | `skill-drift-tests` | ✅ Required |
| CI docs drift | `ci-docs-drift` | ✅ Required |
| Dependency audit | `audit` | ⚠️ Advisory on PR / ✅ on push |
| Lighthouse a11y | `lighthouse` | ⚠️ Advisory (≥ 90) |
| Coverage | `coverage` | ⚠️ Advisory |
| Fuzz | `fuzz` | ⚠️ Advisory |

## Local validation (AUDIT-27 CI-06)

The canonical local entry points and what each covers:

| Command | Covers | Skips |
|---------|--------|-------|
| `bash scripts/check.sh` (root) | Rust fmt/clippy/tests, migration, skill-drift, panic-inventory, UI lint/typecheck/tests, i18n lint, **FTL dedupe**, a11y (advisory), feature registry, plugin-guide parity, windows config drift (NSIS installMode + asInvoker), optional `--docker-dry-run` build | Production UI build, E2E (backend not provisioned) |
| `cd ui && npm run check:all` | UI lint/typecheck/tests, i18n lint, FTL dedupe, bundle budget, E2E (**provisioned** via `npm run e2e` when Docker is up), perf smoke | Rust gates |
| `cd ui && npm run e2e` | Full managed E2E (Docker backend + Vite + Playwright + cleanup) | — |

`check.sh` is the **repository** gate; `check:all` is the **UI-only** gate. Both now share the same gate vocabulary for the gates they have in common, and both record skipped/advisory gates with reasons.

## Flaky-test lifecycle (AUDIT-27 CI-09)

- `scripts/report-flaky.sh` runs the suite N times and lists tests failing intermittently.
- Quarantines live in `scripts/flaky-quarantine.json` — each entry needs `owner`, `issue`, `reason`, `date`, `expiry`.
- `scripts/verify-flaky-quarantine.py` (wired into `flaky-quarantine` CI job and nightly) **fails** on expired, ownerless, or issueless entries.
- `nightly.yml` runs the detector (`flaky-detect`, 2 workspace runs, informational) and a separate fail-closed `flaky-quarantine-registry` verifier, so a detector timeout can never skip the registry gate.

## Failure Modes & Remediation

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `rust-clippy` fails | New warning introduced | Run `cargo clippy --workspace --all-targets -- -D warnings` locally |
| `rust-test-apps` fails | Tauri app crate test/compile failure | The app crates ARE tested now (AUDIT-27 CI-01) — fix the crate; system deps are already installed on the runner |
| `ui-test` act() warning | Component effect fires async without `renderInAct` | Use `renderInAct` / `renderHookInAct` from `ui/src/test-utils/` |
| `e2e` timeout | Server didn't start in time | Check Docker health, Vite port conflict |
| `docker` fails | Binary > 50 MB, or Trivy CRITICAL/HIGH finding | Strip/slim the binary; fix or document the vuln in `.trivyignore` |
| `audit` fails on push | Dependency has known CVE | `cargo update` / pin patched version — this gate is **required on `main` pushes** |
| `flaky-quarantine` fails | An entry is expired / missing issue or owner | Re-investigate the flake, fix it, or renew with an updated issue |
| Cache miss (all jobs slow) | `Cargo.lock` or `package-lock.json` changed | Expected after dependency updates — first run is cold |

## SLO Targets

| Pipeline Phase | Target | Current |
|---------------|--------|---------|
| Total CI (PR, parallel) | < 8 min | ~6 min |
| Rust test (fast, 5 shards + apps) | < 3 min | ~2 min |
| UI test (4 shards) | < 2 min | ~1.5 min |
| E2E (3 shards) | < 8 min | ~6 min |
| Docker build | < 5 min | ~3 min |
