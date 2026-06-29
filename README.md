# OZ-POS

A modular Point-of-Sale framework built with Rust + Tauri v2.

## Architecture

```
oz-pos/
├── apps/
│   ├── desktop-client/     # Tauri v2 shell: IPC commands, app state, plugins
│   │   └── src/commands/   # 65+ Tauri commands grouped by domain
│   └── tablet-client/      # Tablet-optimised Tauri shell
├── crates/
│   ├── oz-cli/             # CLI tool (backup, export/import .ozpkg, migrations)
│   ├── oz-core/            # Domain models, SQLite Store, migrations, settings
│   │   ├── src/db/         # Store facade — typed CRUD per entity
│   │   └── src/ozpkg/      # Encrypted .ozpkg export/import (Argon2id + AES-256-GCM)
│   ├── oz-hal/             # Hardware Abstraction Layer (printer, scanner, drawer, display)
│   ├── oz-logging/         # Structured logging (console, file, syslog, eventlog)
│   ├── oz-lua/             # Lua scripting engine (rlua — discount, tax, validation)
│   ├── oz-payment/         # Payment gateway integrations (Stripe, mock)
│   ├── oz-reporting/       # Report generation (EOD, sales summaries)
│   └── oz-security/        # TLS config, PAN masking, platform keychains
├── foundation/             # Shared primitives: Money, SKU, Cart, contracts
├── modules/                # Pluggable domain modules (CRM, inventory, tax, etc.)
├── platform/               # Kernel, event bus, sync engine, startup
├── ui/                     # React 18 + TypeScript + Vite
│   └── src/
│       ├── api/            # Per-domain invoke() wrappers — no invoke() in components
│       ├── frontend/
│       │   ├── shared/     # Button, Card, Modal, Badge, Toast, Spinner, etc.
│       │   ├── shell/      # AppLayout, RoleBadge, ThemeProvider, ThemeToggle
│       │   └── themes/     # Design tokens, reset, components CSS
│       ├── features/       # 15+ screen components by domain
│       ├── locales/        # Fluent (.ftl) files — 280+ IDs
│       └── __tests__/      # Vitest + testing-library (122 tests, 12 files)
├── docs/
│   ├── ROADMAP.md          # Phased delivery plan (Phase 3 complete)
│   ├── decisions/          # Architecture decision records
│   └── specs/              # Module manifest format, PCI-DSS checklist
├── scripts/examples/       # Example Lua business rule scripts
└── packaging/              # MSI, .deb, .AppImage build configs
```

## Foundation

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Backend | Rust | Domain logic, DB access, hardware control |
| Frontend | React 18 + TS + Vite | POS UI |
| Shell | Tauri v2 | IPC bridge, native window, updater |
| DB | SQLite (rusqlite) | On-device persistence, 25 migrations |
| i18n | @fluent/react | All UI strings in `.ftl` files |
| Hardware HAL | oz-hal traits | USB/TCP/BT/serial/mock drivers for printer, scanner, drawer, display |
| Money | i64 minor units | Never f64 — `Currency`, `Money` structs |
| Export | .ozpkg format | Argon2id + AES-256-GCM + zstd encrypted snapshots |

## Quick Start

```bash
git clone https://github.com/kardelitaitu/oz-pos.git
cd oz-pos
cargo build --workspace
cd ui && npm install && cd ..
cd apps/desktop-client && cargo tauri dev
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
| `cargo test --workspace` | Rust tests (400+) |

## Backend Conventions

- **Money**: `i64 minor_units` + `Currency` — never `f32`/`f64`
- **DB writes**: always inside `rusqlite` transactions
- **Errors**: `thiserror` for libs, `anyhow` for app code
- **Clippy**: must pass `-- -D warnings` before merge
- **Migrations**: `.sql` files in `crates/oz-core/migrations/` registered in `migrations.rs`

## Frontend Conventions

- **No `invoke()` in components** — use per-domain `api/*.ts` wrappers
- **No hardcoded strings** — all text goes through `@fluent/react`
- **Accessibility**: every interactive element has an `aria-label`
- **Money display**: `formatMoney()` from `ui/src/locales/test-utils.tsx`
- **Tests**: every feature screen has a corresponding `__tests__/` file

## Status

**Phase 3 (Transactions & Staff) complete.** 25 migrations, 65+ IPC commands, 15+ screen components, 122 front-end tests, 400+ Rust tests.

See [ROADMAP.md](./docs/ROADMAP.md) for the phased delivery plan.

> last audited 30-06-26 by docs-auditor
