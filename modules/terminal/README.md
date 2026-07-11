# Terminal Module

**Status:** Active (Phase 2 — Module Extraction)

## Overview

The Terminal module manages registered POS terminals: device registration, heartbeat/ping tracking, and terminal configuration.

## Module Info

| Field        | Value                  |
|--------------|------------------------|
| ID           | `terminal`             |
| Version      | `1.0.0`                |
| Dependencies | `[]`                   |
| Permissions  | `terminal:view`, `terminal:edit` |

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for terminal operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_terminal::TerminalModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(TerminalModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "terminal",
  "name": "Terminal",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["terminal:view", "terminal:edit"]
}
```
