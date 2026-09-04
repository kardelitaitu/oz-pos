# Agents Configuration & Rules

<!-- Audit stamp: 2026-09-04 · DSH · status: ACCURATE · version lock: 0.0.36 · 8 pre-commit gates · conventional commits enforced -->

## 🚨 Critical Agent Directives (MUST FOLLOW)

| Rule | Direct Instruction | Why / Context |
|---|---|---|
| **Branching** | **NEVER create new branches. NEVER switch branches.** | Always work directly on the currently active branch unless specifically requested by the user. |
| **Commits** | **ALWAYS commit with format `<type>(<area>): <description>`.** | Must follow conventional commits. Make local commits after each logical task. |
| **Pushing** | **NEVER run `git push` without an explicit direct order.** | Even after completing all checks, wait for the user to explicitly say "push". |
| **Version Lock** | **Version is locked at `0.0.36`. NEVER modify version numbers.** | Do not bump version in `Cargo.toml`, `package.json`, `tauri.conf.json`, etc. |
| **File Paths** | **ALWAYS use forward slashes (`/`) in path arguments on Windows.** | Avoid path escaping bugs. **Never anchor to a hardcoded checkout** (e.g. `C:/My Script/oz-pos/`) — resolve the repo root with `git rev-parse --show-toplevel`, or script-relative with `$PSScriptRoot`/`__file__`/`import.meta.url`, so tools work in any worktree of the multi-root layout (`<base>/main` bare + `<base>/<release>` + `<base>/worktrees/*`). |
| **File Reading** | **ALWAYS read files in small chunks (≤ 500 lines).** | Preserves context window and prevents output truncation. |
| **Discovery** | **ALWAYS use `codebase-memory-mcp` first for code exploration.** | Graph discovery saves context tokens and surfaces call chains faster. |
| **Currency** | **ALWAYS use `Money` struct (`i64` minor units). NEVER use float.** | Monetary values must never use `f32`/`f64`. |
| **DB Writes** | **ALWAYS use `rusqlite` transactions for database writes.** | Never write to SQLite outside an explicit transaction. |

---

## 🛠️ Quick Setup & Pre-Commit Gates

```bash
git config core.hooksPath .githooks   # enable pre-commit hook (fmt + EOL + i18n + bundle-parity + FTL dedupe + column types + PG drift + Go)
```

The `.githooks/pre-commit` hook runs **eight steps** automatically before every commit, in this order (~5–7s total on a typical commit; the i18n gate alone is ~4s):
1. **`cargo fmt --all`** — auto-formats Rust and re-stages what it changed.
2. **Line-ending normalization** — strips CR from staged *text* files in both the working tree and the index, then re-stages them, so the committed blob is LF and `git status` is clean immediately after commit. Backs `.gitattributes` (`* text=auto eol=lf`); a no-op when already LF, and never touches binaries or staged deletions.
3. **`i18n lint`** — runs `scripts/lint-i18n.sh`, fail-closed on three categories: `.id.ftl` siblings byte-identical to their English source, duplicate Fluent keys silently dropped at bundle join, and literal key references resolving in neither locale.
4. **`Bundle parity: staged files only`** — runs `scripts/verify-bundle-parity.py --staged-only --include-getstring --include-nav-keys --include-key-fields --include-dynamic-literals --include-id-maps --check-domain-pairs` over staged files in `features`, `components`, `frontend`, `contexts`, `hooks` and `platform`; fails if any of the **eight checked surfaces** references a key missing from `.ftl` or `.id.ftl`. Before the Fluent page audit this walked only literal `<Localized id>` under `ui/src/features/**`, and skipped every shared-chrome and context file it was handed — so 14 broken keys shipped while it reported clean.
5. **`FTL dedupe dry-run`** — runs `scripts/dedupe-ftl.py --dry-run` to detect duplicate Fluent keys before push.
6. **`Migration column-type lint`** — runs `scripts/verify-migration-column-types.py --staged-only` when `crates/oz-core/migrations/*.sql` is staged; exact-decimal columns must be fixed-point integers (`*_minor`/`*_millionths`), new floats need a justified whitelist entry.
7. **`PG schema drift guard`** — runs `scripts/generate-pg-migration.py --check` when any migration, the registry, or the generator is staged; `20260813_init.pg.sql` is generated, never hand-edited (see [`docs/records/sqlite-pg-roles.md`](./docs/records/sqlite-pg-roles.md)).
8. **`Go gate`** — when the commit stages `apps/license-server/*.go`: `gofmt -w` (auto-fix + re-stage) then `go vet ./...` (hard fail). **Aborts the commit if `go`/`gofmt` are not on PATH** — unlike the optional gates, this one has no skip path.

