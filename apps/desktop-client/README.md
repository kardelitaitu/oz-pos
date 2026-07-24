<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: STALE (2 findings, doc-staleness counts) · F1 (line 17): "156 commands registered" -> 392 #[tauri::command] fns in generate_handler! (apps/desktop-client/src/lib.rs:170) · F2 (line 20): "42 files" in commands/ -> 47 .rs files (apps/desktop-client/src/commands/) · verified accurate: AppState fields (Arc<Mutex<Connection>>, Arc<DriverRegistry>, Option<AppHandle>, Mutex<Option<oneshot::Sender<()>>> scanner_cancel — state.rs:56/59/63/71), barcode:scanned emit via app.emit() (hardware.rs:372), app is Option<AppHandle>, add-command steps match tauri-ipc/SKILL.md -->

# `apps/desktop-client/` — OZ-POS desktop shell

Tauri v2 binary that hosts the React front-end, wires `oz-core` + `oz-hal` behind typed IPC commands, and produces installable bundles.

## Layout

```
apps/desktop-client/
├── Cargo.toml              # oz-pos-app crate
├── tauri.conf.json         # Tauri v2 config (window, updater, capabilities)
├── build.rs                # tauri_build::build()
├── capabilities/
│   └── default.json        # ACL for the main window
├── icons/                  # Full platform icon set (generated via cargo tauri icon)
└── src/
    ├── main.rs             # Binary entry; calls lib::run()
    ├── lib.rs              # Builder, invoke_handler!, run() — 156 commands registered
    ├── error.rs            # AppError (typed, non_exhaustive)
    ├── state.rs            # AppState (DB, driver registry, scanner cancel channel)
    └── commands/           # 42 files, grouped by domain
        ├── audit.rs        # list_audit_log
        ├── auth.rs         # staff_login
        ├── authz.rs        # authorization checks
        ├── branding.rs     # store branding config
        ├── bundles.rs      # product bundles
        ├── categories.rs   # CRUD for product categories
        ├── currencies.rs   # currency_info, list_currencies, get/set_default_currency
        ├── customers.rs    # CRUD for customers
        ├── data.rs         # data management commands
        ├── exchange_rates.rs # CRUD for exchange rates
        ├── features.rs     # list_all_features, set_feature
        ├── gift_cards.rs   # gift card management
        ├── hardware.rs     # cash drawer, receipt printing, scanner lifecycle
        ├── health.rs       # ping, version
        ├── history.rs      # sales history
        ├── inventory_counts.rs # stock counting
        ├── kds.rs          # Kitchen Display System
        ├── loyalty.rs      # loyalty program
        ├── mod.rs          # module re-exports
        ├── offline.rs      # offline mode commands
        ├── plugins.rs      # plugin management
        ├── pos.rs          # core POS pipeline
        ├── product_variants.rs # variant CRUD
        ├── products.rs     # CRUD, barcode lookup, stock adjustment
        ├── promotions.rs   # promotion management
        ├── purchasing.rs   # purchase orders
        ├── refunds.rs      # refund/void processing
        ├── reports.rs      # report generation
        ├── sales.rs        # start_sale, add_line, complete_sale, hold/void, EOD
        ├── scale.rs        # weight scale integration
        ├── settings.rs     # receipt & store settings get/set
        ├── setup.rs        # setup wizard status, complete, feature discovery
        ├── shifts.rs       # staff shift management
        ├── staff.rs        # CRUD for staff, list_roles
        ├── stock_transfers.rs # transfer stock between locations
        ├── store_profiles.rs # multi-store profiles
        ├── sync.rs         # sync commands
        ├── tables.rs       # restaurant table management
        ├── tax.rs          # CRUD for tax rates
        ├── terminals.rs    # terminal management
        ├── void.rs         # void transactions
        └── workspaces.rs   # workspace management
```

## Adding a command

1. Create `apps/desktop-client/src/commands/<feature>.rs` with `#[tauri::command] async fn`.
2. Add `pub mod <feature>;` to `apps/desktop-client/src/commands/mod.rs`.
3. Register in `invoke_handler!` in `apps/desktop-client/src/lib.rs`.
4. Add typed wrapper in `ui/src/api/<feature>.ts`.

Full checklist in `.agents/skills/tauri-ipc/SKILL.md`.

## Icons

Generated from source image:
```bash
cargo tauri icon path/to/1024x1024.png
```
Writes to `icons/` and updates `tauri.conf.json`.

## Running

```bash
cd ui && npm run dev          # Terminal 1: Vite dev server
cargo tauri dev               # Terminal 2: Tauri dev shell
```

`cargo tauri build` produces platform bundles in `target/release/bundle/`.

## Key state

- `AppState` holds a `Mutex<Connection>` for SQLite, a `DriverRegistry` for HAL devices, and a `Mutex<Option<oneshot::Sender<()>>>` for scanner cancellation.
- Scanner background tasks emit `barcode:scanned` events via `app.emit()`.
- The `app` handle is `Option<AppHandle>` — always unwrap via `if let Some(ref app)`.

> last audited 07-07-26 by docs-auditor
