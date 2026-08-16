---
title: Workspaces
description: Choose what each screen does — retail checkout, restaurant service, kitchen, or back office.
category: guides
order: 7
updated: "2026-08-16"
---

## The workspace picker

After signing in, staff land on a grid of workspace cards. Each workspace is
a role for the screen in front of you — what you can do, not where you are:

| Workspace       | What it is                                                                     | Status       |
| --------------- | ------------------------------------------------------------------------------ | ------------ |
| Store POS       | Retail checkout — product lookup, customers, and loyalty                       | Ready        |
| Restaurant POS  | Table-service checkout — menu categories and table management                   | Ready        |
| Kitchen Display | Order queue for the kitchen — tap tickets to advance their status               | Ready        |
| Warehouse       | Products, stock levels, bundles, categories, and inventory reports              | Ready        |
| Admin           | Settings, staff, reports, audit log, and configuration                          | Ready        |

## Access by assignment

Each staff member can only open the workspaces assigned to them — checkout
staff are typically assigned the POS workspaces, kitchen staff the Kitchen
Display. Cards you cannot open are shown disabled, and managers and above
are not assignment-gated. Assignments are set in **Settings → Staff**. See
[User Roles](../user-roles/).

## Pinning and quick launch

Star a workspace to pin it to the front of the grid, and the most recently
used workspace surfaces next. Number keys 1–9 launch a workspace directly.

## Workspace settings

Each workspace has its own settings, so a screen behaves differently
depending on its role. Store POS controls the receipt layout, paper width,
currency and tax display, and the barcode scanner. Restaurant POS controls
the table layout, course firing, and the kitchen printer. Kitchen Display
controls SLA escalation and the new-order sound. See
[Settings](../settings/) for the full list.

## Workspaces belong to a store

Every workspace instance is scoped to a store. On startup the device
resolves its store — from a terminal binding when one is set, otherwise the
primary store — and shows that store's workspaces. See [Stores & Topology](../stores/) and [Terminals](../terminals/).

## Planned workspaces

The picker shows placeholder cards for the workspaces still on the roadmap —
**Loyalty**, **Marketing**, and **Online Orders**. They are marked **Coming
soon** and will become ready workspaces as they ship.

**Kiosk** is not a workspace — it is a locked-down, self-service checkout
mode for an unattended screen. **Reports** are not a workspace either: sales
and analytics dashboards live inside the Admin workspace, under the **Reports**
screen.
