# Quickstart

This guide gets OZ-POS building and running on your machine in under 15 minutes. It's aimed at first-time contributors — for the deeper project conventions, see [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`AGENTS.md`](../AGENTS.md), and the skills under `.agents/skills/`.

---

## Prerequisites

| Tool | Version | Why |
|------|---------|-----|
| **Rust** | 1.88+ stable (`rustup install stable`) | The workspace uses edition 2024 and axum/tower-http deps that require rustc ≥ 1.88 |
| **Node.js** | 18 or 20 LTS | Tauri v2 webview (React 18 + TypeScript) |
| **Tauri v2 prerequisites** | Per [Tauri docs](https://tauri.app/v2/guides/) | WebView2 on Windows, webkit2gtk on Linux, etc. |
| **SQLite** | 3.x (bundled via `rusqlite`) | Local persistence — no separate install needed |
| **Git** | any recent | Source control |

**Tauri v2 platform-specific deps:**

- **Windows 10/11**: Install [WebView2 runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (preinstalled on Windows 11). Install the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload.
- **Linux (Ubuntu/Debian)**: `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`.
- **Linux (Fedora)**: `sudo dnf install webkit2gtk4.1-devel gcc gcc-c++ make curl wget file openssl-devel libappindicator-gtk3-devel librsvg2-devel`.
- **macOS**: Xcode Command Line Tools (`xcode-select --install`).

---

## Clone and build

```bash
# 1. Clone the repository
git clone https://github.com/kardelitaitu/oz-pos.git
cd oz-pos

# 2. Build the Rust workspace
cargo build --workspace

# 3. Install front-end dependencies
cd ui && npm install
cd ..

# 4. Run the Tauri app in development mode
cargo tauri dev
# or, from the ui/ folder:
cd ui && npm run tauri dev
```

The first build will take several minutes (Rust crates + Tauri binaries). Subsequent builds are fast.

---

## Run the tests

```bash
# All library tests (no browser required)
cargo test --workspace --all-features

# UI tests
cd ui && npm run test
```

The Rust test suite is fully offline — no browser, no network, no hardware. Mocks live in `crates/oz-hal/src/drivers/mock.rs` and are gated by the `mock` feature.

---

## Lint and format

```bash
# Format
cargo fmt --all

# Lint (must pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# UI lint
cd ui && npm run lint
cd ui && npm run typecheck
```

`AGENTS.md` makes `cargo fmt --check` and `cargo clippy -- -D warnings` mandatory. CI rejects PRs that fail either.

---

## Project structure (at a glance)

```
oz-pos/
├── Cargo.toml                  # workspace root
├── crates/                     # Rust workspace members (one per oz-* responsibility)
│   ├── oz-core/                # money, currency, cart, sale, inventory
│   ├── oz-hal/                 # hardware abstraction + drivers
│   ├── oz-lua/                 # rlua runtime + script bindings
│   ├── oz-security/            # encryption, secrets, PCI helpers
│   ├── oz-payment/             # Stripe, Square, EMV abstraction
│   ├── oz-reporting/           # analytics + CSV export
│   ├── oz-logging/             # structured logging
│   └── oz-cli/                 # migrations, backup, export CLI
├── apps/desktop-client/        # the desktop Tauri shell
│   └── src/commands/           # Tauri commands (one folder per feature)
├── ui/                         # React + TypeScript front-end
│   └── src/api/                # per-domain invoke() wrappers
├── crates/oz-core/migrations/  # SQL migration files
├── docs/                       # project documentation
├── .agents/skills/             # agent skills (read these when contributing)
└── .github/workflows/          # CI pipelines
```

For the full layout, see [`ARCHITECTURE.md`](./ARCHITECTURE.md).

---

## Skills you'll read most

The first time you work on a layer, read the matching skill under `.agents/skills/`. The onboarding guide will route you:

| If you're touching… | Read this skill |
|---|---|
| Rust in any `oz-*` crate, `Money`, SQL, error types | `rust-backend` |
| A Tauri command, the `pos.ts` wrapper, events | `tauri-ipc` |
| A React component, Fluent strings, accessibility | `ui-components` |
| A device driver, the mock, the registry | `hal-drivers` |
| The workspace, CI, branches, commit messages | `project-scaffold` |
| Drift between skills and code | `skill-drift-guard` |

After making your change, run the drift guard:

```bash
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
```

---

## Your first commit

A safe first change is one of:

1. **Fix a typo in the docs** (branch: `docs/<short-name>`, commit: `docs: ...`).
2. **Add a unit test for a public function** (branch: `test/<short-name>`, commit: `test: ...`).
3. **Address a `clippy` warning** in an existing file (branch: `chore/<short-name>`, commit: `chore: ...`).

For any of these, the full pre-PR checklist is:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
```

All green? Open the PR.

---

## Troubleshooting

### "error: package `oz-core v0.0.1` cannot be built because it requires rustc 1.88 or newer"

You're on an old Rust. Update:

```bash
rustup update stable
rustup show
```

### `cargo tauri dev` fails on Linux with "webkit2gtk not found"

Install the Tauri Linux prerequisites (see the table at the top of this file). The exact package name depends on your distro.

### Tests pass locally but fail in CI

The CI matrix runs on Linux, Windows, and macOS. If you see a failure on a platform you didn't test locally, install the platform's deps and re-run. Don't disable platform-specific tests — fix them.

### `npm install` fails in `ui/`

Make sure you're on Node 18 or 20 LTS (`node --version`). Older versions miss the `fetch` types and `--no-warnings` flag that the build expects.

### "permission denied" running `scripts/check.sh`

```bash
chmod +x scripts/check.sh
```

### Drift guard reports findings on day 1

Expected. The drift guard no-ops the checks that need code (checks 2–4, 7) in the pre-code state. The remaining checks (paths, golden rules, cross-references, audit dates) catch the most common first-day issues — a broken link in `README.md`, a stale skill, an uncommitted `.env` reference.

---

## Where to go next

- [`AGENTS.md`](../AGENTS.md) — the project's coding standards
- [`ARCHITECTURE.md`](./ARCHITECTURE.md) — the deep layout
- [`ROADMAP.md`](./ROADMAP.md) — what's being built and in what order
- [`WHITEPAPER.md`](./WHITEPAPER.md) — the "why" behind the tech choices
- [`.agents/skills/onboarding-guide`](../.agents/skills/onboarding-guide/SKILL.md) — pick the right skill for the layer you're touching

Welcome to OZ-POS. Keep the curtain closed, the merchant happy, and the money integer.

---

> last audited 2026-07-07 by docs-auditor
