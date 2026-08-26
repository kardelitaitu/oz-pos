![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/kardelitaitu/oz-pos?style=flat-square) ![GitHub repo size](https://img.shields.io/github/repo-size/kardelitaitu/oz-pos?style=flat-square) [![Nightly CI](https://github.com/kardelitaitu/oz-pos/actions/workflows/nightly.yml/badge.svg)](https://github.com/kardelitaitu/oz-pos/actions/workflows/nightly.yml)


<!-- Audit stamp: 2026-07-25 · Hermes-Agent · status: ACCURATE · ADR #30 domain modularization complete across modules/* · ADR #31 decentralized UI self-registration active via registerAllFeatures() · F1: 101 .sql migration files · F2: 5,212 Rust tests; 228 UI test files (3,476 tests) · F3: 48 .ftl Fluent translation files (5,700+ IDs) · F4: payment drivers (Stripe, Mock, QRIS, Square) · F5: 618 #[tauri::command] IPC endpoints · re-audited 2026-08-08 by docs-auditor: 117 .sql migrations, 5,860 Rust test markers, 265 UI test files, 48 .ftl (7,152 IDs), 435 #[tauri::command] endpoints, release v0.0.25 -->

# OZ-POS

> **A modular, offline-first Point-of-Sale platform built with Rust and Tauri v2.**

OZ-POS is a Point-of-Sale platform designed for **retail, restaurants, cafés, and specialty businesses** that require reliability, performance, and long-term maintainability.

Unlike traditional monolithic POS applications, OZ-POS is built around a modular architecture where business capabilities are implemented as independent modules. Organizations can deploy only the features they need while developers can extend the platform without modifying the core.

---

## Why OZ-POS?

Modern POS systems often suffer from vendor lock-in, expensive subscriptions, cloud dependency, limited customization, and difficult maintenance. OZ-POS addresses these challenges through a modern software architecture.

| Traditional POS | OZ-POS |
|---|---|
| Monolithic | Modular architecture |
| Cloud required | Offline-first |
| Proprietary integrations | Hardware abstraction layer |
| Difficult customization | Plug-in modules |
| Large desktop footprint | Lightweight Tauri application |
| Vendor lock-in | Open ecosystem |

### Core Principles

- **Offline-first** — Operates without internet connectivity; sync when available
- **Modular by design** — Independent modules for inventory, CRM, reporting, etc.
- **Secure by default** — Encrypted backups, PAN masking, platform keychains
- **Hardware abstraction** — Vendor-independent drivers for printers, scanners, displays
- **Enterprise-grade code quality** — 5,200+ Rust tests, 3,476 frontend tests (228 files), strict Clippy, typed Money, transactional DB

---

## Key Features

| Area | Capabilities |
|------|-------------|
| **Sales** | Fast checkout, barcode scanning, receipt printing, multiple payments, refunds, discounts |
| **Inventory** | Product management, categories, stock adjustments, purchase tracking, movement history |
| **Customer Management** | Profiles, purchase history, loyalty support, future CRM module |
| **Reporting** | Daily sales, product performance, cash reconciliation, inventory reports, export |
| **Security** | Encrypted backups (Argon2id + AES-256-GCM), PAN masking, TLS, platform keychain, audit logging |
| **Hardware** | Receipt printers, barcode scanners, cash drawers, customer displays — USB, Bluetooth, TCP, serial, plus mock drivers for testing |

---

## Architecture

```
                         Applications
      ┌─────────────────────────────────────────────┐
      │  Desktop Client  │  Tablet Client  │ Future │
      └─────────────────────────────────────────────┘
                        │
                        ▼
                   Tauri v2 Shell
                        │
                        ▼
                   Platform Kernel
      ┌────────────────────────────────────────┐
      │ Event Bus │ Sync Engine │ Lifecycle    │
      │ Auth      │ Startup                    │
      └────────────────────────────────────────┘
             │                    │
      ┌──────┴────────────────────┴──────────────┐
      ▼                                         ▼
 Foundation                              Domain Modules
 ┌──────────────┐                    ┌─────────────────┐
 │ Money  SKU   │                    │ Inventory       │
 │ Cart         │                    │ Reporting       │
 │ Contracts    │                    │ CRM             │
 └──────────────┘                    │ Tax / Discounts │
                                     │ Restaurant      │
      │                              │ Loyalty         │
      ▼                              └─────────────────┘
 Infrastructure
 ┌──────────────────────────────────────────────┐
 │ SQLite   │  HAL  │  Security  │  Logging     │
 │ Export   │  Lua Runtime                      │
 └──────────────────────────────────────────────┘
```

Business logic, UI, hardware drivers, and platform services are isolated — new modules and applications can be added without changing the kernel.

---

## Repository Structure

```
oz-pos/
├── apps/
│   ├── desktop-client/     # Tauri v2 shell: IPC commands, app state, plugins
│   ├── tablet-client/      # Tablet-optimised Tauri shell
│   ├── cloud-server/       # Cloud HTTP API (axum, hosted tenants)
│   └── license-server/     # License activation & validation
├── crates/
│   ├── oz-api/             # HTTP API server (axum)
│   ├── oz-cli/             # CLI tool (backup, export/import .ozpkg, migrations)
│   ├── oz-core/            # Domain models, SQLite Store, migrations, settings
│   ├── oz-hal/             # Hardware Abstraction Layer (printer, scanner, drawer, display)
│   ├── oz-logging/         # Structured logging (console, file, syslog, eventlog)
│   ├── oz-lua/             # Lua scripting engine (mlua — discount, tax, validation)
│   ├── oz-notification/    # Email & push notification dispatching
│   ├── oz-payment/         # Payment gateway integrations (Stripe, Square, QRIS, mock)
│   ├── oz-plugin/          # Plugin sandbox & lifecycle (Lua scripting bridge)
│   ├── oz-reporting/       # Report generation (EOD, sales summaries)
│   └── oz-security/        # TLS config, PAN masking, platform keychains
├── foundation/             # Shared primitives: Money, SKU, Cart, contracts
├── modules/                # Pluggable domain modules (CRM, inventory, tax, etc.)
├── platform/               # Kernel, event bus, sync engine, startup
├── ui/                     # React 18 + TypeScript + Vite
│   └── src/
│       ├── api/            # Per-domain invoke() wrappers — no invoke() in components
│       ├── frontend/       # Shared components, shell layout, design tokens
│       ├── features/       # 55+ audited screen components by domain
│       ├── locales/        # Fluent (.ftl) files — 5,700+ IDs across 48 files
│       └── __tests__/      # Vitest + testing-library (228 files, 3,476 tests)
├── docs/                   # ROADMAP.md, ADRs, specs, whitepaper
├── scripts/                # Example Lua business rule scripts, coverage scripts
└── packaging/              # MSI, .deb, .AppImage build configs
```

---

## Technology Stack

| Layer | Technology | Purpose |
|---|---|---|
| Backend | Rust | Domain logic, DB access, hardware control |
| Desktop Shell | Tauri v2 | Native window, IPC bridge, updater |
| Frontend | React 18 + TypeScript + Vite 6 | POS UI |
| Database | SQLite (rusqlite) | On-device persistence, 117 migrations |
| Localization | @fluent/react | All UI strings in `.ftl` files |
| Hardware | oz-hal traits | USB/TCP/BT/serial/mock drivers |
| Money | `i64` minor units | Never `f32`/`f64` — `Currency`, `Money` structs |
| Security | Argon2id + AES-256-GCM + zstd | Encrypted `.ozpkg` snapshots |
| Automation | Lua (mlua) | Discount, tax, validation rules |

---

## Quick Start

```bash
git clone https://github.com/kardelitaitu/oz-pos.git
cd oz-pos
cargo build --workspace
cd ui && npm ci --no-audit --no-fund && cd ..  # see ui/README.md#install-script-approvals
cd apps/desktop-client && cargo tauri dev
```

See [docs/QUICKSTART.md](./docs/QUICKSTART.md) for detailed setup instructions.

---

## Development Commands

### Frontend (ui/)

| Command | Action |
|---|---|
| `npm run dev` | Development server |
| `npm run check:all` | Chained validation: lint → typecheck → test → i18n → E2E* |
| `npm run build` | Production build |
| `npm run typecheck` | TypeScript validation |
| `npm run lint` | ESLint + jsx-a11y |
| `npm run test` | Vitest (228 files, 3,476 tests) |
| `npm run e2e` | Full E2E suite: Docker → Vite → Playwright → cleanup |
| `npm run e2e:headed` | E2E with browser visible |
| `npm run e2e:api` | API integration tests only |
| `npm run e2e:ui` | All UI E2E tests (excl. API) |

> * E2E requires Docker; check:all skips it gracefully if unavailable. See [`ui/README.md`](./ui/README.md) and [`ui/e2e/README.md`](./ui/e2e/README.md) for details.

### Backend (root)

| Command | Action |
|---|---|
| `cargo fmt --all` | Format Rust code |
| `cargo clippy --all-targets -- -D warnings` | Lint |
| `cargo test --workspace` | Run tests (5,200+) |
| `bash scripts/check.sh` | Full local pre-push gate (Rust + UI + migrations) |
| `bash scripts/coverage.sh` | Rust + UI coverage reports |
| `bash scripts/reset-dev-pg.sh` | Reset the dev PostgreSQL container to the committed PG_INIT schema (`.ps1` twin on Windows) |

---

## Testing Strategy

| Layer | Approach |
|---|---|
| **Rust** | Unit tests, integration tests, DB migration tests, HAL mock tests |
| **Frontend** | Component tests, feature tests, localization validation, accessibility checks |
| **Coverage** | LLVM source-based (Rust) + v8 (UI) — HTML + JSON in `coverage/` |

Every PR must pass `cargo fmt`, Clippy, `tsc --noEmit`, and all tests before merge.

---

## Status

**Phase 4 (CRM, Restaurant, Accounting) in progress.** 117 migrations, 435+ IPC commands, 55+ audited screen components, 265 front-end test files, 5,800+ Rust tests.

| Phase | Status | Focus |
|---|---|---|
| 1 | Complete | Platform foundation |
| 2 | Complete | Inventory & Products |
| 3 | Complete | Transactions & Staff |
| 4 | In Progress | CRM, Restaurant, Accounting |
| 5 | In Progress | Multi-store topology, Cloud Sync, Plugin system |

Latest release: **v0.0.29** (on branch `0.0.29`).

See [ROADMAP.md](./docs/ROADMAP.md) for the full phased delivery plan, and [MODULAR_APP_PLAN.md](./docs/MODULAR_APP_PLAN.md) for detailed granular checklists covering feature presets, restaurant workflows, LAN KDS discovery, and Docker cloud server containerization (`apps/cloud-server`).

---

## Contributing

Contributions of all sizes are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for:

- Branch naming and commit conventions
- PR checklist and review guidelines
- Coding standards (Money, DB, errors, etc.)
- Adding new skills and modules
- Security issue reporting

New contributors are encouraged to start with documentation improvements, UI polish, accessibility enhancements, additional tests, or bug fixes labelled **Good First Issue**.

---

## License & Commercial Use

**Proprietary and Confidential — Copyright (c) 2024-2026 OZ-POS Contributors / All Rights Reserved.**

This software (`oz-pos`) is **NOT open source**. No part of this codebase, associated binaries, or documentation may be copied, modified, distributed, sublicensed, hosted, or deployed in any commercial, non-commercial, or production setting without explicit written permission and a valid executed Commercial License Agreement.

See [LICENSE](./LICENSE) for terms and restrictions. For commercial licensing and pricing inquiries, contact: **adikaradwiatmaja@gmail.com**.

> last audited 09-08-26 by buffy
> audit: Phase 1 Core Architecture & API Docs Audit

> status: ACCURATE (verified against actual codebase) · verified accurate: all test counts, modules, crates, security features, and feature capabilities confirmed; version 0.0.25 matches Cargo.toml

