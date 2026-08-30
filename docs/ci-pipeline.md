# CI Pipeline Documentation

> **Canonical CI dashboard** (AUDIT-27 CI-08). This document is the single source of truth for what jobs run in CI, what gates they map to, and which workflows exist. It is verified by `scripts/verify-ci-docs-drift.py` on every PR and local `check.sh` run.

---

## Job Matrix (ci.yml)

| Job ID | Blocks Merge | Workflow | Notes |
|--------|--------------|----------|-------|
| `rust-fmt` | ✅ Required | ci.yml | `cargo fmt --all -- --check` |
| `go` | ✅ Required | ci.yml | `gofmt` + `go vet` + `go test -short` on license-server |
| `unified-healthcheck` | ✅ Required | ci.yml | POSIX sh healthcheck script test |
| `rust-panic-inventory` | ✅ Required | ci.yml | Scan production unwrap/expect |
| `rust-money-format` | ✅ Required | ci.yml | No hardcoded exp-2 money formatting |
| `architecture-boundaries` | ✅ Required | ci.yml | Static boundary enforcement |
| `rust-clippy` | ✅ Required | ci.yml | `cargo clippy --workspace --all-targets --all-features` |
| `rust-test-fast` | ✅ Required | ci.yml | Sharded crate-group tests (PR only) |
| `sync-slow-tests` | ⚠️ Advisory on PR, ✅ Required on push | ci.yml | Platform-sync integration suite (gated) |
| `rust-test-full` | Push path | ci.yml | Full workspace tests (push only) |
| `rust-test-apps` | ✅ Required | ci.yml | App crate unit tests |
| `ui-lint` | ✅ Required | ci.yml | `npm run lint` |
| `ui-typecheck` | ✅ Required | ci.yml | `npm run typecheck` |
| `ui-test` | ✅ Required | ci.yml | `npm run test` (4 shards) |
| `lighthouse` | ⚠️ Advisory | ci.yml | Lighthouse a11y audit (continue-on-error) |
| `docker` | ✅ Required | ci.yml | Build + Trivy scan + Compose smoke |
| `coverage` | ⚠️ Advisory | ci.yml | Coverage report (continue-on-error) |
| `audit` | ⚠️ Advisory on PR, ✅ Required on push | ci.yml | `cargo audit` + `npm audit` |
| `security-pr` | ✅ Required | ci.yml | PR baseline security audit |
| `fuzz` | ⚠️ Advisory | ci.yml | Fuzz tests (gated) |
| `flaky-quarantine` | ✅ Required | ci.yml | Flaky quarantine registry |
| `windows-config` | ✅ Required | ci.yml | NSIS installMode + asInvoker check |
| `skill-drift-tests` | ✅ Required | ci.yml | Skill drift guard bats tests |
| `ci-docs-drift` | ✅ Required | ci.yml | This document vs workflows |
| `e2e-docker-image` | Push path | ci.yml | GHCR push (main only) |
| `e2e` | ✅ Required | ci.yml | Playwright E2E (3 shards) |

---

## Pre-Merge Validation Gates

