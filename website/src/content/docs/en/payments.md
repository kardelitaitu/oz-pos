---
title: Payments & QRIS
description: Accept cash, cards, and Indonesian QR payments.
category: guides
order: 3
updated: "2026-08-15"
---

## Payment methods

OZ-POS supports cash, card, and QRIS payments out of the box. Payment
gateway timeouts never block the sale — the transaction is recorded and
reconciled when the gateway responds.

## QRIS

Indonesian QR payments are native: the terminal displays a QR code for the
customer to scan, and the payment is matched back to the sale automatically.

## Refunds and voids

A refund requires manager permission and writes a matching stock movement, so
inventory and the audit log stay consistent.
