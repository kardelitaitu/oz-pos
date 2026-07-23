# Contributing to OZ-POS

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (2 noted findings) · F1 (doc bug): internal contradiction — line 45 lists `fix/<name>` as a valid branch prefix and line 79 shows `fix(payment):` as a correct example, but line 85 forbids `fix` as a commit prefix; the `fix/` type is simultaneously allowed and forbidden and the doc's own example violates its rule · F2 (minor): references SECURITY.md ("when it exists") — file still absent, but hedged so consistent · verified accurate: all referenced docs/skills/scripts exist (WHITEPAPER, QUICKSTART, ROADMAP, ARCHITECTURE, AGENTS, LICENSE, onboarding-guide, skill-drift-guard, detect.sh, coverage.sh); PR commands match AGENTS.md -->

Thanks for your interest in OZ-POS! This project is a Rust + Tauri v2 POS framework built around a "wizard behind the curtain" philosophy: the merchant sees effortless checkout, and the lean Rust engine silently handles transactions, encryption, hardware, sync, and business logic.

This guide covers how to contribute effectively. The full project conventions live in `AGENTS.md` (the source of truth for coding standards) and the skills under `.agents/skills/` (the source of truth for how to do specific tasks).

---

## Quick links

| Document | What it's for |
|----------|---------------|
| [`AGENTS.md`](./AGENTS.md) | Project-wide coding standards (Rust, Tauri, UI, testing, Git) |
| [`ARCHITECTURE.md`](./docs/ARCHITECTURE.md) | Directory layout and module responsibilities |
| [`ROADMAP.md`](./docs/ROADMAP.md) | Phased milestones and what we're building next |
| [`WHITEPAPER.md`](./docs/WHITEPAPER.md) | Design rationale, tech choices, database strategy |
| [`docs/QUICKSTART.md`](./docs/QUICKSTART.md) | First-time local setup and build |
| [`.agents/skills/onboarding-guide`](./.agents/skills/onboarding-guide/SKILL.md) | Meta-skill that routes you to the right specialized skill |
| [`.agents/skills/skill-drift-guard`](./.agents/skills/skill-drift-guard/SKILL.md) | Detects and patches skill drift — run before opening a PR |

---

## Code of conduct

Be respectful, assume good faith, and keep feedback focused on the work. We are building software that handles real money for real merchants — rigor and care matter more than speed.

---

## Before your first commit

1. **Read [`AGENTS.md`](./AGENTS.md)** — it defines the non-negotiables (Money struct, rusqlite transactions, thiserror/anyhow, clippy `-D warnings`, Conventional Commits, etc.).
2. **Read [`docs/QUICKSTART.md`](./docs/QUICKSTART.md)** — get the project building locally first.
3. **Read the relevant skill** under `.agents/skills/` (the onboarding guide will route you).
4. **Skim [`ROADMAP.md`](./docs/ROADMAP.md)** so you know what's in scope and what's deferred.

If your change touches more than one layer (Rust core, Tauri IPC, UI, HAL, project structure), read each relevant skill in layer order: `rust-backend` → `tauri-ipc` → `ui-components` → `hal-drivers`.

---

## Branch naming

| Prefix | When to use | Example |
|--------|-------------|---------|
| `feat/<name>` | New feature, capability, or user-visible change | `feat/cart-line-discount` |
| `fix/<name>` | Bug fix | `fix/cart-overflow-on-coupon` |
| `docs/<name>` | Documentation only | `docs/i18n-contributor-guide` |
| `chore/<name>` | Maintenance, deps, config, refactor with no behavior change | `chore/bump-tauri-v2.1` |
| `test/<name>` | Test additions or fixes | `test/integration-sales-flow` |
| `refactor/<name>` | Code restructuring, no behavior change | `refactor/extract-payment-port` |

`<name>` is kebab-case, short, and describes the change. The branch is deleted after merge.

---

## Commit messages (Conventional Commits)

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

**Forbidden prefixes:** `update`, `fix`, `changes`, `wip`, `minor`. These are too vague.

---

## Adding a new skill

