# CI Pipeline Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** CI pipeline — gate completeness, test coverage, cache efficiency, flaky-test handling, security checks, artifacts, and E2E orchestration
> **Status:** AUDITED · several required-gate and pipeline-consistency findings require remediation
> **Production code changed:** None

## Scope

This audit evaluates sector 27 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers the main CI workflow, PR E2E workflow, nightly matrix, release workflow, security workflow, local validation scripts, test runners, cache configuration, artifact retention, failure propagation, branch/event conditions, and flaky-test reporting.

Inspected areas:

- `.github/workflows/ci.yml`
- `.github/workflows/e2e-pr.yml`
- `.github/workflows/nightly.yml`
- `.github/workflows/release.yml`
- `.github/workflows/security.yml`
- `.github/workflows/docs.yml`
- `scripts/check.sh`
- `scripts/check-ui.mjs`
- `scripts/run-e2e.mjs`
- `scripts/report-flaky.sh`
- `ui/package.json`
- `docs/ci-pipeline.md`
- `CONTRIBUTING.md`

## Architecture summary

The main `CI` workflow runs Rust formatting and clippy, PR-only fast Rust tests, push-only full Rust tests, UI lint/typecheck/tests, Lighthouse, Docker build and scanning, coverage, dependency audits, fuzzing, skill-drift tests, and a three-way E2E matrix. Separate workflows cover PR E2E, nightly cross-platform/full-matrix validation, releases, security audits, documentation, Android, and iOS.

The pipeline is intentionally parallel and uses `rust-cache`, sccache, npm cache, Vitest transform cache, and BuildKit/GHA cache. UI and Playwright tests are sharded. Local validation is split between the Bash `scripts/check.sh` gate and the cross-platform `ui` `check:all` runner. Flaky detection exists as a reporting script, but no executable quarantine registry or required flake-resolution workflow was found.

The overall structure is broad, but enforcement is uneven. Some jobs are explicitly informational, one PR Rust shard suppresses application test failures, dependency and container scans do not block, PR E2E can skip all tests when no E2E spec changed, and documentation describes an older job matrix. These gaps make the green status less representative than the apparent breadth of the pipeline.

## Findings

### CI-01 — PR application tests are explicitly allowed to fail

**Evidence:** `.github/workflows/ci.yml:116-120` runs `cargo nextest` for `oz-cloud-server`, `oz-pos-app`, and `oz-pos-tablet` with `2>/dev/null || true`. Any compilation or test failure in those packages is discarded, and the loop continues with a successful shell status. The other Rust shard loops use `|| exit 1` at lines 102-114.

**Impact:** A pull request can report a passing `rust-test-fast (apps)` job while application tests did not compile or failed. This is a direct regression escape in the PR gate, particularly affecting the desktop/tablet command surfaces that are most likely to diverge from library tests.

**Severity:** P1 · required-test integrity

**Affected files:** `.github/workflows/ci.yml`, `apps/cloud-server`, `apps/desktop-client`, `apps/tablet-client`, and the branch-protection check for `Rust Test Fast (apps)`.

**Recommendation:** Remove `2>/dev/null || true`. If a package is intentionally unavailable on a runner, encode that as an explicit matrix exclusion or a separately named allowed-skip condition that fails on unexpected errors. Keep stderr visible and add a workflow smoke test or static check that rejects failure-swallowing operators in required test commands.

**Status:** Open

### CI-02 — PR E2E `--changed-only` can skip the entire suite for production changes

**Evidence:** `.github/workflows/e2e-pr.yml:121-127` always passes `--changed-only` on pull requests. `scripts/run-e2e.mjs:getChangedSpecs` only looks for changed files matching `ui/e2e/*.spec.ts`; when no such spec is changed, `runPlaywright` logs “Changed-only: no E2E specs changed — skipping” and returns success without running Playwright. The workflow is triggered by all `ui/**` changes at `.github/workflows/e2e-pr.yml:24-29`, not only E2E specs.

**Impact:** A production UI, API client, routing, localization, or state-management change can receive a green PR E2E check with zero browser tests executed. The result is especially misleading because the workflow still builds Docker images and uploads an apparently successful E2E result artifact.

**Severity:** P1 · regression-detection gap

**Affected files:** `.github/workflows/e2e-pr.yml`, `scripts/run-e2e.mjs`, `ui/e2e/`, and pull-request path filters.

