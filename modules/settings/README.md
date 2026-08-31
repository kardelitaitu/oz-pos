<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings, 1 low-severity observe) · all owned paths verified: crates/oz-core/src/settings.rs + db/settings.rs, commands/{settings,setup,sync}.rs, features/{settings,setup}, api/settings.ts, ui/src/locales/settings.ftl; modules/settings/src/lib.rs has SettingsModule; manifest deps [] match · observe: Overview says settings owns "currency/exchange rate configuration" while modules/currency owns the ISO table + rates — a doc overlap (settings = default-currency config, currency = table/rates), not a false claim · Kernel API matches · RE-AUDITED 31-08 by docs-auditor: manifest.json re-verified (id settings, v1.0.0, deps [], perms view/edit); on_load/on_start/on_stop are stubs (log + "future phases will register handlers") — Lifecycle corrected (previously implied they validate/prepare/clean up); crate has repository.rs/service.rs (SettingsRepository/SettingsService) NOT wired into runtime and NOT parity-tested (no tests/boundary_contract.rs); the currency-ownership observe (settings = default-currency config, currency = ISO table/rates) still holds; normalized footer -->

# Settings Module

**Status:** Active (Phase 2.6 — Proof of Concept)

## Overview

The Settings module owns the store configuration vertical. It handles store name/address/tax ID, receipt formatting options, feature flag management, currency/exchange rate configuration, sync settings, and the setup wizard state.

## Module Info

| Field        | Value            |
|--------------|------------------|
| ID           | `settings`       |
| Version      | `1.0.0`          |
| Dependencies | `[]`             |
| Permissions  | `settings:view`, `settings:edit` |

## Currently Owns

- **Backend** — Settings CRUD, feature flags, currencies (`crates/oz-core/src/settings.rs`, `crates/oz-core/src/db/settings.rs`)
- **Commands** — Settings, setup, and sync Tauri commands (`apps/desktop-client/src/commands/settings.rs`, `apps/desktop-client/src/commands/setup.rs`, `apps/desktop-client/src/commands/sync.rs`)
- **Frontend** — Settings and setup wizard screens (`ui/src/features/settings/`, `ui/src/features/setup/`)
- **API** — TypeScript API client (`ui/src/api/settings.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/settings.ftl`)

In the current phase the runtime settings path still runs through the files above (notably `crates/oz-core/src/settings.rs` and `db/settings.rs`). The crate now also carries a mirror — `repository.rs` (`SettingsRepository`) and `service.rs` (`SettingsService`) — but these are **not yet wired into the runtime** and, unlike tax/inventory, there is no `tests/boundary_contract.rs` pinning them. A subsequent phase will move the implementation fully into `modules/settings/`.

## Lifecycle

The module implements `foundation::contracts::Module`. Its lifecycle hooks are currently **stubs** — each logs a message and returns `Ok(())`; none touch the database or event bus yet:

1. **`on_load`** — logs "validating configuration" (a future phase will register event handlers to react to setting changes)
2. **`on_start`** — logs "ready to manage configuration"
3. **`on_stop`** — logs "cleaning up"

## Registration

Registered with the kernel during application setup:

```rust
use modules_settings::SettingsModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(SettingsModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "settings",
  "name": "Settings",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["settings:view", "settings:edit"]
}
```

> last audited 31-08-26 by docs-auditor
