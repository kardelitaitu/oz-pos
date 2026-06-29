# Staff Module

**Status:** Active (Phase 2.7)

## Overview

The Staff module owns the staff management vertical. It handles user CRUD, role management, authentication, and session handling.

## Module Info

| Field        | Value        |
|--------------|--------------|
| ID           | `staff`      |
| Version      | `1.0.0`      |
| Dependencies | `[]`         |
| Permissions  | `staff:view`, `staff:edit`, `staff:auth` |

## Currently Owns

- **Types** — User and Role domain types (`crates/oz-core/src/user.rs`)
- **Backend** — User/Role CRUD (`crates/oz-core/src/db/staff.rs`)
- **Commands** — Staff Tauri commands (`src-tauri/src/commands/staff.rs`, `auth.rs`)
- **Frontend** — Staff management screen (`ui/src/features/staff/`)
- **API** — TypeScript API client (`ui/src/api/staff.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/*/staff.ftl`)

In the current phase, these files remain in their original locations. They will be physically moved into `modules/staff/` in subsequent phases.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for staff operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_staff::StaffModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(StaffModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "staff",
  "name": "Staff",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["staff:view", "staff:edit", "staff:auth"]
}
```
