# ADR: Topology Phase 5 — Directed Cycle Validation

**Date:** 2026-08-09
**Status:** Implemented

## Problem

The topology contract validated ownership, semantic pairings, and node capabilities, but it allowed a directed operational cycle to reach Apply. A cycle makes route compilation ambiguous and can cause runtime adapters to repeatedly forward the same work between workspace instances.

## Decision

Both validation boundaries reject directed graph cycles:

- The frontend runs a deterministic depth-first cycle check for immediate editor feedback.
- The Rust Apply boundary runs an independent Kahn topological check so direct IPC callers cannot bypass the rule.
- The graph uses persisted `from_node_id → to_node_id` semantics; wire direction markers remain presentation-only as defined by the existing contract.
- The error identifies a node involved in the cycle and uses the localized `cycle-detected` message.
- Missing endpoints are handled by the existing unknown-endpoint validator and do not create false cycle reports.

## Verification

- Frontend topology contract test rejects a two-node operational cycle.
- Rust semantic save test rejects the same cycle at the Apply boundary.
- Existing topology contract tests remain green.
- Rust formatting, i18n lint, and diff checks pass.
