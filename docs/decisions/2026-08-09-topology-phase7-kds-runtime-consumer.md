# ADR: Topology Phase 7 — KDS Runtime Consumer

**Date:** 2026-08-09
**Status:** Implemented

## Problem

Phase 4 compiled Restaurant POS → KDS operation wires into a branch-scoped runtime plan, but the KDS creation path ignored that artifact. A ticket could therefore be created without recording which KDS workspace instance the topology selected, and every scoped KDS board could see it.

## Decision

The scoped KDS sale command consumes the runtime plan for the active branch and POS workspace instance. When it finds a validated `operation-out → operation-in` route, it persists the route's target workspace instance on the KDS order as `target_instance_id`.

Scoped KDS list and queue reads retain legacy tickets with a null target, while targeted tickets are visible only to the matching KDS session instance. This preserves existing deployments during migration without allowing a newly routed ticket to leak to another KDS instance.

The target is stored in the store database, while the runtime plan remains in the global settings database. The branch key is the session's resolved `store_id`, matching the topology branch identity established in Phase 1.

## Deliberate limitation

The current `kds_orders.sale_id` uniqueness constraint supports one routed KDS target per sale. The consumer deterministically selects the first matching operation route; topology fan-out needs a separate schema and delivery design before it is enabled.

## Verification

- Red test reproduced the missing `target_instance_id` column before the migration.
- Core routed-sale test confirms target persistence.
- Desktop runtime-plan selector test confirms source/port/relationship matching.
- UI typecheck and IPC contract tests pass.
- Rust formatting and diff checks pass.
