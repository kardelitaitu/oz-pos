# Inventory Module

**Status:** Active (Phase 2.3 — Proof of Concept)

## Overview

The Inventory module owns the entire product and stock management vertical. It handles product CRUD, barcode lookup, product variants (size/colour/flavour), categories, stock adjustments, and inventory tracking.

## Module Info

| Field        | Value            |
|--------------|------------------|
| ID           | `inventory`      |
| Version      | `1.0.0`          |
| Dependencies | `[]`             |
| Permissions  | `inventory:view`, `inventory:edit`, `inventory:adjust` |

## Currently Owns

- **Backend** — Product CRUD, stock, variants, categories (`crates/oz-core/src/db/products.rs`)
- **Commands** — Product and variant Tauri commands (`apps/desktop-client/src/commands/products.rs`, `apps/desktop-client/src/commands/product_variants.rs`, `apps/desktop-client/src/commands/categories.rs`)
- **Frontend** — Product screens (`ui/src/features/products/`), inventory adjustment (`ui/src/features/inventory/`)
- **API** — TypeScript API client (`ui/src/api/products.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/*/products.ftl`, `ui/src/locales/*/inventory.ftl`)

In the current phase, these files remain in their original locations. They will be physically moved into `modules/inventory/` in subsequent phases.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration and prepares data structures
2. **`on_start`** — Warms caches and verifies stock integrity
3. **`on_stop`** — Flushes state and releases resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_inventory::InventoryModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(InventoryModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "inventory",
  "name": "Inventory",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["inventory:view", "inventory:edit", "inventory:adjust"]
}
```

> last audited 07-07-26 by docs-auditor
