# ADR: Topology Phase 9 — Stock Routing Consumer

**Date:** 2026-08-09
**Status:** Implemented

## Problem

The topology compiler already emitted `stock-out → stock-in` routes into the branch runtime plan, but scoped POS sale completion still resolved stock from the POS workspace instance. A connected Warehouse node therefore had no effect on deduction or the cart's locked location.

## Decision

Scoped POS commands consume the branch runtime plan using the active POS instance as the source. The first validated `stock-routing` route selects a Warehouse workspace instance. That instance is passed to the existing location resolver, so its bound or primary inventory location becomes the sale's deduction location.

The selection is applied at both points that establish the deduction contract:

- `start_sale_scoped` locks the route-selected location on the active cart;
- scoped sale completion and resolved-shortfall completion use the route-selected workspace for stock checks, deductions, alternatives, and audit JSON.

When no stock route exists, the existing POS-instance resolution and legacy canonical-default fallback remain unchanged.

A missing or invalid route target fails before cart creation or cart deletion rather than silently falling back to the default location.

## Deliberate limitation

The current sale deduction model has one primary location per sale. If multiple stock routes are connected, the first runtime route is selected deterministically; cashier-driven split fulfillment remains available through the existing shortfall-resolution flow. A future phase can define topology-native multi-warehouse allocation.

## Verification

- Red test confirmed the runtime stock consumer was absent.
- Runtime-plan regression confirms `stock-routing` source/target matching.
- Rust formatting and desktop focused compilation/tests pass.
- UI typecheck remains subject to unrelated topology-export changes in the working tree.