> ⚠️ **Steps 6, 7 and 8 are local-only.** No live CI job runs them — see the next blockquote. The hook's own Go-gate comment claims CI runs a strict `gofmt -l` check in "`.github/workflows/ci.yml` # go job"; that workflow is retired (`.bak`) and `dev-ci.yml` has **no Go job at all**, so for Go the pre-commit hook is currently the *only* gate, and it is opt-in.

> **What CI actually runs.** `.github/workflows/dev-ci.yml` ("Dev CI") is the **only live workflow** — every other file in that directory is `.bak` and GitHub never executes it (`ci.yml.bak`, `nightly.yml.bak`, `release.yml.bak`, `e2e-pr.yml.bak`, `security.yml.bak`, `android.yml.bak`, `ios.yml.bak`, `deploy.yml.bak`, `website.yml.bak`, `docker-*.yml.bak`), per `23c96330`. Its jobs are `website`, `cargo-check`, `cargo-nextest`, `ui-test`, `i18n`, and `northflank-deploy` (which `needs` all five). So **E2E, a11y, release, security and nightly suites are NOT enforced in CI** — a green Dev CI run is not proof those passed. `scripts/check.sh` is the local full-matrix equivalent; run it before declaring a change verified.
>
> The `i18n` job was restored by the Fluent page audit after `ci.yml` was retired without a replacement. Note that the pre-commit hook is **opt-in per developer**: `core.hooksPath` is set by `scripts/setup-dev.ps1` and is not versioned, so a fresh clone that skips setup has no local gate — CI is the backstop.

---

## 🔑 Global Environment Variables (`OZPOS_*`)

The developer machine's API keys are stored as **user-scope Windows environment variables** with an `OZPOS_` prefix (persisted in `HKCU\Environment`; they survive reboots and are available in every **new** PowerShell session — the session that set them must be reopened). Source of truth: the gitignored `.env` at the repo root.

```powershell
$env:OZPOS_CLOUDFLARE_API_TOKEN        # Cloudflare Workers deploy token
$env:OZPOS_CLOUDFLARE_ACCOUNT_ID       # Cloudflare account id
$env:OZPOS_CLOUDFLARE_ACCESS_KEY       # R2 access key id
$env:OZPOS_CLOUDFLARE_SECRET_ACCESS_KEY# R2 secret access key
$env:OZPOS_CLOUDFLARE_S3_ENDPOINT      # R2 S3 endpoint
$env:OZPOS_NORTHFLANK_API_TOKEN        # Northflank deploy token
$env:OZPOS_OZ_ADMIN_KEY                # admin dashboard API key
$env:OZPOS_OZ_API_SECRET               # JWT signing secret
$env:OZPOS_OZ_ENFORCE_PLANS            # plan gating flag
$env:OZPOS_OZ_LICENSE_PRIVATE_KEY      # RSA license signing key (PEM, multiline)
```

- Use them in commands instead of hardcoding secrets, e.g. the website deploy:
  ```powershell
  $env:CLOUDFLARE_API_TOKEN=$env:OZPOS_CLOUDFLARE_API_TOKEN
  $env:CLOUDFLARE_ACCOUNT_ID=$env:OZPOS_CLOUDFLARE_ACCOUNT_ID
  npm run deploy   # from website/
  ```
- Update `.env` → variables by re-running the save step (same names/prefix); never commit `.env`.
- ⚠️ These are plaintext in the user registry — local-dev convenience only, not a secrets manager.

---

## 💻 Running CLI Tools on Windows

### 1. UI & Front-End CLI (TypeScript / ESLint / Vite)

> ⚠️ **MANDATORY RULE:** `tsc` and `eslint` are **project-local** in `ui/node_modules/.bin/` and are NOT on the system PATH.
> **Agents must ALWAYS run npm scripts from inside the `ui/` directory (`npm run <script>`).** Never invoke bare `tsc` or `eslint`.

