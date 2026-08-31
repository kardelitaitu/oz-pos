<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings) · behavioral claim verified: SaleCompletedReporter (modules/reporting/src/handlers.rs:25) subscribes to sale.completed and creates+inserts into report_sales table (handlers.rs:38/70); ReportingModule implements Module and registers handler in on_load; modules/reporting/manifest.json deps [inventory, sales] match; Kernel::register/load_all/start_all match platform/kernel API · RE-AUDITED 2026-08-31 by docs-auditor: manifest deps [inventory,sales] + perms [reports:view,reports:export] and SaleCompletedReporter (handlers.rs:32) re-confirmed against current HEAD; three post-08-09 commits (767eb0d0 audit, a8c3d9a9 MSL-4/7, bf1ff807 MSL-8) are enhancements not contradictions — MSL-8 made the report_sales insert idempotent (replayed sale.completed logged+skipped), consistent with the README's "inserts a row per completed sale". OPEN (not a doc drift): refunded sales remain in report revenue since no refund event exists — product decision to confirm; README is module-scope and correctly silent · RE-AUDITED 31-08 (cont) by docs-auditor: CORRECTED the 07-22 "registers handler in on_load" claim — on_load is a lifecycle stub (lib.rs:74-77, logs only); the SaleCompletedReporter is actually constructed+subscribed in platform/startup/src/lib.rs:165 (same pattern as loyalty's handler). Body Event Handlers + Lifecycle updated to say so; the module's own doc comment (lib.rs:45 "Registers the SaleCompletedReporter") is similarly imprecise — code-side, left for the author -->

# Reporting Module

**Status:** Active (Phase 3 — Event Bus Subscriber)

## Overview

The reporting module generates and exports sales, inventory, and financial reports. It subscribes to the `sale.completed` domain event to capture sale data for report generation.

## Module Info

| Field        | Value                  |
|--------------|------------------------|
| ID           | `reporting`            |
| Version      | `1.0.0`                |
| Dependencies | `inventory`, `sales`   |
| Permissions  | `reports:view`, `reports:export` |

## Event Handlers

### `SaleCompletedReporter`

Subscribes to the `sale.completed` event. For each completed sale, it inserts a row into the `report_sales` table, capturing sale ID, line items, total, currency, customer, and timestamp. This data is available for aggregated reporting (daily summaries, hourly trends, etc.). The reporter is constructed and subscribed on the event bus in `platform/startup/src/lib.rs` — **not** in this crate's `on_load` (which is a lifecycle stub).

## Lifecycle

The module implements `foundation::contracts::Module`. Its lifecycle hooks are **stubs** — each logs and returns `Ok(())`; notably `on_load` does **not** register the `SaleCompletedReporter` (that wiring lives in `platform/startup`):

1. **`on_load`** — logs "validating configuration"
2. **`on_start`** — logs "ready for reporting"
3. **`on_stop`** — logs "cleaning up"

## Registration

Registered with the kernel during application setup:

```rust
use modules_reporting::ReportingModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(ReportingModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "reporting",
  "name": "Reporting",
  "version": "1.0.0",
  "dependencies": ["inventory", "sales"],
  "permissions": ["reports:view", "reports:export"]
}
```

> last audited 31-08-26 by docs-auditor
