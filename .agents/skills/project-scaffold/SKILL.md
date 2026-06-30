---
name: project-scaffold
description: Project scaffolding, Cargo workspace layout, CI configuration, and Git conventions for OZ-POS. Use when setting up the initial repo, adding a new crate, configuring GitHub Actions, or committing changes.
---

# Project Scaffold, CI & Git

OZ-POS is a multi-crate Cargo workspace with a Tauri front-end, a strict style policy, and a CI pipeline that catches mistakes before they merge. This skill covers the workspace layout, the CI matrix, and the Git workflow.

---

## When to use

- Scaffolding the initial repository (Cargo workspace, crates, CI).
- Adding a new crate to the workspace.
- Adding a new CI check (lint, test, build, security audit).
- Committing a change (branch name, commit message format).
- Reviewing a PR for missing checks, wrong scope, or wrong branch.
- Configuring the GitHub Actions matrix (Linux, Windows, macOS, Android, iOS).

---

## Golden rules

| # | Rule | Why |
|---|------|-----|
| 1 | **Branch names are `feat/<name>`, `fix/<name>`, `docs/<name>`, `chore/<name>`.** | Searchable, automatable. |
| 2 | **Commit messages follow Conventional Commits.** | Auto-generated changelogs, semantic versioning. |
| 3 | **All PRs pass CI before merge.** | Rust fmt, clippy, tests, UI lint, build. |
| 4 | **Never commit `.env`, secrets, or SQLite database files.** | PCI-DSS, basic hygiene. |
| 5 | **One crate per `oz-*` responsibility.** | Compile-time boundaries, fast incremental builds. |

---

## Cargo workspace layout

```toml
# Cargo.toml (workspace root)

[workspace]
resolver = "2"
members = [
    "crates/oz-core",
    "crates/oz-hal",
    "crates/oz-lua",
    "crates/oz-security",
    "crates/oz-payment",
    "crates/oz-reporting",
    "crates/oz-logging",
    "crates/oz-cli",
    "src-tauri",
]

[workspace.package]
version = "0.0.1"
edition = "2024"
rust-version = "1.85"   # edition 2024 requires Rust 1.85+
license = "MIT"

[workspace.dependencies]
# all crates import from here: oz-core = { workspace = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

```
oz-pos/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── oz-core/                # money, currency, cart, sale, inventory domain
│   ├── oz-hal/                 # hardware abstraction + drivers
│   ├── oz-lua/                 # rlua runtime + script bindings
│   ├── oz-security/            # encryption, secrets, PCI helpers
│   ├── oz-payment/             # Stripe, Square, EMV abstraction
│   ├── oz-reporting/           # analytics + CSV export
│   ├── oz-logging/             # structured logging
│   └── oz-cli/                 # migrations, backup, export CLI
├── src-tauri/                  # the desktop/mobile shell
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       ├── error.rs
│       └── state.rs
├── ui/                         # React + TypeScript
│   ├── package.json
│   ├── tsconfig.json
│   └── src/
├── migrations/                 # SQL migration files
│   ├── 20260628_0001_init.sql
│   └── ...
├── docs/
│   ├── ARCHITECTURE.md
│   ├── ROADMAP.md
│   ├── WHITEPAPER.md
│   ├── QUICKSTART.md
│   └── specs/
│       ├── _template/
│       ├── _active/
│       └── _done/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       ├── security.yml
│       └── release.yml
└── scripts/                    # local dev scripts (PowerShell + bash)
```

---

## Scaffolding a new crate

```bash
# from the workspace root
cargo new --lib crates/oz-<name>
```

```toml
# crates/oz-<name>/Cargo.toml

[package]
name = "oz-<name>"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
oz-core = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
```

```rust
// crates/oz-<name>/src/lib.rs

//! <One-line summary of what this crate does>.
//!
//! <Longer paragraph explaining the responsibility, the public surface,
//! and any non-obvious invariants.>

