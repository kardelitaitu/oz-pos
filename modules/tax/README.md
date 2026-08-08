<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings) · all owned paths verified: crates//src/db/tax.rs, commands/tax.rs, features/tax, api/tax.ts, ui/src/locales/tax.ftl; modules/tax/src/lib.rs has TaxModule; manifest deps [] + permissions [tax:view,tax:edit] match · Kernel API matches -->

# Tax Module

**Status:** Active (Phase 2.5 — Proof of Concept)

## Overview

The Tax module owns the tax configuration vertical. It handles tax rate CRUD, product and category tax assignments, and tax calculation helpers.

## Module Info

| Field        | Value            |
|--------------|------------------|
| ID           | `tax`            |
| Version      | `1.0.0`          |
| Dependencies | `[]`             |
| Permissions  | `tax:view`, `tax:edit` |

## Currently Owns

- **Backend** — Tax rate CRUD and assignments (`crates//src/db/tax.rs`)
- **Commands** — Tax Tauri commands (`apps/desktop-client/src/commands/tax.rs`)
- **Frontend** — Tax configuration screen (`ui/src/features/tax/`)
- **API** — TypeScript API client (`ui/src/api/tax.ts`)
- **Locale** — Fluent translation strings (`ui/src/locales/*/tax.ftl`)

This crate is the **contractual layer** for the tax vertical: it owns the canonical domain types (`TaxRate`, `RoundingMode`), which `` re-exports. The implementation layers above remain in their original locations, and the boundary between this crate and those layers is pinned by the cross-layer contract tests in `modules/tax/tests/boundary_contract.rs` (manifest registration ↔ type identity ↔ DB behaviour parity ↔ serde wire shape). A future phase may physically move the implementation into this crate as the module system matures.

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for tax operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_tax::TaxModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(TaxModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "tax",
  "name": "Tax",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["tax:view", "tax:edit"]
}
```

> last audited 2026-09-08 · Phase 3 Module-Level Documentation Audit
> status: ACCURATE (verified against actual codebase)
> last audited 2026-09-08 · Phase 3 Module-Level Documentation Audit
> status: ACCURATE (verified against actual codebase)
