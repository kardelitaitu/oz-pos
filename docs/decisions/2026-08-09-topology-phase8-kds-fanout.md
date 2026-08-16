# ADR: Topology Phase 8 — KDS Fan-Out

**Date:** 2026-08-09
**Status:** Implemented

## Problem

Phase 7 stored one `target_instance_id` on each KDS order. That made a Restaurant POS → multiple KDS topology ambiguous: duplicating `kds_orders` would violate the existing unique `sale_id` constraint, while choosing only one display silently dropped a valid route.

## Decision

Keep one `kds_orders` row per sale and kitchen zone, and normalize delivery targets in:

```text
kds_order_targets(kds_order_id, target_instance_id)
```

The composite primary key makes target attachment idempotent. The legacy `target_instance_id` column remains populated with the first target for backward-compatible API consumers and old rows.

The scoped sale command now consumes every distinct validated POS operation route. Each target KDS instance sees the same ticket through instance-aware list/queue queries; an unrelated KDS instance does not. Legacy orders without target rows remain visible to all KDS instances during migration.

Migration 124 backfills normalized target rows from Phase 7's single-target column, so upgrades do not lose existing routing.

## Invariants

- One sale/zone creates one `kds_orders` row.
- A sale can have zero, one, or many delivery targets.
- Duplicate runtime routes do not create duplicate target rows.
- Removing all topology routes creates an un-targeted legacy-compatible ticket; future work may choose to suppress such tickets entirely.

## Verification

- Red test reproduced the missing `kds_order_targets` table.
- Core fan-out regression confirms one order, two target rows, and visibility from both target instances.
- Runtime-plan test confirms distinct target collection and duplicate suppression.
- Existing single-target and KDS tests remain compatible.
