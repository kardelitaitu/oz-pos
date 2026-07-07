---
name: onboarding-guide
description: Meta-skill that routes tasks to the right OZ-POS skill. Use when starting a new task and unsure which specialized skill applies. Read this first when joining the project or picking up an unfamiliar area.
---

# OZ-POS Onboarding Guide

OZ-POS is a Rust + Tauri v2 POS framework. The codebase is organized into clear layers, and each layer has a dedicated skill. This guide routes you to the right skill for the work you want to do.

> *"Pay no attention to the man behind the curtain."* — The Wizard of Oz
>
> The OZ-POS philosophy: keep the **merchant's** experience effortless by hiding complexity behind a lean Rust engine. Skills are how we keep the engine clean.

---

## 30-second tour

- **Domain types and money**: `oz-core` (Rust library, no I/O).
- **Database**: SQLite via `rusqlite`, all writes in transactions.
- **Hardware**: `oz-hal` (drivers behind `async` traits, mandatory mocks).
- **UI**: Tauri v2 + React 18 + TypeScript, strict, accessible, localized.
- **IPC**: Rust commands in `apps/desktop-client/src/commands/`, front-end wrappers in `ui/src/api/` (per-domain files).
- **Scripting**: `rlua` runtime in `oz-lua` for runtime business rules.
- **Payment**: PCI-aware, swappable processors in `oz-payment`.
- **CI**: GitHub Actions matrix (Linux, Windows, macOS), blocking fmt/clippy/test/UI lint.

If a change touches more than one layer, you will use more than one skill. That's normal.

---

## Skill router

What do you want to do?

| If you want to… | Read this skill first |
|---|---|
| Add or change Rust code in any `oz-*` crate, work with the `Money` struct, write SQL transactions, define error types, or add a `#[cfg(test)]` block | **`rust-backend`** |
| Add a new Tauri command on the backend, register it, and call it from the front-end via `pos.ts` | **`tauri-ipc`** |
| Add or change a React component, screen, hook, or any user-visible string; review accessibility, i18n, or strict TypeScript | **`ui-components`** |
| Add a new device category or vendor driver (barcode, printer, NFC, payment terminal, cash drawer); write the **mandatory mock** | **`hal-drivers`** |
| Scaffold the workspace, add a new crate, configure CI, write commit messages, set up the GitHub Actions matrix | **`project-scaffold`** |
| Detect or patch drift between a skill and the code (broken paths, renamed crates, stale `last audited` dates, outdated dependency versions) | **`skill-drift-guard`** |

If your task touches more than one layer, read each relevant skill in the order shown above (rust-backend → tauri-ipc → ui-components). The skills are designed to be cross-referenced. After making your change, run `skill-drift-guard` to verify the skills still match the code.

**If a code change is touching something a skill describes** (e.g., renaming a public type, moving a file, bumping a dependency), also read `skill-drift-guard` and run its `scripts/detect.sh` before opening the PR.

---

## Skills to defer (no code yet)

These areas are real but the project is pre-code. **Do not create skills for them yet** — wait until the code exists and the boundaries are stable.

- **`oz-lua` scripting** — defer until `oz-core` exposes the functions the Lua runtime will bind to.
- **`oz-payment` processors** (Stripe, Square, EMV) — defer until the `oz-hal` `PaymentTerminal` trait is in place.
- **`oz-security` (encryption, PCI-DSS)** — defer until the first secret is actually being stored.
- **Cloud sync (PostgreSQL/CockroachDB, Redis)** — defer until local sync is implemented and a real outbox exists.
- **Reporting / analytics (oz-reporting)** — defer until the SQLite schema stabilizes. Reports aggregate over the cart, sale, payment, and inventory tables; writing the skill before those tables exist will require frequent rewrites.

When a skill becomes relevant, this guide should be updated to point to it.

---

## Common workflows

### "I'm adding a new feature end-to-end"

1. Read `rust-backend`. Add the domain type and the `Money` flow.
2. Read `tauri-ipc`. Add the command and the `pos.ts` wrapper.
3. Read `ui-components`. Add the screen, hook, and Fluent strings.
4. Read `hal-drivers` only if the feature needs hardware.
5. Read `project-scaffold` to confirm the branch name and commit format.

### "I'm adding a new device"

1. Read `hal-drivers`. Define the trait, implement the driver.
2. Add the **mandatory mock** in `hal/src/drivers/mock.rs` — CI fails without it.
3. If the device has a user-facing setup screen, read `ui-components` for the screen.
4. If the device is invoked from a Tauri command, read `tauri-ipc` for the wiring.

### "I'm fixing a bug"

1. Reproduce with a failing test in the appropriate crate.
2. Read the skill for the layer where the bug lives.
3. Make the fix. Add a regression test. Run the full local check script.
4. Commit with `fix(<scope>): <summary>`.

### "I'm setting up CI for the first time"

1. Read `project-scaffold`. Copy the workflow files.
2. Confirm `cargo fmt`, `clippy -- -D warnings`, and `cargo test --workspace` run green on Linux first.
3. Add Windows and macOS to the matrix.
4. Add the UI job (`npm run lint && typecheck && test && build`).
5. Add a security job (`cargo audit`, `cargo deny`).

---

## When NOT to use these skills

These skills are scoped to the OZ-POS codebase. They do **not** apply if you are working on:

- A different project (the skills reference `oz-core`, `oz-hal`, etc. by name).
- A feature that has nothing to do with a POS (a CLI for a totally different domain, a web app, a game).
- An LLM-driven workflow (OZ-POS does not use LLMs at the framework level).
- A browser-automation workflow (OZ-POS does not drive browsers; it is a desktop app).
- A social-media automation workflow (OZ-POS is not a Twitter/X bot; it runs cash registers).

If any of those describe the task, the right move is to ask the user which codebase they meant, or to spawn a skill-discovery workflow rather than applying these skills.

---

## Where to ask for help

| Question | Where to ask |
|---|---|
| "What does this Rust trait do?" | Read the `///` docs on the trait itself. The skills are guides, not the source of truth — the code is. |
| "How should this work in OZ-POS?" | Read the matching skill. If the skill doesn't cover it, ask Buffy (the AI agent) to extend the skill. |
| "How should this work in general?" | The relevant upstream docs (`embedded-hal`, `rusqlite`, `tauri`, React, Fluent). The skills assume familiarity with these. |
| "Is this a security concern?" | Read `AGENTS.md` first. If still unclear, spawn a security review — OZ-POS handles money and (eventually) card data. |

---

## Keeping the skills fresh

When you discover a pattern that the skills don't cover — a new crate convention, a new CI check, a new accessibility rule — update the relevant skill. The skills are living documents. Add a one-line note at the bottom of the file with `> last audited <DD-MM-YY> by <who>`.

If a skill disagrees with the code, **the code is correct** until proven otherwise. Patch the skill to match, then file an issue if the skill was right and the code is wrong.

---

## Pre-commit checklist (one-liner)

```bash
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --workspace --all-features && \
(cd ui && npm run lint && npm run typecheck && npm run test && npm run build)
```

If this passes locally, the PR is ready.

---

> last audited 28-06-26 by docs-auditor
