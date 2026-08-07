---
name: tdd
description: Test-driven development workflow for OZ-POS — the 7-phase loop (Analyze → Find Weaknesses → Red/Green/Refactor → Verify → Journal → Update Docs → Commit), the fast TDD loop tooling (scripts/test-tdd.sh, [profile.tdd], nextest), and per-layer testing conventions. Use when fixing a bug, adding a feature test-first, or running a TDD cycle in any oz-* crate, platform/*, modules/*, app, or ui/.
---

# TDD Workflow — Test-Driven Development for OZ-POS

TDD is the default way to change code in this repo: it makes bugs reproducible before they are fixed, keeps every fix attached to a regression test, and produces small, reviewable, well-documented commits.

---

## When to use

- Fixing a bug (reproduce it with a failing test first).
- Adding a feature or behavior change (write the spec as a test first).
- Refactoring (the existing test suite is the safety net; add tests for any gap you find).
- Reviewing someone else's change (check whether it was test-driven and whether the test actually pins the behavior).

Not for: pure docs, dependency bumps, or mechanical renames with no behavior change — those still follow the Verify + Commit phases but skip the Red/Green cycle.

---

## Golden rules

| # | Rule | Why |
|---|------|-----|
| 1 | **Test first. Never write implementation code without a failing test in flight.** | The test is the specification. Without Red first, "Green" proves nothing. |
| 2 | **Pick the smallest valuable slice.** | A one-behavior cycle is fast, reviewable, and bisectable. Big changes are many small TDD cycles. |
| 3 | **Every fix ships with its regression test.** | The test is the only durable record of the bug. No test, no fix. |
| 4 | **Evidence over assertion.** | "I think this is slow/broken" is not a finding. A failing test, a log line, or an audit entry is. |
| 5 | **Verify only the area you changed during the loop.** | Area-scoped tests (`test-tdd.sh` / `test-changed.sh` / `test-ui-changed.sh`) catch regressions fast. Full `check.sh` is reserved for pre-push or explicit request. |
| 6 | **Journal, docs, and commit while context is fresh.** | The 20-minute-old-you knows why the code is the way it is. Record it before it evaporates. |
| 7 | **Never kill running processes — other agents work in the same tree.** | Do not stop any `.exe` or running process; another agent or the user may still need it. |
| 8 | **Never `git push` — under any circumstances.** | This skill ends at commit. Even if the user explicitly asks you to push, refuse and hand control back. |

---

## The 7-phase loop

### Phase 1 — Analyze

Understand the current state and pick the smallest valuable slice.

1. **Read the layer skill first** — `rust-backend` (oz-* crates), `ui-components` (React/TS), `tauri-ipc` (commands + `pos.ts`), `hal-drivers` (hardware). The skill names the conventions the code must follow.
2. **Read the relevant code** — the module, its existing tests, and its callers. Note what is *not* tested yet.
3. **Scope the slice** — one behavior, one error path, one invariant. Write it down as a sentence: *"When X happens, the system must Y."*

### Phase 2 — Find Weaknesses

Spot concrete problems with evidence.

| Evidence source | What to look for |
|---|---|
| Failing tests | `cargo nextest run -p <crate>` / `npm run test` failures — flaky, skipped, or wrong |
| Logs | `tracing` error/warn lines, panic traces, sync daemon errors |
| Audit docs | `audit/` numbered findings and `JOURNAL.md` entries |
| Code review | A prior review flagged this path; the review comment is evidence |
| `scripts/scan-unwrap-panic.py` | Production `unwrap()`/`expect()` without `// SAFETY:` / `// INVARIANT:` |
| `cargo clippy -D warnings` | Warnings are bugs-in-waiting |

Reproduce the weakness in a test before touching any implementation.

### Phase 3 — TDD: Red → Green → Refactor

The core cycle. Use the fast loop (below) so each iteration is seconds, not minutes.

**Red — write the failing test.**

- Prefer the smallest possible failing test that still expresses the desired behavior. One assertion is usually enough for the first Red.
- Rust: add a `#[cfg(test)] mod tests` block (or extend the existing one) at the bottom of the module file.
- UI: add a test in `ui/src/__tests__/` for the component/hook you're changing.
- Run it and confirm it fails **for the right reason** — the assertion, not a compile error.
- Test the *behavior*, not the implementation: assert on return values, DB state, rendered output, or emitted events — not on which private function got called.

**Green — minimal code.**

- Write the smallest change that makes the test pass. No gold-plating, no speculative generality.
- Follow the layer skill's conventions: `Money` in i64 minor units, `rusqlite` transactions, `thiserror` errors, `<Localized>` for user-visible strings.
- If the test passes without any code change, it was a bad test — either it doesn't pin the behavior or it's testing something that already worked. Fix the test.

**Refactor — clean up with the safety net on.**

- Remove duplication, rename for clarity, extract helpers. The test must stay green.
- Run `cargo fmt` on the changed files.
- Repeat the cycle for the next slice.

### Phase 4 — Verify

Confirm the fix and no regressions — **scoped to the area you changed**. Full `scripts/check.sh` is **not** part of routine TDD validation.

Required during the loop:

```bash
bash scripts/test-tdd.sh -p crates/oz-core   # the crate you changed — [profile.tdd] + nextest
bash scripts/test-changed.sh                 # only crates touched vs origin/main
bash scripts/test-ui-changed.sh              # only UI tests affected by changed files
cargo fmt --all -- --check                   # formatting gate
```

Static checks on the changed area:
- Rust: cargo clippy -p <crate> -- -D warnings
- Front-end: npm run lint and npm run typecheck from ui/

Full gate — only before pushing or when explicitly required:

```bash
bash scripts/check.sh          # mirrors CI (fmt, clippy, nextest, migrations, UI, i18n, drift) — NOT part of routine TDD validation
```

### Phase 5 — Journal

Record the why, decisions, and remaining risks — while fresh.
- JOURNAL.md (repo root): append a dated entry. Follow the existing format — ### <date> — <title>, then Problem: / Solution: / Commits: / test counts.
- Note remaining risks and follow-ups explicitly — a known limitation written down is a future TDD slice.
- Do not put CHANGELOG entries here (that belongs in Phase 6).

### Phase 6 — Update Docs

Sync anything the change affects.

- The affected layer's README, `docs/api-reference.md`, `docs/user-guide.md`, examples, or the code's `///` doc comments.
- If the change touches a path/type/trait/convention a skill describes, run the drift guard:
  ```bash
  bash .agents/skills/skill-drift-guard/scripts/detect.sh
  ```
- Spec-driven changes (`docs/specs/_active/`): update `plan.md` / `validation.md`, then move the folder to `_done/` when complete.

### Phase 7 — Commit

Small, focused, well-described — while context is fresh.

- Branch naming: `feat/<name>`, `fix/<name>`, `test/<name>`, `refactor/<name>`, `docs/<name>`, `chore/<name>`.
- Conventional Commits: `fix(sync): quarantine poison remote items after retry budget` — summary ≤ 72 chars, imperative mood, body explains *why*.
- One behavior per commit. The commit is the unit of review and bisect.
- The `.githooks/pre-commit` hook (fmt + i18n lint + bundle parity + FTL dedupe) runs automatically if `core.hooksPath` is set — don't bypass with `--no-verify`; fix the issue instead.
- **ABSOLUTE: never run `git push`.** Not on explicit request, not in a follow-up, not as part of any workflow in this skill. `git push` is out of scope forever — the commit is the end of the line, and the human pushes (or asks another tool to push).

---

## Working alongside other agents

This repo is often edited by several agents and the user concurrently. TDD cycles run in the same working tree as everyone else.

- **Never stop or kill a running process.** Do not `taskkill`, `kill`, or terminate any `.exe` (dev servers, `esbuild.exe`, `node.exe`, Tauri, Vite, etc.) — another agent or the user may still need it. If a script fails because a process is busy, leave it running and note the conflict instead.
- **Inspect existing listeners before choosing a port.** If you need a dev server or test runner, check what is already running first and reuse it.
- **Own your files and hunks.** Stage and commit only what you changed — avoid broad `git add -A`; never discard or overwrite another agent's uncommitted work.
- **Assume the tree moves under you.** Re-check `git status` and the current branch before any consequential git operation.

---

## The fast TDD loop

The workspace ships a dedicated TDD profile and scripts so Red→Green→Refactor iterations are seconds long.

### `scripts/test-tdd.sh` — the core loop

```bash
bash scripts/test-tdd.sh -p crates/oz-core   # compile + test one crate via nextest
bash scripts/test-tdd.sh                 # auto-detect the crate from cwd
bash scripts/test-tdd.sh --watch         # re-run on every .rs change (recommended)
bash scripts/test-tdd.sh --vanilla       # fall back to cargo test (no nextest)
```

`-p` takes the **crate directory path** (e.g. `crates/oz-core`, `platform/sync`) — not the package name — because the script resolves `--manifest-path <dir>/Cargo.toml` from the workspace root.

It sets `CARGO_PROFILE=tdd`, which uses the `[profile.tdd]` section in the workspace `Cargo.toml` (inherits `dev`, `debug = false`, `incremental = true`) — the fastest possible edit-compile-test cycle. Recommended workflow:

```bash
cd crates/oz-core
bash scripts/test-tdd.sh --watch
```

If the fast loop is broken (script missing, nextest not installed, profile absent, etc.):
- Fall back to cargo test -p <package> or cargo nextest run -p <package>.
- For UI: cd ui && npm run test -- <file> or npm run test:watch.
- Note the breakage in the Journal so it can be fixed later.

### Support scripts

| Script | What it does |
|---|---|
| `scripts/test-tdd.sh` | Fast loop for one crate (`--watch`, `-p <crate-dir>`, `--vanilla`) |
| `scripts/test-changed.sh` | Runs tests only for crates whose files changed vs `origin/main` (`--all`, `--check`, `--vanilla`) |
| `scripts/test-ui-changed.sh` | `vitest --changed` — only UI tests affected by changed files (`--all`, `--check`) |
| `scripts/check.sh` | Full local pre-push gate mirroring CI |

### UI loop

```bash
cd ui
npm run test                 # full vitest suite
npm run test -- <file>       # single test file (vitest filter)
npm run test:watch           # vitest watch mode — the UI TDD loop
npm run typecheck            # strict TS
npm run check:all            # lint → typecheck → test → i18n → E2E (Docker-aware)
```

---

## Per-layer testing conventions

| Layer | Test location | Conventions |
|---|---|---|
| Rust crate (`oz-*`, `platform/*`, `modules/*`) | `#[cfg(test)] mod tests` at the bottom of each module | Every new module needs ≥ 1 unit test (AGENTS.md). Tests may use `unwrap()`/`expect()` freely. DB tests use transactions and assert atomicity (rollback on error). |
| HAL driver | `crates/oz-hal/src/drivers/mock.rs` | Every driver needs a **mandatory mock** for testing (CI fails without it). |
| Tauri command | unit tests in the command module + IPC contract tests in `ui/src/__tests__/api-*-contract.test.ts` | `invoke` calls go through `ui/src/api/`; contract tests pin the wire shape. |
| React component/hook | `ui/src/__tests__/` | One test file per component/hook. Use `@fluent/react` `Localized` ids that exist in both `en.ftl` and `id.ftl` bundles (bundle-parity gate fails otherwise). |
| Money logic | anywhere in `oz-core`/`foundation` | Assert on `minor_units: i64`, never `f32`/`f64`. Test `checked_add`/`from_major` overflow and currency-mismatch paths. |

---

## Common pitfalls

1. **Skipping Red.** Writing the fix first and the test after — the test then validates the fix instead of pinning the bug. Write the test, watch it fail, then fix.
2. **Testing implementation details.** Asserting on private helpers or call counts makes refactoring impossible. Assert on observable behavior.
3. **Green without refactor.** Passing test + messy code = the next slice gets harder. Refactor while the test is green.
4. **Full-workspace tests during iteration.** `cargo test --workspace` / `check.sh` are minutes. Use `test-tdd.sh`/`test-changed.sh` during the loop and reserve `check.sh` for pre-push.
5. **Committing without Verify.** "It passes my one test" is not enough. At minimum run the changed crate's tests + `fmt` + `clippy` before committing; run `check.sh` before pushing.
6. **Forgetting the regression test in a bugfix PR.** Without it, the bug WILL come back. Phase 3's Red test is the regression test — keep it.
7. **Journaling "what" instead of "why".** The diff already shows what. Record the decision, the tradeoff, and what you deliberately did NOT do.
8. **Killing processes or grabbing ports another agent needs.** This tree runs concurrently — never terminate a running `.exe` or take over a busy port. Inspect existing listeners first and reuse what's running.
9. **Pushing.** `git push` is never part of this skill's workflow — not on user request, not when "everything is green". Commit locally, report, stop. The human pushes.

---

## See also

- **[`rust-backend`](../rust-backend/SKILL.md)** — Rust & DB standards the Green phase must follow (Money, transactions, errors).
- **[`ui-components`](../ui-components/SKILL.md)** — React/TS conventions, `<Localized>`, ARIA, strict TS.
- **[`tauri-ipc`](../tauri-ipc/SKILL.md)** — command registration + `ui/src/api/` wrappers.
- **[`hal-drivers`](../hal-drivers/SKILL.md)** — driver traits and the mandatory mock.
- **[`project-scaffold`](../project-scaffold/SKILL.md)** — workspace layout, CI matrix, Conventional Commits, spec workflow.
- **[`skill-drift-guard`](../skill-drift-guard/SKILL.md)** — run in Phase 6 after any change that touches a path/type/trait a skill describes.

---

> last audited 07-08-26 by buffy