**Recommendation:** Decide explicitly between a true changed-spec optimization and a required smoke suite. At minimum, run a small critical-path smoke set whenever production code changes, and skip only when the change is documentation/test-only through a path-aware job condition. Report a distinct `skipped-no-spec` result rather than treating zero executed tests as “all tests passed.”

**Status:** Open

### CI-03 — Accessibility, dependency, container, coverage, and fuzz checks are informational despite quality claims

**Evidence:** `.github/workflows/ci.yml:238-240` marks the A11y regression job `continue-on-error: true`; the coverage job at `:297-300` is non-blocking; dependency audit at `:333-351` uses both `continue-on-error: true` and `cargo audit || true` / `npm audit ... || true`; fuzzing at `:353-374` is non-blocking and suppresses each fuzz-target failure; and the Docker Trivy scan at `:279-287` uses `continue-on-error: true` plus `exit-code: 0`. `.github/workflows/security.yml` is strict when manually or weekly scheduled, but is not part of the main PR workflow.

**Impact:** The repository can merge or release with known high-severity dependency/container findings, accessibility regressions, coverage failures, or fuzz crashes unless a maintainer notices an informational artifact. The distinction between required and advisory checks is not consistently visible in the user-facing status set.

**Severity:** P1 · quality/security enforcement

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/security.yml`, and branch-protection configuration.

**Recommendation:** Split checks into explicit `required` and `advisory` jobs. Make at least release-image scanning and high/critical dependency vulnerabilities blocking after a reviewed baseline; promote stable A11y checks incrementally; preserve fuzzing as advisory while creating an issue/artifact on crashes. Publish the required/advisory policy in the workflow and `docs/ci-pipeline.md`.

**Status:** Open

### CI-04 — Test and E2E output pipelines rely on implicit shell pipe behavior

**Evidence:** `.github/workflows/ci.yml:209-210` runs Vitest through `... 2>&1 | tee ui/vitest-output.log`, and `:518-520` runs Playwright through `... 2>&1 | tee e2e-output.log`; nightly uses the same patterns at `:117-118` and `:219-221`. GitHub-hosted Bash generally enables `-e` and `pipefail` for `run` steps, but the failure-propagation contract is implicit rather than declared in the workflow. The local scripts also use pipeline-heavy logging, including `scripts/report-flaky.sh:62-68`.

**Impact:** A future shell change, alternate runner shell, or command wrapper can cause test failures to be hidden behind a successful `tee`. This is a maintainability and observability risk rather than a confirmed current failure on GitHub's default Bash.

**Severity:** P2 · failure-reporting reliability

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `scripts/report-flaky.sh`, and workflow shell configuration.

**Recommendation:** Declare `set -euo pipefail` at the beginning of multi-line Bash steps or use `shell: bash --noprofile --norc -eo pipefail {0}`. Prefer writing command output with an explicit status capture when artifacts are needed. Add a small CI-script test that intentionally returns nonzero before `tee` and confirms the wrapper fails.

**Status:** Open

### CI-05 — Parallel Vitest shards share one writable cache key

**Evidence:** `.github/workflows/ci.yml:203-207` gives all four UI shards the same `vitest-cache-${{ runner.os }}-${{ hashFiles('ui/package-lock.json') }}` key. Nightly repeats the pattern at `.github/workflows/nightly.yml:111-115`. The cache is a mutable transform cache written by every shard; the key does not include the shard number or a writer role.

**Impact:** Concurrent cache saves can contend or produce last-writer behavior, potentially generating cache-save conflicts and reducing cache determinism. A cache miss or rejected save should not fail correctness, but it can increase CI duration and make performance regressions difficult to interpret.

**Severity:** P3 · cache efficiency

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `ui/vite.config.ts`, and Vitest cache configuration.

**Recommendation:** Either include `${{ matrix.shard }}` in the cache key and restore key or designate one shard as the cache writer. Confirm the actual Vitest cache path and measure hit/save rates before and after the change; do not treat a cache optimization as a correctness prerequisite.

**Status:** Open

### CI-06 — Local `check:all` and repository `check.sh` do not represent the same validation contract

**Evidence:** `scripts/check-ui.mjs` runs lint, typecheck, Vitest, i18n lint, FTL dedupe, and optional E2E, skipping E2E when Docker is unavailable. `scripts/check.sh` runs Rust gates, migration smoke/idempotency, skill drift, UI lint/typecheck/tests, i18n lint, and feature-registry parity, but skips the production UI build and E2E unless separately invoked with `--docker-dry-run`; it also does not run FTL dedupe or the A11y test suite. `ui/package.json` exposes both runners as separate commands.

**Impact:** “All checks passed” means different things depending on whether a contributor runs `npm run check:all` or `bash scripts/check.sh`. A local green check can omit a gate that a maintainer expects, while a skipped Docker E2E can be mistaken for full validation.

**Severity:** P2 · developer-experience and gate parity

**Affected files:** `scripts/check.sh`, `scripts/check-ui.mjs`, `ui/package.json`, `AGENTS.md`, and CI workflow definitions.

**Recommendation:** Define one canonical validation matrix with explicit required, advisory, and environment-dependent gates. Make each runner print the same gate names/status vocabulary, record skipped gates with reasons, and add a contract test or generated manifest so local scripts and CI cannot silently drift. Keep the cross-platform UI runner, but document that it is UI-only rather than a complete repository check.

**Status:** Open

### CI-07 — `check:all` checks Docker but does not provision the E2E environment

**Evidence:** `scripts/check-ui.mjs` checks `docker info` and, when Docker is available, invokes `npm run test:e2e` at the E2E gate. `ui/package.json` defines `test:e2e` as `playwright test --config e2e/playwright.config.ts`, while the Docker/Vite orchestration lives in `npm run e2e` (`scripts/run-e2e.mjs`). Therefore `npm run check:all` does not start the Docker backend, license server, Redis, or Vite server before calling Playwright. The runner can report an E2E failure from missing services, or a local pre-existing server can make the result environment-dependent.

**Impact:** The documented unified UI check does not actually execute its E2E gate as described. Developers may see a failure unrelated to the changed code or believe E2E ran against the managed backend when it did not. This also diverges from `scripts/run-e2e.mjs`, which owns startup, readiness, cleanup, and diagnostics.

**Severity:** P1 · validation correctness

**Affected files:** `scripts/check-ui.mjs`, `ui/package.json`, `scripts/run-e2e.mjs`, `AGENTS.md`, and `docs/ci-pipeline.md`.

**Recommendation:** Invoke `npm run e2e` from `check-ui.mjs` when Docker is available, or explicitly rename the gate to “Playwright against existing services” and validate the required service URLs first. Preserve `--no-docker`/skip behavior for environments without Docker, and add a script test proving the selected command provisions or verifies each required service.

**Status:** Open

### CI-08 — CI documentation is stale relative to the current workflow matrix

**Evidence:** `docs/ci-pipeline.md` says it was last updated 2026-07-20 and documents 14 CI jobs, while `.github/workflows/ci.yml` contains the additional `fuzz` job at `:353-380` and has separate E2E Docker-image and E2E jobs. The document's table and pre-merge gate descriptions do not capture the current non-blocking A11y, dependency, Trivy, and fuzz behavior in sufficient detail. The file itself contains an audit stamp noting the omitted fuzz job.

**Impact:** Contributors and reviewers cannot reliably infer which jobs run on PRs, which checks block merges, or why a green pipeline may still have failed advisory checks. Stale SLOs and job counts also make capacity and incident analysis less trustworthy.

**Severity:** P2 · operational documentation

**Affected files:** `docs/ci-pipeline.md`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `.github/workflows/e2e-pr.yml`, and branch-protection settings.

**Recommendation:** Generate or manually reconcile the job inventory and gate policy as part of CI changes. Include event conditions, advisory/non-blocking status, current job count, E2E variants, and the canonical local commands. Add a documentation drift check that at least verifies named jobs and scripts still exist.

**Status:** Open

### CI-09 — Flaky-test detection reports candidates but does not close the quarantine loop

**Evidence:** `scripts/report-flaky.sh:1-20` runs nextest repeatedly and reports tests that fail in some runs but pass in others. Its suggested next step at the end is to add `#[cfg_attr(feature = "slow-tests", ignore)]`, open an issue, or investigate. No machine-readable flaky-test allowlist, quarantine registry, required issue reference, or CI job invoking the script was found in the inspected workflows. `CONTRIBUTING.md` documents reporting guidance but does not create an enforced lifecycle.

