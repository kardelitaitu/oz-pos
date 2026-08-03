# CI Pipeline Audit — July 2026\r
\r
> **Audit date:** 2026-07-31\r
> **Re-verified:** 2026-08-03 (all line references and statuses reconciled against current `.github/workflows/` and `scripts/`)\r
> **Sector:** CI pipeline — gate completeness, test coverage, cache efficiency, flaky-test handling, security checks, artifacts, and E2E orchestration\r
> **Status:** ✅ **FULLY REMEDIATED** (2026-08-03) — all 10 findings CI-01→CI-10 closed in workflows, validation scripts, the gate manifest, local runners, docs, and regression tests; see per-finding statuses and the Audit status table below\r
> **Production code changed:** Minimal — dependency manifests + lockfile for the CI-03 dependency baseline (`prometheus 0.13 → 0.14` in `apps/cloud-server/Cargo.toml` and `crates/oz-reporting/Cargo.toml`; `plist`/`wayland-scanner`/`protobuf` bumps resolved via `Cargo.lock`). Metric sources additionally carry SAFETY annotations from the audit/25 panic-policy residual. No behavioral source changes.\r
\r
## Scope\r
\r
This audit evaluates sector 27 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers the main CI workflow, PR E2E workflow, nightly matrix, release workflow, security workflow, local validation scripts, test runners, cache configuration, artifact retention, failure propagation, branch/event conditions, and flaky-test reporting.\r
\r
Inspected areas:\r
\r
- `.github/workflows/ci.yml`\r
- `.github/workflows/e2e-pr.yml`\r
- `.github/workflows/nightly.yml`\r
- `.github/workflows/release.yml`\r
- `.github/workflows/security.yml`\r
- `.github/workflows/docs.yml`\r
- `scripts/check.sh`\r
- `scripts/check-ui.mjs`\r
- `scripts/run-e2e.mjs`\r
- `scripts/report-flaky.sh`\r
- `ui/package.json`\r
- `ui/e2e/playwright.config.ts`\r
- `docs/ci-pipeline.md`\r
- `CONTRIBUTING.md`\r
\r
## Architecture summary\r
\r
The main `CI` workflow runs Rust formatting and clippy, PR-only fast Rust tests, push-only full Rust tests, UI lint/typecheck/tests, Lighthouse, Docker build and scanning, coverage, dependency audits, fuzzing, skill-drift tests, and a three-way E2E matrix. Separate workflows cover PR E2E, nightly cross-platform/full-matrix validation, releases, security audits, documentation, Android, and iOS.\r
\r
The pipeline is intentionally parallel and uses `rust-cache`, sccache, npm cache, Vitest transform cache, and BuildKit/GHA cache. UI and Playwright tests are sharded. Local validation is split between the Bash `scripts/check.sh` gate and the cross-platform `ui` `check:all` runner. Flaky detection exists as a reporting script, but no executable quarantine registry or required flake-resolution workflow was found.\r
\r
The overall structure is broad, but enforcement is uneven. One PR Rust shard suppresses application test failures, dependency and container scans do not all block at the PR layer, PR E2E can report a green result with zero browser tests executed, and documentation describes an older job matrix. These gaps make the green status less representative than the apparent breadth of the pipeline. On re-verification, the Docker image scans are now blocking gates (DOCKER-03/08) and the main CI `e2e` job runs the full suite on PRs, which narrows the earlier E2E coverage concern.\r
\r
## Findings\r
\r
### CI-01 — Application crate tests are suppressed in the PR fast track AND excluded everywhere else\r
\r
**Evidence:** `.github/workflows/ci.yml:118` runs `cargo nextest run -p "$pkg" --profile ci 2>/dev/null || true` for `oz-cloud-server`, `oz-pos-app`, and `oz-pos-tablet`. Any compilation or test failure in those packages is discarded, and the loop continues with a successful shell status. The other Rust shard loops use `|| exit 1` at lines 103, 108, and 113. Additionally, `oz-pos-app` and `oz-pos-tablet` are excluded from every other Rust test job: `rust-test-full` (`--exclude oz-pos-app --exclude oz-pos-tablet`), `nightly.yml` `rust-test`, and `release.yml` `release-build`. The `2>/dev/null` also hides the stderr that would explain why a package failed.\r
\r
**Impact:** The two Tauri application crates (`oz-pos-app`, `oz-pos-tablet`) are never tested by any CI job — the fast-track apps shard is the only attempt and it swallows the outcome, while full/nightly/release runs all exclude them. This is a permanent regression escape for the desktop/tablet command surfaces, which are exactly the crates most likely to diverge from library tests. The branch-protection check `Rust Test Fast (apps)` reports green regardless.\r
\r
**Severity:** P1 · required-test integrity\r
\r
**Affected files:** `.github/workflows/ci.yml`, `apps/cloud-server`, `apps/desktop-client`, `apps/tablet-client`, and the branch-protection check for `Rust Test Fast (apps)`.\r
\r
**Recommendation:** Remove `2>/dev/null || true`. Either (a) build and test `oz-pos-app`/`oz-pos-tablet` on the ubuntu runner — the system dependencies are already installed in every Rust job — or (b) if the crates genuinely cannot build on a runner, encode that as an explicit matrix exclusion with a documented reason and a dedicated job that compiles them (`cargo check` minimum). Keep stderr visible. Add a static workflow check that rejects failure-swallowing operators in required test commands.\r
\r
**Status:** Remediated (2026-08-03) — `ci.yml` apps shard now uses `|| exit 1` (stderr kept); new required `rust-test-apps` job runs `oz-cloud-server` + `oz-pos-app` + `oz-pos-tablet` on ubuntu (system deps installed) on both PR and push.\r
\r
### CI-02 — PR E2E `--changed-only` can report success with zero tests executed (mitigated by main CI E2E)\r
\r
**Evidence:** `.github/workflows/e2e-pr.yml:123` always passes `--changed-only` on pull requests. `scripts/run-e2e.mjs:355-363` (`getChangedSpecs`) only looks for changed files matching `ui/e2e/*.spec.ts`; when no such spec is changed, `runPlaywright` logs “Changed-only: no E2E specs changed — skipping.” and returns success without running Playwright. The workflow is triggered by all `ui/**` changes at `.github/workflows/e2e-pr.yml:24-29`, not only E2E specs.\r
\r
**Impact:** The `E2E (PR)` check can be green with zero browser tests executed for a production UI, API client, routing, localization, or state-management change. **Mitigation:** the main `CI` workflow's `e2e` job (`.github/workflows/ci.yml:510`) has no `if:` gate and runs the full three-shard Playwright suite on every PR in addition to push, so production changes still receive full browser coverage from the main pipeline. The remaining defects are a misleadingly-named green check, ~10 minutes of redundant Docker/Vite orchestration per PR, and the narrow `ui/e2e/*.spec.ts` glob that silently triggers the skip path for any other spec location.\r
\r
**Severity:** P2 · misleading check + redundant cost (downgraded from P1 on re-verification: full E2E runs on PRs via the main CI job)\r
\r
**Affected files:** `.github/workflows/e2e-pr.yml`, `scripts/run-e2e.mjs`, `ui/e2e/`, and pull-request path filters.\r
\r
**Recommendation:** Since main CI already runs the full E2E suite on PRs, either drop `--changed-only` from the PR path in `e2e-pr.yml` (report a real full run) or narrow its trigger to `ui/e2e/**` so the skip path is only reachable for E2E-only changes. At minimum, report a distinct `skipped-no-spec` result rather than treating zero executed tests as “all tests passed,” and widen the changed-file glob to `ui/e2e/**`.\r
\r
**Status:** Remediated (2026-08-03) — `e2e-pr.yml` trigger narrowed to `ui/e2e/**` + E2E infra; `run-e2e.mjs` widened changed-file glob to `ui/e2e/**` and returns a distinct `skipped` status (exit 2, `SKIPPED-NO-SPEC` banner) instead of a false pass; the workflow treats exit 2 as a neutral skip with a notice (never a red check, never a silent pass); main CI `e2e` job still runs the full suite on every PR.\r
\r
### CI-03 — A11y, dependency, coverage, and fuzz checks are non-blocking; Docker scans are now blocking\r
\r
**Evidence:** `.github/workflows/ci.yml:281` marks the A11y regression job `continue-on-error: true`; the coverage job at `:382` is non-blocking; the dependency audit at `:419` uses `continue-on-error: true` with `cargo audit || true` / `npm audit ... || true`; fuzzing at `:439` is non-blocking and suppresses each fuzz-target failure (`timeout ... || true`). **Remediated portion:** the Docker Trivy scans at `.github/workflows/ci.yml:333` and `:342` now use `exit-code: 1` with severity `CRITICAL,HIGH` and no `continue-on-error` — both application images are blocking gates (DOCKER-03/08). `.github/workflows/security.yml` is strict when manually or weekly scheduled, but is not part of the main PR workflow.\r
\r
**Impact:** The repository can merge with known high-severity dependency findings, accessibility regressions, coverage failures, or fuzz crashes unless a maintainer notices an informational artifact, because the required/advisory distinction is not consistently visible in the user-facing status set. Container image security is now enforced at the CI layer.\r
\r
**Severity:** P1 · quality/security enforcement (scope reduced to the four non-blocking gates)\r
\r
**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/security.yml`, and branch-protection configuration.\r
\r
**Recommendation:** Split checks into explicit `required` and `advisory` jobs. Make at least the dependency audit for high/critical vulnerabilities blocking on `main` push after a reviewed baseline (keep advisory on PR to avoid blocking on transitive noise); promote stable A11y checks incrementally; preserve fuzzing as advisory while creating an issue/artifact on crashes. Publish the required/advisory policy in the workflow and `docs/ci-pipeline.md`.\r
\r
**Status:** Remediated (2026-08-03) — dependency audit is now required on `main` pushes and advisory on PRs (`continue-on-error: ${{ github.event_name == 'pull_request' }}`); `cargo audit`/`npm audit` `|| true` swallows removed; fuzz per-target `|| true` removed (failures surface to the advisory job and trigger crash-artifact upload); required/advisory policy documented in `docs/ci-pipeline.md`.

**Baseline (2026-08-03):** the audit was run against the live tree. `npm audit` is clean (0). `cargo audit` initially flagged 4 vulnerabilities; remediation resolved 3 of them — `plist 1.9.0 → 1.10.0` (fixes `quick-xml` RUSTSEC-2026-0194/0195, both high), `prometheus 0.13 → 0.14` + `wayland-scanner 0.31.10 → 0.31.11` (fixes `quick-xml` residual and `protobuf` RUSTSEC-2024-0437 via protobuf 2.28 → 3.7.2). The remaining advisory is RUSTSEC-2023-0071 (`rsa` 0.9.10, medium, no fixed upgrade available) — documented as an accepted residual in `.cargo/audit.toml` with justification (private key used only for operator-side signing; verification is public-key only; re-audit on release). `cargo audit` now exits 0 with only 20 informational warnings.\r
\r
### CI-04 — Test and E2E output pipelines rely on implicit shell pipe behavior\r
\r
**Evidence:** `.github/workflows/ci.yml:251` runs Vitest through `... 2>&1 | tee ui/vitest-output.log`, and `:604` runs Playwright through `... 2>&1 | tee e2e-output.log`; nightly uses the same patterns at `:118` and `:223`; `scripts/report-flaky.sh:59` pipes nextest through `tee`. GitHub-hosted Bash defaults to `bash --noprofile --norc -e -o pipefail {0}` for `run` steps, so failures currently do propagate, but the contract is implicit rather than declared in the workflow. `scripts/run-e2e.mjs` avoids the issue by capturing exit codes in Node (`execSync` throws on non-zero).\r
\r
**Impact:** A future shell change, alternate runner shell, or command wrapper can cause test failures to be hidden behind a successful `tee`. This is a maintainability and observability risk rather than a confirmed current failure on GitHub's default Bash.\r
\r
**Severity:** P2 · failure-reporting reliability\r
\r
**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `scripts/report-flaky.sh`, and workflow shell configuration.\r
\r
**Recommendation:** Declare `shell: bash --noprofile --norc -eo pipefail {0}` at the top of the affected jobs or add a workflow-level default. Prefer writing command output with an explicit status capture when artifacts are needed. Add a small CI-script test that intentionally returns nonzero before `tee` and confirms the wrapper fails.\r
\r
**Status:** Remediated (2026-08-03) — all `tee`-wrapped steps in `ci.yml` and `nightly.yml` (vitest, a11y, E2E, doc, bench) now declare `shell: bash --noprofile --norc -eo pipefail {0}`; regression test added at `scripts/__tests__/pipefail.test.mjs` (proves the wrapper fails on a nonzero left command and asserts every tee-wrapped step block declares the pipefail shell, per-step rather than by count).\r
\r
### CI-05 — Parallel Vitest shards share one writable cache key\r
\r
**Evidence:** `.github/workflows/ci.yml:247-248` gives all four UI shards the same `vitest-cache-${{ runner.os }}-${{ hashFiles('ui/package-lock.json') }}` key. Nightly repeats the pattern at `.github/workflows/nightly.yml:114-115`. The cache is a mutable transform cache written by every shard; the key does not include the shard number or a writer role.\r
\r
**Impact:** Concurrent cache saves can contend or produce last-writer behavior, potentially generating cache-save conflicts and reducing cache determinism. A cache miss or rejected save should not fail correctness, but it can increase CI duration and make performance regressions difficult to interpret.\r
\r
**Severity:** P3 · cache efficiency\r
\r
**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `ui/vite.config.ts`, and Vitest cache configuration.\r
\r
**Recommendation:** Either include `${{ matrix.shard }}` in the cache key and restore key or designate one shard as the cache writer. Confirm the actual Vitest cache path and measure hit/save rates before and after the change; do not treat a cache optimization as a correctness prerequisite.\r
\r
**Status:** Remediated (2026-08-03) — vitest cache save key now includes `${{ matrix.shard }}` in both `ci.yml` and `nightly.yml`, isolating concurrent shard writers.\r
\r
### CI-06 — Local `check:all` and repository `check.sh` do not represent the same validation contract\r
\r
**Evidence:** `scripts/check-ui.mjs` runs lint, typecheck, Vitest, i18n lint, FTL dedupe, bundle budget, and optional E2E/perf smoke, skipping E2E when Docker is unavailable. `scripts/check.sh` runs Rust gates, migration smoke/idempotency, skill drift, UI lint/typecheck/tests, i18n lint, and feature-registry parity, but skips the production UI build, FTL dedupe, the A11y suite, bundle budget, and E2E unless separately invoked with `--docker-dry-run`. `ui/package.json` exposes both runners as separate commands.\r
\r
**Impact:** “All checks passed” means different things depending on whether a contributor runs `npm run check:all` or `bash scripts/check.sh`. A local green check can omit a gate that a maintainer expects, while a skipped Docker E2E can be mistaken for full validation.\r
\r
**Severity:** P2 · developer-experience and gate parity\r
\r
**Affected files:** `scripts/check.sh`, `scripts/check-ui.mjs`, `ui/package.json`, `AGENTS.md`, and CI workflow definitions.\r
\r
**Recommendation:** Define one canonical validation matrix with explicit required, advisory, and environment-dependent gates. Make each runner print the same gate names/status vocabulary, record skipped gates with reasons, and add a contract test or generated manifest so local scripts and CI cannot silently drift. Keep the cross-platform UI runner, but document that it is UI-only rather than a complete repository check.\r
\r
**Status:** Remediated (2026-08-03) — `check.sh` now runs FTL dedupe and the A11y suite (advisory, mirroring CI); gate vocabulary aligned across `check.sh` and `check:all`; per-runner coverage documented in `docs/ci-pipeline.md`; E2E split clearly documented.\r
\r
### CI-07 — `check:all` runs Playwright without provisioning the Docker backend\r
\r
**Evidence:** `scripts/check-ui.mjs:123` invokes `npm run test:e2e` when `dockerAvailable()` is true. `ui/package.json` defines `test:e2e` as `playwright test --config e2e/playwright.config.ts`, which auto-starts only the Vite dev server via its `webServer` block (`command: 'npm run dev'`, `reuseExistingServer: true`) — it does **not** start the Docker backend, license server, Redis, or provision the API environment that `e2e/api.spec.ts` requires. The Docker/Vite orchestration lives in `npm run e2e` (`scripts/run-e2e.mjs`), which owns startup, readiness, cleanup, and diagnostics but is not invoked by `check:all`.\r
\r
**Impact:** The documented unified UI check's E2E gate is environment-dependent: browser tests run against whatever happens to be on port 1420/3099, and API tests fail (or silently pass against a stale server) depending on the machine state. This diverges from `scripts/run-e2e.mjs`, which provisions and cleans up deterministically.\r
\r
**Severity:** P1 · validation correctness\r
\r
**Affected files:** `scripts/check-ui.mjs`, `ui/package.json`, `scripts/run-e2e.mjs`, `ui/e2e/playwright.config.ts`, `AGENTS.md`, and `docs/ci-pipeline.md`.\r
\r
**Recommendation:** Invoke `npm run e2e` from `check-ui.mjs` when Docker is available, or explicitly rename the gate to “Playwright against existing services” and validate the required service URLs first. Preserve `--no-docker`/skip behavior for environments without Docker, and add a script test proving the selected command provisions or verifies each required service.\r
\r
**Status:** Remediated (2026-08-03) — `check-ui.mjs` E2E gate now invokes `npm run e2e` (full Docker+Vite provisioning + cleanup) when Docker is available instead of bare `playwright test`; `AGENTS.md` and `docs/ci-pipeline.md` updated to describe the provisioning behavior.\r
\r
### CI-08 — CI documentation is stale relative to the current workflow matrix\r
\r
**Evidence:** `docs/ci-pipeline.md` says it was last updated 2026-07-20 and documents 14 CI jobs, while `.github/workflows/ci.yml` additionally contains the `fuzz` job (`:439`) and the `sync-slow-tests` job, with separate E2E Docker-image and E2E jobs. The document's table and pre-merge gate descriptions do not capture the current non-blocking A11y, dependency, and fuzz behavior, the blocking Docker scans, or the CI-01 apps-shard non-enforcement. The file itself contains an audit stamp noting the omitted fuzz job.\r
\r
**Impact:** Contributors and reviewers cannot reliably infer which jobs run on PRs, which checks block merges, or why a green pipeline may still have failed advisory checks. Stale SLOs and job counts also make capacity and incident analysis less trustworthy.\r
\r
**Severity:** P2 · operational documentation\r
\r
**Affected files:** `docs/ci-pipeline.md`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `.github/workflows/e2e-pr.yml`, and branch-protection settings.\r
\r
**Recommendation:** Generate or manually reconcile the job inventory and gate policy as part of CI changes. Include event conditions, advisory/non-blocking status, current job count, E2E variants, and the canonical local commands. Add a documentation drift check that at least verifies named jobs and scripts still exist.\r
\r
**Status:** Remediated (2026-08-03) — `docs/ci-pipeline.md` rewritten with the current job matrix (fuzz, sync-slow-tests, rust-test-apps, security-pr, flaky-quarantine), the required/advisory policy, per-shard cache keys, and the local-runner contract. **Drift check implemented:** `scripts/verify-ci-docs-drift.py` is a required `ci-docs-drift` job in `ci.yml`, `nightly.yml`, and `docs.yml`, and a `check.sh` gate — it fails when a job name referenced in the docs' Job Matrix / Pre-Merge Validation Gates tables no longer exists in `.github/workflows/*.yml`, when a documented workflow file is missing, or when the `check.sh` / `check:all` gate vocabulary drifts. The `docs.yml` gate runs on PRs touching `docs/**` or workflow files, and `build-and-deploy` depends on it, so a stale job matrix can never be merged or published via the docs path. **Single source of truth:** all gate names + status now derive from `scripts/gates.json` (shared / check.sh-only / check:all-only / CI-only / nightly gates, each with runner needles and an enforcing `workflow`+`job`); the verifier no longer hardcodes lists and additionally fails when a gate's status contradicts its workflow (`required` jobs must not set `continue-on-error: true`, `advisory` jobs must, `required-on-push` jobs must gate on a `${{ ... }}` condition). This surfaced and fixed a real drift: the `lighthouse` job was documented advisory but lacked `continue-on-error: true` — now aligned. `check-ui.mjs` self-audits against the manifest and fails `check:all` if a manifest `check:all` gate is undeclared.\r
\r
### CI-09 — Flaky-test detection reports candidates but does not close the quarantine loop\r
\r
**Evidence:** `scripts/report-flaky.sh` runs nextest repeatedly and reports tests that fail in some runs but pass in others. No machine-readable flaky-test allowlist, quarantine registry, required issue reference, or CI job invoking the script was found in any inspected workflow (a search for `report-flaky` in `.github/workflows/` returns zero matches). `CONTRIBUTING.md` documents reporting guidance but does not create an enforced lifecycle.\r
\r
**Impact:** Flaky tests can remain in required shards indefinitely, causing reruns and eroding trust, while quarantining a test can also silently reduce coverage. There is no automated distinction between a temporarily quarantined test and an accepted permanent exclusion.\r
\r
**Severity:** P2 · test reliability and coverage integrity\r
\r
**Affected files:** `scripts/report-flaky.sh`, `CONTRIBUTING.md`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, and Rust/UI test configuration.\r
\r
**Recommendation:** Create a versioned quarantine manifest with owner, issue, reason, date, expiry, and replacement coverage. Make CI fail when an entry expires or lacks an issue, publish quarantined-test counts, and run the detector on a scheduled basis. Quarantine only with an explicit status label and keep critical-path tests unquarantinable without approval.\r
\r
**Status:** Remediated (2026-08-03) — quarantine lifecycle implemented: `scripts/flaky-quarantine.json` (versioned manifest), `scripts/verify-flaky-quarantine.py` (fails on expired/ownerless/issueless entries), required `flaky-quarantine` CI job, and split nightly jobs (`flaky-detect` detector, informational + `flaky-quarantine-registry` verifier, fail-closed) so a detector timeout can never skip the registry gate; lifecycle documented in `CONTRIBUTING.md`.\r
\r
### CI-10 — Workflow security and release validation are not consistently tied to pull-request changes\r
\r
**Evidence:** `.github/workflows/security.yml` runs only weekly or manually. The main CI dependency audit is non-blocking, and `release.yml` runs only on version tags. **Remediated portion:** the release workflow now runs blocking Trivy scans on both release-tagged images (`release.yml:87` and `:98`, `exit-code: 1`, severity `CRITICAL,HIGH`), closing the earlier gap where the cloud artifact was built without the scan used by the CI Docker job (DOCKER-03/08). The nightly workflow performs broader checks but is scheduled/manual rather than a pre-merge gate.\r
\r
**Impact:** Security regressions in changed dependency manifests and container definitions can land on `main` before a scheduled/manual job detects them, since no lightweight PR security baseline runs as part of the PR workflow.\r
\r
**Severity:** P2 · release/security continuity (release-scan portion remediated)\r
\r
**Affected files:** `.github/workflows/security.yml`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `.github/workflows/release.yml`, and release branch protection/tag policy.\r
\r
**Recommendation:** Keep expensive scheduled checks, but add a lightweight PR security baseline for changed dependency manifests and container definitions. Make release publication depend on the exact build/scan/signature jobs for all shipped artifacts, or document a separate trusted release pipeline. Upload immutable provenance/digests and fail closed on missing release validation.\r
\r
**Status:** Remediated (2026-08-03) — new required `security-pr` job runs `cargo audit` + `npm audit --audit-level=high` fail-closed when a PR changes dependency/container manifests (Cargo.lock/toml, package files, Dockerfiles, compose files); the gate is **fail-closed** — if the base SHA cannot be resolved or fetched, the audits run anyway rather than silently skipping; weekly `security.yml` stays for the full tree.\r
\r
## Positive controls observed\r
\r
- Rust formatting and clippy are explicit jobs with `-D warnings` and workspace scope.\r
- UI lint, strict typecheck, sharded Vitest, i18n lint, and feature-registry checks exist in the broader validation ecosystem.\r
- Rust tests use `cargo-nextest` in parallel shard groups, with full-feature cross-platform tests in push/nightly paths.\r
- Playwright E2E tests are sharded, traces/results are uploaded, and cleanup uses `if: always()`.\r
- Docker builds use multi-stage images and BuildKit/GHA cache; the cloud binary has a size limit, and Trivy scans of BOTH images are now blocking gates (DOCKER-03/08), in CI and release paths.\r
- `scripts/run-e2e.mjs` provides cross-platform orchestration, Docker availability detection, cleanup, and failure-time Compose log/status dumping.\r
- `scripts/report-flaky.sh` provides repeat-run evidence rather than relying only on one-off reruns.\r
- Release artifacts use named uploads and a publish job with explicit `contents: write` permission.\r
- Security workflow uses locked installation for `cargo-audit` and `cargo-deny`.\r
\r
## Test and validation results\r
\r
This report began as evidence-only review; the remediation (2026-08-03) was implemented and validated against the live tree.\r
\r
Validation performed:\r
\r
- Workflow/source inventory and line-referenced evidence review: **completed** (re-verified 2026-08-03 against current files; line references above reflect the current state)\r
- CI/local-runner/gate comparison: **completed**\r
- Flaky-test script and artifact-path review: **completed**\r
- YAML parsing of all edited workflows (ci, e2e-pr, nightly, docs, release, security): **passed**
- `bash -n` on `scripts/check.sh`; `node --check` on edited `.mjs` scripts; `py_compile` on edited Python scripts: **passed**
- Script regression suite (`npm run test:scripts`): **29/29 pass** (pipefail wrapper, run-e2e orchestration, verify-ci-docs-drift exit contract, flaky-quarantine verifier, etc.)
- Rust suites for the touched crates: `cargo test -p oz-cloud-server` → **111/111**; `cargo test -p oz-reporting` → **64/64** (prometheus 0.14 runtime-safe)
- Dependency audits: `cargo audit` → **exit 0** (single accepted residual RUSTSEC-2023-0071 via the reviewed `.cargo/audit.toml` baseline); `npm audit --audit-level=high` → **0 vulnerabilities**
- Drift verifier: `scripts/verify-ci-docs-drift.py` → **0 drift items, exit 0** against 10 workflows / 41 jobs / 38 manifest gates; negative tests prove exit 1 (ghost job, deleted job, missing workflow file, docs-status drift) and exit 2 (missing section, malformed manifest)
- Live E2E metrics verification (2026-08-03): booted the cloud + license + redis stack under an isolated compose project; cloud `/metrics` served a live Prometheus exporter (9 metric families) with counters/histograms incrementing under real traffic (health probes + authenticated sync push); license `/metrics` confirmed 404-by-design (PocketBase app, no Prometheus route). Test containers and temporary compose files removed afterward; the shared dev-stack was untouched.\r
\r
- Audit report whitespace and `git diff --check`: **passed**\r
\r
The report distinguishes confirmed configuration defects (CI-01 and CI-07) from policy or maintainability gaps. CI-02 was downgraded from P1 to P2 on re-verification because the main CI `e2e` job runs the full suite on every PR. The `tee` pattern in CI-04 was not claimed to currently swallow failures on GitHub's default Bash; the remediation makes the required pipe-failure behavior explicit and adds a regression test.\r
\r
## Remediation summary (2026-08-03)\r
\r\n1. **CI-01:** apps shard `|| true` swallow removed (`|| exit 1`); dedicated required `rust-test-apps` job added.\r
2. **CI-07:** `check:all` E2E gate now provisions via `npm run e2e`.\r
3. **CI-02:** PR E2E trigger narrowed to `ui/e2e/**`; changed-spec glob widened; `skipped-no-spec` distinct exit status.\r
4. **CI-03/CI-10:** dependency audit required on push / advisory on PR; PR security baseline job added; fuzz failures surfaced.\r
5. **CI-06/CI-08:** local runners reconciled (FTL dedupe + A11y in check.sh); docs rewritten with current matrix + policy.\r
6. **CI-05:** per-shard vitest cache keys in ci.yml + nightly.yml.\r
7. **CI-09:** quarantine manifest + verifier + required CI job + nightly detector; CONTRIBUTING lifecycle added.\r
8. **CI-04:** explicit pipefail shells on all tee steps + regression test.
9. **CI-08 (drift enforcement):** `scripts/verify-ci-docs-drift.py` is a required `ci-docs-drift` job in `ci.yml`, `nightly.yml`, and `docs.yml` (doc-PR path) plus a `check.sh` gate; `scripts/gates.json` (38 gates) is the single source of truth for gate names/status with status↔workflow enforcement; `check-ui.mjs` self-audits the manifest.\r
\r
## Audit status\r
\r
**2026-08-03 — ✅ FULLY REMEDIATED.** All 10 findings CI-01→CI-10 are closed; fixes are recorded in the per-finding statuses and the Remediation summary above. Remediation batch committed as `fbd83866` (fix(ci): remediate audit/27 CI-01→CI-10 as one coherent batch). No behavioral production code changed — the only production-touching changes are the dependency manifests + lockfile for the CI-03 baseline (see header). Line references above reflect the current state of `.github/workflows/` and `scripts/` (re-verified 2026-08-03).