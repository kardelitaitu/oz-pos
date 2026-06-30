# Currency/Exchange Module

**Status:** Active (Phase 2 — Module Extraction)

## Overview

The Currency/Exchange module manages currencies and exchange rates. It provides the ISO-4217 currency table, default currency configuration, and exchange rate CRUD for multi-currency transactions.

## Module Info

| Field        | Value                        |
|--------------|------------------------------|
| ID           | `currency`                   |
| Version      | `1.0.0`                      |
| Dependencies | `[]`                         |
| Permissions  | `currency:view`, `currency:edit` |

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for currency operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_currency::CurrencyModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(CurrencyModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "currency",
  "name": "Currency/Exchange",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["currency:view", "currency:edit"]
}
```
