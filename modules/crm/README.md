# CRM Module

**Status:** Active (Phase 2.4 — Proof of Concept)

## Overview

The CRM module owns the customer relationship management vertical. It handles customer CRUD (create, read, update, delete), loyalty points tracking, and purchase history.

## Module Info

| Field        | Value        |
|--------------|--------------|
| ID           | `crm`        |
| Version      | `1.0.0`      |
| Dependencies | `[]`         |
| Permissions  | `crm:view`, `crm:edit` |

## Currently Owns

- **Backend** — Customer CRUD (`crates/oz-core/src/db/customers.rs`)
- **Commands** — Customer Tauri commands (`apps/desktop-client/src/commands/customers.rs`)
- **Frontend** — Customer management screen (`ui/src/features/customers/`)
- **API** — TypeScript API client (`ui/src/api/customers.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/*/customers.ftl`)

In the current phase, these files remain in their original locations. They will be physically moved into `modules/crm/` in subsequent phases.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for customer operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_crm::CrmModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(CrmModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "crm",
  "name": "CRM",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["crm:view", "crm:edit"]
}
```

> last audited 2026-07-07 by docs-auditor
