<!-- Audit stamp: 2026-07-26 · rewritten after the previous file was found corrupted (2.8 MB of repeated garbage text); content reconstructed from modules/loyalty/manifest.json, src/{lib,models,repository,service,error}.rs, and platform/startup/src/lib.rs -->

# Loyalty Module

**Status:** Active (registered with the kernel; domain logic still in its original locations)

## Overview

The Loyalty module owns the customer loyalty program: tier definitions with
per-tier earning multipliers, point earn and redeem, and member account
management. Points are earned automatically from completed sales via the
`sale.completed` event.

## Module Info

| Field        | Value |
|--------------|-------|
| ID           | `loyalty` |
| Crate        | `modules-loyalty` |
| Version      | `1.0.0` |
| Dependencies | `["crm"]` — a loyalty account belongs to a CRM customer |
| Permissions  | `loyalty:view`, `loyalty:earn`, `loyalty:redeem`, `loyalty:manage` |

## Currently Owns

- **Models** — `LoyaltyTier`, `LoyaltyAccount`, `LoyaltyTransaction`,
  `LoyaltyAccountWithDetails` (`src/models.rs`)
- **Repository** — account and transaction queries (`src/repository.rs`)
- **Service** — earn/redeem orchestration (`src/service.rs`)
- **Errors** — `LoyaltyError` (`src/error.rs`)
- **Event handler** — `LoyaltyEarnHandler` is wired on `sale.completed` in
  `platform/startup/src/lib.rs`, not in this crate's `on_load`

## Misplaced: gift cards

`src/models.rs` also defines `GiftCard`, `GiftCardTransaction`,
`GiftCardWithTransactions`, `IssueGiftCardInput`, `GiftCardFilter`, and
`RedeemGiftCardResult`. These are stored-value instruments, not loyalty
points, and belong to `modules/giftcards` — which now exists as a stub for
exactly that reason. Until the migration lands, treat these types as
deprecated in this crate and do not add to them here.

## Lifecycle

Implements `foundation::contracts::Module`:

1. **`on_load`** — validates configuration and dependencies
2. **`on_start`** — initialises state
3. **`on_stop`** — cleans up resources

`dependencies()` returns `&["crm"]`, matching `manifest.json`; a test asserts
the two cannot drift apart.

## Registration

Registered in `platform_startup::init_module_system` alongside every other
vertical. Note that this module was defined but **not registered** for
several releases, so its lifecycle hooks never ran even though
`LoyaltyEarnHandler` was subscribed on the bus. The
`every_module_manifest_is_registered` test in
`platform/startup/src/startup_tests.rs` now prevents that class of bug.

```rust
use modules_loyalty::LoyaltyModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(modules_crm::CrmModule::new()))?; // dependency
kernel.register(Box::new(LoyaltyModule::new()))?;
kernel.load_all()?;   // topologically sorts crm before loyalty
kernel.start_all()?;
```

## Manifest

```json
{
  "id": "loyalty",
  "name": "Loyalty",
  "version": "1.0.0",
  "dependencies": ["crm"],
  "permissions": [
    "loyalty:view",
    "loyalty:earn",
    "loyalty:redeem",
    "loyalty:manage"
  ]
}
```