**Impact:** Flaky tests can remain in required shards indefinitely, causing reruns and eroding trust, while quarantining a test can also silently reduce coverage. There is no automated distinction between a temporarily quarantined test and an accepted permanent exclusion.

**Severity:** P2 · test reliability and coverage integrity

**Affected files:** `scripts/report-flaky.sh`, `CONTRIBUTING.md`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, and Rust/UI test configuration.

**Recommendation:** Create a versioned quarantine manifest with owner, issue, reason, date, expiry, and replacement coverage. Make CI fail when an entry expires or lacks an issue, publish quarantined-test counts, and run the detector on a scheduled basis. Quarantine only with an explicit status label and keep critical-path tests unquarantinable without approval.

**Status:** Open

### CI-10 — Workflow security and release validation are not consistently tied to pull-request changes

**Evidence:** `.github/workflows/security.yml:3-7` runs only weekly or manually. `.github/workflows/release.yml:12-14` runs only on version tags. The main CI dependency audit is non-blocking, and the release workflow at `:66-70` builds the cloud Docker artifact without the blocking scan used by the CI Docker job. The nightly workflow performs broader checks but is scheduled/manual rather than a pre-merge gate.

**Impact:** Security and release-specific regressions can land on `main` before a scheduled/manual job detects them. The tag release path may produce an artifact that was not validated through the same image scan and companion-service checks as the regular CI path.

