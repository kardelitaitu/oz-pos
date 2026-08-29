---
name: pr-repair
description: Systematic workflow for diagnosing, reproducing, repairing, and verifying failed tests and CI checks on a GitHub pull request in OZ-POS. Covers gh CLI diagnosis, scoped reproduction (Rust, UI, E2E, gates, drift scripts), repair patterns, and verification protocols.
---

# PR Repair — Fixing Failed Tests and CI Checks on Pull Requests

This skill defines the standardized, disciplined workflow for diagnosing, reproducing, fixing, and verifying failed tests or failing CI checks on a GitHub pull request in the OZ-POS repository.

---

## When to use

- CI checks are failing on an active pull request (`gh pr checks` shows failures).
- A PR review reports regressions or test failures.
- A rebase or branch sync against `main` broke existing tests.
- Gate scripts or linters (bundle-parity, i18n, clippy, doc-drift) failed during CI execution.

---

## Golden Rules

| # | Rule | Why |
|---|------|-----|
| 1 | **Evidence first — diagnose before touching code.** | Never guess why a CI check failed. Use `gh pr checks` and `gh run view --log-failed` to extract the exact failure trace. |
| 2 | **Reproduce locally in isolation.** | Reproduce the failing test or check locally using the smallest possible command before writing fixes. |
| 3 | **Minimal surgical fixes.** | Address the root cause. Never delete assertions, skip tests, widen tolerances, or suppress linters unless the test was demonstrably testing an obsolete specification. |
| 4 | **Maintain architectural standards.** | Money values stay in `i64` minor units (`Money`), database writes in `rusqlite` transactions, UI text in `@fluent/react` (`<Localized id="...">`), and Tauri IPC routed through `ui/src/api/`. |
| 5 | **Version is locked at `0.0.31`.** | Never modify the version number in `Cargo.toml`, `package.json`, or any manifest. |
| 6 | **Scope verification to the affected area.** | Run targeted tests during iteration. Full `scripts/check.sh` is reserved for final pre-push or explicit requests. |
| 7 | **Never kill running background processes.** | Do not kill `.exe` or background services that may belong to other agents or active dev servers. |
| 8 | **Never `git push` without an explicit direct command.** | Always stop at local commit. Even after full verification, ask or wait for the user to explicitly tell you to push. |

---

## The 5-Phase Repair Loop

```
┌─────────────────┐     ┌──────────────────┐     ┌──────────────────────┐
│  1. Identify &  │ ──> │  2. Diagnose CI  │ ──> │  3. Reproduce Locally │
│  Checkout PR    │     │  Failure Logs    │     │  (Scoped Isolation)  │
└─────────────────┘     └──────────────────┘     └──────────────────────┘
                                                            │
                                                            ▼
┌─────────────────┐     ┌──────────────────┐     ┌──────────────────────┐
│ 5. Commit & Ask │ <── │   Verify Local   │ <── │ 4. Root Cause &      │
│ for Push Auth   │     │   Pass           │     │    Surgical Fix      │
└─────────────────┘     └──────────────────┘     └──────────────────────┘
```

---

### Phase 1 — Identify & Checkout the PR

Find and switch to the target PR branch:

```powershell
# 1. Check status of PRs relevant to current workspace
gh pr status

# 2. Or list open PRs across the repository
gh pr list --state open --limit 5

# 3. Checkout the target PR locally (e.g. PR #55)
gh pr checkout <PR_NUMBER>

# 4. Ensure local tracking branch is up to date
git pull origin $(git branch --show-current)
```

---

### Phase 2 — Diagnose CI Failure Logs

Inspect which checks failed and download the specific failure logs:

```powershell
# View all checks for the PR
gh pr checks <PR_NUMBER>

# Filter only failed checks
gh pr checks <PR_NUMBER> --failed

# Find recent workflow runs for the PR branch
gh run list --branch $(git branch --show-current) --limit 3

# View the exact failing step logs directly in terminal
gh run view <RUN_ID> --log-failed
```

Analyze the log output to determine which category the failure belongs to:
1. **Rust unit / integration test failure**
2. **Dev PostgreSQL drift** (`Db("db error")`)
3. **Rust formatting or Clippy lint**
4. **UI test failure** (Vitest / Jest)
5. **UI Typecheck (`tsc`) or ESLint failure**
6. **E2E Playwright test failure**
7. **Localization / Bundle-parity failure** (`.ftl` mismatch)
8. **Documentation / Architecture boundary script drift**

---

### Phase 3 — Reproduce Locally (Scoped Isolation)

Always run the reproduction command corresponding to the failed CI gate:

#### 1. Rust Unit & Logic Tests
```powershell
# Run a specific failing test in a specific crate
cargo test -p <crate_name> <test_name> -- --nocapture

# Or use the fast TDD loop
bash scripts/test-tdd.sh -p crates/<crate_name>
```

#### 2. Dev PostgreSQL Drift (`Db("db error")`)
When PG tests like `crates/oz-api/src/pg_tests.rs` or `apps/cloud-server/src/db_tests.rs` fail with cryptic `Db("db error")`:
```powershell
# Reset dev PostgreSQL schema drift
bash scripts/reset-dev-pg.sh
# (or on Windows PowerShell directly: .\scripts\reset-dev-pg.ps1)

# Re-run the failing test
cargo test -p oz-api --test pg_tests
```

