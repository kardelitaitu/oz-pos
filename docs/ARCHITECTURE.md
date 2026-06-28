# OZ-POS – Codebase Architecture

## Overview
This document describes the directory layout and module responsibilities for **OZ-POS**. The design supports:
- Rust core engine (transaction handling, persistence)
- Hardware Abstraction Layer (HAL) for barcode scanners, printers, NFC, etc.
- Embedded Lua scripting for dynamic business rules
- REST API server (axum + JWT) for third-party integrations
- Tauri v2 UI built with React/TypeScript
- Multi‑platform targets: Windows PC, Linux PC, Android tablet, iPad
- Scalable database strategy (SQLite on‑device, optional cloud sync)

---
## Directory Layout
```
oz-pos/
├─ Cargo.toml                # Workspace definition (10 members)
├─ rust-toolchain.toml       # Pinned Rust toolchain
├─ package.json              # Front‑end package manager (React/TS)
├─ crates/                   # Rust workspace crates
│   ├─ oz-core/              # Core engine: domain types, Money, Cart, Sale, migrations, DB facade
│   │   ├─ Cargo.toml
│   │   ├─ src/
│   │   │   ├─ lib.rs        # Crate root, re-exports
│   │   │   ├─ money.rs      # Money(i64, Currency) — integer-only arithmetic
│   │   │   ├─ cart.rs       # Cart / CartLine — in-memory sale pipeline
│   │   │   ├─ sale.rs       # Sale / SaleLine — state machine (Pending→Active→Completed|Voided)
│   │   │   ├─ product.rs    # Product domain type
│   │   │   ├─ category.rs   # Category type (id, name, colour)
│   │   │   ├─ inventory.rs  # Inventory domain type
│   │   │   ├─ sku.rs        # Sku, LineId types
│   │   │   ├─ db.rs         # Store<'a> — typed CRUD facade over &Connection
│   │   │   ├─ settings.rs   # Settings persistence layer
│   │   │   ├─ features.rs   # Feature enum (32 flags), registry, presets
│   │   │   ├─ migrations.rs # Embedded SQL migration runner
│   │   │   └─ error.rs      # CoreError enum
│   │   └─ migrations/       # SQL migration files (001–004)
│   ├─ oz-hal/               # Hardware Abstraction Layer
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       ├─ lib.rs        # Public API
│   │       ├─ traits/       # Device traits (BarcodeScanner, ReceiptPrinter, CashDrawer)
│   │       ├─ drivers/      # Mock + real device drivers
│   │       ├─ registry.rs   # DriverRegistry (Arc<RwLock<HashMap<…>>>)
│   │       ├─ types.rs      # Barcode, BarcodeSymbology, DeviceInfo
│   │       └─ error.rs      # HalError enum
│   ├─ oz-api/               # REST API server (axum + JWT auth)
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       ├─ lib.rs        # Router builder, AppState, server start (port 3099)
│   │       ├─ auth.rs       # JWT create/validate + auth middleware
│   │       └─ routes/       # health, tokens, products, categories endpoints
│   ├─ oz-lua/               # Lua scripting runtime (scaffold)
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # LuaError type (Phase 3: rlua embedding)
│   ├─ oz-security/          # Security crate (scaffold)
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # SecurityError type (Phase 2: key-ring, TLS, PCI-DSS)
│   ├─ oz-payment/           # Payment processor crate (scaffold)
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # PaymentError type (Phase 4: PaymentProcessor trait)
│   ├─ oz-reporting/         # Reporting crate (scaffold)
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # ReportingError type (Phase 5: CSV, aggregation)
│   ├─ oz-logging/           # Structured logging crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # oz_logging::init() + LoggingError
│   └─ oz-cli/               # CLI binary `oz`
│       ├─ Cargo.toml
│       └─ src/
│           └─ main.rs       # clap entry-point: migrate, backup, export
├─ src-tauri/                # Tauri v2 application shell
│   ├─ Cargo.toml
│   ├─ tauri.conf.json       # Window config, bundle targets, updater
│   ├─ capabilities/
│   │   └─ default.json      # Tauri v2 permissions
│   └─ src/
│       ├─ lib.rs            # run(): init logging, AppState, register commands
│       ├─ main.rs           # Entry point (Windows: windows_subsystem)
│       ├─ state.rs          # AppState: Mutex<Connection>, DriverRegistry, AppHandle
│       ├─ error.rs          # AppError (tagged JSON)
│       └─ commands/         # Tauri commands: health, sales, hardware
├─ ui/                       # Tauri front‑end (React/TS)
│   ├─ package.json
│   ├─ vite.config.ts        # Build config
│   ├─ tsconfig.json
│   └─ src/
│       ├─ main.tsx          # Entry point
│       ├─ App.tsx           # Root component
│       ├─ api/
│       │   └─ pos.ts        # THE ONLY place that calls invoke()
│       ├─ types/
│       │   └─ domain.ts     # TypeScript mirrors: CartId, LineId, Sku, Money
│       ├─ features/         # Feature-scoped screens (sales/)
│       ├─ components/       # Reusable React components
│       ├─ hooks/            # Custom React hooks
│       ├─ locales/          # Fluent localisation files (en-US.ftl)
│       ├─ styles/           # CSS design tokens and styles
│       └─ __tests__/        # Vitest + Testing Library tests
├─ scripts/                  # Build helpers, pre-push checks
│   ├─ check.sh              # Pre-push gate: fmt + clippy + test + drift-guard
│   └─ check.ps1             # PowerShell equivalent
├─ docs/                     # Project documentation
│   ├─ ARCHITECTURE.md       # This document
│   ├─ ROADMAP.md            # Planned milestones & feature priorities
│   ├─ WHITEPAPER.md         # Design rationale, tech choices
│   └─ QUICKSTART.md         # First-time local setup
├─ .github/
│   └─ workflows/
│       ├─ ci.yml            # Lint → test → build (Linux, Windows, macOS matrix)
│       └─ security.yml      # Weekly cargo audit + cargo deny
├─ .agents/
│   └─ skills/               # Agent skill definitions
├─ README.md                 # Project overview
├─ LICENSE                   # MIT
└─ .gitignore
```