**Severity:** P2 · release/security continuity

**Affected files:** `.github/workflows/security.yml`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `.github/workflows/release.yml`, and release branch protection/tag policy.

**Recommendation:** Keep expensive scheduled checks, but add a lightweight PR security baseline for changed dependency manifests and container definitions. Make release publication depend on the exact build/scan/signature jobs for all shipped artifacts, or document a separate trusted release pipeline. Upload immutable provenance/digests and fail closed on missing release validation.

**Status:** Open

## Positive controls observed

- Rust formatting and clippy are explicit jobs with `-D warnings` and workspace scope.
- UI lint, strict typecheck, sharded Vitest, i18n lint, and feature-registry checks exist in the broader validation ecosystem.
- Rust tests use `cargo-nextest` in parallel shard groups, with full-feature cross-platform tests in push/nightly paths.
- Playwright E2E tests are sharded, traces/results are uploaded, and cleanup uses `if: always()`.
- Docker builds use multi-stage images and BuildKit/GHA cache; the cloud binary has a size limit.
- `scripts/run-e2e.mjs` provides cross-platform orchestration, Docker availability detection, cleanup, and failure-time Compose log/status dumping.
- `scripts/report-flaky.sh` provides repeat-run evidence rather than relying only on one-off reruns.
- Release artifacts use named uploads and a publish job with explicit `contents: write` permission.
- Security workflow uses locked installation for `cargo-audit` and `cargo-deny`.

## Test and validation results

This was an evidence-only audit; no workflow, script, test, or production code was changed.

Validation performed:

- Workflow/source inventory and line-referenced evidence review: **completed**
- CI/local-runner/gate comparison: **completed**
- Flaky-test script and artifact-path review: **completed**
- YAML parsing or GitHub-hosted workflow execution: **not run locally**
- Full CI, E2E, and flaky-test runs: **not run during this documentation-only audit**
- Audit report whitespace, `git diff --check`, finding count, and audit-only scope review: **passed**

The report intentionally distinguishes confirmed configuration defects (CI-01, CI-02, and CI-07) from policy or maintainability gaps. The `tee` pattern in CI-04 is not claimed to currently swallow failures on GitHub's default Bash; the finding is that the required pipe-failure behavior is implicit and untested.

## Recommended remediation order

1. **CI-01:** Remove the PR application-test failure suppression immediately.
2. **CI-02/CI-07:** Ensure production changes execute a managed critical E2E smoke set and never report zero tests as a full pass.
3. **CI-03/CI-10:** Define and enforce the required/advisory security and quality gate policy.
4. **CI-06/CI-08:** Reconcile local runners, CI workflows, documentation, and branch-protection names.
5. **CI-05:** Isolate or redesign the parallel Vitest cache writer and measure cache behavior.
6. **CI-09:** Add ownership and expiry to flaky-test quarantine before scaling exclusions.
7. **CI-04:** Make shell failure propagation explicit and add a regression test for wrapper scripts.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests, workflow validation, and updated gate documentation.
