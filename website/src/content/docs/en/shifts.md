---
title: Shifts & Reconciliation
description: Close cashier shifts cleanly with a full audit trail.
category: guides
order: 4
updated: "2026-08-16"
---

## Opening a shift

A cashier opens a shift on a register before serving customers. The **Open
Shift** dialog accepts an optional **opening balance** — the float in the
drawer at the start of the day, for example `100.00`. Only that cashier's
sales count toward their shift, and a live clock shows how long the shift has
been running — it stays anchored to the original opening time, so a restart
or app update never resets it. Only one shift is open on a register at a
time.

## Cash payouts

Money can leave the drawer mid-shift without closing it — for example a safe
drop. **Record Payout** takes an amount and a reason (defaulting to
`safe drop`), and the payout is subtracted from the expected cash so the
reconciliation at close stays accurate.

## Closing and reconciling

**Close Shift** takes the **counted** cash in the drawer and any optional
notes. It shows the expected total versus what was counted and flags the
**difference** — tagged **Over** or **Short** — before the register accepts
the close, so discrepancies surface at the counter instead of at the end of
the month. The close is refused while a sale is still in progress; complete
or clear it first. A summary of the closed shift is shown immediately.

## Shift history and end of day

The shift management screen lists every shift with its status, open and close
times, opening and counted balances, expected cash, difference, and sales,
and opens a full per-shift report. The **End-of-Day Report** rolls up today's
shifts: KPI cards (total revenue, average sale, voids, discounts), a cash
reconciliation (total opening vs total counted vs total expected, with a net
difference), a payment breakdown, and sales by hour — printable and
exportable.

## Audit history

Every sale, void, refund, payout, and stock adjustment is recorded with the
user and terminal that made it, so every shift reconciles back to a complete
audit trail.