---
## Module Details

### oz-core
- **Responsibilities**: Foundation crate. Every other crate depends on it.
- **Key types**:
  - `Money(i64 minor_units, Currency)` — integer-only, checked arithmetic. Never f32/f64.
  - `Currency([u8; 3])` — ISO-4217 currency code.
  - `Cart` / `CartLine` — in-memory sale pipeline with currency matching.
  - `Sale` / `SaleLine` — transaction lifecycle state machine: `Pending → Active → Completed | Voided`.
  - `Product`, `Category`, `Inventory`, `Sku` — domain types with serde.
  - `Feature` — 32 toggleable feature flags with dependency resolution and 4 store presets.
  - `Store<'a>` — typed CRUD facade over `&Connection`. All writes inside transactions.
- **Migrations**: Embedded via `include_str!` at `crates/oz-core/migrations/`. Run on startup by both `oz-api` and `src-tauri`.
- **Rules**: `#![deny(unsafe_code)]`, `#![warn(missing_docs)]`.

### oz-hal
- **Responsibilities**: Uniform async API for all peripheral devices.
- **Traits**: `BarcodeScanner`, `ReceiptPrinter`, `CashDrawer` (async, in `traits/`).
- **Registry**: `DriverRegistry` — `HashMap<String, Arc<dyn Trait>>` per device category behind `RwLock`. Register/lookup/discover.
- **Mock driver**: In `drivers/mock.rs` — programmable queues, error injection, call counters. Gated by the `mock` feature. Required for all tests.
- Business code only uses traits via `DriverRegistry`; never imports concrete drivers.

