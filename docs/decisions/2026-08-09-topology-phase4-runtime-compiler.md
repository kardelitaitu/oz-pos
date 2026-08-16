# ADR: Topology Phase 4 — Runtime Route Compiler

**Date:** 2026-08-09
**Status:** Implemented

## Problem

The topology Apply flow persisted semantic wires only inside the editor diagram. Runtime code had no stable artifact containing operational routes, so a valid stock, transfer, ticket, hardware, or Restaurant POS → KDS connection could be visually correct while runtime adapters had nothing to consume.

## Decision

Every topology save now compiles non-location semantic wires into a branch-scoped runtime plan:

```text
oz-pos/topology-runtime/<branch-id>
```

The unscoped compatibility path is `oz-pos/topology-runtime`. Each route stores only stable runtime fields:

- wire ID;
- source workspace/instance ID;
- target workspace/instance ID;
- source and target semantic port IDs;
- relationship type.

Canvas coordinates, display names, labels, and geometric anchors are intentionally excluded. The diagram and runtime plan are written in the same SQLite transaction, and saving an empty operational graph replaces the prior plan with an empty route list so removed wires do not remain active in the runtime artifact.

The compiler runs after semantic and structural validation, so direct IPC callers cannot inject invalid runtime routes. Branch-scoped keys preserve the Phase 1 isolation guarantee.

## Follow-up

Runtime adapters still need to consume the compiled plan. The next runtime slice should connect one consumer, beginning with KDS ticket target selection or inventory route resolution, and add an end-to-end behavior test.

## Verification

- Runtime-plan compilation test covers Restaurant POS → KDS operation routing.
- Branch isolation test covers separate runtime keys for separate branches.
- Rust topology tests and formatting pass.
