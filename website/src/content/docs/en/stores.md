---
title: Stores & Topology
description: Model branches, registers, and warehouses in one visual editor.
category: guides
order: 6
updated: "2026-08-16"
---

## The topology editor

Stores, registers, warehouses, and hardware are arranged in a visual diagram —
the **Visual Store & Workspace Topology Builder**. Nodes are dragged from the
palette (or added with the number keys) and wired together on a canvas with
zoom, pan, minimap, auto-layout, snap-to-grid, and undo/redo. Ready-made
**Retail** and **Resto & KDS** presets scaffold a full store in one click, and
a **Test Order Simulation** sends test tickets through the layout so you can
watch the flow before going live.

## Nodes and connections

Each node is a real piece of your business: **Store** (branch location),
**Retail POS**, **Restaurant POS**, **Kitchen Display (KDS)**, **Warehouse**,
**Stock Room**, and **Hardware** (printers and peripherals). Cards expose typed
ports — **Location**, **Operation**, **Stock In/Out**, **Ticket**, and
**Device** — and connecting two ports asks what the wire means: stock routing,
inventory transfer, ticket routing, device connection, or operation. Wire
direction cycles one-way → reverse → two-way, so the diagram shows exactly
which way stock, tickets, and operations flow.

## Validation

The editor validates the layout as you build. An issues panel flags problems
live: exactly one branch location per graph, every workspace connected to its
branch via **Location In**, every KDS fed by a Restaurant POS via **Operation
In**, no directed cycles, and no duplicate nodes or wires. Warehouse warnings
appear when storage is at capacity or nothing routes stock into it.

## Applying changes

Applying the topology is a manager- or owner-only action — everyone else sees
a view-only canvas. Apply shows a diff summary of what will change (created,
updated, archived, type-changed, with the revision number) before it is saved.
If the topology changed on another register meanwhile, the editor loads the
latest version and asks you to re-apply.

## Branches, templates, and sharing

Topologies live per branch. A **Compare Branches** view shows what differs
between two branches and can focus on the differences. Templates save a layout
for reuse, and a topology can be **exported** to the clipboard and **imported**
elsewhere — handy for rolling out the same layout to every branch.

## Plan limits

The number of stores, registers, and warehouses is set by your plan tier. The
editor flags anything that exceeds your limits before you apply it, and
multiple warehouses or warehouse capacity limits require a Pro tier license.

## Keep devices in sync

Devices pull the topology when they reconnect, so a new register appears on
every screen without manual setup.
