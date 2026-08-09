# ADR: Topology Phase 2 — KDS Operation Source Parity

**Date:** 2026-08-09
**Status:** Implemented

## Problem

The editor pairing table allowed `location-out → operation-in`, and both frontend and backend validation accepted any `generic → operation-in` wire for a KDS. This allowed a Branch Location or a non-restaurant workspace to appear operationally connected to a KDS, even though the intended contract is Restaurant POS `operation-out → operation-in`.

## Decision

A KDS Operation In connection is valid only when:

- the wire relationship is `generic`;
- the source port is `operation-out`; and
- the source workspace has `typeKey = restaurant-pos`.

The Branch Location pairing to `operation-in` is removed. Both the pure frontend contract and the Rust Apply boundary report `invalid-operation-source` for an invalid source, while preserving the existing missing and multiple input errors.

## Verification

- Frontend semantic contract and pairing tests cover invalid sources.
- Rust semantic save tests cover invalid and valid Restaurant POS sources.
- English and Indonesian validation messages remain bundle-parity complete.
- `npm run test -- src/__tests__/topologyCard.test.ts src/__tests__/topologyContract.test.ts`
- `cargo test -p oz-pos-app semantic_save_rejects_operation_feed_from_non_restaurant_pos --lib`
- `cargo test -p oz-pos-app semantic_save_accepts_kds_operation_feed_from_restaurant_pos --lib`
- `npm run typecheck`
- `npm run lint -- --quiet`
