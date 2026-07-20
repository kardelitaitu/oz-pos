# CI Pipeline Dashboard — OZ-POS

> Last updated: 2026-07-20

## Job Matrix

| Job | Trigger | Runtime | Cache | Shards |
|-----|---------|---------|-------|--------|
| `rust-fmt` | PR + push | ~30s | None (no deps) | — |
| `rust-clippy` | PR + push | ~3min | rust-cache + sccache | — |
| `rust-test-fast` | PR only | ~2min each | rust-cache + sccache | 5-way |
| `rust-test-full` | Push only | ~5min | rust-cache + sccache | 2 OS |
| `ui-lint` | PR + push | ~40s | npm cache | — |
| `ui-typecheck` | PR + push | ~30s | npm cache | — |
| `ui-test` | PR + push | ~2min each | npm + vitest cache | 4-way |
| `lighthouse` | PR + push | ~2min | npm cache | — |
| `docker` | PR + push | ~3min | Docker layer cache | — |
| `coverage` | PR + push | ~5min | rust-cache | — |
| `audit` | PR + push | ~30s | — | — |
| `skill-drift-tests` | PR + push | ~20s | — | — |
| `e2e-docker-image` | Push only | ~4min | Docker GHCR | — |
| `e2e` | PR + push | ~6min each | npm + rust-cache + Docker GHCR | 3-way |

## Caching Strategy

### Rust (cargo)
- **rust-cache** (`Swatinem/rust-cache@v2`): Caches `target/` directories keyed by `Cargo.lock`. `save-always: true` ensures cache persists even on job failure.
- **sccache** (`mozilla/sccache-action@v0.0.10`): Compiler cache shared across jobs via S3-compatible storage. Reduces clean-build time by ~60%.

### Node.js (npm)
- **npm cache** (`actions/setup-node@v4` with `cache: 'npm'`): Caches `~/.npm` keyed by `package-lock.json`.
- **vitest cache** (`actions/cache@v4`): Persists `node_modules/.cache/vitest` for transform cache reuse. Keyed by `package-lock.json`.

### Docker
- **BuildKit inline cache**: Uses `BUILDKIT_INLINE_CACHE=1` and `type=gha` cache backend.
- **GHCR pre-built images** (`e2e-docker-image` job): Pushes `oz-pos-cloud:e2e` to GHCR on push to main. E2E jobs pull this image before building, reusing layers.

### E2E
- **Playwright browsers**: Installed via `npx playwright install chromium --with-deps`. Browser binaries are cached by setup-node's cache.
- **Docker layer cache**: Pulled from GHCR before local build, reducing Docker build time by ~80%.

## Pre-Merge Validation Gates

The following jobs must pass before a PR can be merged. **Note:** These gates are enforced via GitHub branch protection rules which must be configured manually in **Settings → Branches → Branch protection rules → `main` → Require status checks to pass before merging**. The CI config alone does not auto-enforce these — it only defines the jobs.

| Gate | Job | Blocks Merge |
|------|-----|-------------|
| Format | `rust-fmt` | ✅ Required |
| Lint (Rust) | `rust-clippy` | ✅ Required |
| Lint (UI) | `ui-lint` | ✅ Required |
| TypeCheck | `ui-typecheck` | ✅ Required |
| Unit Tests (Rust) | `rust-test-fast` (5 shards) | ✅ Required |
| Unit Tests (UI) | `ui-test` (4 shards) | ✅ Required |
| E2E Tests | `e2e` (3 shards) | ✅ Required |
| Docker Build | `docker` | ✅ Required |
| Lighthouse a11y | `lighthouse` | ⚠️ Advisory (≥ 90 threshold) |
| Coverage | `coverage` | ⚠️ Non-blocking |
| Audit | `audit` | ⚠️ Non-blocking |
| Skill Drift | `skill-drift-tests` | ✅ Required |

## Failure Modes & Remediation

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `rust-clippy` fails | New warning introduced | Run `cargo clippy --workspace --all-targets -- -D warnings` locally |
| `rust-test-fast` timeout | Test hangs or is too slow | Check for infinite loops, add `#[ignore]` for slow tests behind `slow-tests` feature |
| `ui-test` act() warning | Component effect fires async without `renderInAct` | Use `renderInAct` / `renderHookInAct` helper from `ui/src/test-utils/` |
| `e2e` timeout | Server didn't start in 90s | Check Docker health, Vite port conflict |
| `docker` size check | Binary exceeds 50 MB | Check for debug symbols, run `strip` on binary |
| `audit` finds vulnerability | Dependency has known CVE | Run `cargo update` or pin to patched version |
| Cache miss (all jobs slow) | `Cargo.lock` or `package-lock.json` changed | Expected after dependency updates — first run is cold |

## SLO Targets

| Pipeline Phase | Target | Current |
|---------------|--------|---------|
| Total CI (PR, parallel) | < 8 min | ~6 min |
| Rust test (fast, 5 shards) | < 3 min | ~2 min |
| UI test (4 shards) | < 2 min | ~1.5 min |
| E2E (3 shards) | < 8 min | ~6 min |
| Docker build | < 5 min | ~3 min |