| Gate | Job (ci.yml) | Status | Runners |
|------|--------------|--------|---------|
| UI lint | `ui-lint` | Required | `check.sh` (ui lint), `check:all` (eslint) |
| UI typecheck | `ui-typecheck` | Required | `check.sh` (ui typecheck), `check:all` (type check) |
| UI unit tests | `ui-test` | Required | `check.sh` (ui test), `check:all` (unit tests) |
| i18n lint | `ui-test` | Required | `check.sh` (i18n lint), `check:all` (i18n lint) |
| FTL dedupe | — | Required | `check.sh` (ftl dedupe), `check:all` (ftl dedupe) |
| Rust fmt | `rust-fmt` | Required | `check.sh` (cargo fmt) |
| Clippy | `rust-clippy` | Required | `check.sh` (clippy) |
| Rust tests | `rust-test-fast` | Required | `check.sh` (test workspace, test doctests) |
| Architecture boundaries | `architecture-boundaries` | Required | `check.sh` (architecture boundaries) |
| No raw params (ADR #7 Phase 4) | — | Required | `check.sh` (no-raw-params) |
| No hardcoded money format | `rust-money-format` | Required | `check.sh` (hardcoded-money-format) |
| Docker build smoke | — | Required | `check.sh` (docker build) |
| Migration smoke | — | Required | `check.sh` (migration) |
| Skill drift guard | `skill-drift-tests` | Required | `check.sh` (skill-drift) |
| Panic inventory | `rust-panic-inventory` | Required | `check.sh` (panic-inventory) |
| A11y regression | `ui-test` | Advisory | `check.sh` (a11y) |
| Feature registry parity | — | Required | `check.sh` (feature registry) |
| Plugin-guide parity | — | Required | `check.sh` (plugin-guide parity) |
| CI docs drift | `ci-docs-drift` | Required | `check.sh` (ci docs drift) |
| Windows config drift | `windows-config` | Required | `check.sh` (windows config) |
| Unified healthcheck | `unified-healthcheck` | Required | `check.sh` (healthcheck script test) |
| Bundle budget | — | Required | `check:all` (bundle budget) |
| E2E tests | `e2e` | Required | `check:all` (e2e) |
| Perf smoke | — | Required | `check:all` (perf smoke) |
| Rust test apps | `rust-test-apps` | Required | — |
| Rust test full | `rust-test-full` | Required | — |
| Sync slow tests | `sync-slow-tests` | Required | — |
| Docker build + scan | `docker` | Required | — |
| Security PR baseline | `security-pr` | Required | — |
| Lighthouse a11y | `lighthouse` | Advisory | — |
| Coverage | `coverage` | Advisory | — |
| Dependency audit | `audit` | Required on push | — |
| Fuzz | `fuzz` | Advisory | — |
| Flaky quarantine registry | `flaky-quarantine` | Required | — |
| E2E Docker image | `e2e-docker-image` | Required | — |
| Nightly rust test | `rust-test` (nightly.yml) | Required | — |
| Nightly rust doc | `rust-doc` (nightly.yml) | Required | — |
| Nightly UI tests | `ui-test` (nightly.yml) | Required | — |
| Nightly E2E | `e2e` (nightly.yml) | Required | — |
| Nightly flaky detection | `flaky-detect` (nightly.yml) | Advisory (step) | — |
| Nightly flaky registry | `flaky-quarantine-registry` (nightly.yml) | Required | — |
| Nightly benchmarks | `benchmarks` (nightly.yml) | Required | — |
| Nightly license-server full Go tests | `license-server-test` (nightly.yml) | Required | — |

---

## Workflow inventory

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | push/PR to main | Primary CI pipeline (lint, test, build, scan) |
| `nightly.yml` | schedule (daily) + dispatch | Nightly Rust/doc/UI/E2E + flaky detection |
| `release.yml` | tag push (v*) | Release build, sign, attest, publish |
| `security.yml` | schedule (weekly) + dispatch | Cargo audit/deny + container scan |
| `android.yml` | push/PR to main | Android build |
| `ios.yml` | push/PR to main | iOS build |
| `e2e-pr.yml` | PR to main | E2E on PRs |
| `deploy.yml` | push to main | Website deploy |
| `docker-digest-drift.yml` | schedule | Docker digest drift check |
| `docker-persistence.yml` | schedule | Docker persistence check |
| `website.yml` | push to main | Website build + deploy |

---

## Gate Vocabulary

The gate vocabulary is defined in `scripts/gates.json` and shared by:
- `.github/workflows/ci.yml` (CI jobs)
- `.github/workflows/nightly.yml` (nightly jobs)
- `scripts/check.sh` (local pre-push)
- `scripts/check-ui.mjs` (`npm run check:all`)
- `scripts/verify-ci-docs-drift.py` (this document)

Every gate has:
- **id** — stable identifier
- **label** — human-readable name
- **status** — `required` | `advisory` | `required-on-push`
- **runners** — which local runners declare it (`check.sh`, `check:all`)
- **ci** — workflow + job mapping (for CI enforcement)

### Status semantics

| Status | Meaning | Workflow enforcement |
|--------|---------|---------------------|
| `required` | Must pass on every PR and push | Job has NO `continue-on-error` |
| `advisory` | Reports status, never blocks merge | Job/step HAS `continue-on-error: true` |
| `required-on-push` | Advisory on PR, required on push | Job has conditional `continue-on-error: ${{ ... }}` |

---

## Local runners

### `scripts/check.sh` (bash)

Comprehensive pre-push gate mirroring CI. Runs:
1. `cargo fmt`
2. `cargo clippy --workspace`
3. No raw params (ADR #7)
4. Architecture boundaries
5. No hardcoded money format
6. `cargo nextest run --workspace`
7. Migration smoke
8. Skill drift guard
9. Panic inventory
10. `npm ci` + UI lint/typecheck/test
11. i18n lint
12. FTL dedupe
13. Feature registry parity
14. Topology contract parity
15. Plugin-guide parity
16. Windows config drift
17. Release toolchain self-tests
17. Healthcheck script test
18. CI docs drift
19. Optional: Docker build (`--docker-dry-run`)

### `scripts/check-ui.mjs` (Node, cross-platform)

`npm run check:all` from `ui/` directory. Runs:
1. ESLint
2. TypeScript typecheck
3. Unit tests (vitest)
4. i18n lint
5. FTL dedupe
6. Bundle budget
7. E2E tests (if Docker available)
8. Perf smoke (if Playwright available)

---

## Adding a new gate

1. Add entry to `scripts/gates.json` with `id`, `label`, `status`, `runners`, and `ci` mapping
2. Add the gate declaration to `scripts/check.sh` and/or `scripts/check-ui.mjs`
3. Add the corresponding job to the appropriate workflow (`.github/workflows/*.yml`)
4. Update this document (`docs/ci-pipeline.md`) — the Job Matrix and Pre-Merge Validation Gates tables
5. Run `python3 scripts/verify-ci-docs-drift.py` locally to verify

---

## Removing a gate

1. Remove from `scripts/gates.json`
2. Remove from `scripts/check.sh` and/or `scripts/check-ui.mjs`
3. Remove or disable the corresponding workflow job
4. Update this document
5. Run `python3 scripts/verify-ci-docs-drift.py` to verify

---

*Generated and maintained by the OZ-POS team. Last verified by `verify-ci-docs-drift.py`.*