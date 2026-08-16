---
title: Terminals
description: Register and configure the devices that run OZ-POS.
category: guides
order: 8
updated: "2026-08-16"
---

## What a terminal is

Terminals — the registers you see in the topology — are the devices that run
OZ-POS: a counter register, a tablet, or a kitchen screen. Each terminal has
a name and a device identifier (hostname or MAC address) that the app reports
automatically. Managing terminals requires the manager role.

## Registering a terminal

Open the Terminals screen and register the device. Give it a readable name
("Front Counter") and the device identifier, and optionally a shared secret
for sync authentication and JSON metadata. Terminals can be deactivated or
deleted later; deleting is permanent.

## Feature overrides

By default a terminal inherits every feature your plan enables. Overrides
force a feature on or off for one device only. Overrides are grouped the way
the app organizes them:

- **Sales** — retail, restaurant, discount and tax engines, promotions,
  product bundles, loyalty, kitchen display, and table management
- **Payments** — cash, card, and multi-currency
- **Inventory & Products** — inventory tracking, product variants,
  categories, and barcode scanning
- **Hardware** — receipt printing, cash drawer, customer display, and NFC
  reader
- **Staff & Security** and **System**

Typical use: disable card payments on a self-service kiosk, or turn the
kitchen display on for a single screen. Reset all overrides to return a
terminal to plan defaults.

## Terminal preferences

Each terminal keeps its own preferences: **sound volume**, **dark mode**, and
**auto-zero the weight scale on boot**. These follow the device, not the
logged-in user, so a counter and a kitchen screen can each behave the way
their spot needs.

## Device binding

Bind a terminal to a store and a workspace instance so the device boots
straight into that screen instead of the picker — a kitchen screen that is
always the Kitchen Display, a counter that is always the Store POS. Clearing
the binding returns the device to the workspace picker.

## Terminal status

The multi-store dashboard tracks **active**, **online**, and **total**
terminals and shows terminal status per store, so you can see at a glance
which devices are up and working. Devices report in when they reconnect, and
a terminal that has been offline shows up here before it causes a surprise
at the counter.

## Terminals in the topology

Terminals appear in the topology editor alongside your stores and
warehouses, and the layout syncs to every device on reconnect. See
[Stores & Topology](../stores/) and [Workspaces](../workspaces/).
