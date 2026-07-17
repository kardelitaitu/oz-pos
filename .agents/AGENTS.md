# Agents Configuration

## Global Rules

- Maintain documentation integrity. Preserve all existing comments and docstrings unless explicitly modified.
- Never switch local branches unless explicitly asked by the user.

## Quick Setup

```bash
git config core.hooksPath .githooks   # enable pre-commit hook (cargo fmt + i18n lint + bundle-parity + FTL dedupe)
```

The `.githooks/pre-commit` hook runs four gates before every commit (~1s total):

1. **`cargo fmt --all`** — auto-formats staged Rust files and re-stages them.
2. **`i18n lint`** — runs `scripts/lint-i18n.sh` (catches `.id.ftl` byte-identical to its `.ftl` sibling + Fluent key duplicates + an informational bundle-parity surface).
3. **`Bundle parity: staged files only`** — runs `scripts/verify-bundle-parity.py --staged-only …` on staged `.tsx` / `.ts` files in `ui/src/features/**`; fails-closed if any new `<Localized id>` references a key missing from one or both `.ftl` bundles.
4. **`FTL dedupe dry-run`** — runs `scripts/dedupe-ftl.py --dry-run` so any duplicate Fluent key surfaces BEFORE push.

Without this `core.hooksPath` set, all four gates are silently bypassed at commit time (CI catches them later, but only the i18n lint as an informational surface; the bundle-parity + FTL dedupe checks run only at CI time).

For comprehensive local validation that mirrors the entire CI matrix (not just the pre-commit subset), see [`scripts/check.sh`](./scripts/check.sh). For the full first-time setup walkthrough (4 gates explained, chmod, verify hint), see [`.agents/skills/onboarding-guide/SKILL.md#first-time-setup`](./.agents/skills/onboarding-guide/SKILL.md#first-time-setup).

## Running UI CLI Tools on Windows (tsc / eslint)

`tsc` and `eslint` are **project-local** — they live in `ui/node_modules/.bin/` and are
NOT on the system PATH by default. On Windows PowerShell every command that calls
these tools must prefix the PATH for that session, because each shell subprocess
starts fresh.

### Correct pattern (PowerShell)

Prepend `ui\node_modules\.bin` to `$env:PATH` **in the same command** as the tool:

```powershell
# Type-check the whole project (run from project root)
$env:PATH = "$PWD\ui\node_modules\.bin;" + $env:PATH; tsc --noEmit

# Lint a single file
$env:PATH = "$PWD\ui\node_modules\.bin;" + $env:PATH; eslint ui/src/frontend/shell/AppShell.tsx
```

### Preferred alternative — use npm scripts

`ui/package.json` already wraps the tools as npm scripts, and npm resolves
`node_modules/.bin` automatically on every platform:

| Task | Command (run from `ui/`) |
|------|--------------------------|
| Type-check | `npm run typecheck` |
| Lint | `npm run lint` |
| Lint + auto-fix | `npm run lint:fix` |
| Build (type-check + bundle) | `npm run build` |
| Tests | `npm run test` |

```powershell
# Example — always run from the ui/ directory
cd ui
npm run typecheck
npm run lint
```

> **Rule:** Agents must use `npm run <script>` (not bare `tsc`/`eslint`) unless the
> PATH prefix pattern above is applied first. Never assume `tsc` or `eslint` are
> globally available on this machine.

### If node_modules is missing

Run `npm install` inside `ui/` before any of the above:

```powershell
cd ui
npm install
```

---

## Project Specific Rules

- Follow the POS software framework conventions.
- Ensure all code follows the project's coding standards.
- **Version is locked at `0.0.9`.** Never change the version number
  (in `Cargo.toml`, `tauri.conf.json`, `package.json`, `CHANGELOG.md`,
  or anywhere else) unless the user explicitly asks you to bump it.

### Rust Standards
- Format all Rust code with `rustfmt` before committing.
- Run `cargo clippy -- -D warnings` and resolve all warnings.
- Every public function, struct, and trait must have a doc comment (`///`).
- Prefer `thiserror` for error types and `anyhow` for application-level error propagation.
- Store all monetary values as integer minor units (`i64`) using the `Money` struct; never use `f32`/`f64` for currency.
- Use `rusqlite` with transactions for all database writes; never write outside a transaction.

### Tauri / UI Standards
- Tauri commands must be defined in `apps/desktop-client/src/commands/` or `apps/tablet-client/src/commands/` and registered in their respective `lib.rs`.
- Front-end API calls go through `ui/src/api/` (per-domain files); do not call `invoke` directly in components.
- All React components must have ARIA labels and pass `eslint-plugin-jsx-a11y` checks.
- Use `@fluent/react` for all user-visible strings; no hardcoded English strings in JSX.

### Testing Standards
- Every new Rust module must include a `#[cfg(test)]` block with at least one unit test.
- HAL drivers must have a mock implementation in `crates/oz-hal/src/drivers/mock.rs` for testing.
- Front-end components must have a corresponding test in `ui/src/__tests__/`.

### Git & Branch Policy
- Branch naming: `feat/<name>`, `fix/<name>`, `docs/<name>`, `chore/<name>`, `test/<name>`, `refactor/<name>`.
- **Always make a local commit after each major modification.** Whenever a logical task, feature step, or significant code change is completed and verified locally, commit it before moving on to the next task. The commit message must accurately and comprehensively explain what was changed across all committed files.

- **Never run `git push` without an explicit, direct order from the user.** Even after committing code or completing verification, always wait for the user to explicitly instruct you to push before executing any `git push` command.
- All PRs must pass the CI pipeline (lint, test, build) before merging.

- CI only triggers on the `main` branch (push + pull_request). Feature-branch
  pushes do not run CI; open a PR targeting `main` to validate changes.
- Never commit secrets, `.env` files, or SQLite database files.

> [!NOTE]
> This file serves as the central place to define agents, rules, and customisation for the POS framework.
