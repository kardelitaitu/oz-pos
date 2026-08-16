---
title: Inventory & Warehouses
description: Track stock across warehouses with movement history.
category: guides
order: 5
updated: "2026-08-16"
---

## Stock levels

Stock is tracked per product per location (warehouse or register). Sales
decrement stock automatically, and each register serves from the location it
is assigned to. A location picker switches the current view, so levels are
always read in context.

## Adjustments

Stock adjustments run as a two-step flow: pick the product, then choose a
reason — **Restock** (supplier delivery), **Stock take correction**, **Customer
return**, **Damaged / spoiled**, **Write-off / expiry**, **Transfer to other
location**, or a custom reason — and enter the change. Every adjustment writes
a movement ledger entry, so any change can be traced back to who did it, when,
and why.

## Stock counts

A stock count reconciles the system against what is physically on the shelf.
Start an **inventory shift** (for example `Night shift count`), count, and the
corrections are recorded against the shift. Counts are listed with status
filters, and each one opens a detail view with its history, so a discrepancy
found later is still explainable.

## Thresholds and alerts

Low-stock alerts flag products below their threshold. Thresholds are
configured per location, with a **Global (All Locations)** fallback for
products that have no location-specific setting, and each threshold can be
enabled or disabled independently.

## Transfers and transit

Stock moves between locations as recorded transfers. In-transit items are
audited with their source, destination, quantity, and send time; overdue
transit is flagged so nothing is lost between shelves. A mistaken transfer can
be **reversed**, returning the stock to its source location.

## Purchase orders

Restocking via suppliers goes through purchase orders: manage suppliers, create
an order with a supplier and order date, and **Receive** it when the delivery
arrives — the received quantities land in stock automatically.

## Reports and the movement ledger

The **Inventory Report** shows stock, threshold, unit price and cost, margin,
and stock value per product, and can be printed or exported as CSV. The
**Inventory Transaction Log** lists every movement — transfers, stock counts,
and manual adjustments — as a single ledger of where stock came from and went.
