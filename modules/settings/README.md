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
- **Commands** — Settings, setup, and sync Tauri commands (`src-tauri/src/commands/settings.rs`, `setup.rs`, `sync.rs`)
- **Frontend** — Settings and setup wizard screens (`ui/src/features/settings/`, `ui/src/features/setup/`)
- **API** — TypeScript API client (`ui/src/api/settings.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/*/settings.ftl`)

In the current phase, these files remain in their original locations. They will be physically moved into `modules/settings/` in subsequent phases.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares settings for access
3. **`on_stop`** — Cleans up resources

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