#![deny(unsafe_code)]   // unless the crate genuinely needs unsafe
#![warn(missing_docs)]  // public items must be documented

pub mod error;

pub use error::<Crate>Error;
```

Then add the new crate to the workspace `members` in the root `Cargo.toml`.

**Rules:**
- `#![warn(missing_docs)]` is on by default. Public items without `///` produce warnings; fix them, don't suppress.
- `#![deny(unsafe_code)]` unless the crate is `oz-hal` (drivers may need `unsafe` for FFI). Even then, wrap `unsafe` blocks with `// SAFETY:` comments.
- Each crate has a `README.md` with a one-paragraph summary, public API overview, and example.
- The crate's `mod.rs` re-exports the public surface so users can `use oz_<name>::Type;`.

---

## Git workflow

### Branch naming

| Prefix | When to use | Example |
|--------|-------------|---------|
| `feat/<name>` | New feature, capability, or user-visible change | `feat/cart-line-discount` |
| `fix/<name>` | Bug fix | `fix/cart-overflow-on-coupon` |
| `docs/<name>` | Documentation only | `docs/i18n-contributor-guide` |
| `chore/<name>` | Maintenance, deps, config, refactor with no behavior change | `chore/bump-tauri-v2.1` |
| `test/<name>` | Test additions or fixes | `test/integration-sales-flow` |
| `refactor/<name>` | Code restructuring, no behavior change | `refactor/extract-payment-port` |

`<name>` is kebab-case, short, and describes the change. The branch is deleted after merge.

### Commit message format (Conventional Commits)

```
<type>(<optional scope>): <short summary> [optional body] [optional footer(s)]
```

- `type` matches the branch prefix.
- Summary is ≤ 72 characters, imperative mood ("add" not "added").
- Body explains *why*; the diff shows *what*.
- Footer for breaking changes: `BREAKING CHANGE: <description>`.

**Examples:**

```
feat(cart): apply line-level discounts before tax

Line-level discounts were applied after tax computation, producing
incorrect totals for high-tax jurisdictions. Apply discounts to the
line subtotal first, then tax the discounted amount.

Closes #142
```

```
fix(payment): retry once on transient network errors

Stripe occasionally returns 502 on authorization. A single retry with
a 250ms backoff recovers most cases without idempotency risk.
```

```
chore: bump tauri to v2.1 and refresh lockfile
```

**Forbidden prefixes:** `update`, `fix`, `changes`, `wip`, `minor`. These are too vague.

---

## CI pipeline

`.github/workflows/ci.yml` runs on every push and PR.

```yaml
name: CI

on: [push, pull_request]

jobs:
  rust:
    name: Rust (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --workspace --all-features

  ui:
    name: UI
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - run: cd ui && npm ci
      - run: cd ui && npm run lint
      - run: cd ui && npm run typecheck
      - run: cd ui && npm run test
      - run: cd ui && npm run build

  sql:
    name: Migration check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo run -p oz-cli -- migrate --check   # exits non-zero if migrations are unrunnable
```

**Rules:**
- `fail-fast: false` on the matrix — one platform failing doesn't hide others.
- Cache `~/.cargo` and `target/` via `Swatinem/rust-cache`.
- `cargo fmt --check` and `cargo clippy -- -D warnings` are blocking. Fix the warnings, don't suppress them.
- Tests must pass on **all three desktop OSes**. Don't disable platform-specific tests; fix them.

A separate `security.yml` runs `cargo audit` and `cargo deny` against the dependency tree.

---

## Local pre-commit

```bash
# scripts/check.sh (POSIX) or scripts/check.ps1 (Windows)
set -euo pipefail

echo "==> fmt"
cargo fmt --all -- --check

echo "==> clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> test"
cargo test --workspace --all-features

echo "==> ui lint"
(cd ui && npm run lint)

echo "==> ui typecheck"
(cd ui && npm run typecheck)
```

