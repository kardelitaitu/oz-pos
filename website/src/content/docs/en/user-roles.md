---
title: User Roles
description: Five permission presets decide what each staff account can do and see.
category: guides
order: 9
updated: "2026-08-16"
---

## What a role is

Every staff account has a role — a permission preset that decides what the
account can do and see. Roles come from a fixed taxonomy of five presets,
shown when you manage staff in **Settings → Staff**.

## The five roles

| Access area                       | Staff | Manager | Auditor | Admin | Owner |
| --------------------------------- | ----- | ------- | ------- | ----- | ----- |
| Sales & checkout                  | ✓     | ✓       | —       | ✓     | ✓     |
| Voids & refunds                   | —     | ✓       | —       | ✓     | ✓     |
| Payments (cash, card, settle)     | ✓     | ✓       | —       | ✓     | ✓     |
| Discounts (apply)                 | ✓     | ✓       | —       | ✓     | ✓     |
| Attach customer & loyalty at checkout | ✓ | ✓       | —       | ✓     | ✓     |
| Shifts (open, close)              | ✓     | ✓       | view    | ✓     | ✓     |
| Products & catalog                | —     | ✓       | read    | ✓     | ✓     |
| Edit product cost                 | —     | ✓       | —       | ✓     | ✓     |
| Inventory (adjust, transfer, count) | —   | ✓       | read    | ✓     | ✓     |
| Customers & loyalty (manage)      | —     | ✓       | read    | ✓     | ✓     |
| Promotions (manage)               | —     | ✓       | —       | ✓     | ✓     |
| Staff accounts (create, update)   | —     | ✓       | read    | ✓     | ✓     |
| Manage roles                      | —     | —       | —       | ✓     | ✓     |
| Delete staff                      | —     | —       | —       | —     | ✓     |
| Settings                          | —     | ✓       | read    | ✓     | ✓     |
| Reports & analytics               | —     | ✓       | view    | ✓     | ✓     |
| Audit log                         | —     | ✓       | view    | ✓     | ✓     |
| Kitchen Display (view, update)    | ✓     | ✓       | view    | ✓     | ✓     |
| Terminals (register, edit, delete) | —    | ✓       | —       | ✓     | ✓     |
| Workspace access                  | assigned | ✓     | ✓       | ✓     | ✓     |

Legend: **✓** full access · **read** view only · **assigned** only the
workspaces assigned to the account · **—** no access.

## The planned model

This matrix is the target for the codebase:

- **Staff is a checkout-operations role.** It keeps the actions performed at
  the register — processing sales, payments, in-cart discounts, attaching
  customers and loyalty, opening and closing shifts — plus the workspaces
  assigned to it. Every management surface (products, inventory, customers,
  promotions, staff, settings, reports, audit, terminals) requires **manager
  or above**, and voids, refunds, and price-sensitive actions are manager+
  as well.
- **Owner** is seeded with a global wildcard. **Admin** is global except
  ownership transfer, billing, and irreversible actions such as staff
  deletion. **Auditor** is global and read-only: it views operational data
  and the audit log but never manages, never exports, and never sees
  sensitive profile fields.
- **Custom** is a sixth preset — no permissions of its own; an admin picks
  every permission manually. It is not shown in the standard staff dropdown
  yet.

## Implementation status

The four gaps in the plan have been closed:

- **The `Staff` preset is now checkout-only** (`platform/core/src/rbac.rs`):
  it keeps sales processing, payments, in-cart discounts, customer and
  loyalty attach, shift open/close, table-service operations, KDS, and
  workspace switching — and nothing else. `sales:void`, `sales:refund`,
  `payments:refund`, `products:*`, `staff:*`, `reports:*`, `audit:*`,
  `terminals:*`, `inventory:*`, and `promotions:*` were removed, with pinned
  tests updated to the new model.
- **All management screens are explicitly gated.** Customers, Sales History,
  and both Dashboard screens now declare `requiredRole: 'manager'`, and the
  `'manager'` gate no longer admits Staff anywhere.
- **Auditor reaches its read-only screens.** Routing honors
  `requiredPermission` (mirroring backend `has_permission`): `audit:view` on
  the audit log, `reports:view` / `inventory:view` on the report screens,
  `products:read`, `customers:view`, `staff:read`, `settings:read`,
  `shifts:view_any`, and `loyalty:view` on the matching management screens.
- **Analytics is aligned.** The Analytics screen now declares
  `requiredRole: 'manager'` with `analytics:view` as the authoritative
  permission key.
- **In-page action buttons are permission-aware.** The management-level
  gate (`isManager`) no longer admits Staff, so the Void, Refund, price
  override, audit mark-reviewed/export, and full settings-card buttons are
  hidden for Staff instead of rendering a backend denial. The dev-mock
  (`ui/src/dev-mock/tauri-api.ts`) exercises the real five-role model —
  retired Cashier/Kitchen are gone everywhere, including the role badges,
  icons, and workspace picker.
