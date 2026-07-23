<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings) · all owned paths verified: crates/oz-core/src/db/sales.rs, apps/desktop-client/src/commands/pos.rs, ui/src/features/sales, ui/src/api/sales.ts, ui/src/locales/sales.ftl; modules/sales/manifest.json deps [inventory] match; Module trait + Kernel::register/load_all/start_all match platform/kernel API · Status "Phase 2.2 POC" consistent with files still in original locations -->

# Sales Module

**Status:** Active (Phase 2.2 — Proof of Concept)

## Overview

The Sales module is the core point-of-sale vertical. It owns the entire sale pipeline: cart management, checkout, payment processing, sales history, void/refund, held orders, and end-of-day reports.

## Module Info

| Field        | Value        |
|--------------|--------------|
| ID           | `sales`      |
| Version      | `1.0.0`      |
| Dependencies | `[inventory]` |
| Permissions  | `sales:void`, `sales:refund`, `reports:view` |

## Currently Owns

- **Backend** — Sales CRUD and business logic (`crates/oz-core/src/db/sales.rs`)
- **Commands** — POS pipeline Tauri commands (`apps/desktop-client/src/commands/pos.rs`)
- **Frontend** — Sale screens (`ui/src/features/sales/`)
- **API** — TypeScript API client (`ui/src/api/sales.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/sales.ftl`)

In the current phase, these files remain in their original locations. They will be physically moved into `modules/sales/` in subsequent phases.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration and checks dependencies
2. **`on_start`** — Initializes state and prepares for sale processing
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during `AppState::new()`:

```rust
use modules_sales::SalesModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(SalesModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "sales",
  "name": "Sales",
  "version": "1.0.0",
  "dependencies": ["inventory"],
  "permissions": ["sales:void", "sales:refund", "reports:view"]
}
```

> last audited 07-07-26 by docs-auditor