#### 3. Rust Formatting & Linter
```powershell
# Check formatting
cargo fmt --all -- --check

# Run Clippy on the affected crate
cargo clippy -p <crate_name> --all-targets --all-features -- -D warnings
```

#### 4. UI Unit Tests (Vitest)
```powershell
cd ui
# Run a specific test file
npx vitest run src/__tests__/<test_file>.test.tsx

# Or run tests on changed files
npm run test
```

#### 5. UI Typecheck & ESLint
```powershell
cd ui
# Type-check
npm run typecheck

# Lint
npm run lint
```

#### 6. E2E Playwright Tests
```powershell
cd ui
# Run specific E2E test suite
npm run e2e:api
npm run e2e:ui
```

#### 7. Localization & Fluent Bundle Parity
```powershell
# Verify Fluent key parity between en.ftl and id.ftl
bash scripts/lint-i18n.sh

# Check duplicate FTL keys
python scripts/dedupe-ftl.py --dry-run

# Check bundle parity against UI references
python scripts/verify-bundle-parity.py
```

#### 8. Documentation & Script Boundary Gates
```powershell
# Check CI docs drift
python scripts/verify-ci-docs-drift.py

# Check architectural boundary rules
python scripts/verify-architecture-boundaries.py

# Check hardcoded money format violations
python scripts/verify-no-hardcoded-money-format.py
```

---

### Phase 4 — Root-Cause & Apply Surgical Fix

Classify the root cause and apply the appropriate repair:

#### Scenario A: Logic Bug Introduced by PR Changes
- **Symptom:** Test expected value `A`, but received `B` due to recent edits.
- **Repair:** Modify the implementation to adhere to the expected business contract. Follow the Golden Rules: `Money` in `i64` minor units, all DB writes inside `rusqlite` transactions.

#### Scenario B: Test Assumptions Outdated by Intentional Feature Change
- **Symptom:** Feature specification deliberately changed (e.g. new status code or updated response structure), but test assertions were not updated.
- **Repair:** Update the test assertion to reflect the new intended contract. Add a code comment explaining why the expectation changed.

#### Scenario C: Dev Database Schema Drift
- **Symptom:** Migration was committed or altered, but test DB container schema was stale.
- **Repair:** Execute `bash scripts/reset-dev-pg.sh` and ensure migrations in `20260813_init.pg.sql` or subsequent migration files match test expectations.

#### Scenario D: Missing or Unsynced Fluent Localization
- **Symptom:** `<Localized id="foo_bar">` added in React JSX, but missing in `ui/src/locales/en.ftl` or `ui/src/locales/id.ftl`.
- **Repair:** Add the key and corresponding translated string to **both** `en.ftl` and `id.ftl`.

#### Scenario E: UI Accessibility (`aria-*`) or Touch Target Failure
- **Symptom:** Component fails ESLint `jsx-a11y` or touch target minimum size (44x44px for tablet).
- **Repair:** Add proper `aria-label`, `role`, or ensure button dimensions meet tablet POS criteria.

---

### Phase 5 — Verify, Commit, and Push Protocol

#### 1. Verify Locally
Run the targeted check again to verify that the failure is resolved:
```powershell
# E.g. re-run the specific crate test or UI check
cargo test -p <crate_name> <test_name>
cargo fmt --all -- --check
```

#### 2. Verify Pre-Commit Gates
Confirm git status and run quick pre-commit validations:
```powershell
git status
```

#### 3. Commit Locally
Make a clean, descriptive local commit explaining the repair:
```powershell
git add <repaired_files>
git commit -m "fix(<scope>): repair <test_name_or_failure_description>"
```

#### 4. Push Protocol — Explicit Permission Required
> [!IMPORTANT]
> **NEVER run `git push` autonomously.**
> Once the local commit is made, present your findings and the applied fix to the user, and ask for permission before pushing.

```powershell
# Only run after user explicitly says to push:
git push origin $(git branch --show-current)

# Monitor the CI checks until green
gh pr checks <PR_NUMBER> --watch
```

---

## Quick Reference: CI Failure to Local Command

| CI Check Name | Root Cause Indicator | Local Reproduction Command |
|---|---|---|
| `Rust Test / test-workspace` | Rust assertion or panic | `cargo test -p <crate> <test_name> -- --nocapture` |
| `PG Integration / pg_tests` | `Db("db error")` drift | `bash scripts/reset-dev-pg.sh && cargo test -p oz-api --test pg_tests` |
| `Rust Lint / cargo fmt` | Formatting discrepancy | `cargo fmt --all` |
| `Rust Lint / clippy` | Compiler/clippy warning | `cargo clippy -p <crate> --all-targets --all-features -- -D warnings` |
| `UI Test / vitest` | Component/unit test failure | `cd ui && npx vitest run src/__tests__/<test_file>` |
| `UI Lint / tsc` | TypeScript type error | `cd ui && npm run typecheck` |
| `UI Lint / eslint` | Lint or a11y violation | `cd ui && npm run lint` |
| `E2E Playwright` | Browser flow timeout or diff | `cd ui && npm run e2e:ui` |
| `i18n Lint / bundle-parity` | Missing key in `.ftl` | `bash scripts/lint-i18n.sh && python scripts/verify-bundle-parity.py` |
| `CI Docs Drift` | Undocumented script or workflow | `python scripts/verify-ci-docs-drift.py` |
