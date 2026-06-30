# `src-tauri/` — OZ-POS desktop & mobile shell

Tauri v2 binary that hosts the React front-end, wires `oz-core` + `oz-hal` behind typed IPC commands, and produces installable bundles.

## Layout

```
src-tauri/
├── Cargo.toml              # oz-pos-app crate
├── tauri.conf.json         # Tauri v2 config (window, updater, capabilities)
├── build.rs                # tauri_build::build()
├── capabilities/
│   └── default.json        # ACL for the main window
├── icons/                  # Full platform icon set (generated via cargo tauri icon)
└── src/
    ├── main.rs             # Binary entry; calls lib::run()
    ├── lib.rs              # Builder, invoke_handler!, run() — 62 commands registered
    ├── error.rs            # AppError (typed, non_exhaustive)
    ├── state.rs            # AppState (DB, driver registry, scanner cancel channel)
    └── commands/           # 16 files, grouped by domain
        ├── audit.rs        # list_audit_log
        ├── auth.rs         # staff_login
        ├── categories.rs   # CRUD for product categories
        ├── currencies.rs   # currency_info, list_currencies, get/set_default_currency
        ├── customers.rs    # CRUD for customers
        ├── exchange_rates.rs # CRUD for exchange rates
        ├── features.rs     # list_all_features, set_feature
        ├── hardware.rs     # cash drawer, receipt printing, scanner lifecycle
        ├── health.rs       # ping, version
        ├── mod.rs          # module re-exports
        ├── products.rs     # CRUD, barcode lookup, stock adjustment
        ├── sales.rs        # start_sale, add_line, complete_sale, hold/void, EOD
        ├── settings.rs     # receipt & store settings get/set
        ├── setup.rs        # setup wizard status, complete, feature discovery
        ├── staff.rs        # CRUD for staff, list_roles
        └── tax.rs          # CRUD for tax rates
```

## Adding a command

1. Create `src/commands/<feature>.rs` with `#[tauri::command] async fn`.
2. Add `pub mod <feature>;` to `src/commands/mod.rs`.
3. Register in `invoke_handler!` in `src/lib.rs`.
4. Add typed wrapper in `ui/src/api/pos.ts`.

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

> last audited 28-06-26 by docs-auditor
