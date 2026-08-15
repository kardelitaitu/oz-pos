---
title: Offline-First Mode
description: How OZ-POS keeps working with zero connectivity.
category: guides
order: 1
updated: "2026-08-15"
---

## Nothing stops at the counter

Sales, shifts, stock movements, and settings changes all write to the local
database first. A lost connection never blocks a transaction.

## The offline queue

Every change is appended to an outbound queue. When connectivity returns, the
queue drains in order and the server acknowledges each item.

## Conflicts

Because each register works on its own local data and merges are order-based,
conflicts are rare and resolve deterministically — the latest change wins for
each record.
