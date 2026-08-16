# ADR: Topology Phase 10 — Multi-Warehouse Stock Allocation

**Date:** 2026-08-09
**Status:** Implemented

## Problem

Phase 9 consumed only the first `stock-routing` wire from a POS workspace. Additional warehouse routes were ignored, forcing the cashier to enter split-fulfillment resolutions when the first warehouse lacked enough stock.

## Decision

A scoped POS completion now reads every distinct `stock-routing` target from the branch runtime plan in route order. Each target resolves to its primary inventory location, duplicate locations are removed while preserving the first route's priority, and the core sale-deduction transaction greedily fills each sale line from those locations.

The allocation contract is:

1. Prefer the first configured route.
2. Consume only the available quantity at that location.
3. Continue through later routes until the requested quantity is fulfilled.
4. Deduct all allocations atomically in one SQLite transaction.
5. Persist every location/quantity pair in `sales.deduction_locations` so refunds and voids restore the original sources.
6. Roll back the entire sale when the combined configured routes cannot fulfill a line.

Legacy callers and sales without topology routes retain the existing single-location resolver path. Cashier-resolved shortfalls remain available only for demand that exceeds the total configured route capacity or for legacy flows.

## Deliberate limitation

The cart API still exposes one `deduction_location_id` for compatibility and therefore displays the first route's location at cart start. Completion is the authoritative multi-location allocation boundary; a future UI slice can expose the full route plan on the cart and show the predicted split before payment.

## Verification

- Route-order allocation unit tests pass.
- Multi-location sale regression passes: 3 units from route A plus 5 from route B fulfill an 8-unit sale without cashier resolutions.
- All `db::sales::tests` pass (96 tests).
- Desktop runtime-plan route-order test passes.
- Rust formatting passes.
