# Purchasing Module

**Status:** Stub (lifecycle only — no domain logic yet)

## Overview

The Purchasing module will own the inbound side of stock: supplier records,
purchase orders, goods receipt, and supplier returns. It is the counterpart to
`inventory`, which owns on-hand quantities but not how they got there.

## Module Info

| Field        | Value |
|--------------|-------|
| ID           | `purchasing` |
| Crate        | `modules-purchasing` |
| Version      | `0.1.0` |
| Dependencies | `["inventory"]` — receiving a PO increments stock |
| Permissions  | `purchasing:view`, `purchasing:order`, `purchasing:receive`, `purchasing:manage` |
| Feature flag | `purchase-orders` (`crates/oz-core/src/features.rs`) |

## Currently Owns

Nothing. `PurchasingModule` registers with the kernel, declares its dependency
on `inventory`, and logs its lifecycle transitions. No tables, no commands, no
event subscriptions.

## Why it exists now

Registering the stub means the dependency edge `purchasing → inventory` is
exercised by the kernel's topological sort from the first commit, and the
`every_module_manifest_is_registered` parity test keeps it wired. Adding real
logic later is then an additive change inside one crate rather than a
cross-cutting one.

## Promotion Checklist

- [ ] `models.rs` — `Supplier`, `PurchaseOrder`, `PurchaseOrderLine`, `GoodsReceipt`
- [ ] `repository.rs` — PO tables and queries (namespace: `purchasing_*`)
- [ ] `service.rs` — receive-a-PO in a single transaction so stock and the PO
      status can never disagree
- [ ] Emit `stock.adjusted` on receipt rather than writing inventory tables
      directly
- [ ] Tauri commands in the owning app's `commands/` directory
- [ ] Gate the UI on the `purchase-orders` feature flag

All monetary amounts use `Money` (`i64` minor units) — unit cost, landed cost,
and PO totals included. See `modules/README.md` for the full promotion path.
