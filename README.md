# OZ-POS

A modular Point-of-Sale framework built with Rust + Tauri v2.

## Architecture

```
oz-pos/
├── crates/
│   ├── oz-api/           # REST API server (axum)
│   ├── oz-cli/           # CLI tool for offline ops
│   ├── oz-core/          # Domain models, SQLite Store, migrations, settings
│   ├── oz-hal/           # Hardware Abstraction Layer (printers, scanners, cash drawer)
│   ├── oz-logging/       # Structured logging (console, file, syslog)
│   ├── oz-lua/           # Lua scripting engine (rlua)
│   ├── oz-payment/       # Payment gateway integrations
│   ├── oz-reporting/     # Report generation (EOD, sales summaries)
│   └── oz-security/      # TLS config, masking, platform keychains
├── src-tauri/            # Tauri v2 shell: IPC commands, app state, plugins
│   └── src/commands/     # 62 Tauri commands grouped by domain
├── ui/                   # React 18 + TypeScript + Vite
│   └── src/
│       ├── api/pos.ts    # Single invoke() bridge — no invoke() in components
│       ├── components/   # Shared UI (Card, Button, Badge, Toast, etc.)
│       ├── features/     # 13 screen components by domain
│       ├── locales/      # Fluent (.ftl) files — 256 IDs
│       └── __tests__/    # Vitest + testing-library (122 tests, 12 files)
└── docs/
```

## Foundation

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Backend | Rust | Domain logic, DB access, hardware control |
| Frontend | React 18 + TS + Vite | POS UI |
| Shell | Tauri v2 | IPC bridge, native window, updater |
| DB | SQLite (rusqlite) | On-device persistence, 13 migrations |
| i18n | @fluent/react | All UI strings in `.ftl` files |
| Hardware HAL | oz-hal traits | USB/TCP/BT/mock drivers for printer, scanner, cash drawer |
| Money | i64 minor units | Never f64 — `Currency`, `Money` structs |

## Quick Start

```bash
git clone https://github.com/kardelitaitu/oz-pos.git
cd oz-pos
cargo build --workspace
cd ui && npm install && cd ..
cargo tauri dev
```

### Key scripts (ui/)

| Command | Action |
|---------|--------|
| `npm run typecheck` | `tsc --noEmit` |
| `npm run test` | `vitest run` (122 tests, 12 files) |
| `npm run lint` | ESLint + jsx-a11y |

### Key scripts (root)

| Command | Action |
|---------|--------|
| `cargo clippy --all-targets` | Rust lint |
| `cargo test --workspace` | Rust tests (380+) |

## Backend Conventions

- **Money**: `i64 minor_units` + `Currency` — never `f32`/`f64`
- **DB writes**: always inside `rusqlite` transactions
- **Errors**: `thiserror` for libs, `anyhow` for app code
- **Clippy**: must pass `-- -D warnings` before merge
- **Migrations**: `.sql` files in `crates/oz-core/migrations/` registered in `migrations.rs`

## Frontend Conventions

- **No `invoke()` in components** — use `api/pos.ts` wrappers
- **No hardcoded strings** — all text goes through `@fluent/react`
- **Accessibility**: every interactive element has an `aria-label`
- **Money display**: `formatMoney()` from `types/domain.ts`
- **Tests**: every feature screen has a corresponding `__tests__/` file

## Status

Phase 1 MVP complete. 13 migrations, 62 IPC commands, 13 screen components, 122 front-end tests, 380+ Rust tests.

See [TODO.md](./TODO.md) for outstanding items.

> last audited 28-06-26 by docs-auditor