| Task | Command (Run from `ui/`) | Description |
|---|---|---|
| **All UI Gates** | `npm run check:all` | Chained gate: lint → typecheck → test → i18n → E2E |
| **Type Check** | `npm run typecheck` | Runs project TypeScript type-checking |
| **Lint** | `npm run lint` | Runs ESLint (a11y + React rules) |
| **Lint Auto-Fix** | `npm run lint:fix` | Runs ESLint with auto-fix |
| **Unit Tests** | `npm run test` | Runs Vitest unit tests |
| **Build Bundle** | `npm run build` | Validates bundle compilation |
| **E2E Suite** | `npm run e2e` | Docker backend + Vite + Playwright + cleanup |
| **E2E UI Only** | `npm run e2e:ui` | Runs Playwright UI spec subset |
| **E2E + Rebuild Images** | `npm run e2e -- --build` | Rebuilds stale `e2e-{cloud,license}-server` images first |

> **Stale-image guard:** `compose up --pull=missing` never rebuilds a tag that already exists, so `run-e2e.mjs` **refuses to start** when a locally-built image predates its own sources (exit 3 · `E2E SETUP FAILED`). Re-run with `--build`. Agents must not treat a green local E2E run as proof of current backend behaviour unless the guard passed. Skipped under `CI`, where the workflow builds from the checkout.

*If `node_modules` is missing, install cleanly:*
```powershell
cd ui
npm ci --no-audit --no-fund
```

### 2. Rust Backend CLI

- **Active Development Iteration:**
  - Quick compilation check: `cargo check -p <crate>`
  - Run specific test: `cargo test -p <crate> <test_name>`
  - 🛑 **Agents must NOT run `cargo clippy` or full workspace tests (`cargo test --workspace`) during routine iteration.**
- **Pre-Push / Final Verification Only:**
  - `cargo fmt --all`
  - `cargo clippy --all-targets --all-features -- -D warnings` (must resolve all warnings)
  - Full workspace tests prior to pushing

---

## 📐 Architecture & Coding Standards

### 1. Rust Standards
- Every public function, struct, enum, and trait must have a doc comment (`///`).
- **Module Documentation:** Every production `.rs` file must start with a short module-level doc comment (`//!`, 5–15 lines max: purpose, key types, main functions, invariants).
- Use `thiserror` for error types and `anyhow` for application-level error propagation.
- **Monetary Values:** Store as integer minor units (`i64`) using `Money`. Never use `f32`/`f64`.
- **Database Writes:** Must run inside a `rusqlite` transaction.

### 2. Test File Organization
- Keep production `.rs` files under 1,000 lines (preferably < 600 lines).
- **Never put unit tests inside production `.rs` files.**
  - Place unit tests in a sibling file named `*_tests.rs` (e.g. `sales.rs` → `sales_tests.rs`).
  - At the bottom of the production file, wire:
    ```rust
    #[cfg(test)]
    #[path = "sales_tests.rs"]
    mod tests;
    ```
  - Inside `sales_tests.rs`, start with `use super::*;`.
  - Integration tests belong in the top-level `tests/` directory (outside `src/`).

### 3. Tauri & UI Standards
- Tauri IPC commands live in `apps/desktop-client/src/commands/` or `apps/tablet-client/src/commands/` and are registered in their respective `lib.rs`.
- Front-end API calls must route through `ui/src/api/` (per-domain files). **Never call `invoke(...)` directly inside React components.**
- **"Settings" disambiguation:** "The Tauri Settings page" means `ui/src/features/settings/SettingsPage.tsx` — the master–detail UI (route `settings`) with the top-right Save button. Do not confuse it with the other `settings` surfaces: `ui/src/api/settings.ts` (IPC client) · `ui/src/contexts/SettingsContext.tsx` (shared state) · `apps/desktop-client/src/commands/settings.rs` and `apps/tablet-client/src/commands/settings.rs` (IPC commands) · `crates/oz-core/src/settings.rs` and `crates/oz-core/src/db/settings.rs` (backend service/DB) · `modules/settings/` (the kernel module — a lifecycle stub, not the UI).
- **⚠️ "Is this screen dead code?" needs three greps, not one.** Feature screens are registered **lazily**, so a `from '…'` search finds nothing and the screen looks unreachable. `ui/src/features/*/register.tsx` opens with `const X = lazy(() => import('./X'))` and then calls `registerPage({ route, component: X, … })` + `registerNavItem(...)`. Concluding "not routed" from one grep is how a live, nav-linked manager screen got written up as deletable. Check all three:
  ```bash
  git grep -n "ScreenName" -- ui/src ':!ui/src/__tests__'   # no -head/-First truncation
  git grep -n "route: '<route>'" -- ui/src                  # page + nav + workspace cards
  ```
  Also beware truncated output: `ui/src/__tests__/` sorts before `ui/src/features/`, so `| head -6` on an importer search shows only test files and hides the real registration.