Skills are the project's living documentation. When you discover a pattern that the skills don't cover — a new crate convention, a new CI check, a new accessibility rule — write a new skill under `.agents/skills/<skill-name>/SKILL.md`.

**Anatomy of a good skill:**

1. **YAML frontmatter** with `name` and `description` (description is what the agent router matches against).
2. **"When to use"** section — be explicit about the trigger conditions.
3. **"Golden rules"** table — the non-negotiables for this area.
4. **Concrete examples** with copy-pasteable code.
5. **"Common pitfalls"** section at the end.
6. **Footer**: `> last audited <DD-MM-YY> by <who>`.

After adding a skill, update `.agents/skills/onboarding-guide/SKILL.md` so the router table points to it. The `skill-drift-guard` script will catch the omission on the next CI run.

---

## Before opening a PR

Run the local checks (the CI matrix is the source of truth, but a local pass catches 90% of issues):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

For UI changes:

```bash
cd ui && npm run lint && npm run typecheck && npm run test && npm run build
```

For coverage spot-checks (optional, not part of the PR gate yet):

```bash
bash scripts/coverage.sh         # rust + ui
bash scripts/coverage.sh rust    # just rust
bash scripts/coverage.sh ui      # just ui
```

Reports land in `coverage/{rust,ui}/index.html`. The CI `coverage` job uploads the same artifacts on every push to `main`. Use them to spot under-tested modules after refactors.

If your change touches something a skill describes (a path, a type, a trait, a dependency version, a golden rule), run the drift guard before opening the PR:

```bash
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
```

If the report surfaces findings, fix them in the same PR. If a finding is wrong or out of scope, open an issue.

---

## PR checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes on Linux, Windows, macOS
- [ ] `cargo audit` and `cargo deny` are clean
- [ ] `cd ui && npm run lint && npm run typecheck && npm run test && npm run build` all pass (if UI changed)
- [ ] `skill-drift-guard` reports zero manual findings (or findings are addressed in this PR)
- [ ] No `.env`, `.db`, `*.key`, or `target/` files in the diff
- [ ] Commit messages follow Conventional Commits
- [ ] Branch name matches the change type
- [ ] If a spec was opened for this change, its folder was moved from `docs/specs/_active/` to `docs/specs/_done/`
- [ ] Public items have `///` doc comments
- [ ] New skills are added to `onboarding-guide`'s router table
- [ ] User-visible strings use `@fluent/react` (no hardcoded English in JSX)
- [ ] Money is `i64` minor units, never `f32`/`f64`
- [ ] All database writes happen inside a `rusqlite::Transaction`

---

## Reviewing a PR

When reviewing, focus on:

1. **Correctness first.** Does the change do what it claims? Are there edge cases the tests miss?
2. **Standards compliance.** Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and the drift guard. Don't merge if they fail.
3. **Money safety.** Any change touching `Money`, currency, or totals gets a second look.
4. **Database writes.** Any new write path must use a `Transaction`. No exceptions.
5. **Public API surface.** Public items need `///` docs. Trait changes need an ADR or spec note.
6. **Skill alignment.** Does the change match what the relevant skill says? If not, either the change is wrong or the skill is.

Be specific in your review comments. "This is wrong" is not actionable; "This Money conversion can overflow when the cart total exceeds i64::MAX / 100" is.

---

## Reporting issues

Open a GitHub issue with:

- A clear, specific title
- Steps to reproduce (for bugs) or the user story (for features)
- Expected vs actual behavior
- Platform (Windows, Linux, macOS, Android, iPad) and version
- Relevant logs or screenshots
- A link to the relevant skill (if the issue is a skill/code mismatch)

For **security issues**, do **not** open a public issue. Email the maintainers directly (see `SECURITY.md` when it exists; until then, use the GitHub security advisory flow).

---

## License

By contributing, you agree that your contributions will be licensed under the same proprietary license as the project (see [`LICENSE`](./LICENSE)). OZ-POS is proprietary and confidential — copyright remains with the OZ-POS Contributors.

---

> last audited 17-07-26 by docs-auditor
