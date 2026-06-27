# OZ-POS – Codebase Architecture

## Overview
This document describes the directory layout and module responsibilities for **OZ-POS**. The design supports:
- Rust core engine (transaction handling, persistence)
- Hardware Abstraction Layer (HAL) for barcode scanners, printers, NFC, etc.
- Embedded Lua scripting for dynamic business rules
- Tauri v2 UI built with React/TypeScript
- Multi‑platform targets: Windows PC, Linux PC, Android tablet, iPad
- Scalable database strategy (SQLite on‑device, optional cloud sync)

---
## Directory Layout
```
oz-pos/
├─ Cargo.toml                # Workspace definition
├─ tauri.conf.json           # Tauri v2 configuration
├─ package.json              # Front‑end package manager (React/TS)
├─ .env.example              # Environment variable template
├─ src/                      # Rust workspace crates
│   ├─ oz-core/              # Core engine crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       ├─ lib.rs        # Public API (Engine, models)
│   │       ├─ models/       # Product, Order, Money, Currency structs
│   │       ├─ export/       # Export/Import (.ozpkg) module
│   │       └─ db.rs         # SQLite wrapper & sync hooks
│   ├─ oz-hal/               # Hardware Abstraction Layer
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       ├─ lib.rs        # Device traits (BarcodeScanner, Printer)
│   │       └─ drivers/      # USB, Bluetooth, serial implementations
│   ├─ oz-lua/               # Lua scripting runtime
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # Load/execute scripts, expose Rust functions
│   ├─ oz-security/          # Security crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # Key‑ring abstraction, TLS config, PCI‑DSS helpers
│   ├─ oz-logging/           # Structured logging crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # tracing + tracing-subscriber initializer
│   ├─ oz-payment/           # Payment processor crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # PaymentProcessor trait, Stripe/Square/mock impls
│   ├─ oz-reporting/         # Reporting crate
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # CSV/sales/inventory report generators
│   ├─ oz-perf/              # Performance & profiling helpers
│   │   ├─ Cargo.toml
│   │   └─ src/
│   │       └─ lib.rs        # tokio-console & flamegraph macros
│   └─ oz-cli/               # CLI utilities (migrations, backup, export)
│       ├─ Cargo.toml
│       └─ src/
│           ├─ main.rs       # clap entry-point
│           ├─ migrations.rs # DB migration runner
│           ├─ backup.rs     # SQLite snapshot export/import
│           └─ export_import.rs # Encrypted .ozpkg export/import
├─ ui/                       # Tauri front‑end (React/TS)
│   ├─ package.json
│   ├─ vite.config.ts        # Build config
│   └─ src/
│       ├─ components/       # Reusable React components
│       ├─ pages/            # Route-level page components
│       ├─ hooks/            # Custom React hooks
│       ├─ i18n/             # Fluent localisation files (en.ftl, th.ftl…)
│       └─ api/
│           └─ pos.ts        # Calls to Rust backend via Tauri commands
├─ assets/                   # UI mockups, icons, printer logos
├─ scripts/                  # Build helpers, schema generators
├─ migrations/               # SQL migration files (managed by refinery)
├─ docs/                     # Project documentation
│   ├─ ARCHITECTURE.md       # This document
│   ├─ ROADMAP.md            # Planned milestones & feature priorities
│   ├─ WHITEPAPER.md         # Design rationale, tech choices
│   └─ QUICKSTART.md         # First-time local setup
├─ .github/
│   └─ workflows/
│       ├─ ci.yml            # Lint → test → build pipeline
│       └─ release.yml       # Release automation & Tauri bundle upload
├─ CONTRIBUTING.md           # Coding standards, branch policy, PR checklist
└─ README.md                 # Project overview
```

---
## Module Details

### oz-core
- **Responsibilities**: transaction lifecycle, state machines, data models, persistence, sync interface, export/import.
- **Key Files**:
  - `src/lib.rs` – Exposes `Engine`, public functions.
  - `src/models/` – `Product`, `Order`, `Customer`, `Money`, `Currency` structs.
  - `src/db.rs` – SQLite wrapper (`rusqlite`), async sync hooks.
  - `src/export/` – `.ozpkg` pack/unpack and AES-256-GCM crypto helpers.

### oz-hal
- **Responsibilities**: uniform API for all peripheral devices.
- **Traits**: `Device`, `BarcodeScanner`, `Printer`, `NfcReader`.
- **Drivers**: concrete implementations for USB, Bluetooth, serial devices.
- Built on `embedded-hal` traits, making it portable across Windows, Linux, Android, and iPad.

### oz-lua
- Embeds `rlua` to execute merchant-provided Lua scripts.
- Exposes safe Rust functions (e.g., `apply_discount`, `calculate_tax`).
- Allows dynamic rule changes without recompiling the core.

### oz-security
- OS key-ring abstraction for secret storage.
- TLS configuration helpers for cloud sync traffic.
- PCI-DSS checklist utilities (tokenisation, encrypted field storage).

### oz-logging
- `tracing` + `tracing-subscriber` with JSON, syslog, and Windows Event Log outputs.
- Single `oz_logging::init()` call wires up all log sinks.

### oz-payment
- Defines `PaymentProcessor` trait.
- Pluggable implementations: Stripe, Square, EMV, or a mock for testing.

### oz-reporting
- SQL aggregation queries on the local SQLite DB.
- Generates sales and inventory CSV exports.
- Extensible to push data to cloud warehouses.

### oz-perf
- Macro helpers for `tokio-console` integration and flamegraph generation.
- Performance benchmarks: barcode lookup < 1 ms, transaction commit < 5 ms.

### oz-cli
- Uses `clap` for sub-commands: `init-db`, `migrate`, `run`, `backup`, `restore`, `export`, `import`.
- Entry point for developer tooling and merchant data management.

---
## Build & Run Instructions
1. **Install Rust toolchain** (stable) and `cargo`.
2. **Install Node.js** (>=18) for the front‑end.
3. **Install Tauri prerequisites** – see Tauri docs for platform‑specific SDKs.
4. **Bootstrap workspace**:
   ```bash
   cargo build --workspace
   cd ui && npm install && npm run dev   # launches Tauri dev window
   ```
5. **Run on Android/iPad** – Use Tauri's mobile targets (requires Android SDK / Xcode).

---
## Extensibility
- New device drivers can be added under `hal/src/drivers/` by implementing the `Device` trait.
- Additional business logic can be scripted in Lua files placed in the `scripts/` directory.
- Payment gateway integrations can be introduced as separate crates linked to `core`.

---
## License & Contributions
- Open‑source MIT license.
- Contributions welcome via pull‑requests; follow the project's coding standards (Rust fmt, Clippy, TypeScript lint).

---
*Document generated on 2026‑06‑28.*
