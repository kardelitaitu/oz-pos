---
title: Payments & QRIS
description: Accept cash today — QRIS, cards, and e-wallets are coming soon.
category: guides
order: 3
updated: "2026-08-16"
---

## Payment methods

- **Cash** — available today. Enter the amount tendered and the change is
  calculated for you.
- **Debit** — coming soon.
- **Credit** — coming soon.
- **QRIS** — coming soon. Indonesian QR payments: the checkout will show a QR
  code for the customer to scan, matched back to the sale automatically.
- **E-wallet** — coming soon.

Debit, credit, and e-wallets follow the same pattern as QRIS: the sale is
recorded immediately and reconciled when the gateway responds, so a gateway
timeout never blocks the counter.

## Open bills

**Open Bill** is a choice in the payment screen that saves the cart *without*
taking payment, under a customer name — for example `John Doe` or a table.
Open bills are listed separately from held orders, are not tied to a shift,
and can be resumed and paid later — a running tab. When a bill is finally
paid, it is removed from the list.

## Hold orders

Cashiers can park the current sale without paying for it. **Hold** in the cart
panel opens a prompt to name the order so it can be found later, and the sale
leaves the screen with a counter showing how many orders are held. Resume any
held order from the held orders list (or press **F4**). Multiple holds can be
open at once, and they survive restarts and app updates.

Parking is for busy counters: ring up a customer, hold the sale, serve the
next one, and resume when the first customer is ready to pay.

## Refunds and voids

A refund requires manager permission and writes a matching stock movement, so
inventory and the audit log stay consistent.
