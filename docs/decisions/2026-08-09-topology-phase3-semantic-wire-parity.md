# ADR: Topology Phase 3 — Semantic Wire Validation Parity

**Date:** 2026-08-09
**Status:** Implemented

## Problem

The editor's typed pairing table constrained newly authored wires, but the Rust Apply boundary validated only Branch Location ownership and KDS operation ownership. A caller could submit a semantically incompatible stock, transfer, ticket, or hardware wire directly to the command and persist it. The frontend also lacked validation for forged wires whose port pair was legal but whose node types could not produce or consume those ports.

## Decision

Validate every non-location semantic wire at both boundaries:

- The frontend reuses the same pairing matrix as drag gating and relationship selection.
- Node capabilities are checked in addition to port ids:
  - stock and transfer feeds terminate at a warehouse;
  - ticket feeds originate at KDS and terminate at hardware;
  - operation feeds remain Restaurant POS → KDS;
  - hardware connections remain hardware → hardware.
- The Rust Apply boundary mirrors those rules for direct IPC callers.
- Location wires retain their specialized ownership and cardinality validation.
- KDS operation wires retain the dedicated `invalid-operation-source` error so the user receives precise guidance.
- The future-facing `generic-out → generic-in` pair remains allowed by the closed semantic matrix, even though no current node emits it.

## Verification

- Frontend tests cover matrix compatibility, invalid port/relationship combinations, invalid ticket endpoints, and valid stock routing.
- Rust tests cover invalid semantic pairs, invalid ticket producers, valid stock routing, and valid Restaurant POS → KDS operation feeds.
- English and Indonesian validation bundles include the incompatible-connection message.
- `npm run test -- src/__tests__/topologyCard.test.ts src/__tests__/topologyContract.test.ts`
- `cargo test -p oz-pos-app semantic_save_rejects_mismatched_non_location_wire --lib`
- `cargo test -p oz-pos-app semantic_save_rejects_ticket_wire_from_non_kds_workspace --lib`
- `cargo test -p oz-pos-app semantic_save_accepts_kds_operation_feed_from_restaurant_pos --lib`
- `npm run typecheck`
- `npm run lint -- --quiet`
