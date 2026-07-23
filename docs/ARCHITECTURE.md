<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: STALE (multiple findings — superseded early-architecture snapshot) · F1: "15+ members" -> 28 workspace members · F2: "oz-core migrations 20 embedded" -> 98 .sql files (crates/oz-core/migrations/) · F3: scaffold crates (oz-lua/oz-security/oz-reporting) are now IMPLEMENTED — oz-lua has apply_discount/calc_line_tax/validate_order/load_dir; oz-security has full keyring/TLS/mask; oz-reporting has daily_summary/menu_engineering/metrics engines (contradicts "Scaffold Crates" section) · F4: Node ">=18" -> ui/package.json engines >=22 · F5: i18n paths wrong (en-US.ftl, styles/) -> per-feature bundles.ftl + ui/src/frontend/themes/tokens.css · F6: "LICENSE: MIT" -> proprietary (All Rights Reserved) · F7: commands "62+" -> 47 desktop modules, 618 total IPC (README audit) · accurate: 9 modules, Feature 32 flags, React18+@fluent/react+pos.ts rule, oz-hal DriverRegistry/traits, oz-api port 3099 -->

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
├─ Cargo.toml                # Workspace definition (15+ members)
├─ rust-toolchain.toml       # Rust toolchain (stable)
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
│   │   │   ├── db/          # Store CRUD modules (sales, products, categories, inventory, tax, customers, staff, settings, offline, audit)
│   │   │   ├── events.rs    # Domain events (SaleCompleted, etc.)
│   │   │   ├── offline.rs   # Offline queue
│   │   │   ├── sync_client.rs # Cloud sync client (HTTP POST via reqwest)
│   │   │   ├── tax_rate.rs  # Tax rate domain type
│   │   │   ├── customer.rs  # Customer domain type
│   │   │   ├── staff.rs     # Staff / Role domain types
│   │   │   ├── refund.rs    # Refund domain type
│   │   │   ├── settings.rs  # Settings persistence layer
│   │   │   ├── features.rs  # Feature enum (32 flags), registry, presets
│   │   │   ├── migrations.rs# Embedded SQL migration runner (20 migrations)
│   │   │   └── error.rs     # CoreError enum
│   │   └── migrations/      # SQL migration files (001–020)
│   ├─ oz-hal/               # Hardware Abstraction Layer
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       ├─ lib.rs        # Public API
│   │       ├─ traits/       # Device traits (BarcodeScanner, ReceiptPrinter, CashDrawer)
│   │       ├─ drivers/      # Mock + real device drivers
│   │       │   ├─ mock.rs        # Programmable mocks for all traits
│   │       │   ├─ escpos.rs      # Shared ESC/POS formatting constants and helpers
│   │       │   ├─ usb_scanner.rs # USB HID barcode scanner (real)
│   │       │   ├─ serial_scanner.rs # Serial port scanner (stub)
│   │       │   ├─ usb_printer.rs # USB receipt printer (stub, ESC/POS)
│   │       │   ├─ bt_printer.rs  # Bluetooth SPP receipt printer
│   │       │   └─ tcp_printer.rs # TCP/network receipt printer (raw port 9100)
│   │       ├─ transport/
│   │       │   ├─ mod.rs
│   │       │   ├─ usb.rs    # USB enumeration, VID/PID matching, open/claim
│   │       │   ├─ serial.rs # Serial port enumeration, BT port detection
│   │       │   └─ tcp.rs    # TCP connection helper for network printers
│   │       ├─ registry.rs   # DriverRegistry (discover, register, lookup)
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
├─ apps/desktop-client/      # Tauri v2 application shell
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
- **Migrations**: 20 embedded SQL files in `crates/oz-core/migrations/`. Registered and run by `migrations.rs`; executed on startup by `platform-startup`.
- **Rules**: `#![deny(unsafe_code)]`, `#![warn(missing_docs)]`.

### oz-hal
- **Responsibilities**: Uniform async API for all peripheral devices.
- **Traits**: `BarcodeScanner`, `ReceiptPrinter`, `CashDrawer` (async, in `traits/`).
- **Registry**: `DriverRegistry` — `HashMap<String, Arc<dyn Trait>>` per device category behind `RwLock`. Register/lookup/discover. `discover()` probes USB + serial hardware at startup.
- **Transport layer** (`transport/`): `usb.rs` enumerates HID-class and printer-class USB devices by known VID/PID pairs. `serial.rs` enumerates serial ports with POS adapter detection and Bluetooth SPP port filtering. `tcp.rs` provides async TCP connection helpers for network printers (port 9100).
- **Real drivers**:
  - `UsbHidBarcodeScanner` — USB HID interrupt transfers, HID keycode → ASCII conversion, Enter-terminated scan accumulation.
  - `SerialBarcodeScanner` — serial port read until `\r`/`\n` terminator, configurable baud rate.
  - `UsbReceiptPrinter` — ESC/POS formatting over USB bulk OUT.
  - `BtReceiptPrinter` — Bluetooth SPP printer via virtual COM port. Auto-discovered by `serial::probe_bluetooth()`.
  - `TcpReceiptPrinter` — TCP/network printer via raw port 9100. Registered through `registry.register_tcp_printer()` with user-provided IP/hostname.
