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

Subscribes to the `sale.completed` event. For each completed sale, it inserts a row into the `report_sales` table, capturing sale ID, line items, total, currency, customer, and timestamp. This data is available for aggregated reporting (daily summaries, hourly trends, etc.).

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for reporting operations
3. **`on_stop`** — Cleans up resources

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