- **⚠️ A `:!` pathspec built by inline concatenation silently returns zero matches.** In PowerShell argument position, `git grep -n X -- ui/src ':!ui/src/__tests__' ':!dir/'+$f+'.tsx'` yields **0 hits** with no error, while assigning the same string to a variable first yields the correct 2. A false "no references" result is precisely the evidence that makes a live screen look dead — this produced a wrong "not routed, delete it" conclusion once already. Build exclusion pathspecs as their own variable, and sanity-check any zero-reference dead-code claim with one unfiltered `git grep`.
- **Accessibility:** All React components must have ARIA labels and pass `eslint-plugin-jsx-a11y` checks.
- **Localization:** All user-visible strings must use `@fluent/react`. No hardcoded English strings in JSX.

### 4. Database & Hardware
- **HAL Drivers:** Hardware drivers must have a mock implementation in `crates/oz-hal/src/drivers/mock.rs`.
- **SQLite is the schema source of truth:** `crates/oz-core/migrations/*.sql` + the registry in `migrations.rs` (registry order is canonical). See [`docs/records/sqlite-pg-roles.md`](./docs/records/sqlite-pg-roles.md).
- **`init.pg.sql` is generated, never hand-edited:** after any migration change run `python3 scripts/generate-pg-migration.py` and re-stage `crates/oz-core/migrations/20260813_init.pg.sql`. The **pre-commit** PG schema drift guard (step 7) fails on drift — but there is currently **no CI backstop for it**: the `pg-schema-drift` and `migration-column-types` jobs lived in `ci.yml`, which was retired to `.bak` in `23c96330`, and neither was restored in `dev-ci.yml`. Only the `i18n` job was brought back. So on a clone without `core.hooksPath` set, a hand-edited or stale `init.pg.sql` commits and merges clean.
- **PostgreSQL Drift:** When modifying Postgres schemas, run `bash scripts/reset-dev-pg.sh` to re-synchronize the shared dev container schema (`oz-pg-test-15432`).

---

## 🌿 Git & Commit Policy

### 1. Branch Policy
- **Never create new branches unless asked specifically by the user.** Always work directly on the currently active branch.
- **Never switch local branches unless explicitly asked by the user.**
- Branch naming convention (when explicitly requested): `feat/<name>`, `fix/<name>`, `docs/<name>`, `chore/<name>`, `test/<name>`, `refactor/<name>`.

### 2. Commit Format Requirement
Every commit message **MUST** strictly follow the conventional format:
```
<type>(<area>): <description>

[optional body explaining changes, rationale, or issue references]
```

- **`<type>`** must be one of:
  - `feat`: New feature or capability
  - `fix`: Bug fix
  - `docs`: Documentation updates
  - `chore`: Maintenance, dependencies, configs
  - `test`: Adding or modifying tests
  - `refactor`: Code refactoring without functional changes
  - `style`: Cosmetic changes that alter no runtime behaviour — `cargo fmt` wraps, CSS adjustments, copy edits. In established use (`130c7556`, `c3b7c72b`, `ad9c60e9`, `e0f2ca9b`, `04465711`, `2d517b55`, `7dde51c2`, `cfd0f183`); added to this list so the documented set matches practice and the `commit-msg` gate does not reject it.
  - `perf`: Performance improvements
  - `ci`: CI workflows, GitHub Actions, build scripts
  - `audit`: Code audit stamps and remediations
- **`<area>`**: Domain, crate, or component (e.g. `sales`, `admin`, `website`, `ci`, `core`, `desktop-client`, `ui`, `licensing`, `agents`).
- **`<description>`**: Imperative, concise summary of the change (e.g. `add gift card tender`, `resolve modal overflow`).

> **This is now actually enforced** by `.githooks/commit-msg`, which rejects a
> non-conforming subject and prints the allowed list. Subject line only — bodies
> stay free-form. Git-generated messages (`Merge …`, `Revert …`, `fixup!`,
> `squash!`) and an empty subject pass through, so normal git workflows still
> work. Like every other hook here it needs `core.hooksPath`, so a fresh clone
> that skips `scripts/setup-dev.ps1` is not gated.

### 3. Commit Cadence & Push Rule
- **Always make a local commit after each major modification.** Whenever a logical task or feature step is completed and verified locally, commit it before moving on to the next task.
- **Never run `git push` without an explicit, direct order from the user.** Even after committing code or completing verification, always wait for the user to explicitly instruct you to push before executing any `git push` command.
- Never commit secrets, `.env` files, or SQLite database files (`*.db`, `*.sqlite`).