- **Shared ESC/POS** (`escpos.rs`): all printer drivers use a single `format_receipt()` helper and shared cut/init constants.
- **Mock driver**: In `drivers/mock.rs` — programmable queues, error injection, call counters. Required for all tests.
- Business code only uses traits via `DriverRegistry`; never imports concrete drivers.
- Blocking USB/serial I/O wrapped in `tokio::task::spawn_blocking`. Device handles held behind `tokio::sync::Mutex`.

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
`oz-lua`, `oz-payment`, `oz-reporting` currently contain error types and doc headers. Full implementations planned for later phases:
- **oz-lua** → Phase 3 (rlua embedding for dynamic business rules)
- **oz-payment** → Phase 4 (PaymentProcessor trait, Stripe/Square/mock impls)
- **oz-reporting** → Phase 5 (SQL aggregation, CSV export, dashboards)

#### oz-security (implemented)
- **Keyring trait** with three platform-native backends: Windows Credential Manager (`windows-sys`), macOS Keychain (`security-framework`), Linux Secret Service (`zbus`).
- **InMemoryKeyring** fallback for development/CI.
- **TlsConfig** — client cert + CA bundle loading, validation, builder API.
- **Mask** — card number masking for PCI-DSS safe display.

### oz-logging
- `tracing` + `tracing-subscriber` with env-filter.
- Single `oz_logging::init()` call wires up log sinks. Used by `apps/desktop-client` and `oz-api`.
- JSON formatter, syslog, and Windows Event Log outputs planned for Phase 2.

### apps/desktop-client & apps/tablet-client (Tauri v2 Shells)
Each app crate has an identical command surface, wired through `platform-startup`:
- **Entry point**: `main.rs` → `lib.rs::run()`.
- **State**: `AppState` holds `Mutex<Connection>` (SQLite WAL mode), `Arc<DriverRegistry>`, `AppHandle`.
- **Commands** (62+ across health, sales, hardware, tax, staff, customers, products, inventory, offline, reporting, settings, currency).
- **Error**: `AppError` — tagged JSON with `{kind, message}`, `From` impls for `CoreError`, `HalError`, `tauri::Error`.

### platform/ (Platform Crates)
- **platform-core**: Shared DB schema, Store facade, migration runner for all platform crates.
- **platform-startup**: Initialisation orchestration — DB setup, migration run, event handler registration, audit logging.
- **platform-sync**: Offline-first sync engine with `SyncTransport` (reqwest-based HTTP push/pull), conflict detection, retry logic.

### modules/ (Business Modules)
9 modules wired via the event bus in `platform-startup`:
- **sales**, **inventory**, **crm**, **tax**, **settings**, **staff**, **reporting**, **terminal**, **currency**
- Each module registers event handlers (e.g. `SaleCompleted` → stock decrement, audit log, report update).

### ui/ (React Frontend)
- **Stack**: React 18 + TypeScript + Vite + `@fluent/react` (i18n) + Vitest (testing).
- **Architecture rule**: Components never call `invoke()` directly — they go through `ui/src/api/pos.ts`.
- **i18n rule**: All user-visible strings use `@fluent/react`. No hardcoded English in JSX.
- **Types**: `ui/src/types/domain.ts` mirrors Rust types with branded TypeScript (CartId, LineId, Sku, Money).

---
## Build & Run Instructions
1. **Install Rust toolchain** (stable) and `cargo`.
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
- See [MODULAR_APP_PLAN.md](./MODULAR_APP_PLAN.md) for detailed execution roadmaps covering dynamic module lifecycle hot-reloading (`platform/kernel`), LAN peer-to-peer KDS sync, and Docker containerized cloud server deployments (`apps/cloud-server`).

---
## License & Commercial Governance
- **Proprietary & Confidential (`All Rights Reserved`)**: See [`LICENSE`](../LICENSE) for terms.
- No commercial deployment, redistribution, or modification is permitted without an executed commercial license agreement from OZ-POS Contributors.
- Internal developer contributions are governed under proprietary contributor agreements; all code strictly adheres to pre-commit quality gates (`cargo fmt + clippy + i18n lint + bundle parity`).

---
*Document generated on 2026‑06‑29.*