Run this before pushing. The CI matrix is the source of truth, but a local pass catches 90% of issues.

---

## What NOT to commit

- `.env`, `.env.local`, `.env.production` — secrets.
- `*.db`, `*.sqlite`, `*.sqlite3` — local databases.
- `target/`, `node_modules/`, `dist/` — build artifacts.
- `*.key`, `*.pem`, `secrets/` — credentials.
- `Cargo.lock` is **committed** for binaries (the Tauri app). For library crates, leave it out of `.gitignore` and let consumers pin it.

A `.gitignore` template:

```gitignore
# Build artifacts
/target/
/ui/dist/
/ui/node_modules/

# Local state
*.db
*.sqlite
*.sqlite3
.env
.env.local
*.key
*.pem

# Editor
.vscode/
.idea/
*.swp

# OS
.DS_Store
Thumbs.db
```

---

## Spec workflow (optional but recommended)

Larger changes are tracked as specs under `docs/specs/_active/<id>/`. Each spec is a small package:

```
docs/specs/_active/0042-cart-discount-engine/
├── spec.yaml          # metadata: id, title, status, owner, priority, scope
├── plan.md            # baseline, implementation steps, API changes, validation
└── validation.md      # acceptance criteria, test commands, visual checks
```

**Status values** (by directory):

- `_template/` — `draft`
- `_active/` — `approved`, `in-progress`, `implemented`, `needs-human-approval`
- `_done/` — `done`

When a spec finishes, move its folder from `_active/` to `_done/`.

**Rules:**
- Specs are for changes that touch more than one crate or have user-visible behavior.
- Bug fixes and small features don't need a spec.
- A spec's `id` is `NNNN-kebab-summary` (zero-padded to 4 digits, kebab-case summary).
- The `owner` is the human or team responsible. The `implementer` is whoever writes the code (often an agent).

---

## Adding a new CI check — checklist

- [ ] Define the check in `.github/workflows/<name>.yml`.
- [ ] Add the corresponding local script to `scripts/check.sh` and `scripts/check.ps1`.
- [ ] Document the check in this skill (so future contributors know it exists).
- [ ] Update the pre-push checklist at the bottom of this file if it's a blocking check.
- [ ] Add a status badge to `README.md`.

---

## Common pitfalls

1. **Adding a new crate to the wrong place** (e.g., `src/` instead of `crates/`). The workspace `members` list must include it, and it should follow the `oz-<name>` naming.
2. **Committing `Cargo.lock` for a library crate.** It's only for binaries. The `src-tauri` and `oz-cli` binaries should commit it.
3. **Using `git commit --no-verify`** to skip pre-commit hooks. Fix the issue, don't bypass it.
4. **Renaming a branch after pushing.** The PR link changes, CI re-runs needlessly. Pick the name right the first time.
5. **Squash-merging a multi-commit feature branch** — fine, but the squash message must be a clean Conventional Commit, not the WIP history.
6. **Adding `cargo update` to a feature PR.** Bumping unrelated deps makes the diff unreviewable. Update in a separate `chore/` PR.
7. **Forgetting to register a new command** in `tauri::generate_handler!` — the command compiles but is not callable. CI's smoke test catches this.
8. **Setting `fail-fast: true`** on the CI matrix. One flaky platform hides failures on others.

---

## Pre-push checklist (every PR)

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes on Linux, Windows, macOS
- [ ] `cargo audit` and `cargo deny` are clean
- [ ] `cd ui && npm run lint && npm run typecheck && npm run test && npm run build` all pass
- [ ] No `.env`, `.db`, `*.key`, or `target/` files in the diff
- [ ] Commit messages follow Conventional Commits
- [ ] Branch name matches the change type
- [ ] Spec folder moved to `_done/` if it was a spec-driven change

---

> last audited 28-06-26 by docs-auditor
