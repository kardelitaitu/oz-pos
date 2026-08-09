# ADR: Topology Phase 1 — Branch-Scoped Persistence

**Date:** 2026-08-09
**Status:** Implemented

## Problem

Topology diagrams were persisted under one global `oz-pos/topology` settings key. Selecting a different Branch Location changed the editor view, but saving one branch could overwrite the diagram for every other branch.

## Decision

Branch-aware topology commands derive a dedicated settings key:

```text
oz-pos/topology/<branch-id>
```

The UI passes the active branch through load and Apply IPC calls. The backend validates the branch identifier, persists the diagram under that branch key, and rejects an Apply when the requested branch differs from the semantic Branch Location in the graph.

The old unscoped key remains available for compatibility with legacy callers. A legacy diagram is read for a branch only when its canonical `store_profile_id` proves that it belongs to that branch; ambiguous geometry is never copied into a branch by guesswork.

Apply recovery journals the branch identity so compensation restores the same branch-specific key after a failed cross-database mutation.

## Verification

- UI IPC contract tests cover branch arguments.
- The editor passes the active branch to topology loading.
- TopologyScreen passes the selected branch to Apply.
- Rust unit tests cover key isolation, invalid key characters, legacy branch matching, and the Tauri command round trip.
- `cargo test -p oz-pos-app commands::topology::tests --lib`
- `npm run test -- src/__tests__/api-ipc-contract.test.ts src/__tests__/TopologyScreen.test.tsx`
- `npm run typecheck`
- `npm run lint -- --quiet`