### oz-api
- **Responsibilities**: Standalone REST API server for third-party integrations and headless operation.
- **Stack**: axum 0.8 + jsonwebtoken + tower-http.
- **Server**: Listens on port 3099 (`OZ_API_PORT` env var). `AppState` wraps `Arc<Mutex<Connection>>`.
- **Auth**: JWT HS256 tokens. `POST /api/v1/tokens` creates them. `auth_middleware` guards protected routes.
- **Routes**:
  - Public: `GET /api/v1/health`, `POST /api/v1/tokens`
  - Protected (JWT): `GET/POST /api/v1/products`, `GET /api/v1/products/{sku}`, `PATCH /api/v1/products/{sku}/stock`, `GET /api/v1/categories`
- **Tests**: 30+ integration tests on seeded in-memory databases.

### oz-cli
- **Responsibilities**: Command-line administration tool (`oz` binary).
- **Subcommands** (via clap): `migrate` (working), `backup` (stub), `export` (stub).
- Uses `anyhow` for error propagation.

### Scaffold Crates
`oz-lua`, `oz-security`, `oz-payment`, `oz-reporting` currently contain only error types (`LuaError`, `SecurityError`, `PaymentError`, `ReportingError`) and doc headers. Full implementations planned for later phases:
- **oz-lua** → Phase 3 (rlua embedding for dynamic business rules)
- **oz-security** → Phase 2 (OS key-ring, TLS config, PCI-DSS helpers)
- **oz-payment** → Phase 4 (PaymentProcessor trait, Stripe/Square/mock impls)
- **oz-reporting** → Phase 5 (SQL aggregation, CSV export, dashboards)

### oz-logging
- `tracing` + `tracing-subscriber` with env-filter.
- Single `oz_logging::init()` call wires up log sinks. Used by `src-tauri` and `oz-api`.
- JSON formatter, syslog, and Windows Event Log outputs planned for Phase 2.

### src-tauri (Tauri v2 Shell)
- **Entry point**: `main.rs` → `lib.rs::run()`.
- **State**: `AppState` holds `Mutex<Connection>` (SQLite WAL mode), `Arc<DriverRegistry>`, `AppHandle`.
- **Commands** (5, registered via `invoke_handler!`): `ping`, `version`, `start_sale`, `add_line`, `complete_sale`, `open_cash_drawer`, `print_receipt`.
- **Error**: `AppError` — tagged JSON with `{kind, message}`, `From` impls for `CoreError`, `HalError`, `tauri::Error`.

### ui/ (React Frontend)
- **Stack**: React 18 + TypeScript + Vite + `@fluent/react` (i18n) + Vitest (testing).
- **Architecture rule**: Components never call `invoke()` directly — they go through `ui/src/api/pos.ts`.
- **i18n rule**: All user-visible strings use `@fluent/react`. No hardcoded English in JSX.
- **Types**: `ui/src/types/domain.ts` mirrors Rust types with branded TypeScript (CartId, LineId, Sku, Money).

---
## Build & Run Instructions
1. **Install Rust toolchain** (≥ 1.88 stable) and `cargo`.
2. **Install Node.js** (≥ 18) for the front‑end.
3. **Install Tauri prerequisites** — see [Tauri docs](https://tauri.app/v2/guides/) for platform‑specific SDKs.
4. **Bootstrap workspace**:
   ```bash
   cargo build --workspace
   cd ui && npm install && cd ..
   cargo tauri dev          # launches Tauri dev window
   ```
5. **Run on Android/iPad** — Use Tauri's mobile targets (requires Android SDK / Xcode).

---
## Extensibility
- New device drivers can be added under `crates/oz-hal/src/drivers/` by implementing the relevant trait.
- Additional business logic can be scripted in Lua files placed in a `scripts/` directory (Phase 3).
- Payment gateway integrations can be introduced as separate crates linked to `oz-core`.
- New REST endpoints go in `crates/oz-api/src/routes/` and are registered in `lib.rs`.

---
## License & Contributions
- Open‑source MIT license.
- Contributions welcome via pull‑requests; follow the project's coding standards (Rust fmt, Clippy, TypeScript lint).

---
*Document generated on 2026‑06‑28.*
