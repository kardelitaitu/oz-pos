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
| Inventory       | Products, stock levels, bundles, categories, and inventory reports              | Ready        |
| Admin           | Settings, staff, reports, audit log, and configuration                          | Ready        |
| Reports         | Sales, inventory, and analytics dashboards — KPIs, charts, and exports          | Ready        |
| Kiosk           | Self-service checkout — customers tap to start, order, and pay themselves       | Coming soon  |
| Bar / Beverage Station | Ticket display for the bar — bartenders see and complete drink orders    | Coming soon  |

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

**Kiosk** — a self-service checkout workspace on the roadmap: a screen for
customers to tap to start, build their order, and pay without staff. It is
listed in the picker as **Coming soon** and will appear as a ready workspace
once it ships.

**Bar / Beverage Station** — a ticket display for the bar, like the Kitchen
Display but for drinks: bartenders see drink orders fire from the kitchen
queue and complete them. It is also listed as **Coming soon**.
